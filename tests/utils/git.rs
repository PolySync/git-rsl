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

fn set_config(repo: &Repository, dev_num: usize) {
    set_gnupg_home();
    let mut config = repo.config().expect("failed to find config file");
    let dev_name = format!("dev-{}", dev_num);
    let dev_email = format!("{}@the-internets.com", dev_name);
    config.set_str("user.name", &dev_name).expect("failed to set user name");
    config.set_str("user.email", &dev_email).expect("failed to set user email");
    config.set_str("user.signingkey", "0A361BB1").expect("failed to set signing key");
    config.set_str("gpg.program", "gpg2").expect("failed to set gpg program");
}

pub fn init(path: &Path) -> (Repository, Repository) {
    set_gnupg_home();

    let local = Repository::init(path.join("0")).expect("failed to create local repo");
    set_config(&local, 1);
    commit(&local, "Initial commit");

    let remote = Repository::init_bare(path.join("main.git")).expect("failed to create main repo");
    local.remote("origin", remote.path().to_str().expect("failed to stringify remote path")).expect("failed to remote");
    push(&local, "master");

    (remote, local)
}

pub fn clone(path_to_main: &Path, path_to_clone: &Path, dev_num: usize) -> Repository {
    let main_path_str = path_to_main.to_str().expect("failed to cast Path to string");

    let repo = Repository::clone(main_path_str, path_to_clone).expect("failed to clone remote repository");
    set_config(&repo, dev_num+1);

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
    Command::new("git").stdout(Stdio::null()).stderr(Stdio::null()).args(&["pull", "origin", branch_name]).current_dir(repo.workdir().expect("failed to get workdir of repo")).status().expect("failed to perform git pull").success()
}

pub fn branch(repo: &Repository, name: &str) -> bool {
    Command::new("git").stdout(Stdio::null()).stderr(Stdio::null()).args(&["branch", name]).current_dir(repo.workdir().expect("failed to get workdir of repo")).status().expect("failed to perform git branch").success()
}

pub fn checkout(repo: &Repository, branch_name: &str) -> bool {
    Command::new("git").stdout(Stdio::null()).stderr(Stdio::null()).args(&["checkout", branch_name]).current_dir(repo.workdir().expect("failed to get workdir of repo")).status().expect("failed to perform git checkout").success()
}

#[test]
fn git_branch() {
    let temp_dir = TempDir::new("test").expect("failed to create temporary directory");
    let (_, mut locals) = super::create_system_state(&temp_dir.path(), 1);
    let local = &mut locals.pop().expect("failed to pop repo");

    let random_branch_name = Generator::default().next().expect("failed to get a name");

    assert!(branch(local, &random_branch_name));

    assert!(local.find_branch(&random_branch_name, BranchType::Local).is_ok());
}

#[test]
fn git_checkout() {
    let temp_dir = TempDir::new("test").expect("failed to create temporary directory");
    let (_, mut locals) = super::create_system_state(&temp_dir.path(), 1);
    let local = &mut locals.pop().expect("failed to pop repo");

    let random_master_commit_msg = Generator::default().next().expect("failed to get a name");
    commit(local, &random_master_commit_msg);

    let random_branch_name = Generator::default().next().expect("failed to get a name");
    branch(local, &random_branch_name);
    assert!(checkout(local, &random_branch_name));
    let random_branch_commit_msg = Generator::default().next().expect("failed to get a name");
    commit(local, &random_branch_commit_msg);

    let mut commit = local.head().expect("failed to get head").peel_to_commit().expect("failed to get latest commit...");
    assert!(commit.summary() == Some(&random_branch_commit_msg));

    assert!(checkout(local, "master"));
    commit = local.head().expect("failed to get head").peel_to_commit().expect("failed to get latest commit...");
    assert!(commit.summary() == Some(&random_master_commit_msg));
}

#[test]
fn git_commit() {
    let temp_dir = TempDir::new("test").expect("failed to create temporary directory");
    let (_, mut locals) = super::create_system_state(&temp_dir.path(), 1);
    let local = &mut locals.pop().expect("failed to pop repo");

    let random_commit_msg = Generator::default().next().expect("failed to get a name");

    assert!(commit(local, &random_commit_msg));

    let commit = local.head().expect("failed to get head").peel_to_commit().expect("failed to get latest commit...");

    assert!(commit.summary() == Some(&random_commit_msg));
}

#[test]
fn git_push() {
    let temp_dir = TempDir::new("test").expect("failed to create temporary directory");
    let (main, mut locals) = super::create_system_state(&temp_dir.path(), 1);
    let local = &mut locals.pop().expect("failed to pop repo");

    commit(local, "C2");

    let oid = local.head().expect("failed to get head").target().expect("failed to get C2 oid...");
    assert!(main.find_commit(oid).is_err());

    push(local, "master");
    assert!(main.find_commit(oid).is_ok());
}

#[test]
fn git_pull() {
    let temp_dir = TempDir::new("test").expect("failed to create temporary directory");
    let (main, mut locals) = super::create_system_state(&temp_dir.path(), 2);
    let local_1 = &mut locals.pop().expect("failed to pop repo");
    let local_2 = &mut locals.pop().expect("failed to pop repo");

    commit(local_1, "C2");
    let oid = local_1.head().expect("failed to get head").target().expect("failed to get C2 oid...");
    assert!(main.find_commit(oid).is_err());

    push(local_1, "master");

    assert!(local_2.find_commit(oid).is_err());

    pull(local_2, "master");

    assert!(local_2.find_commit(oid).is_ok());
}