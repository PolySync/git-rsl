use std::collections::HashSet;
use std::fmt;
use std::vec::Vec;

use crypto::digest::Digest;
use crypto::sha3::Sha3;
use git2::{Oid, Reference, Repository, Remote, Revwalk, BranchType};

use common::Nonce;
use common::NonceBag;
use common::PushEntry;

const RSL_BRANCH: &'static str = "RSL";
const REFLOG_MSG: &'static str = "Retrieve RSL branchs from remote";

#[derive(Debug)]
pub enum RSLError {
    Problem(),
}


#[derive(Debug)]
pub enum RSLType {
    Local(LocalRSL),
    Remote(RemoteRSL),
}

#[derive(Debug)]
pub struct RSL {
    kind: RSLType,
    //remote: Remote,
    head: Oid,
    last_push_entry: Option<PushEntry>,
}

impl RSL {

}

pub trait HasRSL {
    fn read_rsl(&self) -> Result<(RSL, RSL, NonceBag, Nonce), RSLError>;
    fn read_local_rsl(&self) -> Result<RSL, RSLError>;
    fn read_remote_rsl(&self) -> Result<RSL, RSLError>;
    fn init_rsl_if_needed(&self) -> Result<(RSL, RSL, NonceBag, Nonce), RSLError>;
    fn rsl_init<'repo>(repo: &'repo Repository, remote: &mut Remote) -> Result<(&'repo RSL, &'repo RSL, &'repo NonceBag, &'repo Nonce), RSLError>;
    fn fetch_rsl<'repo>(repo: &'repo Repository, remote: &mut Remote) -> Result<(), RSLError>;
    fn push_rsl<'repo>(repo: &'repo Repository, remote: &mut Remote) -> Result, RSLError
    fn commit_push_entry<'repo>(repo: &'repo Repository, push_entry: &PushEntry) -> Result<(), RSLError>
}

impl HasRSL for Repository {

    fn rsl_init(&self, remote: &mut Remote) -> Result<RSL, RSLError> {


        // TODO: figure out a way to orphan branch; .branch() needs a commit ref. For now, find first commit and use that as ancestor for RSL
        let initial_commit = match find_first_commit(repo) {
            Ok(r) => r,
            Err(e) => return Err(RSLError::Problem()),
        };

        // create new RSL branch
        let rsl_ref = self.branch(RSL_BRANCH, &initial_commit, false).unwrap();

        // create new RSL
        let local_rsl = RSL {
            kind: Local,
            //remote: remote,
            head: rsl_ref,
            last_push_entry: None,
        };

        // save random nonce locally
        let nonce = match Nonce::new() {
            Ok(n) => n,
            Err(_) => return Err(RSLError::Problem())
        };
        repo.write_nonce(nonce);

        // create new nonce bag with initial nonce
        let nonce_bag = NonceBag::new();
        nonce_bag.insert(&nonce);

        //  nonce bag (inlcuding commit)
        repo.commit_nonce_bag(&nonce_bag);

        // push new rsl branch
        repo.push_rsl();

        let remote_rsl = match repo.fetch_rsl(&remote) {
            () => (),
            Err(e) => return Err(e)
        };

        Ok(remote_rsl, local_rsl, nonce_bag, nonce)

    }

    fn read_rsl(&self) -> Result<(RSL, RSL, NonceBag, Nonce), RSLError> {
        let remote_rsl = match self.read_remote_rsl {
            Ok(rsl) => rsl,
            Err(e) => return Err(RSLError::Problem())
        };
        let local_rsl = match self.read_local_rsl {
            Ok(rsl) => rsl,
            Err(e) => return Err(RSLError::Problem())
        };
        let nonce_bag = match self.read_nonce_bag {
            Ok(nb) => nb,
            Err(e) => return Err(RSLError::Problem())
        };
        let nonce = match self.read_nonce {
            Ok(n) => n,
            Err(e) => return Err(RSLError::Problem()),
        };
        Ok((remote_rsl, local_rsl, nonce_bag, nonce))
    }

    fn read_local_rsl(&self) -> Result<RSL, RSLError> {
        let kind = Local;
        let reference = match self.find_branch(RSL_BRANCH, BranchType::Local) {
                Err(e) => return Err(RSLError::Problem()),
                Ok(rsl) => (rsl.into_reference()),
        };
        let head = match reference.target() {
            Some(oid) => oid,
            None => return Err(RSLError::Problem()),
        };
        let last_push_entry = self.last_push_entry(&head);
        Ok(RSL {kind, head, last_push_entry})
    }

    fn read_remote_rsl(&self) -> Result<RSL, RSLError> {
        let kind = Remote;
        let reference = match self.find_branch(RSL_BRANCH, BranchType::Remote) {
                Err(e) => Err(RSLError::Problem()),
                Ok(rsl) => (rsl.into_reference()),
        };
        let head = match reference.target() {
            Some(oid) => oid,
            None => return Err(RSLError::Problem()),
        };
        let last_push_entry = self.last_push_entry(&head);
        Ok(RSL {kind, head, last_push_entry})
    }

    fn commit_push_entry(&self, push_entry; &PushEntry) -> Result<(), RSLError> {
        let mut index = self.index()?;
        //index.add_path(self.path().join("NONCE_BAG"))?;
        let oid = index.write_tree()?;
        let signature = self.signature().unwrap();
        let message = push_entry.to_string();
        let parent_commit_ref = match self.find_reference(RSL_BRAN) {
            Ok(r) => r,
            Err(e) => Err(RSLError::Problem()),
        };
        let parent_commit = match parent_commit_ref.peel_to_commit() {
            Ok(c) => c,
            Err(e) => panic!("couldn't find parent commit: {}", e),
        };
        let tree = self.find_tree(oid)?;
        self.commit(Some("RSL"), //  point HEAD to our new commit
            &signature, // author
            &signature, // committer
            &message, // commit message
            &tree, // tree
            &[&parent_commit]) // parents
    }


    fn fetch_rsl(&self, remote: &Remote) -> Result<(), RSLError> {
        // not sure the behavior here if the branch doesn't exist
        common::fetch(self, remote, &[RSL_BRANCH], Some(REFLOG_MSG)) {
            Ok(()) => Ok(()),
            Err(e) => return Err(q)
        }
    }

    fn init_rsl_if_needed(&self) -> Result<(RSL, RSL, NonceBag, Nonce), RSLError> {
        // validate that RSL does not exist locally or remotely
        let remote_rsl = match (self.find_branch(RSL_BRANCH, BranchType::Remote), self.find_branch(RSL_BRANCH, BranchType::Local)) {
            (Ok(_), _) => Err(RSLError::Problem()),
            (_, Ok(_)) => Err(RSLError::Problem()),
            (Err(_), Err(_)) => (self.rsl_init()),
        };
    }

    fn push_rsl(&self, rsl: &RSL) -> Result<(), RSLError> {
        common::push(self, rsl.remote, &[RSL_BRANCH]);
    }

    fn last_push_entry(repo: &Repository, tree_tip: &Oid) -> Option<PushEntry> {
        let mut revwalk: Revwalk = repo.revwalk().expect("Failed to make revwalk");
        revwalk.push(tree_tip);
        revwalk.set_sorting(git2::SORT_REVERSE);
        let last_push_entry = None;
        let mut current = tree_tip;
        while current != None {
            match PushEntry::from_oid(&repo, &current){
                Some(pe) -> return pe,
                None -> (),
            }
            current = revwalk.next();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::test_helper::*;

    #[test]
    fn commit_push_entry() {
        let repo = setup().unwrap();
        let oid = Oid::from_str("decbf2be529ab6557d5429922251e5ee36519817").unwrap();
        let entry = PushEntry {
                //related_commits: vec![oid.to_owned(), oid.to_owned()],
                branch: String::from("branch_name"),
                head: repo.head().unwrap().target().unwrap(),
                prev_hash: String::from("fwjjk42ofw093j"),
                nonce_bag: NonceBag::new(),
                signature: String::from("gpg signature"),
        };
        let oid = entry.commit_to_rsl(&repo).unwrap();
        let obj = repo.find_commit(oid).unwrap();
        assert_eq!(&obj.message().unwrap(), &"hello");
        teardown(&repo);
    }
}
