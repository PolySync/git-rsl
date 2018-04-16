extern crate kevlar_laces;
extern crate git2;

use std::process::{Command, Stdio};
use git2::Repository;

pub fn push(mut repo: &mut Repository, branch_name: &str) -> bool {
    kevlar_laces::run(&mut repo, &[branch_name], &"origin", &"push").is_ok()
}

fn merge(repo: &Repository, branch_name: &str) -> bool {
    Command::new("git").stdout(Stdio::null()).stderr(Stdio::null()).args(&["merge", &format!("origin/{}", branch_name)]).current_dir(repo.workdir().unwrap()).status().expect("failed to do git merge").success()
}

pub fn pull(mut repo: &mut Repository, branch_name: &str) -> bool {
    match kevlar_laces::run(&mut repo, &[branch_name], &"origin", &"fetch") {
        Ok(_) => merge(&repo, branch_name),
        Err(_) => false,
    }
}