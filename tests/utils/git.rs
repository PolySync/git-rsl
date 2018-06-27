use git2::{BranchType, Repository};
use git_rsl::utils::test_helper::*;
use names::Generator;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn set_gnupg_home() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let gnupg_fixture_path = fixtures_dir.join("fixture.gnupghome");

    env::set_var(
        "GNUPGHOME",
        gnupg_fixture_path
            .to_str()
            .expect("failed to set gnupghome"),
    );
}

fn set_config(repo: &mut Repository, dev_num: usize) {
    set_gnupg_home();
    let mut config = repo.config().expect("failed to find config file");
    let dev_name = format!("dev-{}", dev_num);
    let dev_email = format!("{}@the-internets.com", dev_name);
    config
        .set_str("user.name", &dev_name)
        .expect("failed to set user name");
    config
        .set_str("user.email", &dev_email)
        .expect("failed to set user email");
    config
        .set_str("user.signingkey", "0A361BB1")
        .expect("failed to set signing key");
    config
        .set_str("gpg.program", "gpg2")
        .expect("failed to set gpg program");
}

pub fn clone(remote: &Repository, clone: usize) -> Repository {
    let temp_dir = remote
        .path()
        .parent()
        .expect("failed to get temp dir containing remote");

    let mut repo = Repository::clone(
        remote
            .path()
            .to_str()
            .expect("failed to stringify remote path"),
        temp_dir.join(format!("{}", clone)),
    ).expect("failed to clone remote repository");

    set_config(&mut repo, clone + 1);

    repo
}

#[allow(dead_code)]
pub fn log(repo: &Repository, branch_name: &str) {
    let output = Command::new("git")
        .args(&["log", "--show-signature", branch_name])
        .current_dir(repo.path())
        .output()
        .expect("failed to perform git commit");

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
}

pub fn commit(repo: &Repository, msg: &str) -> bool {
    Command::new("git")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(&["commit", "--allow-empty", "-S", "-m", msg])
        .current_dir(repo.workdir().expect("failed to get workdir of repo"))
        .status()
        .expect("failed to perform git commit")
        .success()
}

pub fn push(repo: &Repository, branch_name: &str) -> bool {
    Command::new("git")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(&["push", "origin", branch_name])
        .current_dir(repo.workdir().expect("failed to get workdir of repo"))
        .status()
        .expect("failed to perform git push")
        .success()
}

pub fn pull(repo: &Repository, branch_name: &str) -> bool {
    Command::new("git")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(&["pull", "--ff-only", "origin", branch_name])
        .current_dir(repo.workdir().expect("failed to get workdir of repo"))
        .status()
        .expect("failed to perform git pull")
        .success()
}

pub fn branch(repo: &Repository, name: &str) -> bool {
    Command::new("git")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(&["branch", name])
        .current_dir(repo.workdir().expect("failed to get workdir of repo"))
        .status()
        .expect("failed to perform git branch")
        .success()
}

pub fn checkout(repo: &Repository, branch_name: &str) -> bool {
    Command::new("git")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(&["checkout", branch_name])
        .current_dir(repo.workdir().expect("failed to get workdir of repo"))
        .status()
        .expect("failed to perform git checkout")
        .success()
}

#[test]
fn git_branch() {
    let mut context = setup_fresh();
    {
        let random_branch_name = Generator::default().next().expect("failed to get a name");

        assert!(branch(&context.local, &random_branch_name));

        assert!(
            context
                .local
                .find_branch(&random_branch_name, BranchType::Local)
                .is_ok()
        );
    }
}

#[test]
fn git_checkout() {
    let mut context = setup_fresh();
    {
        let random_master_commit_msg = Generator::default().next().expect("failed to get a name");
        commit(&context.local, &random_master_commit_msg);

        let random_branch_name = Generator::default().next().expect("failed to get a name");
        branch(&context.local, &random_branch_name);
        assert!(checkout(&context.local, &random_branch_name));
        let random_branch_commit_msg = Generator::default().next().expect("failed to get a name");
        commit(&context.local, &random_branch_commit_msg);

        let local = &context.local;

        let mut commit = local
            .head()
            .expect("failed to get head")
            .peel_to_commit()
            .expect("failed to get latest commit...");
        assert!(commit.summary() == Some(&random_branch_commit_msg));

        assert!(checkout(&context.local, "master"));
        commit = local
            .head()
            .expect("failed to get head")
            .peel_to_commit()
            .expect("failed to get latest commit...");
        assert!(commit.summary() == Some(&random_master_commit_msg));
    }
}

#[test]
fn git_commit() {
    let mut context = setup_fresh();
    {
        let random_commit_msg = Generator::default().next().expect("failed to get a name");

        assert!(commit(&context.local, &random_commit_msg));

        let commit = context
            .local
            .head()
            .expect("failed to get head")
            .peel_to_commit()
            .expect("failed to get latest commit...");

        assert!(commit.summary() == Some(&random_commit_msg));
    }
}

#[test]
fn git_push() {
    let mut context = setup_fresh();
    {
        commit(&context.local, "C2");

        let oid = context
            .local
            .head()
            .expect("failed to get head")
            .target()
            .expect("failed to get C2 oid...");
        assert!(&context.remote.find_commit(oid).is_err());

        push(&context.local, "master");
        assert!(&context.remote.find_commit(oid).is_ok());
    }
}

#[test]
fn git_pull() {
    let mut context = setup_fresh();
    {
        let mut locals = super::setup_local_repos(&context, 2);
        let local_1 = &mut locals.pop().expect("failed to pop repo");
        let local_2 = &mut locals.pop().expect("failed to pop repo");

        commit(local_1, "C2");
        let oid = local_1
            .head()
            .expect("failed to get head")
            .target()
            .expect("failed to get C2 oid...");
        assert!(&context.remote.find_commit(oid).is_err());

        push(local_1, "master");

        assert!(local_2.find_commit(oid).is_err());

        pull(local_2, "master");

        assert!(local_2.find_commit(oid).is_ok());
    }
}
