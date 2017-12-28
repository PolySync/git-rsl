use std::collections::HashSet;
use std::fmt;
use std::vec::Vec;

use crypto::digest::Digest;
use crypto::sha3::Sha3;
use git2::{Oid, Reference, Repository};

use common::Nonce;

use serde_json::{self, Error};
use serde::ser::{Serialize, Serializer, SerializeStruct};


//#[derive(Deserialize)]
pub struct PushEntry {
    related_commits: Vec<Oid>,
    branch: String,
    head: Option<Oid>,
    prev_hash: String,
    nonce_bag: HashSet<Nonce>,
    signature: String,
}

impl Serialize for PushEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let mut state = serializer.serialize_struct("PushEntry", 6)?;

        let mut related_commits = self.related_commits.iter()
                                .map(|oid| { oid.to_string() })
                                .collect::<Vec<_>>();
                                state.serialize_field("branch", &self.branch)?;
        let head = match self.head {
            Some(oid) => oid.to_string(),
            None => String::from(""),
        };

        state.serialize_field("related_commits", &related_commits)?;
        state.serialize_field("head", &head)?;
        state.serialize_field("prev_hash", &self.prev_hash)?;
        state.serialize_field("nonce_bag", &self.nonce_bag)?;
        state.serialize_field("signature", &self.signature)?;
        state.end()
    }
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

    pub fn to_str(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
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
        write!(f, "{}", "not implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn it_works(){

    }

    #[test]
    fn to_string() {
        let oid = Oid::from_str("decbf2be529ab6557d5429922251e5ee36519817").unwrap();
        let push_entry = PushEntry {
            related_commits: vec![oid.to_owned(), oid.to_owned()],
            branch: String::from("branch_name"),
            head: None,
            prev_hash: String::from("fwjjk42ofw093j"),
            nonce_bag: HashSet::new(),
            signature: String::from("gpg signature"),

        };
        let serialized = "{\"branch\":\"branch_name\",\"related_commits\":[\"decbf2be529ab6557d5429922251e5ee36519817\",\"decbf2be529ab6557d5429922251e5ee36519817\"],\"head\":\"\",\"prev_hash\":\"fwjjk42ofw093j\",\"nonce_bag\":[],\"signature\":\"gpg signature\"}";
        assert_eq!(&push_entry.to_str().unwrap(), &serialized)
    }
}
