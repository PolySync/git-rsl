use std::path::Path;
use std::env;
use std::fs::{self, File};
use std::str;
use std::ffi::OsStr;
use std::io::prelude::*;


use std::process::{Command, Output};

use super::git;

use fs_extra::dir::*;
use tempdir::TempDir;

use git2::{Repository, REPOSITORY_OPEN_BARE};
use rand::{Rng, thread_rng};

pub struct Context {
    pub local: Repository,
    pub remote: Repository
}

impl Context {
    pub fn without_remote_rsl(&mut self) -> &mut Context {
        // delete RSL branch in remote
        let cmd = Command::new("git")
            .current_dir(self.remote.path().parent().unwrap())
            .arg("branch")
            .args(&["-D", "RSL"])
            .output().unwrap();
        if cmd.status.success() != true {
            panic!("{}", str::from_utf8(cmd.stderr.as_ref()).unwrap())
        };
        // prune RSL branch in local
        let cmd = Command::new("git")
            .current_dir(self.local.path().parent().unwrap())
            .arg("remote")
            .arg("prune")
            .arg("origin")
            .output().unwrap();
        if cmd.status.success() != true {
            panic!("{}", str::from_utf8(cmd.stderr.as_ref()).unwrap())
        };
        self
    }

    pub fn without_local_rsl(&mut self) -> &mut Context {
        //
        let cmd = Command::new("git")
            .current_dir(self.local.path().parent().unwrap())
            .arg("branch")
            .args(&["-D", "RSL"])
            .output().unwrap();
        if cmd.status.success() != true {
            panic!("{}", str::from_utf8(cmd.stderr.as_ref()).unwrap())
        };
        // remove remote tracking branch without deleting in origin
        let cmd = Command::new("git")
            .current_dir(self.local.path().parent().unwrap())
            .arg("branch")
            .args(&["-D", "-r", "origin/RSL"])
            .output().unwrap();
        if cmd.status.success() != true {
            panic!("{}", str::from_utf8(cmd.stderr.as_ref()).unwrap())
        };
        self
    }

    pub fn without_rsl(&mut self) -> &mut Context {
        self.without_local_rsl();
        self.without_remote_rsl();
        self
    }

    pub fn checkout(&mut self, branch: &str) -> &mut Context {
        let cmd = Command::new("git")
        .current_dir(self.local.path().parent().unwrap())
        .args(&["checkout", branch])
        .output().unwrap();
        if cmd.status.success() != true {
            panic!("{}", str::from_utf8(cmd.stderr.as_ref()).unwrap())
        }
        self
    }
}

pub fn setup_fresh() -> Context {
    // create temporary directory
    // init git repo in temp directory
    // init bare remote repo
    // set remote origin to remote repo
    let local_dir = TempDir::new("rsl_test").unwrap().into_path();
    let local = Repository::init(&local_dir).unwrap();

    let relative_path = Path::new("work.txt");
    {
        let file_path = local_dir.join(relative_path);
        let mut file = File::create(file_path).unwrap();
        file.write_all(b"some work").unwrap();
    }
    let _commit_id = git::add_and_commit(&local, Some(&relative_path), "Add example text file", "master").unwrap();
    let remote_dir = format!("{}.git", &local_dir.to_str().unwrap());
    create_all(&remote_dir, true).unwrap();
    let remote = Repository::init_bare(&remote_dir).unwrap();
    &local.remote("origin", &remote_dir);
    Context{local, remote}
}

pub fn teardown_fresh(context: Context) {
    rm_rf(context.local.path().parent().unwrap());
    rm_rf(context.remote.path());
}

pub fn setup() -> Context {
    let mut fixture_dir = env::current_dir().unwrap();
    &fixture_dir.push("fixtures/fixture.git");
    let suffix: String = thread_rng().gen_ascii_chars().take(12).collect();

    // create remote repo dir and copy .git from fixture
    let remote_repo_name = format!("/tmp/rsl_test{}_remote", suffix);
    let path_to_remote_repo = Path::new(&remote_repo_name);
    let git_dir = Path::new(".git");
    let full_path_to_git_dir = path_to_remote_repo.join(git_dir);
    let mut options = CopyOptions::new();
    options.overwrite = true;

    create_all(&full_path_to_git_dir, true).unwrap();
    copy(fixture_dir, path_to_remote_repo, &options).unwrap();

    let orig_name = path_to_remote_repo.join("fixture.git");
    fs::rename(orig_name.as_path(), full_path_to_git_dir).unwrap();


    // create local developer directory and clone repo from remote
    // TODO clone all branches
    let local_repo_name = format!("/tmp/rsl_test{}_local", suffix);
    let path_to_local_repo = Path::new(&local_repo_name);
    create_all(&path_to_local_repo, true).unwrap();
    let remote_url = format!("file://{}", &path_to_remote_repo.to_str().unwrap());
    Repository::clone(&remote_url, &path_to_local_repo).unwrap();

    // create local RSL branch from remote (ugh)
    let cmd = Command::new("git")
    .current_dir(path_to_local_repo)
    .args(&["branch", "RSL"])
    .args(&["--track", "origin/RSL"])
    .output().unwrap();
    if cmd.status.success() != true {
        panic!("{}", str::from_utf8(cmd.stderr.as_ref()).unwrap())
    }

    // add local Nonce
    fixture_dir = env::current_dir().unwrap();
    &fixture_dir.push("fixtures/fixture.NONCE");
    fs::copy(fixture_dir, &path_to_local_repo.join(".git").join("NONCE")).unwrap();

    let local = match Repository::open(&path_to_local_repo) {
        Ok(repo) => repo,
        Err(e) => panic!("setup failed: {:?}", e),
    };

    // open remote repo as bare
    let remote = open_bare_repository(&path_to_remote_repo.join(".git"));
    Context {local, remote}
}

pub fn teardown(context: Context) -> () {
    rm_rf(context.local.path().parent().unwrap());
    rm_rf(context.remote.path().parent().unwrap());
}

pub fn do_work_on_branch(repo: &Repository, branch_name: &str) -> () {
    git::checkout_branch(&repo, format!("refs/heads/{}", branch_name).as_str()).unwrap();
    git::add_and_commit(&repo, None, "a commit with some work", branch_name).unwrap();
}

fn open_bare_repository<P>(path: P) -> Repository
    where P: AsRef<Path>, P: AsRef<OsStr> {
    Repository::open_ext(&path, REPOSITORY_OPEN_BARE,  &[] as &[&OsStr]).unwrap()
}

fn rm_rf(path: &Path) -> () {
    fs::remove_dir_all(&path).unwrap();
    ()
}
