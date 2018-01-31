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

use common::nonce::NonceError;
use common::nonce_bag::NonceBagError;



const RSL_BRANCH: &'static str = "RSL";
const REFLOG_MSG: &'static str = "Retrieve RSL branchs from remote";

#[derive(Debug)]
pub enum RSLError {
    Problem(),
    GitError(git2::Error),
    NonceError(NonceError),
    NonceBagError(NonceBagError),
}

impl From<git2::Error> for RSLError {
    fn from(error: git2::Error) -> Self {
        RSLError::GitError(error)
    }
}

impl From<common::nonce_bag::NonceBagError> for RSLError {
    fn from(error: NonceBagError) -> Self {
        RSLError::NonceBagError(error)
    }
}

impl From<common::nonce::NonceError> for RSLError {
    fn from(error: NonceError) -> Self {
        RSLError::NonceError(error)
    }
}

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

    fn find_first_commit(repo: &Repository) -> Result<Commit, git2::Error> {
        let mut revwalk: Revwalk = repo.revwalk().expect("Failed to make revwalk");
        revwalk.push_head();
        // Result<oid> || Result<option>
        let result = match revwalk.last() { // option<Oid>
            Some(r) => r, // option<result<oid, err>> => result<oid, err>
            None => Err(git2::Error::from_str("Couldn't find commit")), // option
        };
        match result { // result = result<oid>
            Ok(r) => repo.find_commit(r), // result<oid> => Result<commit, error>
            Err(e) => Err(e) // result<error> => result<error>
        }
    }

}

pub trait HasRSL {
    fn read_rsl(&self) -> Result<(RSL, RSL, NonceBag, Nonce), RSLError>;
    fn read_local_rsl(&self) -> Result<RSL, RSLError>;
    fn read_remote_rsl(&self) -> Result<RSL, RSLError>;
    fn init_rsl_if_needed(&self, remote: &mut Remote) -> Result<(), RSLError>;
    fn rsl_init_global(&self, remote: &mut Remote) -> Result<(), RSLError>;
    fn rsl_init_local(&self) -> Result<(), RSLError>;
    fn fetch_rsl(&self, remote: &mut Remote) -> Result<(),
     RSLError>;
    fn commit_push_entry(&self, push_entry: &PushEntry) -> Result<Oid, RSLError>;
    fn push_rsl(&self, remote: &mut Remote) -> Result<(), RSLError>;
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

    fn rsl_init_global(&self, remote: &mut Remote) -> Result<(), RSLError> {
        println!("Initializing Reference State Log for this repository.");
        // TODO
        // make new parentless commit, with your nonce bag in it
        ///push it to a new branch called RSL.
        // if the push is successful,
        // then fetch the remote and do the verification routine and ff it to local....?
        // which if verification sees you have no local RSL branch it just lets you go ahead and fast forward? Or should it already exist?

        // TODO: figure out a way to orphan branch; .branch() needs a commit ref. For now, find first commit and use that as ancestor for RSL
        // Update: this is highly possible with the flexibility of git2rust. Just need to make a commit with no parent and then give it the name of a nonexistent rsl ref as the head to update and it will create the branch automatically
        let initial_commit = match RSL::find_first_commit(self) {
            Ok(r) => r,
            Err(e) => return Err(RSLError::Problem()),
        };

        // create new RSL branch
        let rsl_ref = match self.branch(RSL_BRANCH, &initial_commit, false) {
            Ok(branch) => branch.get().target().unwrap(), // this unwrap is ok I think
            Err(e) => return Err(RSLError::Problem()),
        };

        // create new RSL
        let local_rsl = RSL {
            kind: RSLType::Local,
            //remote: remote,
            head: rsl_ref,
            last_push_entry: None,
        };

        // save random nonce locally
        let nonce = Nonce::new()?;
        self.write_nonce(&nonce)?;

        // create new nonce bag with initial nonce
        let mut nonce_bag = NonceBag::new();
        nonce_bag.insert(nonce)?;

        //  nonce bag (inlcuding commit)
        self.write_nonce_bag(&nonce_bag)?;
        self.commit_nonce_bag().expect("couldn't commit nonce bag");

        // push new rsl branch
        self.push_rsl(remote)?;

        // put this in a loop ? with a max try timeout
        match self.fetch_rsl(remote) {
            Ok(()) => (),
            Err(e) => return Err(e)
        };

        let remote_rsl = match self.read_remote_rsl() {
            Ok(rsl) => rsl,
            Err(e) => return Err(RSLError::Problem()),
        };

        Ok(())

    }

    fn rsl_init_local(&self) -> Result<(), RSLError> {
        // TODO implement
        Ok(())
    }

    fn read_rsl(&self) -> Result<(RSL, RSL, NonceBag, Nonce), RSLError> {
        let remote_rsl = match self.read_remote_rsl() {
            Ok(rsl) => rsl,
            Err(e) => return Err(RSLError::Problem())
        };
        let local_rsl = match self.read_local_rsl() {
            Ok(rsl) => rsl,
            Err(e) => return Err(RSLError::Problem())
        };
        let nonce_bag = match self.read_nonce_bag() {
            Ok(nb) => nb,
            Err(e) => return Err(RSLError::Problem())
        };
        let nonce = match self.read_nonce() {
            Ok(n) => n,
            Err(e) => return Err(RSLError::Problem()),
        };
        Ok((remote_rsl, local_rsl, nonce_bag, nonce))
    }

    fn read_local_rsl(&self) -> Result<RSL, RSLError> {
        let kind = RSLType::Local;
        let reference = match self.find_branch(RSL_BRANCH, BranchType::Local) {
                Err(e) => return Err(RSLError::Problem()),
                Ok(rsl) => (rsl.into_reference()),
        };
        let head = match reference.target() {
            Some(oid) => oid,
            None => return Err(RSLError::Problem()),
        };
        let last_push_entry = self.find_last_push_entry(&head);
        Ok(RSL {kind, head, last_push_entry})
    }

    fn read_remote_rsl(&self) -> Result<RSL, RSLError> {
        let kind = RSLType::Remote;
        let branch = self.find_branch(RSL_BRANCH, BranchType::Remote)?;
        let reference = branch.into_reference();

        let head = match reference.target() {
            Some(oid) => oid,
            None => return Err(RSLError::Problem()),
        };
        let last_push_entry = self.find_last_push_entry(&head);
        Ok(RSL {kind, head, last_push_entry})
    }

    fn commit_push_entry(&self, push_entry: &PushEntry) -> Result<Oid, RSLError> {
        let mut index = self.index()?;
        //index.add_path(self.path().join("NONCE_BAG"))?;
        let oid = index.write_tree()?;
        let signature = self.signature().unwrap();
        let message = push_entry.to_string();
        let parent_commit_ref = match self.find_branch(RSL_BRANCH, BranchType::Local) {
            Ok(r) => r,
            Err(e) => panic!("RSL Branch not found: {:?}", e),
        };
        let parent_commit = match parent_commit_ref.get().peel_to_commit() {
            Ok(c) => c,
            Err(e) => return Err(RSLError::GitError(e)),
        };
        let tree = self.find_tree(oid)?;
        let rsl_head = format!("refs/heads/{}", RSL_BRANCH);

        match self.commit(
            Some(&rsl_head), //  point HEAD to our new commit
            &signature, // author
            &signature, // committer
            &message, // commit message
            &tree, // tree
            &[&parent_commit]
        ) {
            Ok(oid) => Ok(oid),
            Err(e) => return Err(RSLError::GitError(e)),
        }
    }


    fn fetch_rsl(&self, remote: &mut Remote) -> Result<(), RSLError> {
        // not sure the behavior here if the branch doesn't exist
        // should return Some(()) or Some(Reference) if remote exists and None if it doesn't exist and Err if it failed for some other reason.
        common::fetch(self, remote, &[RSL_BRANCH], Some(REFLOG_MSG))?;
        Ok(())
    }

    fn init_rsl_if_needed(&self, remote: &mut Remote) -> Result<(), RSLError> {
        // validate that RSL does not exist locally or remotely
        match (self.find_branch(RSL_BRANCH, BranchType::Remote), self.find_branch(RSL_BRANCH, BranchType::Local)) {
            (Err(_), Err(_)) => {self.rsl_init_global(remote)?;
                                Ok(())}, // first use of git-rsl for repo
            (Ok(_), Err(_)) => {self.rsl_init_local()?;
                                Ok(())}, // first use of git-rsl for this developer in this repo
            (Err(_), Ok(_)) => Err(RSLError::Problem()), // local exists but global not found
            (Ok(_), Ok(_)) => Ok(()), // RSL already set up
        }
    }

    fn push_rsl(&self, remote: &mut Remote) -> Result<(), RSLError> {
        println!("gets here : )");
        common::push(self, remote, &[RSL_BRANCH])?;
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
        context.without_rsl();
        let mut remote = context.local.find_remote("origin").unwrap();
        let result = context.local.init_rsl_if_needed(&mut remote).unwrap();
        assert_eq!(result, ()); // returns successfully
        // local rsl branch exists
        // local nonce exists
        // remote rsl branch exists
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
