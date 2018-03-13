use std::io::Write;
use std::fs::OpenOptions;
use std::collections::HashSet;

use std::path::Path;

use std::io::prelude::*;


use git2::{self, Repository, Oid};
use serde_json;

use nonce::Nonce;
use errors::*;
use utils::git;

const NONCE_BAG_PATH: & str = "NONCE_BAG";

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct NonceBag {
    pub bag: HashSet<Nonce>,
}

impl NonceBag {
    pub fn new() -> NonceBag {
            NonceBag {bag: HashSet::new()}
    }

    pub fn insert(&mut self, nonce: Nonce) -> Result<()> {
        if self.bag.insert(nonce) {
            Ok(())
        } else {
            bail!("problem inserting nonce into bag")
        }
    }

    pub fn remove(&mut self, nonce: &Nonce) -> Result<()> {
        if self.bag.remove(nonce) {
            Ok(())
        } else {
            bail!("problem removing nonce from bag")
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

    // TODO change read to read Nonce bag JSON object rather than
    // naked nonces
    fn read_nonce_bag(&self) -> Result<NonceBag> {
        let nonce_bag_path = &self.path().parent().unwrap().join(NONCE_BAG_PATH);
        let mut f = OpenOptions::new().read(true).write(true).create(true).open(&nonce_bag_path).chain_err(|| "couldn't open nonce bag for reading")?;
        let mut nonce_bag = NonceBag::new();

        let file_size = f.metadata()?.len();
        let mut bytes_read = 0u64;

        let mut bytes: [u8; 32] = [0; 32];
        let mut nonce: Nonce;
        while bytes_read < file_size {
            bytes_read += f.read(&mut bytes)? as u64;
            nonce = Nonce { bytes };
            nonce_bag.insert(nonce)?;
        }
        Ok(nonce_bag)
    }

    fn write_nonce_bag(&self, nonce_bag: &NonceBag) -> Result<()> {
        //let text = nonce_bag.to_string()?;
        let nonce_bag_path = self.path().parent().unwrap().join(NONCE_BAG_PATH);
        let mut f = OpenOptions::new().write(true).create(true).open(&nonce_bag_path).chain_err(|| "couldn't open nonce bag file for writing")?;
         for nonce in &nonce_bag.bag {
             match f.write(&nonce.bytes) {
                 Ok(32) => Ok(()),
                 Ok(_e) => bail!("what the hell is wrong with ur nonce bag"),
                 Err(e) => Err(e).chain_err(|| "failed to write nonce bytes to bag"),
             };
         }
        // TODO change nonce file to store as JSON rather than naked nonces,
        // then write as below
        // f.write_all(&text.as_bytes()).chain_err(|| "couldnt write to nonce bag file")?;
         Ok(())
    }

    fn commit_nonce_bag(&self) -> Result<Oid> {
        println!("current head: {:?}", self.head()?.name().unwrap());

        let mut index = self.index()
            .chain_err(|| "couldn't find index")?;
        let path = Path::new(NONCE_BAG_PATH);
        index.add_path(path)
            .chain_err(|| "couldn't add path")?;
        let oid = index.write_tree()
            .chain_err(|| "couldn't write tree")?;
        let signature = self.signature()
            .chain_err(|| "couldn't generate sig")?;
        let message = "Update nonce bag";
        let parent_commit = self.head()?.peel_to_commit()?;
        let tree = self.find_tree(oid).chain_err(|| "couldn't find tree")?;
        let commit_oid = git::commit_signed(
            self,
            "refs/heads/RSL", //  point HEAD to our new commit
            &signature, // author
            &signature, // committer
            message, // commit message
            &tree, // tree
            &[&parent_commit])
            .chain_err(|| "failed to commit nonce bag")?;
        debug_assert!(self.state() == git2::RepositoryState::Clean);

        //let head_tree = self.head()?.peel_to_tree()?;
        //index.read_tree(&tree);
        //index.clear()?;
        //index.write()?;
        //let mut opts = CheckoutBuilder::new();
        //opts.force();
        //self.checkout_head(Some(&mut opts))?;

        // TODO sign commit after making it
        // git::sign_commit(self, oid)

        let commit = self.find_commit(commit_oid)?;

        self.reset_default(Some(commit.as_object()), ["NONCE_BAG"].iter())?;

        Ok(commit_oid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::test_helper::*;

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
        let serialized = "{\"bag\":[{\"bytes\":[145,161,65,251,112,184,238,36,105,54,150,202,74,26,148,121,106,40,239,155,31,232,49,251,215,71,200,240,105,73,0,84]},{\"bytes\":[100,223,169,31,154,84,127,151,178,254,47,129,230,74,10,10,170,13,31,199,167,68,28,149,131,10,110,201,71,146,214,78]},{\"bytes\":[165,36,170,43,1,62,34,53,25,160,177,19,87,62,189,151,168,134,196,85,33,237,9,52,198,39,79,32,180,145,165,132]}]}";
        let nonce_bag = NonceBag::from_str(&serialized).unwrap();
        assert!(nonce_bag.bag.contains(&NONCE1));
        assert!(nonce_bag.bag.contains(&NONCE2));
        assert!(nonce_bag.bag.contains(&NONCE3));
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
