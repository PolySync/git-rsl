use std::cmp::Eq;
use std::cmp::PartialEq;
use std::io::Write;
use std::fs::OpenOptions;
use std::collections::HashSet;

use std::io::BufReader;
use std::io::BufRead;
use std::path::Path;
use std::fmt;
use std::error;


use git2::{self, Reference, Repository, Oid, BranchType};
use git2::Error;
use serde_json;

use common::Nonce;
use common::nonce::NonceError;

const NONCE_BAG_PATH: &'static str = "NONCE_BAG";
const RSL_BRANCH: &'static str = "RSL";
const REFLOG_MSG: &'static str = "Retrieve RSL branchs from remote";



#[derive(Debug)]
pub enum NonceBagError {
    NoNonceBagFile(::std::io::Error),
    NonceBagReadError(::std::io::Error),
    NonceBagWriteError(::std::io::Error),
    NonceBagInsertError(),
    NonceBagUpdateError(),
    ReadWriteError(::std::io::Error),
    NonceError(NonceError),
    NonceBagCheckoutError(::git2::Error),
    InvalidNonceBag(NonceError),
    GitError(git2::Error),
}

impl From<git2::Error> for NonceBagError {
    fn from(error: git2::Error) -> Self {
        NonceBagError::GitError(error)
    }
}

impl From<::std::io::Error> for NonceBagError {
    fn from(error: ::std::io::Error) -> Self {
        NonceBagError::ReadWriteError(error)
    }
}

impl From<NonceError> for NonceBagError {
    fn from(error: NonceError) -> Self {
        NonceBagError::NonceError(error)
    }
}

impl error::Error for NonceBagError {
    fn description(&self) -> &str {
        "ahhhh"
    }
}

impl fmt::Display for NonceBagError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        format!("{:?}", self)
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct NonceBag {
    pub bag: HashSet<Nonce>,
}

impl NonceBag {
    pub fn new() -> NonceBag {
            NonceBag {bag: HashSet::new()}
    }

    pub fn insert(&mut self, nonce: Nonce) -> Result<(), NonceBagError> {
        match self.bag.insert(nonce) {
            true => Ok(()),
            false => Err(NonceBagError::NonceBagInsertError())
        }
    }

    pub fn remove(&mut self, nonce: &Nonce) -> Result<(), NonceBagError> {
        match self.bag.remove(nonce) {
            true => Ok(()),
            false => Err(NonceBagError::NonceBagUpdateError())
        }
    }

    pub fn from_str(string: &str) -> Result<NonceBag, NonceError> {
        let result = serde_json::from_str(string)?;
        Ok(result)
    }
    pub fn to_string(&self) -> Result<String, NonceError> {
        let result = serde_json::to_string(self)?;
        Ok(result)
    }
}

pub trait HasNonceBag {
    fn read_nonce_bag(&self) -> Result<NonceBag, NonceBagError>;
    fn write_nonce_bag(&self, nonce_bag: &NonceBag) -> Result<(), NonceBagError>;
    fn commit_nonce_bag(&self) -> Result<Oid, NonceBagError>;
}

impl HasNonceBag for Repository {

    fn read_nonce_bag(&self) -> Result<NonceBag, NonceBagError> {
        let current_branch = match self.head() {
            Ok(b) => b,
            Err(e) => return Err(NonceBagError::NonceBagCheckoutError(e)),
        };
        let current_branch_name = current_branch.name().unwrap();
        //let remote_nonce_branch = match self.find_branch(RSL_BRANCH, BranchType::Remote).map(|b| try!(b.name())) {
        //    Ok(branch) => branch,
        //    Err(e) => return Err(NonceBagError::NonceBagCheckoutError(e)),
        //};

        // TODO checkout rsl branch work tree instead of changing head, read, then checkout HEAD work tree again
        //
        // let branch = self.find_branch("RSL", BranchType::Local)?.into_reference().peel_to_commit()?.into_object();
        // self.checkout_tree(&branch, None)?; // Option<CheckoutBuilder>

        self.set_head(&RSL_BRANCH)?;

        let nonce_bag_path = &self.path().parent().unwrap().join(NONCE_BAG_PATH);
        let mut f = match OpenOptions::new().read(true).write(true).create(true).open(&nonce_bag_path) {
            Ok(f) => f,
            Err(e) => return Err(NonceBagError::NonceBagReadError(e)),
        };
        let mut nonce_bag = NonceBag::new();
        let file = BufReader::new(&f);
        for (_num, line) in file.lines().enumerate() {
             let l = line.unwrap();
             let existing_nonce = match Nonce::from_str(&l) {
                 Ok(n) => n,
                 Err(e) => return Err(NonceBagError::InvalidNonceBag(e)),
             };
             &nonce_bag.insert(existing_nonce);
         }
         &self.set_head(&current_branch_name);
         Ok(nonce_bag)
    }

    fn write_nonce_bag(&self, nonce_bag: &NonceBag) -> Result<(), NonceBagError> {
        let branch = self.find_branch("RSL", BranchType::Local)?.into_reference().peel_to_commit()?.into_object();

        self.checkout_tree(&branch, None)?; // Option<CheckoutBuilder>
        self.set_head("refs/heads/RSL")?;

        let text = nonce_bag.to_string()?;
        let nonce_bag_path = self.path().parent().unwrap().join(NONCE_BAG_PATH);
        let mut f = match OpenOptions::new().write(true).create(true).open(&nonce_bag_path) {
             Ok(f) => f,
             Err(e) => return Err(NonceBagError::NonceBagReadError(e)),
         };
         // for nonce in &nonce_bag.bag {
         //     match f.write(&nonce.bytes) {
         //         Ok(32) => (),
         //         Ok(_e) => panic!("what the hell is wrong with ur nonce bag"),
         //         Err(e) => return Err(NonceBagError::NonceBagWriteError(e)),
         //     };
         // }
         f.write_all(&text.as_bytes())?;
         Ok(())
    }

    fn commit_nonce_bag(&self) -> Result<Oid, NonceBagError> {
        let mut index = self.index()?;
        let path = Path::new(NONCE_BAG_PATH);
        index.add_path(&path)?;
        let oid = index.write_tree().unwrap();
        let signature = self.signature()?;
        let message = "Update nonce bag";
        let parent_commit_ref = match self.find_branch("RSL", BranchType::Local) {
            Ok(r) => r.into_reference(),
            Err(e) => panic!("couldn't find parent commit: {}", e),
        };
        let parent_commit = match parent_commit_ref.peel_to_commit() {
            Ok(c) => c,
            Err(e) => panic!("couldn't find parent commit: {}", e),
        };
        let tree = self.find_tree(oid)?;
        let commit_oid = self.commit(Some("HEAD"), //  point HEAD to our new commit
            &signature, // author
            &signature, // committer
            &message, // commit message
            &tree, // tree
            &[&parent_commit])?;
        Ok(commit_oid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONCE1: Nonce = Nonce {bytes: [145,161,65,251,112,184,238,36,105,54,150,202,74,26,148,121,106,40,239,155,31,232,49,251,215,71,200,240,105,73,0,84]};
    const NONCE2: Nonce = Nonce { bytes: [100,223,169,31,154,84,127,151,178,254,47,129,230,74,10,10,170,13,31,199,167,68,28,149,131,10,110,201,71,146,214,78]};
    const NONCE3: Nonce = Nonce { bytes: [165,36,170,43,1,62,34,53,25,160,177,19,87,62,189,151,168,134,196,85,33,237,9,52,198,39,79,32,180,145,165,132]};

    fn bag_a() -> NonceBag {
        let mut bag = NonceBag::new();
        bag.bag.insert(NONCE1);
        bag.bag.insert(NONCE2);
        bag.bag.insert(NONCE3);
        bag
    }

    #[test]
    fn eq() {
        assert_eq!(bag_a(), bag_a());
    }

    #[test]
    fn neq() {
        let mut bag = bag_a();
        bag.bag.remove(&NONCE1);
        assert_ne!(bag, bag_a());
    }

    #[test]
    fn to_string_and_back() {
        let mut bag = bag_a();
        let result = NonceBag::to_string(&bag).unwrap();
        let bag2 = NonceBag::from_str(&result).unwrap();
        assert_eq!(bag, bag2)
    }

    #[test]
    fn from_str() {
        let serialized = "{\"bag\":[{\"bytes\":[145,161,65,251,112,184,238,36,105,54,150,202,74,26,148,121,106,40,239,155,31,232,49,251,215,71,200,240,105,73,0,84]},{\"bytes\":[100,223,169,31,154,84,127,151,178,254,47,129,230,74,10,10,170,13,31,199,167,68,28,149,131,10,110,201,71,146,214,78]},{\"bytes\":[165,36,170,43,1,62,34,53,25,160,177,19,87,62,189,151,168,134,196,85,33,237,9,52,198,39,79,32,180,145,165,132]}]}";
        let nonce_bag = NonceBag::from_str(&serialized).unwrap();
        assert!(nonce_bag.bag.contains(&NONCE1));
        assert!(nonce_bag.bag.contains(&NONCE2));
        assert!(nonce_bag.bag.contains(&NONCE3));
    }
 }
