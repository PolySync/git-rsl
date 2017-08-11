use std::collections::HashSet;
use std::fmt;
use std::vec::Vec;

use crypto::digest::Digest;
use crypto::sha3::Sha3;
use git2::{Oid, Reference, Repository};

use common::Nonce;

pub struct PushEntry {
    related_commits: Vec<Oid>,
    branch: String,
    head: Option<Oid>,
    prev_hash: String,
    nonce_bag: HashSet<Nonce>,
    signature: String,
}

impl PushEntry {
    //TODO Implement
    pub fn new(repo: &Repository, branch_str: &str) -> PushEntry {
        let branch = repo.find_reference(branch_str);
        PushEntry {
            related_commits: Vec::new(),
            branch: String::from(branch_str),
            head: None,
            prev_hash: String::from(""),
            nonce_bag: HashSet::new(),
            signature: String::from(""),
        }
    }

    pub fn hash(&self) -> String {
        let mut hasher = Sha3::sha3_512();

        hasher.input_str( &format!("{}", self) );

        hasher.result_str()
    }

    //TODO implement
    pub fn from_str(string: String) -> PushEntry {
        PushEntry {
            related_commits: Vec::new(),
            branch: String::from(""),
            head: None,
            prev_hash: String::from(""),
            nonce_bag: HashSet::new(),
            signature: String::from(""),
        }
    }

    //TODO implement
    pub fn from(reference: &Reference) -> Option<PushEntry> {
        Some( PushEntry {
            related_commits: Vec::new(),
            branch: String::from(""),
            head: None,
            prev_hash: String::from(""),
            nonce_bag: HashSet::new(),
            signature: String::from(""),
        })
    }

}

impl fmt::Display for PushEntry {
    //TODO Implement
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unimplemented")
    }
}
