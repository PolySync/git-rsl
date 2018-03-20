use std::path::Path;
use std::env;
use std::fs::{self, File};
use std::str;
use std::io::prelude::*;
use std::path::PathBuf;

use super::git;

use fs_extra::dir::*;
use tempdir::TempDir;

use git2::Repository;

pub struct Context {
    pub local: Repository,
    pub remote: Repository,
    pub repo_dir: PathBuf,
}

pub fn setup_fresh() -> Context {
    // create temporary directory
    let local_dir = TempDir::new("rsl_test").unwrap().into_path();

    // init git repo in temp directory
    let local = Repository::init(&local_dir).unwrap();

    // copy test config into local git repo dir
    let config_path = &local.path().join("config");
    fs::copy("fixtures/fixture.gitconfig", config_path).unwrap();

    // set gpghome for this process
    let gnupghome = env::current_dir()
        .unwrap()
        .join("fixtures/fixture.gnupghome");
    env::set_var("GNUPGHOME", gnupghome.to_str().unwrap());

    // add and commit some work
    let relative_path = Path::new("work.txt");
    let absolute_path = &local.path().parent().unwrap().join(&relative_path);
    create_file_with_text(&absolute_path, &"some work");
    let _commit_id = git::add_and_commit(
        &local,
        Some(&relative_path),
        "Add example text file",
        "master",
    ).unwrap();

    // init bare remote repo with same state
    let remote_dir = format!("{}.git", &local_dir.to_str().unwrap());
    create_all(&remote_dir, true).unwrap();
    let remote = Repository::init_bare(&remote_dir).unwrap();

    let repo_dir = local_dir;

    // set remote origin to remote repo
    &local.remote("origin", &remote_dir);
    Context {
        local,
        remote,
        repo_dir,
    }
}

pub fn create_file_with_text<P: AsRef<Path>>(path: P, text: &str) -> () {
    //let file_path = path.as_path();
    let mut file = File::create(path.as_ref()).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

pub fn teardown_fresh(context: Context) {
    rm_rf(context.local.path().parent().unwrap());
    rm_rf(context.remote.path());
}

pub fn do_work_on_branch(repo: &Repository, branch_name: &str) -> () {
    git::checkout_branch(&repo, format!("refs/heads/{}", branch_name).as_str()).unwrap();
    git::add_and_commit(&repo, None, "a commit with some work", branch_name).unwrap();
}

fn rm_rf(path: &Path) -> () {
    fs::remove_dir_all(&path).unwrap();
    ()
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Config;

    #[test]
    fn setup_config() {
        let context = setup_fresh();
        let cfg = Config::open(&context.local.path().join("config")).unwrap();
        let username = cfg.get_entry("user.username").unwrap();
        assert_eq!(username.value(), Some("idontexistanythingaboutthat"));
        teardown_fresh(context)
    }
}
