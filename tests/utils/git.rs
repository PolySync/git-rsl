use std::path::{Path, PathBuf};
use std::env;
use std::process::{Command, Stdio};
use git2::{Repository, BranchType};
use tempdir::TempDir;
use names::Generator;

fn set_gnupg_home() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let gnupg_fixture_path = fixtures_dir.join("fixture.gnupghome");

    env::set_var("GNUPGHOME", gnupg_fixture_path.to_str().expect("failed to set gnupghome"));
}

fn set_config(repo: &mut Repository, dev_num: usize) {
    set_gnupg_home();
    let mut config = repo.config().expect("failed to find config file");
    let dev_name = format!("dev-{}", dev_num);
    let dev_email = format!("{}@the-internets.com", dev_name);
    config.set_str("user.name", &dev_name).expect("failed to set user name");
    config.set_str("user.email", &dev_email).expect("failed to set user email");
    config.set_str("user.signingkey", "0A361BB1").expect("failed to set signing key");
    config.set_str("gpg.program", "gpg2").expect("failed to set gpg program");
}

pub fn clone(remote: &Repository, clone: usize) -> Repository {
    let temp_dir = remote.path().parent().expect("failed to get temp dir containing remote");

    let mut repo = Repository::clone(remote.path().to_str().expect("failed to stringify remote path"), temp_dir.join(format!("{}", clone))).expect("failed to clone remote repository");

    set_config(&mut repo, clone + 1);

    repo
}

#[allow(dead_code)]
pub fn log(repo: &Repository, branch_name: &str) {
    let output = Command::new("git").args(&["log", "--show-signature", branch_name]).current_dir(repo.path()).output().expect("failed to perform git commit");

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
}

pub fn commit(repo: &Repository, msg: &str) -> bool {
    Command::new("git").stdout(Stdio::null()).stderr(Stdio::null()).args(&["commit", "--allow-empty", "-S", "-m", msg]).current_dir(repo.workdir().expect("failed to get workdir of repo")).status().expect("failed to perform git commit").success()
}

pub fn push(repo: &Repository, branch_name: &str) -> bool {
    Command::new("git").stdout(Stdio::null()).stderr(Stdio::null()).args(&["push", "origin", branch_name]).current_dir(repo.workdir().expect("failed to get workdir of repo")).status().expect("failed to perform git push").success()
}

pub fn pull(repo: &Repository, branch_name: &str) -> bool {
    Command::new("git").stdout(Stdio::null()).stderr(Stdio::null()).args(&["pull", "--ff-only"]).current_dir(repo.workdir().expect("failed to get workdir of repo")).status().expect("failed to perform git pull").success()
}

pub fn branch(repo: &Repository, name: &str) -> bool {
    Command::new("git").stdout(Stdio::null()).stderr(Stdio::null()).args(&["branch", name]).current_dir(repo.workdir().expect("failed to get workdir of repo")).status().expect("failed to perform git branch").success()
}

pub fn checkout(repo: &Repository, branch_name: &str) -> bool {
    Command::new("git").stdout(Stdio::null()).stderr(Stdio::null()).args(&["checkout", branch_name]).current_dir(repo.workdir().expect("failed to get workdir of repo")).status().expect("failed to perform git checkout").success()
}