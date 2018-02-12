use std::fmt;
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};

use git2::Repository;
use rand::os::OsRng;
use rand::{Rand, Rng};

use serde_json;

use errors::*;

#[derive(Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Nonce {
    pub bytes: [u8; 32],
}

impl Nonce {
    pub fn new() -> Result<Nonce> {
        let mut rng = OsRng::new().chain_err(|| "no randum number generator")?;
        Ok(rng.gen())
    }

    pub fn from_str(string: &str) -> Result<Nonce> {
        let mut bytes: [u8; 32] = [0; 32];
        let mut cursor = io::Cursor::new(string);
        cursor.read_exact(&mut bytes).chain_err(|| "nonce read error")?;
        Ok(Nonce { bytes })
    }

    pub fn from_json(string: &str) -> Result<Nonce> {
        let result = serde_json::from_str(string)
            .chain_err(|| "couldn't parse nonce from string")?;
        Ok(result)
    }
}

impl Hash for Nonce {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state)
    }
}

impl Rand for Nonce {
    fn rand<R: Rng>(rng: &mut R) -> Nonce {
        let mut random_bytes: [u8; 32] = [0; 32];
        rng.fill_bytes(&mut random_bytes);
        Nonce { bytes: random_bytes }
    }
}

impl fmt::Display for Nonce {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text: String = serde_json::to_string(self).unwrap();
        write!(f, "{}", text)
    }
}


pub trait HasNonce {
    fn read_nonce(&self) -> Result<Nonce>;
    fn write_nonce(&self, nonce: &Nonce) -> Result<()>;
}


impl HasNonce for Repository {

    fn read_nonce(&self) -> Result<Nonce> {
        let mut bytes: [u8; 32] = [0; 32];
        let nonce_path = &self.path().join("NONCE");
        let mut f = OpenOptions::new().read(true).write(true).create(true).open(&nonce_path).chain_err(|| "could not open nonce file for reading")?;

        f.read_exact(&mut bytes).chain_err(|| "could not parse nonce file")?;
        Ok(Nonce { bytes: bytes })
    }

    fn write_nonce(&self, nonce: &Nonce) -> Result<()> {
        let nonce_path = self.path().join("NONCE");
        let mut f = OpenOptions::new().write(true).create(true).open(&nonce_path).chain_err(|| "could not open nonce for writing")?;

        f.write_all(&nonce.bytes).chain_err(|| "could not write nonce")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use utils::test_helper::*;
    use std::fs::File;
    use super::*;

    const FAKE_NONCE: Nonce = Nonce { bytes: [224, 251, 50, 63, 34, 58, 207, 35, 15, 74, 137, 143, 176, 178, 92, 226, 103, 114, 220, 224, 180, 21, 241, 2, 213, 252, 126, 245, 137, 245, 119, 45] };

    #[test]
    fn equality(){
        assert_eq!(FAKE_NONCE, FAKE_NONCE)
    }

    #[test]
    fn inequality(){
        let nonce1 = Nonce { bytes: [168, 202, 85, 60, 50, 231, 189, 13, 197, 149, 177, 98, 8, 162, 2, 25, 211, 51, 159, 84, 228, 203, 184, 235, 219, 10, 118, 213, 97, 190, 187, 239] };

        assert_ne!(nonce1, FAKE_NONCE)
    }

    #[test]
    fn write_nonce() {
        let context = setup();
        {
            let repo = &context.local;
            repo.write_nonce(&FAKE_NONCE).unwrap();
            let nonce_file = &repo.path().join("NONCE");
            let mut f = File::open(&nonce_file)
                        .expect("file not found");
            let mut contents = vec![];
            let string = f.read_to_end(&mut contents)
                        .expect("something went wrong reading the file");
            assert_eq!(contents, FAKE_NONCE.bytes);
        }
        teardown(context);
    }

    #[test]
    fn read_nonce() {
        let context = setup();
        {
            let repo = &context.local;
            let nonce = repo.read_nonce().unwrap();
            let nonce2 = Nonce { bytes: [168, 202, 85, 60, 50, 231, 189, 13, 197, 149, 177, 98, 8, 162, 2, 25, 211, 51, 159, 84, 228, 203, 184, 235, 219, 10, 118, 213, 97, 190, 187, 239] };
            assert_eq!(nonce, nonce2);
        }
        teardown(context);
    }

    #[test]
    fn to_string(){
        let serialized = "{\"bytes\":[224,251,50,63,34,58,207,35,15,74,137,143,176,178,92,226,103,114,220,224,180,21,241,2,213,252,126,245,137,245,119,45]}";
        assert_eq!(&FAKE_NONCE.to_string(), &serialized)

    }

    #[test]
    fn from_json(){
        let serialized = "{\"bytes\":[224,251,50,63,34,58,207,35,15,74,137,143,176,178,92,226,103,114,220,224,180,21,241,2,213,252,126,245,137,245,119,45]}";
        let deserialized = Nonce::from_json(&serialized).unwrap();
        assert_eq!(&deserialized, &FAKE_NONCE)
    }
}
