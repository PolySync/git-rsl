use std::cmp::Eq;
use std::cmp::PartialEq;
use std::fmt;
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};

use git2::Repository;
use rand::os::OsRng;
use rand::{Rand, Rng};

use serde_json;

#[derive(Debug)]
pub enum NonceError {
    NoRandomNumberGenerator(::std::io::Error),
    NoNonceFile(::std::io::Error),
    NonceReadError(::std::io::Error),
    NonceWriteError(::std::io::Error),
    JsonError(serde_json::Error),
}

impl From<serde_json::Error> for NonceError {
    fn from(error: serde_json::Error) -> Self {
        NonceError::JsonError(error)
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Nonce {
    pub bytes: [u8; 32],
}

impl Nonce {
    pub fn new() -> Result<Nonce, NonceError> {
        let mut rng = match OsRng::new() {
            Ok(rng) => rng,
            Err(e) => return Err(NonceError::NoRandomNumberGenerator(e)),
        };

        Ok(rng.gen())
    }

    pub fn from_str(string: &str) -> Result<Nonce, NonceError> {
        let mut bytes: [u8; 32] = [0; 32];
        let mut cursor = io::Cursor::new(string);
        match cursor.read_exact(&mut bytes) {
            Ok(_) => Ok(Nonce { bytes }),
            Err(e) => Err(NonceError::NonceReadError(e)),
        }
    }

    pub fn from_json(string: &str) -> Result<Nonce, NonceError> {
        let result = serde_json::from_str(string)?;
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
    fn read_nonce(&self) -> Result<Nonce, NonceError>;
    fn write_nonce(&self, nonce: &Nonce) -> Result<(), NonceError>;
}


impl HasNonce for Repository {

    fn read_nonce(&self) -> Result<Nonce, NonceError> {
        let mut bytes: [u8; 32] = [0; 32];
        let nonce_path = &self.path().join("NONCE");
        let mut f = match OpenOptions::new().read(true).write(true).create(true).open(&nonce_path) {
            Ok(f) => f,
            Err(e) => return Err(NonceError::NonceReadError(e)),
        };
        match f.read_exact(&mut bytes) {
            Ok(_) => Ok(Nonce { bytes: bytes }),
            Err(e) => Err(NonceError::NonceReadError(e)),
        }

    }

    fn write_nonce(&self, nonce: &Nonce) -> Result<(), NonceError> {
        let nonce_path = self.path().join("NONCE");
        let mut f = match OpenOptions::new().write(true).create(true).open(&nonce_path) {
            Ok(f) => f,
            Err(e) => return Err(NonceError::NonceReadError(e)),
        };

        match f.write_all(&nonce.bytes) {
            Ok(_) => Ok(()),
            Err(e) => Err(NonceError::NonceWriteError(e)),

        }
    }
}
// fn generate_nonce(repo: &Repository) -> [u8; 32] {
//     let mut nonce_buffer: [u8; 32] = [0; 32];
//     let mut rng = match OsRng::new() {
//         Ok(rng) => rng,
//         Err(e) => {
//             println!("Error: Unable to get OS-level randon number generator to create nonce");
//             println!("  {}", e);
//             process::exit(60);
//         },
//     };
//     rng.fill_bytes(&mut nonce_buffer);
//     let nonce_path = repo.path().join("NONCE");
//     let mut f = open_nonce_file(&nonce_path);
//
//     match f.write_all(&nonce_buffer) {
//         Ok(_) => (),
//         Err(e) => {
//             println!("Error: Unable to write nonce to {}", nonce_path.display());
//             println!("  {}", e);
//             process::exit(62);
//         },
//
//     };
//
//     nonce_buffer
// }
//
// fn open_nonce_file(nonce_path: &Path) -> File {
//     match File::open(&nonce_path) {
//         Ok(f) => f,
//         Err(e) => {
//             println!("Error: Unable to open/create nonce in {}", nonce_path.display());
//             println!("  {}", e);
//             process::exit(61);
//         },
//     }
// }
//
// fn read_current_nonce(repo: &Repository) -> Option<[u8; 32]> {
//     let mut nonce: [u8; 32] = [0; 32];
//     let nonce_path = repo.path().join("NONCE");
//     let mut f = open_nonce_file(&nonce_path);
//     match f.read_exact(&mut nonce) {
//         Ok(_) => Some(nonce),
//         Err(_) => {
//             println!("Warning: No nonce found in {}", nonce_path.display());
//             println!("  Lack of a nonce is acceptable for the first secure fetch, but a problem afterwards.");
//             None
//         },
//     }
// }

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
        let repo = setup();
        repo.write_nonce(&FAKE_NONCE);
        let nonce_file = &repo.path().join("NONCE");
        let mut f = File::open(&nonce_file)
                    .expect("file not found");
        let mut contents = vec![];
        let string = f.read_to_end(&mut contents)
                    .expect("something went wrong reading the file");
        assert_eq!(contents, FAKE_NONCE.bytes);
        teardown(&repo);
    }

    #[test]
    fn read_nonce() {
        let repo = setup();
        let nonce = repo.read_nonce().unwrap();
        let nonce2 = Nonce { bytes: [168, 202, 85, 60, 50, 231, 189, 13, 197, 149, 177, 98, 8, 162, 2, 25, 211, 51, 159, 84, 228, 203, 184, 235, 219, 10, 118, 213, 97, 190, 187, 239] };
        assert_eq!(nonce, nonce2);
        teardown(&repo);
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
