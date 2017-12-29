use std::path::Path;
use std::env;
use std::fs;

use fs_extra::dir::*;
use fs_extra::error::*;

use std::io::Read;

use git2::Repository;
use rand::{Rng, thread_rng};


pub fn setup() -> Result<Repository>  {
    let mut fixture_dir = env::current_dir().unwrap();
    &fixture_dir.push("fixtures/.git");
    let suffix: String = thread_rng().gen_ascii_chars().take(12).collect();
    let dir_name = format!("/tmp/rsl_test{}", suffix);
    let path_to = Path::new(&dir_name);
    create_all(&path_to, true)?;
    let mut options = CopyOptions::new();
    options.overwrite = true;

    copy(fixture_dir, path_to, &options)?;

    match Repository::open(&path_to) {
        Ok(repo) => Ok(repo),
        Err(e) => panic!("setup failed: {:?}", e),
    }
}

pub fn teardown(repo: &Repository) -> Result<()> {
    let path = repo.path().parent().unwrap();
    match fs::remove_dir_all(&path) {
        Ok(()) => Ok(()),
        Err(e) => panic!("Teardown failed: {:?}", e),
    }
}
