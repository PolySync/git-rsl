use std::cmp::Eq;
use std::cmp::PartialEq;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write, Error};
use std::io::{self, BufRead};

use std::fs::OpenOptions;

use git2::Repository;
use rand::os::OsRng;
use rand::{Rand, Rng};

#[derive(Debug)]
pub enum NonceError {
    NoRandomNumberGenerator(::std::io::Error),
    NoNonceFile(::std::io::Error),
    NonceReadError(::std::io::Error),
    NonceWriteError(::std::io::Error),
}

#[derive(Debug, Copy, Clone)]
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
}

impl PartialEq for Nonce {
    fn eq(&self, other: &Nonce) -> bool {
        self.bytes == other.bytes
    }

    fn ne(&self, other: &Nonce) -> bool {
        self.bytes != other.bytes
    }
}

impl Eq for Nonce {
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


pub trait HasNonce {
    fn read_nonce(&self) -> Result<Nonce, NonceError>;
    fn write_nonce(&self, nonce: Nonce) -> Result<(), NonceError>;
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

    fn write_nonce(&self, nonce: Nonce) -> Result<(), NonceError> {
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
    use super::*;
    use std::path::Path;
    use std::env;
    use common::fs_extra;
    use common::fs_extra::dir::*;
    use common::fs_extra::error::*;

    fn setup() -> Repository {
        let mut fixture_dir = env::current_dir().unwrap();
        &fixture_dir.push("fixtures/.git");
        let path_to = Path::new("/tmp/rsl_test");
        create_all(&path_to, true);

        let mut options = CopyOptions::new();
        options.overwrite = true;

        let res = copy(fixture_dir, path_to, &options);

        match Repository::open(&path_to) {
            Ok(repo) => repo,
            Err(e) => panic!("setup failed: {:?}", e),
        }
    }

    fn teardown() -> Result<()> {
        let tmp_dir = Path::new("/tmp/rsl_test");
        match remove(tmp_dir) {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        }
    }

    #[test]
    fn equality(){
        let nonce1 = Nonce { bytes: [224, 251, 50, 63, 34, 58, 207, 35, 15, 74, 137, 143, 176, 178, 92, 226, 103, 114, 220, 224, 180, 21, 241, 2, 213, 252, 126, 245, 137, 245, 119, 45] };
        let nonce2 = Nonce { bytes: [224, 251, 50, 63, 34, 58, 207, 35, 15, 74, 137, 143, 176, 178, 92, 226, 103, 114, 220, 224, 180, 21, 241, 2, 213, 252, 126, 245, 137, 245, 119, 45] };

        assert_eq!(nonce1, nonce2)
    }

    #[test]
    fn inequality(){
        let nonce1 = Nonce { bytes: [124, 251, 50, 63, 34, 58, 207, 35, 15, 74, 137, 143, 176, 178, 92, 226, 103, 114, 220, 224, 180, 21, 241, 2, 213, 252, 126, 245, 137, 245, 119, 45] };
        let nonce2 = Nonce { bytes: [224, 251, 50, 63, 34, 58, 207, 35, 15, 74, 137, 143, 176, 178, 92, 226, 103, 114, 220, 224, 180, 21, 241, 2, 213, 252, 126, 245, 137, 245, 119, 45] };

        assert_ne!(nonce1, nonce2)
    }

    #[test]
    fn write_nonce() {
        let repo = setup();
        let nonce1 = Nonce { bytes: [124, 251, 50, 63, 34, 58, 207, 35, 15, 74, 137, 143, 176, 178, 92, 226, 103, 114, 220, 224, 180, 21, 241, 2, 213, 252, 126, 245, 137, 245, 119, 45] };
        repo.write_nonce(nonce1);
        let mut f = File::open("/tmp/rsl_test/.git/NONCE")
                    .expect("file not found");
        let mut contents = vec![];
        let string = f.read_to_end(&mut contents)
                    .expect("something went wrong reading the file");
        println!("string: {:?}", contents);
        assert_eq!(contents, nonce1.bytes);
        teardown();
    }
}
