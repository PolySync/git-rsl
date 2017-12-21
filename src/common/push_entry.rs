use std::collections::HashSet;
use std::fmt;
use std::vec::Vec;

use crypto::digest::Digest;
use crypto::sha3::Sha3;
use git2::{Oid, Reference, Repository};

use common::Nonce;

use serde_json::Error;

//#[derive(Serialize, Deserialize)]
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
    pub fn new(repo: &Repository, branch_str: &str, prev: String) -> PushEntry {
        let branch = repo.find_reference(branch_str);
        PushEntry {
            related_commits: Vec::new(),
            branch: String::from(branch_str),
            head: None,
            prev_hash: prev,
            nonce_bag: HashSet::new(),
            signature: String::from(""),
        }
    }

    pub fn prev_hash(&self) -> String {
        self.prev_hash.clone()
    }

    pub fn hash(&self) -> String {
        let mut hasher = Sha3::sha3_512();

        hasher.input_str( &format!("{}", self) );

        hasher.result_str()
    }

    //TODO implement done?
    pub fn from_str(string: String) -> Option<PushEntry> {
    //    let p: PushEntry = serde_json::from_str(string)?;

        Some( PushEntry {
            related_commits: Vec::new(),
            branch: String::from(""),
            head: None,
            prev_hash: String::from(""),
            nonce_bag: HashSet::new(),
            signature: String::from(""),
        })
    }

    pub fn from_ref(reference: &Reference) -> Option<PushEntry> {
        match reference.target() {
            Some(oid) => PushEntry::from_oid(oid),
            None => None,
        }
    }

    //TODO implement
    pub fn from_oid(oid: Oid) -> Option<PushEntry> {

        //let p: PushEntry = serde_json::from_str(string)?;

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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works(){

    }
}
