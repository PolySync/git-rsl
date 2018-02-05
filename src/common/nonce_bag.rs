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
use common::errors::*;

const NONCE_BAG_PATH: &'static str = "NONCE_BAG";
const RSL_BRANCH: &'static str = "RSL";
const REFLOG_MSG: &'static str = "Retrieve RSL branchs from remote";

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct NonceBag {
    pub bag: HashSet<Nonce>,
}

impl NonceBag {
    pub fn new() -> NonceBag {
            NonceBag {bag: HashSet::new()}
    }

    pub fn insert(&mut self, nonce: Nonce) -> Result<()> {
        match self.bag.insert(nonce) {
            true => Ok(()),
            false => bail!("problem inserting nonce into bag"),
        }
    }

    pub fn remove(&mut self, nonce: &Nonce) -> Result<()> {
        match self.bag.remove(nonce) {
            true => Ok(()),
            false => bail!("problem removing nonce from bag"),
        }
    }

    pub fn from_str(string: &str) -> Result<NonceBag> {
        let result = serde_json::from_str(string).chain_err(|| "couldn't parse nonce bag as JSON")?;
        Ok(result)
    }
    pub fn to_string(&self) -> Result<String> {
        let result = serde_json::to_string(self).chain_err(|| "couldn't serialize nonce bag struct")?;
        Ok(result)
    }
}

pub trait HasNonceBag {
    fn read_nonce_bag(&self) -> Result<NonceBag>;
    fn write_nonce_bag(&self, nonce_bag: &NonceBag) -> Result<()>;
    fn commit_nonce_bag(&self) -> Result<Oid>;
}

impl HasNonceBag for Repository {

    fn read_nonce_bag(&self) -> Result<NonceBag> {
        let nonce_bag_path = &self.path().parent().unwrap().join(NONCE_BAG_PATH);
        let mut f = OpenOptions::new().read(true).write(true).create(true).open(&nonce_bag_path).chain_err(|| "couldn't open nonce bag for reading")?;
        let mut nonce_bag = NonceBag::new();
        let file = BufReader::new(&f);
        for (_num, line) in file.lines().enumerate() {
             let l = line.unwrap();
             let existing_nonce = Nonce::from_str(&l).chain_err(|| "couldn't parse into nonce bytes")?;
             &nonce_bag.insert(existing_nonce);
         }

         Ok(nonce_bag)
    }

    fn write_nonce_bag(&self, nonce_bag: &NonceBag) -> Result<()> {
        let text = nonce_bag.to_string()?;
        let nonce_bag_path = self.path().parent().unwrap().join(NONCE_BAG_PATH);
        let mut f = OpenOptions::new().write(true).create(true).open(&nonce_bag_path).chain_err(|| "couldn't open nonce bag fiile for writing")?;
         // for nonce in &nonce_bag.bag {
         //     match f.write(&nonce.bytes) {
         //         Ok(32) => (),
         //         Ok(_e) => panic!("what the hell is wrong with ur nonce bag"),
         //         Err(e) => return Err(NonceBagError::NonceBagWriteError(e)),
         //     };
         // }
         f.write_all(&text.as_bytes()).chain_err(|| "couldnt write to nonce bag file")?;
         Ok(())
    }

    fn commit_nonce_bag(&self) -> Result<Oid> {
        let mut index = self.index()
            .chain_err(|| "couldn't find index")?;
        let path = Path::new(NONCE_BAG_PATH);
        index.add_path(&path)
            .chain_err(|| "couldn't add path")?;
        let oid = index.write_tree()
            .chain_err(|| "couldn't write tree")?;
        let signature = self.signature()
            .chain_err(|| "couldn't generate sig")?;
        let message = "Update nonce bag";
        let parent_commit = self.find_branch("RSL", BranchType::Local)
            .chain_err(|| "coudln't find parent commit")?
            .into_reference()
            .peel_to_commit()
            .chain_err(|| "couldn't find parent commit OID")?;
        let tree = self.find_tree(oid).chain_err(|| "couldn't find tree")?;
        let commit_oid = self.commit(Some("HEAD"), //  point HEAD to our new commit
            &signature, // author
            &signature, // committer
            &message, // commit message
            &tree, // tree
            &[&parent_commit])
            .chain_err(|| "failed to commit nonce bag")?;
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
