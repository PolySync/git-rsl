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
    let repo_name = format!("/tmp/rsl_test{}", suffix);
    let path_to_repo = Path::new(&repo_name);
    let git_dir = Path::new(".git");
    let full_path_to_git_dir = path_to_repo.join(git_dir);
    let mut options = CopyOptions::new();
    options.overwrite = true;

    create_all(&full_path_to_git_dir, true).unwrap();
    copy(fixture_dir, path_to_repo, &options).unwrap();

    let orig_name = path_to_repo.join("fixture.git");
    fs::rename(orig_name.as_path(), full_path_to_git_dir).unwrap();

    let repo = match Repository::open(&path_to_repo) {
        Ok(repo) => repo,
        Err(e) => panic!("setup failed: {:?}", e),
    };
    repo
}

pub fn teardown(repo: &Repository) -> Result<()> {
    let path = repo.path().parent().unwrap();
    match fs::remove_dir_all(&path) {
        Ok(()) => Ok(()),
        Err(e) => panic!("Teardown failed: {:?}", e),
    }
}
