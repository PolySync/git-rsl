use std::fmt;

use crypto::digest::Digest;
use crypto::sha3::Sha3;
use git2::{self, Oid, Reference, Repository, BranchType};
//use libgit2_sys::GIT_OID_RAWSZ;

use nonce_bag::{NonceBag};

use serde_json;
use serde::ser::{Serialize};
use utils;


#[serde(remote = "Oid")]
#[derive(Serialize, Deserialize)]
struct OidDef {
    #[serde(serialize_with = "utils::buffer_to_hex", deserialize_with = "utils::hex_to_buffer", getter = "get_raw_oid")]
    raw: Vec<u8>,
}

fn get_raw_oid(oid: &Oid) -> Vec<u8> {
    // TODO this should be changed back to [u8: GIT_OID_RAWSZ] when libgit2-sys can be added as a dependency again (i.e. when both of the packages are tagged at or above 0.7.1)
    //let mut oid_array: [u8; GIT_OID_RAWSZ] = Default::default();

    let mut oid_array: [u8; 20] = Default::default();
    oid_array.copy_from_slice(oid.as_bytes());
    oid_array.to_vec()
}

// Provide a conversion to construct the remote type Oid from OidDef.
impl From<OidDef> for Oid {
    fn from(def: OidDef) -> git2::Oid {
        Oid::from_bytes(&def.raw).unwrap()
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct PushEntry {
    //#[serde(with = "Vec::<OidDef>")]
    //pub related_commits: Vec<Oid>,
    pub branch: String,
    #[serde(with = "OidDef")]
    pub head: Oid,
    pub prev_hash: String,
    pub nonce_bag: NonceBag,
    pub signature: String,
}

impl PushEntry {
    //TODO Implement
    pub fn new(repo: &Repository, branch_str: &str, prev: String, nonce_bag: NonceBag) -> PushEntry {
        let branch_head = repo.find_branch(branch_str, BranchType::Local).unwrap().get().target().unwrap();

        PushEntry {
//            related_commits: Vec::new(),
            branch: String::from(branch_str),
            head: branch_head,
            prev_hash: prev,
            nonce_bag: nonce_bag,
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

    pub fn from_str(string: &str) -> Option<PushEntry> {
        match serde_json::from_str(string) {
            Ok(p) => Some(p),
            Err(_) => None,
        }
    }

    pub fn from_ref(repo: &Repository, reference: &Reference) -> Option<PushEntry> {
        match reference.target() {
            Some(oid) => PushEntry::from_oid(repo, &oid),
            None => None,
        }
    }

    pub fn from_oid(repo: &Repository, oid: &Oid) -> Option<PushEntry> {
        let commit = match repo.find_commit(oid.clone()) {
            Ok(c) => c,
            Err(e) => panic!("couldn't find commit {:?}", oid),
        };
        let message = match commit.message() {
            Some(m) => m,
            None => panic!("commit message contains invalid UTF-8"),
        };
        match serde_json::from_str(&message) {
            Ok(p) => Some(p),
            Err(_) => None,
        }
    }

}

impl fmt::Display for PushEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text: String = serde_json::to_string_pretty(self).unwrap();
        write!(f, "{}", text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::test_helper::*;
    use utils::git;
    use rsl::HasRSL;

    #[test]
    fn to_string_and_back() {
        let oid = Oid::from_str("decbf2be529ab6557d5429922251e5ee36519817").unwrap();
        let entry = PushEntry {
                //related_commits: vec![oid.to_owned(), oid.to_owned()],
                branch: String::from("branch_name"),
                head: oid,
                prev_hash: String::from("fwjjk42ofw093j"),
                nonce_bag: NonceBag::new(),
                signature: String::from("gpg signature"),
        };
        let serialized = &entry.to_string();
        let deserialized = PushEntry::from_str(&serialized).unwrap();
        assert_eq!(entry, deserialized)
    }

    #[test]
    fn from_string() {
        let string = "{\n  \"branch\": \"branch_name\",\n  \"head\": {\n    \"raw\": \"decbf2be529ab6557d5429922251e5ee36519817\"\n  },\n  \"prev_hash\": \"fwjjk42ofw093j\",\n  \"nonce_bag\": {\n    \"bag\": []\n  },\n  \"signature\": \"gpg signature\"\n}";
        let entry = PushEntry {
                //related_commits: vec![oid.to_owned(), oid.to_owned()],
                branch: String::from("branch_name"),
                head: Oid::from_str("decbf2be529ab6557d5429922251e5ee36519817").unwrap(),
                prev_hash: String::from("fwjjk42ofw093j"),
                nonce_bag: NonceBag::new(),
                signature: String::from("gpg signature"),
        };
        let deserialized = PushEntry::from_str(&string).unwrap();

        assert_eq!(deserialized, entry)
    }


    #[test]
    fn from_oid() {
        let context = setup_fresh();
        {
            let repo = &context.local;
            // RSL commit only works on RSL branch
            let mut rem = repo.find_remote("origin").unwrap().to_owned();
            repo.rsl_init_global(&mut rem).unwrap();
            git::checkout_branch(repo, "refs/heads/RSL").unwrap();
            let entry = PushEntry {
                    //related_commits: vec![oid.to_owned(), oid.to_owned()],
                    branch: String::from("branch_name"),
                    head: Oid::from_str("decbf2be529ab6557d5429922251e5ee36519817").unwrap(),
                    prev_hash: String::from("fwjjk42ofw093j"),
                    nonce_bag: NonceBag::new(),
                    signature: String::from("gpg signature"),
            };
            let oid = repo.commit_push_entry(&entry).unwrap();

            assert_eq!(PushEntry::from_oid(&repo, &oid).unwrap(), entry);
        }
        teardown_fresh(context);
    }
}
