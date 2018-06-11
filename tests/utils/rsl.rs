extern crate git_rsl;
extern crate git2;

use std::process::{Command, Stdio};
use self::git_rsl::{BranchName, RemoteName};
use git2::Repository;

pub fn push(mut repo: &mut Repository, branch_name: &BranchName) -> bool {
    git_rsl::secure_push_with_cleanup(&mut repo, &RemoteName::new("origin"), branch_name).is_ok()
}

fn merge(repo: &Repository, branch_name: &BranchName) -> bool {
    Command::new("git").stdout(Stdio::null()).stderr(Stdio::null()).args(&["merge", &format!("origin/{}", branch_name.as_ref())]).current_dir(repo.workdir().unwrap()).status().expect("failed to do git merge").success()
}

pub fn pull(mut repo: &mut Repository, branch_name: &BranchName) -> bool {
    match git_rsl::secure_fetch_with_cleanup(&mut repo, &RemoteName::new("origin"), branch_name) {
        Ok(_) => merge(&repo, branch_name),
        Err(_) => false,
    }
}