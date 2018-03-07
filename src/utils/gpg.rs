use errors::*;
use git2::Oid;
use gpgme::{Context, Protocol};
use std;
use std::io::prelude::*;

use std::fs::File;
use std::process::Command;
use tempfile::NamedTempFile;

pub fn verify_commit_signature(_oid: Oid) -> Result<()> {
    return Ok(())
}

/// Signs with the provided key,
/// or else uses the default signing key
pub fn detached_sign(input: &str, key_id: Option<&str>, gpghome: Option<&str>) -> Result<Vec<u8>> {
    let mut ctx = Context::from_protocol(Protocol::OpenPgp)?;
    if let Some(path) = gpghome {
        ctx.set_engine_home_dir(path);
    }

    let mut output = Vec::new();
    let result = ctx.sign_detached(input, &mut output).chain_err(|| "gpg signing failed")?;

    Ok(output)
}

pub fn verify_detached_signature(sig: &Vec<u8>, buf: &str, gpghome: Option<&str>) -> Result<bool> {
    // create new context for operations
    let mut ctx = Context::from_protocol(Protocol::OpenPgp)?;
    if let Some(path) = gpghome {
        ctx.set_engine_home_dir(path);
    }
    let result = ctx.verify_detached(sig, buf).chain_err(|| "gpg verification failed")?;

    // TODO what settings do you need for the context?
    Ok(true)
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

    // read contents of sigfile to buffer and delete file
    let mut buffer = Vec::new();
    let sig_path = file.path().with_extension("sig");
    let mut signature_file = File::open(&sig_path)?;
    signature_file.read_to_end(&mut buffer)?;
    std::fs::remove_file(&sig_path)?;

    Ok(buffer)
}

// gpg2 --verify sig doc
pub fn cli_verify_detached_signature(sig: &Vec<u8>, buf: &str, gpghome: Option<&str>) -> Result<bool> {
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
    fn detached_sign() {
        // sign as test user Testy McTesterson
        let gpghome = "./fixtures/fixture.gnupghome";

        // get doc to sign as a string
        let mut doc_data = String::new();
        let mut document = File::open("./fixtures/test.txt").unwrap();
        document.read_to_string(&mut doc_data).unwrap();

        let signature = super::detached_sign(&doc_data, None, Some(&gpghome)).unwrap();

        let result = super::verify_detached_signature(&signature, &doc_data, Some(&gpghome)).unwrap();
        assert!(result);
    }

    #[test]
    fn verify_detached_signature() {
        // use test fixture Testy McTesterson's keyring
        let gpghome = "./fixtures/fixture.gnupghome";

        // get document and signature to be verified
        let mut doc_data = String::new();
        let mut sig_data = Vec::new();
        let mut document = File::open("./fixtures/test.txt").unwrap();
        document.read_to_string(&mut doc_data).unwrap();
        let mut sig = File::open("./fixtures/test.txt.sig").unwrap();
        sig.read_to_end(&mut sig_data);

        let result = super::verify_detached_signature(&sig_data, &doc_data, Some(&gpghome)).unwrap();
        assert!(result)
    }

    #[test]
    fn verify_bad_signature_fails() {
        // use test fixture Testy McTesterson's keyring
        let gpghome = "./fixtures/fixture.gnupghome";

        // get document and signature to be verified
        let mut doc_data = String::new();
        let mut sig_data = Vec::new();
        let mut document = File::open("./fixtures/test.txt").unwrap();
        document.read_to_string(&mut doc_data).unwrap();
        let mut sig = File::open("./fixtures/test.txt.sig").unwrap();
        sig.read_to_end(&mut sig_data);

        // mess with signature
        sig_data[10] += 1;
        sig_data = vec![0,1,2,3];

        let result = super::verify_detached_signature(&sig_data, &doc_data, Some(&gpghome));
        assert!(result.is_err())
    }

    #[test]
    fn cli_verify_detached_signature() {
        // use test fixture Testy McTesterson's keyring
        let gpghome = "./fixtures/fixture.gnupghome";

        // get document and signature to be verified
        let mut doc_data = String::new();
        let mut sig_data = Vec::new();
        let mut document = File::open("./fixtures/test.txt").unwrap();
        document.read_to_string(&mut doc_data).unwrap();
        let mut sig = File::open("./fixtures/test.txt.sig").unwrap();
        sig.read_to_end(&mut sig_data);

        let result = super::cli_verify_detached_signature(&sig_data, &doc_data, Some(&gpghome)).unwrap();
        assert!(result)
    }

    #[test]
    fn cli_detached_sign() {
        // sign as test user Testy McTesterson
        let gpghome = "./fixtures/fixture.gnupghome";

        // get doc to sign as a string
        let mut doc_data = String::new();
        let mut document = File::open("./fixtures/test.txt").unwrap();
        document.read_to_string(&mut doc_data).unwrap();

        let signature = super::cli_detached_sign(&doc_data, Some(&gpghome)).unwrap();

        let result = super::cli_verify_detached_signature(&signature, &doc_data, Some(&gpghome)).unwrap();
        assert!(result);
    }

    #[test]
    fn cli_sign_and_verify() {
        // sign as test user Testy McTesterson
        let gpghome = "./fixtures/fixture.gnupghome";

        let data = "some data to sign";
        let signature = super::cli_detached_sign(&data, Some(&gpghome)).unwrap();
        let valid = super::cli_verify_detached_signature(&signature, &data, Some(&gpghome)).unwrap();
        assert!(valid)
    }
}
