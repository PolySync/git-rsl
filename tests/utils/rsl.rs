extern crate git_rsl;
extern crate git2;

use std::process::{Command, Stdio};
use git_rsl::errors::{Error, ErrorKind};
use git2::Repository;

const INVALID_FETCH_RSL: &str = "Couldn\'t fetch; No push entry for latest commit on target branch. It is likely that someone pushed without using git-rsl. Please have that developer secure-push the branch and try again.";

pub fn push(mut repo: &mut Repository, branch_name: &str) -> bool {
    match git_rsl::secure_push_with_cleanup(&mut repo, branch_name, &"origin") {
        Ok(()) => true,
        Err(error) => {
            match error {
                Error(ErrorKind::InvalidRSL, _) => false,
                Error(ErrorKind::Msg(msg), _) => {
                    if msg == String::from(INVALID_FETCH_RSL) {
                        false
                    } else {
                        panic!("Something broke and it didn't detect an invalid RSL error: {:?}", msg)
                    }
                },
                _ => panic!("RSL error without detection {:?}", error),
            }
        }
    }
}

fn merge(repo: &Repository, branch_name: &BranchName) -> bool {
    Command::new("git").stdout(Stdio::null()).stderr(Stdio::null()).args(&["merge", &format!("origin/{}", branch_name.as_ref())]).current_dir(repo.workdir().unwrap()).status().expect("failed to do git merge").success()
}

pub fn pull(mut repo: &mut Repository, branch_name: &str) -> bool {
    match git_rsl::secure_fetch_with_cleanup(&mut repo, branch_name, &"origin") {
        Ok(()) => merge(&repo, branch_name),
        Err(error) => {
            match error {
                Error(ErrorKind::InvalidRSL, _) => false,
                Error(ErrorKind::Msg(msg), _) => {
                    if msg == String::from(INVALID_FETCH_RSL) {
                        false
                    } else {
                        panic!("Something broke and it didn't detect an invalid RSL error: {:?}", msg)
                    }
                },
                _ => panic!("RSL error without detection {:?}", error),
            }
        }
    }
}