use std::cmp::Eq;
use std::cmp::PartialEq;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write, Error};
use std::fs::OpenOptions;
use std::collections::HashSet;

use std::io::BufReader;
use std::io::BufRead;

use git2::{Oid, Reference, Repository};

use rand::os::OsRng;
use rand::{Rand, Rng};

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


#[derive(Debug, Clone)]
pub struct NonceBag {
    pub bag: HashSet<Nonce>,
}

impl NonceBag {
    pub fn new() -> Result<NonceBag, NonceBagError> {
        Ok(NonceBag {bag: HashSet::new()})
    }

    pub fn insert(&mut self, nonce: Nonce) -> Result<(), NonceBagError> {
        match self.bag.insert(nonce) {
            true => Ok(()),
            false => Err(NonceBagError::NonceBagInsertError())
        }
    }
}

pub trait HasNonceBag {
    fn read_nonce_bag(&self, &Reference) -> Result<NonceBag, NonceBagError>;
    fn write_nonce_bag(&self, nonce_bag: NonceBag) -> Result<(), NonceBagError>;
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
        let mut nonce_bag = match NonceBag::new() {
            Ok(b) => b,
            Err(e) => return Err(e)
        };
        let file = BufReader::new(&f);
        for (num, line) in file.lines().enumerate() {
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

     fn write_nonce_bag(&self, nonce_bag: NonceBag) -> Result<(), NonceBagError> {
         let nonce_bag_path = self.path().join("NONCE_BAG");
         let mut f = match OpenOptions::new().write(true).create(true).open(&nonce_bag_path) {
             Ok(f) => f,
             Err(e) => return Err(NonceBagError::NonceBagReadError(e)),
         };
         for nonce in &nonce_bag.bag {
             match f.write(&nonce.bytes) {
                 Ok(32) => (),
                 Ok(e) => panic!("what the hell is wrong with ur nonce bag"),
                 Err(e) => return Err(NonceBagError::NonceBagWriteError(e)),
             };
         }
         Ok(())
     }
 }
