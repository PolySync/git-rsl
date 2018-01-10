use std::cmp::Eq;
use std::cmp::PartialEq;
use std::io::Write;
use std::fs::OpenOptions;
use std::collections::HashSet;

use std::io::BufReader;
use std::io::BufRead;
use std::path::Path;


use git2::{Reference, Repository};
use serde_json;

use common::Nonce;
use common::nonce::NonceError;

#[derive(Debug)]
pub enum NonceBagError {
    NoNonceBagFile(::std::io::Error),
    NonceBagReadError(::std::io::Error),
    NonceBagWriteError(::std::io::Error),
    NonceBagInsertError(),
    NonceBagCheckoutError(::git2::Error),
    InvalidNonceBag(NonceError)
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

    pub fn from_json(string: &str) -> Result<NonceBag, NonceError> {
        let result = serde_json::from_str(string)?;
        Ok(result)
    }
    pub fn to_json(&self) -> Result<String, NonceError> {
        let result = serde_json::to_string(self)?;
        Ok(result)
    }
}

pub trait HasNonceBag {
    fn read_nonce_bag(&self, &Reference) -> Result<NonceBag, NonceBagError>;
    fn write_nonce_bag(&self, nonce_bag: &NonceBag) -> Result<(), NonceBagError>;
}

impl HasNonceBag for Repository {

    fn read_nonce_bag(&self, remote_nonce: &Reference) -> Result<NonceBag, NonceBagError> {
        let current_branch = match self.head() {
            Ok(b) => b,
            Err(e) => return Err(NonceBagError::NonceBagCheckoutError(e)),
        };
        let current_branch_name = current_branch.name().unwrap();
        self.set_head(remote_nonce.name().unwrap());

        let nonce_bag_path = &self.path().join("NONCE_BAG");
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
         let nonce_bag_path = self.path().join("NONCE_BAG");
         let mut f = match OpenOptions::new().write(true).create(true).open(&nonce_bag_path) {
             Ok(f) => f,
             Err(e) => return Err(NonceBagError::NonceBagReadError(e)),
         };
         for nonce in &nonce_bag.bag {
             match f.write(&nonce.bytes) {
                 Ok(32) => (),
                 Ok(_e) => panic!("what the hell is wrong with ur nonce bag"),
                 Err(e) => return Err(NonceBagError::NonceBagWriteError(e)),
             };
         }
         Ok(())
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
    // fails 5/6 times because of no ordering in the bag
    #[test]
    fn serialize() {
        let mut bag = bag_a();
        let serialized = String::from("{\"bag\":[{\"bytes\":[145,161,65,251,112,184,238,36,105,54,150,202,74,26,148,121,106,40,239,155,31,232,49,251,215,71,200,240,105,73,0,84]},{\"bytes\":[100,223,169,31,154,84,127,151,178,254,47,129,230,74,10,10,170,13,31,199,167,68,28,149,131,10,110,201,71,146,214,78]},{\"bytes\":[165,36,170,43,1,62,34,53,25,160,177,19,87,62,189,151,168,134,196,85,33,237,9,52,198,39,79,32,180,145,165,132]}]}");
        let result = NonceBag::to_json(&bag).unwrap();
        assert_eq!(&result, &serialized)
    }

    #[test]
    fn deserialize() {
        let serialized = "{\"bag\":[{\"bytes\":[145,161,65,251,112,184,238,36,105,54,150,202,74,26,148,121,106,40,239,155,31,232,49,251,215,71,200,240,105,73,0,84]},{\"bytes\":[100,223,169,31,154,84,127,151,178,254,47,129,230,74,10,10,170,13,31,199,167,68,28,149,131,10,110,201,71,146,214,78]},{\"bytes\":[165,36,170,43,1,62,34,53,25,160,177,19,87,62,189,151,168,134,196,85,33,237,9,52,198,39,79,32,180,145,165,132]}]}";
        let nonce_bag = NonceBag::from_json(&serialized).unwrap();
        assert!(nonce_bag.bag.contains(&NONCE1));
        assert!(nonce_bag.bag.contains(&NONCE2));
        assert!(nonce_bag.bag.contains(&NONCE3));
    }
 }
