use std::path::Path;
use std::env;
use std::fs;

use fs_extra::dir::*;
use fs_extra::error::*;

use git2::Repository;
use rand::{Rng, thread_rng};


pub fn setup() -> Repository  {
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
    let local_repo_name = format!("/tmp/rsl_test{}_local", suffix);
    let path_to_local_repo = Path::new(&local_repo_name);
    create_all(&path_to_local_repo, true).unwrap();
    let remote_url = format!("file://{}", &path_to_remote_repo.to_str().unwrap());
    Repository::clone(&remote_url, &path_to_local_repo).unwrap();

    // add local Nonce
    fixture_dir = env::current_dir().unwrap();
    &fixture_dir.push("fixtures/fixture.NONCE");
    fs::copy(fixture_dir, &path_to_local_repo.join(".git").join("NONCE")).unwrap();

    let local_repo = match Repository::open(&path_to_local_repo) {
        Ok(repo) => repo,
        Err(e) => panic!("setup failed: {:?}", e),
    };
    local_repo
}

pub fn teardown(repo: &Repository) -> Result<()> {
    let path = repo.path().parent().unwrap();
    match fs::remove_dir_all(&path) {
        Ok(()) => Ok(()),
        Err(e) => panic!("Teardown failed: {:?}", e),
    }
}
