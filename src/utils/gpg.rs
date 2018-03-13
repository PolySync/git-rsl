use errors::*;
use git2::Oid;
use gpgme::{Context, Protocol};
use std;
use std::io::prelude::*;

use std::fs::File;
use std::process::Command;
use tempfile::NamedTempFile;

pub fn verify_commit_signature(_oid: Oid) -> Result<()> {
    Ok(())
}

/// Signs with the provided key,
/// or else uses the default signing key
pub fn detached_sign(input: &str, _key_id: Option<&str>, gpghome: Option<&str>) -> Result<String> {
    let mut ctx = Context::from_protocol(Protocol::OpenPgp)?;

    // resolve gpg home in order of provided path, environment variable, default, or give up
    if let Some(path) = gpghome {
        ctx.set_engine_home_dir(path)?;
    } else if let Some(path) = find_gpg_home() {
        ctx.set_engine_home_dir(path)?;
    } else {
        bail!("couldn't generate signature; gpg home not set");
    }
    ctx.set_armor(true);

    let mut output = Vec::new();
    ctx.sign_detached(input, &mut output).chain_err(|| "gpg signing failed")?;
    // TODO this should always be valid utf8  if the ascii-armored signature succeeded and we get this far but still...get rid of this unwrap please
    let string_version = String::from_utf8(output).unwrap();

    Ok(string_version)
}

pub fn verify_detached_signature(sig: &str, buf: &str, gpghome: Option<&str>) -> Result<bool> {
    // create new context for operations
    let mut ctx = Context::from_protocol(Protocol::OpenPgp)?;

    // resolve gpg home in order of provided path, environment variable, default, or give up
    if let Some(path) = gpghome {
        ctx.set_engine_home_dir(path)?;
    } else if let Some(path) = find_gpg_home() {
        ctx.set_engine_home_dir(path)?;
    } else {
        bail!("couldn't generate signature; gpg home not set");
    }
    ctx.set_armor(true);
    ctx.verify_detached(sig, buf).chain_err(|| "gpg verification failed")?;

    // return true if we verified successfully
    Ok(true)
}

/// gpg2 --detach-sig <buf>
pub fn cli_detached_sign(buf: &str, gpghome: Option<&str>) -> Result<Vec<u8>> {

    // write content to be signed to temporary file
    let cwd = std::env::current_dir()?;
    let mut file = NamedTempFile::new_in(cwd)?;
    file.write_all(buf.as_bytes())?;

    let mut cmd = Command::new("gpg2");
    cmd.arg("--detach-sign");
    cmd.arg(file.path());

    // set gpghome if provided or search for default
    if let Some(path) = gpghome {
        cmd.env("GNUPGHOME", path);
    } else if let Some(path) = find_gpg_home() {
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
pub fn cli_verify_detached_signature(sig: &[u8], buf: &str, gpghome: Option<&str>) -> Result<bool> {
    let mut message_file = NamedTempFile::new()?;
    message_file.write_all(buf.as_bytes())?;
    let mut sig_file = NamedTempFile::new()?;
    sig_file.write_all(sig)?;

    let mut cmd = Command::new("gpg2");
    cmd.arg("--verify");
    cmd.arg(sig_file.path());
    cmd.arg(message_file.path());

    // set gpghome if provided or search for default
    if let Some(path) = gpghome {
        cmd.env("GNUPGHOME", path);
    } else if let Some(path) = find_gpg_home() {
        cmd.env("GNUPGHOME", path);
    }

    let status = cmd.status()
        .expect("failed to execute process");

    Ok(status.success()) // 0 exit code means verified
}

fn find_gpg_home() -> Option<String> {
    if let Ok(home) = std::env::var("GNUPGHOME") {
        Some(home)
    } else if let Some(path) = std::env::home_dir() {
        match path.join(".gnupg").into_os_string().into_string() {
            Ok(p) => Some(p),
            Err(_e) => None,
        }
    } else {
        None
    }
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
        let mut sig_data = String::new();
        let mut document = File::open("./fixtures/test.txt").unwrap();
        document.read_to_string(&mut doc_data).unwrap();
        let mut sig = File::open("./fixtures/test.txt.asc").unwrap();
        sig.read_to_string(&mut sig_data).unwrap();

        let result = super::verify_detached_signature(&sig_data, &doc_data, Some(&gpghome)).unwrap();
        assert!(result)
    }

    #[test]
    fn verify_bad_signature_fails() {
        // use test fixture Testy McTesterson's keyring
        let gpghome = "./fixtures/fixture.gnupghome";

        // get document and signature to be verified
        let mut doc_data = String::new();
        let mut sig_data = String::new();
        let mut document = File::open("./fixtures/test.txt").unwrap();
        document.read_to_string(&mut doc_data).unwrap();
        let mut sig = File::open("./fixtures/test.txt.asc").unwrap();
        sig.read_to_string(&mut sig_data).unwrap();

        // mess with signature
        sig_data = String::from("fhio2340929f3");

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
        let mut sig = File::open("./fixtures/test.txt.asc").unwrap();
        sig.read_to_end(&mut sig_data).unwrap();

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
