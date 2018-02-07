use std::collections::HashSet;
use std::fmt;
use std::vec::Vec;

use crypto::digest::Digest;
use crypto::sha3::Sha3;
use git2::{self, Oid, Reference, Repository, Remote, Revwalk, BranchType, Commit};
use git2::Error;

use common::{self, Nonce, HasNonce};
use common::{NonceBag, HasNonceBag};
use common::PushEntry;
use common::errors::*;

const RSL_BRANCH: &'static str = "RSL";
const REFLOG_MSG: &'static str = "Retrieve RSL branchs from remote";


#[derive(Debug)]
pub enum RSLType {
    Local,
    Remote,
}

#[derive(Debug)]
pub struct RSL {
    pub kind: RSLType,
    //remote: &'repo Remote,
    pub head: Oid,
    pub last_push_entry: Option<PushEntry>,
}

impl RSL {

    fn find_first_commit(repo: &Repository) -> Result<Commit> {
        let mut revwalk: Revwalk = repo.revwalk().chain_err(|| "Failed to make revwalk")?;
        revwalk.push_head();
        let result = revwalk.last()
            .ok_or("Couldn't find commit")?
            .chain_err(|| "revwalk returned bad commit")?;
        repo.find_commit(result).chain_err(|| "could not find first commit")
    }

}

pub trait HasRSL {
    fn read_rsl(&self) -> Result<(RSL, RSL, NonceBag, Nonce)>;
    fn read_local_rsl(&self) -> Result<RSL>;
    fn read_remote_rsl(&self) -> Result<RSL>;
    fn init_rsl_if_needed(&self, remote: &mut Remote) -> Result<()>;
    fn rsl_init_global(&self, remote: &mut Remote) -> Result<()>;
    fn rsl_init_local(&self) -> Result<()>;
    fn fetch_rsl(&self, remote: &mut Remote) -> Result<()>;
    fn commit_push_entry(&self, push_entry: &PushEntry) -> Result<Oid>;
    fn push_rsl(&self, remote: &mut Remote) -> Result<()>;
    fn find_last_push_entry(&self, tree_tip: &Oid) -> Option<PushEntry>;

}

impl HasRSL for Repository {

    fn find_last_push_entry(&self, tree_tip: &Oid) -> Option<PushEntry> {
        let mut revwalk: Revwalk = self.revwalk().expect("Failed to make revwalk");
        revwalk.push(tree_tip.clone());
        //revwalk.set_sorting(git2::SORT_REVERSE);
        let last_push_entry: Option<PushEntry> = None;
        let mut current = Some(tree_tip.clone());
        while current != None {
            match PushEntry::from_oid(self, &current.unwrap()){
                Some(pe) => return Some(pe),
                None => (),
            }
            current = revwalk.next().and_then(|res| res.ok()); // .next returns Opt<Res<Oid>>
        }
        None
    }

    fn rsl_init_global(&self, remote: &mut Remote) -> Result<()> {
        println!("Initializing Reference State Log for this repository.");
        // TODO
        // make new parentless commit, with your nonce bag in it
        ///push it to a new branch called RSL.
        // if the push is successful,
        // then fetch the remote and do the verification routine and ff it to local....?
        // which if verification sees you have no local RSL branch it just lets you go ahead and fast forward? Or should it already exist?

        // TODO: figure out a way to orphan branch; .branch() needs a commit ref. For now, find first commit and use that as ancestor for RSL
        // Update: this is highly possible with the flexibility of git2rust. Just need to make a commit with no parent and then give it the name of a nonexistent rsl ref as the head to update and it will create the branch automatically
        let initial_commit = RSL::find_first_commit(self).chain_err(|| "couldn't find first commit")?;

        // create new RSL branch
        // TODO try if let macro
        let rsl_ref = self.branch(RSL_BRANCH, &initial_commit, false).chain_err(|| "coudln't find rsl branch")?.get().target().chain_err(|| "couldn't find rsl head oid")?;
        // create new RSL
        let local_rsl = RSL {
            kind: RSLType::Local,
            //remote: remote,
            head: rsl_ref,
            last_push_entry: None,
        };
        common::checkout_branch(self, "RSL")?;


        // save random nonce locally
        let nonce = Nonce::new()?;
        self.write_nonce(&nonce).chain_err(|| "couldn't write local nonce")?;

        // create new nonce bag with initial nonce
        let mut nonce_bag = NonceBag::new();
        nonce_bag.insert(nonce).chain_err(|| "couldn't add new nonce to bag")?;
        self.write_nonce_bag(&nonce_bag)?;
        self.commit_nonce_bag()?;

        // push new rsl branch
        self.push_rsl(remote)?;

        // put this in a loop ? with a max try timeout
        self.fetch_rsl(remote)?;

        let remote_rsl = self.read_remote_rsl()?;

        Ok(())

    }

    fn rsl_init_local(&self) -> Result<()> {
        // TODO implement
        Ok(())
    }

    fn read_rsl(&self) -> Result<(RSL, RSL, NonceBag, Nonce)> {
        let remote_rsl = self.read_remote_rsl().chain_err(|| "remote rsl read error")?;
        let local_rsl = self.read_local_rsl().chain_err(|| "local rsl read error")?;
        let nonce_bag = self.read_nonce_bag().chain_err(|| "nonce bag read error")?;
        let nonce = self.read_nonce().chain_err(|| "nonce read error")?;
        Ok((remote_rsl, local_rsl, nonce_bag, nonce))
    }

    fn read_local_rsl(&self) -> Result<RSL> {
        let kind = RSLType::Local;
        let branch = self.find_branch(RSL_BRANCH, BranchType::Local).chain_err(|| "couldnt find RSL branch")?;
        let reference = branch.into_reference();
        let head = reference.target().ok_or("could not find RSL branch tip OID")?;
        let last_push_entry = self.find_last_push_entry(&head);
        Ok(RSL {kind, head, last_push_entry})
    }

    fn read_remote_rsl(&self) -> Result<RSL> {
        let kind = RSLType::Remote;
        let branch = self.find_branch("origin/RSL", BranchType::Remote).chain_err(|| "could not find RSL branch")?;
        let reference = branch.into_reference();
        let head = reference.target().ok_or("could not find head reference")?;
        let last_push_entry = self.find_last_push_entry(&head);
        Ok(RSL {kind, head, last_push_entry})
    }

    fn commit_push_entry(&self, push_entry: &PushEntry) -> Result<Oid> {
        let mut index = self.index().chain_err(|| "could not find index")?;
        //index.add_path(self.path().join("NONCE_BAG"))?;
        let oid = index.write_tree().chain_err(|| "could not write tree")?;
        let signature = self.signature().unwrap();
        let message = push_entry.to_string();
        let parent_commit_ref = self.find_branch(RSL_BRANCH, BranchType::Local).chain_err(|| "RSL Branch not found: {:?}")?;
        let parent_commit = parent_commit_ref.get().peel_to_commit().chain_err(|| "could not find parent commit")?;
        let tree = self.find_tree(oid).chain_err(|| "could not find tree")?;
        let rsl_head = format!("refs/heads/{}", RSL_BRANCH);

        self.commit(
            Some(&rsl_head), //  point HEAD to our new commit
            &signature, // author
            &signature, // committer
            &message, // commit message
            &tree, // tree
            &[&parent_commit]
        ).chain_err(|| "could not commit push entry")
    }


    fn fetch_rsl(&self, remote: &mut Remote) -> Result<()> {
        // not sure the behavior here if the branch doesn't exist
        // should return Some(()) or Some(Reference) if remote exists and None if it doesn't exist and Err if it failed for some other reason.
        common::fetch(self, remote, &[RSL_BRANCH], Some(REFLOG_MSG)).chain_err(|| "could not fetch RSL")?;
        Ok(())
    }

    fn init_rsl_if_needed(&self, remote: &mut Remote) -> Result<()> {
        // validate that RSL does not exist locally or remotely
        match (self.find_branch(RSL_BRANCH, BranchType::Remote), self.find_branch(RSL_BRANCH, BranchType::Local)) {
            (Err(_), Err(_)) => {self.rsl_init_global(remote).chain_err(|| "could not initialize remote RSL")?;
                                Ok(())}, // first use of git-rsl for repo
            (Ok(_), Err(_)) => {self.rsl_init_local().chain_err(|| "could not initialize loxal rsl")?;
                                Ok(())}, // first use of git-rsl for this developer in this repo
            (Err(_), Ok(_)) => bail!("RSL exists locally but not globally"), // local exists but global not found
            (Ok(_), Ok(_)) => Ok(()), // RSL already set up
        }
    }

    fn push_rsl(&self, remote: &mut Remote) -> Result<()> {
        println!("gets here : )");
        common::push(self, remote, &[RSL_BRANCH]).chain_err(|| "could not push rsl");
        Ok(())
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::test_helper::*;

    #[test]
    fn rsl_init() {
        let mut context = setup();
        {
            context.without_rsl();
            let mut remote = context.local.find_remote("origin").unwrap().to_owned();
            let result = &context.local.init_rsl_if_needed(&mut remote).unwrap();
            assert_eq!(result, &()); // returns successfully
            // local rsl branch exists
            // local nonce exists
            // remote rsl branch exists
        }
        teardown(context);
    }

    #[test]
    fn commit_push_entry() {
        let mut context = setup();
        context.checkout("RSL");
        {
            let repo = &context.local;
            let entry = PushEntry {
                    //related_commits: vec![oid.to_owned(), oid.to_owned()],
                    branch: String::from("branch_name"),
                    head: repo.head().unwrap().target().unwrap(),
                    prev_hash: String::from("hash_of_last_pushentry"),
                    nonce_bag: NonceBag::new(),
                    signature: String::from("gpg signature"),
            };
            let oid = repo.commit_push_entry(&entry).unwrap();
            let obj = repo.find_commit(oid).unwrap();
            let new_head = repo.find_branch("RSL", BranchType::Local).unwrap();
            assert_eq!(oid, new_head.into_reference().target().unwrap());
            assert_eq!(&obj.message().unwrap(), &"hello");
        }
        teardown(context);
    }
}
