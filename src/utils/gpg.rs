use git2::{Oid};
use errors::*;

pub fn verify_signature(_oid: Oid) -> Result<()> {
    return Ok(())
}


pub fn detached_sign(buf: &str, key_id: &str) -> (&str) {
    // gpg2 --detach-sig buf
}

pub fn detached_signature_verify(buf: &str, sig: &str) -> bool {
    // gpg2 --verify sig doc
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn verify_signature() {
        assert!(super::detached_signature_verify(buf, sig))
    }
}
