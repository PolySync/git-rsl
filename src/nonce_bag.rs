use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;

use std::path::Path;

use std::io::prelude::*;

use git2::{self, Oid, Repository};
use serde_json;

use errors::*;
use nonce::Nonce;
use utils::git;

const NONCE_BAG_PATH: &str = "NONCE_BAG";

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct NonceBag {
    bag: HashSet<Nonce>,
}

impl NonceBag {
    pub fn new() -> NonceBag {
        NonceBag {
            bag: HashSet::new(),
        }
    }

    pub fn insert(&mut self, nonce: Nonce) -> bool {
        self.bag.insert(nonce)
    }

    pub fn remove(&mut self, nonce: &Nonce) -> bool {
        self.bag.remove(nonce)
    }

    pub fn contains(&self, nonce: &Nonce) -> bool {
        self.bag.contains(nonce)
    }

    pub fn from_str(string: &str) -> Result<NonceBag> {
        let result = serde_json::from_str(string).chain_err(|| "couldn't parse nonce bag as JSON")?;
        Ok(result)
    }
    pub fn to_string(&self) -> Result<String> {
        let result =
            serde_json::to_string(self).chain_err(|| "couldn't serialize nonce bag struct")?;
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
        let mut f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&nonce_bag_path)
            .chain_err(|| "couldn't open nonce bag for reading")?;
        let mut buffer = String::new();
        f.read_to_string(&mut buffer)?;
        let nonce_bag = NonceBag::from_str(&buffer)?;
        Ok(nonce_bag)
    }

    fn write_nonce_bag(&self, nonce_bag: &NonceBag) -> Result<()> {
        let text = nonce_bag.to_string()?;
        let nonce_bag_path = self.path().parent().unwrap().join(NONCE_BAG_PATH);
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&nonce_bag_path)
            .chain_err(|| "couldn't open nonce bag file for writing")?;
        f.write_all(&text.as_bytes())
            .chain_err(|| "couldnt write to nonce bag file")?;
        Ok(())
    }

    fn commit_nonce_bag(&self) -> Result<Oid> {
        let path = Path::new(NONCE_BAG_PATH);
        let message = "Update nonce bag";

        let commit_oid = git::add_and_commit_signed(self, Some(&path), &message, &"refs/heads/RSL")
            .chain_err(|| "failed to commit nonce bag")?;

        debug_assert!(self.state() == git2::RepositoryState::Clean);

        Ok(commit_oid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::test_helper::*;

    fn nonce_1() -> Nonce {
        Nonce::from_bytes([
            145, 161, 65, 251, 112, 184, 238, 36, 105, 54, 150, 202, 74, 26, 148, 121, 106, 40,
            239, 155, 31, 232, 49, 251, 215, 71, 200, 240, 105, 73, 0, 84,
        ])
    }
    fn nonce_2() -> Nonce {
        Nonce::from_bytes([
            100, 223, 169, 31, 154, 84, 127, 151, 178, 254, 47, 129, 230, 74, 10, 10, 170, 13, 31,
            199, 167, 68, 28, 149, 131, 10, 110, 201, 71, 146, 214, 78,
        ])
    }
    fn nonce_3() -> Nonce {
        Nonce::from_bytes([
            165, 36, 170, 43, 1, 62, 34, 53, 25, 160, 177, 19, 87, 62, 189, 151, 168, 134, 196, 85,
            33, 237, 9, 52, 198, 39, 79, 32, 180, 145, 165, 132,
        ])
    }

    fn bag_a() -> NonceBag {
        let mut bag = NonceBag::new();
        bag.insert(nonce_1());
        bag.insert(nonce_2());
        bag.insert(nonce_3());
        bag
    }

    #[test]
    fn eq() {
        assert_eq!(bag_a(), bag_a());
    }

    #[test]
    fn neq() {
        let mut bag = bag_a();
        bag.bag.remove(&nonce_1());
        assert_ne!(bag, bag_a());
    }

    #[test]
    fn to_string_and_back() {
        let bag = bag_a();
        let result = NonceBag::to_string(&bag).unwrap();
        let bag2 = NonceBag::from_str(&result).unwrap();
        assert_eq!(bag, bag2)
    }

    #[test]
    fn write_and_read() {
        let context = setup_fresh();
        {
            let repo = &context.local;
            let bag = bag_a();
            &repo.write_nonce_bag(&bag).expect("bad write");
            let res = repo.read_nonce_bag().expect("bad read");
            assert_eq!(res, bag);
        }
        teardown_fresh(context);
    }

    #[test]
    fn from_str() {
        let serialized = r#"{
            "bag": [
                {"bytes": [145,161,65,251,112,184,238,36,105,54,150,202,74,26,148,121,106,40,239,155,31,232,49,251,215,71,200,240,105,73,0,84]},
                {"bytes": [100,223,169,31,154,84,127,151,178,254,47,129,230,74,10,10,170,13,31,199,167,68,28,149,131,10,110,201,71,146,214,78]},
                {"bytes": [165,36,170,43,1,62,34,53,25,160,177,19,87,62,189,151,168,134,196,85,33,237,9,52,198,39,79,32,180,145,165,132]}
            ]
        }"#;
        let nonce_bag = NonceBag::from_str(&serialized).unwrap();
        assert_eq!(nonce_bag.bag.contains(&nonce_1()), true);
        assert_eq!(nonce_bag.bag.contains(&nonce_2()), true);
        assert_eq!(nonce_bag.bag.contains(&nonce_3()), true);
    }

    #[test]
    fn commit_nonce_bag() {
        let context = setup_fresh();
        let bag = NonceBag::new();
        &context.local.write_nonce_bag(&bag).unwrap();
        &context.local.commit_nonce_bag().unwrap();
        assert!(context.local.state() == git2::RepositoryState::Clean);
        teardown_fresh(context)
    }
}
