use errors::*;
use git2::Oid;
use gpgme::{Context, Protocol};
use std;
use std::io::prelude::*;

use std::fs::File;
use std::process::Command;
use tempfile::NamedTempFile;

pub fn verify_signature(_oid: Oid) -> Result<()> {
    return Ok(())
}

/// Signs with the provided key,
/// or else uses the default signing key
pub fn detached_sign(buf: &str, key_id: Option<&str>) -> Result<String> {
    Ok(String::from("signature"))
}

pub fn verify_detatched() -> Result<bool> {
    // create new context for operations
    let ctx = Context::from_protocol(Protocol::OpenPgp)?;
    // TODO what settings do you need for the context?
    Ok(false)
    // verify
}

/// gpg2 --detach-sig <buf>
pub fn cli_detached_sign(buf: &str, gpghome: Option<&str>) -> Result<Vec<u8>> {
    // write content to be signed to temporary file
    let cwd = std::env::current_dir()?;
    let mut file = NamedTempFile::new_in(cwd)?;
    file.write_all(&buf.as_bytes())?;

    let mut cmd = Command::new("gpg2");

    cmd.arg("--detach-sign");
    cmd.arg(file.path());

    // add gpghome if using other than default
    if let Some(path) = gpghome {
        cmd.env("GNUPGHOME", path);
    }

    if !cmd.status()?.success() {
        bail!("unable to generate signature")
    }

    let mut buffer = Vec::new();
    let mut signature_file = File::open(file.path().join(".sig"))?;
    signature_file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

// gpg2 --verify sig doc
pub fn cli_detached_signature_verify(buf: &str, sig: &Vec<u8>, gpghome: Option<&str>) -> Result<bool> {
    let mut message_file = NamedTempFile::new()?;
    message_file.write_all(&buf.as_bytes());
    let mut sig_file = NamedTempFile::new()?;
    sig_file.write_all(&sig);

    let mut cmd = Command::new("gpg2");
    cmd.arg("--verify");
    cmd.arg(sig_file.path());
    cmd.arg(message_file.path());

    if let Some(path) = gpghome {
        cmd.env("GNUPGHOME", path);
    }

    let status = cmd.status()
        .expect("failed to execute process");

    Ok(status.success()) // 0 exit code means verified
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::prelude::*;

    #[test]
    fn cli_detached_verify_signature() {
        // use test fixture Testy McTesterson's keyring
        let gpghome = "./fixtures/fixture.gnupghome";

        // get document and signature to be verified
        let mut doc_data = String::new();
        let mut sig_data = Vec::new();
        let mut document = File::open("./fixtures/test.txt").unwrap();
        document.read_to_string(&mut doc_data).unwrap();
        let mut sig = File::open("./fixtures/test.txt.sig").unwrap();
        sig.read_to_end(&mut sig_data);

        let result = super::cli_detached_signature_verify(&doc_data, &sig_data, Some(&gpghome)).unwrap();
        assert!(result)
    }

    fn cli_detached_sign() {
        // sign as test user Testy McTesterson
        let gpghome = "./fixtures/fixture.gnupghome";

        // get premade sig for comparison
        let mut sig_data = Vec::new();
        let mut sig = File::open("./fixtures/test.txt.sig").unwrap();
        sig.read_to_end(&mut sig_data);

        // get doc to sign as a string
        let mut doc_data = String::new();
        let mut document = File::open("./fixtures/test.txt").unwrap();
        document.read_to_string(&mut doc_data).unwrap();

        let signature = super::cli_detached_sign(&doc_data, Some(&gpghome)).unwrap();
        assert_eq!(signature, sig_data)
    }
}
