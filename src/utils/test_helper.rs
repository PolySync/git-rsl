use std::path::Path;
use std::env;
use std::fs::{self, File};
use std::str;
use std::ffi::OsStr;
use std::io::prelude::*;
use std::path::PathBuf;



use std::process::{Command, Output};

use super::git;

use fs_extra::dir::*;
use tempdir::TempDir;

use git2::{Repository, REPOSITORY_OPEN_BARE, Config};
use rand::{Rng, thread_rng};

pub struct Context {
    pub local: Repository,
    pub remote: Repository,
    pub repo_dir: PathBuf,
}

impl Context {
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
    let local_dir = TempDir::new("rsl_test").unwrap().into_path();

    // init git repo in temp directory
    let local = Repository::init(&local_dir).unwrap();

    // copy test config into local git repo dir
    let config_path = &local.path().join("config");
    fs::copy("fixtures/fixture.gitconfig", config_path).unwrap();

    let relative_path = Path::new("work.txt");
    {
        let file_path = local_dir.join(relative_path);
        let mut file = File::create(file_path).unwrap();
        file.write_all(b"some work").unwrap();
    }
    let _commit_id = git::add_and_commit(&local, Some(&relative_path), "Add example text file", "master").unwrap();

    // init bare remote repo with same state
    let remote_dir = format!("{}.git", &local_dir.to_str().unwrap());
    create_all(&remote_dir, true).unwrap();
    let remote = Repository::init_bare(&remote_dir).unwrap();

    let repo_dir = local_dir;

    // set remote origin to remote repo
    &local.remote("origin", &remote_dir);
    Context{local, remote, repo_dir}
}

pub fn teardown_fresh(context: Context) {
    rm_rf(context.local.path().parent().unwrap());
    rm_rf(context.remote.path());
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_config() {
        let context = setup_fresh();
        let cfg = Config::open(&context.local.path().join("config")).unwrap();
        let username = cfg.get_entry("user.username").unwrap();
        assert_eq!(username.value(), Some("idontexistanythingaboutthat"));
        teardown_fresh(context)
    }
}
