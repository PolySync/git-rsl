pub mod git;
pub mod attack;
pub mod model;
pub mod rsl;

use git2::Repository;
use std::path::Path;
use utils::model::{State, Action, Repo};
use tempdir::TempDir;

pub const NUM_STARTING_ACTIONS_LOW: usize = 5;
pub const NUM_STARTING_ACTIONS_HIGH: usize = 10;
pub const NUM_INTERMEDIATE_ACTIONS_LOW: usize = 2;
pub const NUM_INTERMEDIATE_ACTIONS_HIGH: usize = 5;

pub fn repo_has_unique_state(repo: &Repo) -> bool {
    repo.branches["master"].commits.len() > 1 || repo.branches.len() > 1
}

pub fn collect_actions(state: &State) -> Vec<Action> {
    let mut actions: Vec<Action> = Vec::new();

    let mut s = Box::new(state.clone());

    while s.prev_state.is_some() {
        actions.push(s.clone().action.unwrap());
        s = s.clone().prev_state.unwrap();
    }

    actions.reverse();

    actions
}

pub fn create_system_state(temp_dir: &Path, local_count: usize) -> (Repository, Vec<Repository>) {
    let (remote, local) = git::init(&temp_dir);

    let mut locals: Vec<Repository> = Vec::new();
    locals.push(local);

    for i in 1..local_count {
        let local_path = &temp_dir.join(format!("{}", i));
        let local = git::clone(&remote.path(), local_path, i);
        locals.push(local);
    }

    (remote, locals)
}

#[test]
fn check_system_state() {
    let temp_dir = TempDir::new("test").expect("failed to create temporary directory");
    let (main, mut locals) = create_system_state(&temp_dir.path(), 2);
    let local_2 = &mut locals.pop().expect("failed to pop repo");
    let local_1 = &mut locals.pop().expect("failed to pop repo");

    assert!(git::commit(&local_1, "some words here"));
    assert!(git::push(&local_1, "master"));
    assert!(git::pull(&local_2, "master"));
    assert!(git::commit(&local_2, "here are more words"));
    assert!(git::push(&local_2, "master"));
    assert!(git::pull(&local_1, "master"));

    git::log(&main, "master");

    let temp_dir_2 = TempDir::new("test").expect("failed to create temporary directory");
    let (main_2, mut locals_2) = create_system_state(&temp_dir_2.path(), 2);
    let local_2_2 = &mut locals_2.pop().expect("failed to pop repo");
    let local_2_1 = &mut locals_2.pop().expect("failed to pop repo");

    assert!(git::commit(&local_2_1, "main 222222222222222"));
    assert!(git::push(&local_2_1, "master"));
    assert!(git::pull(&local_2_2, "master"));
    assert!(git::commit(&local_2_2, "more main 22222222222"));
    assert!(git::push(&local_2_2, "master"));
    assert!(git::pull(&local_2_1, "master"));

    git::log(&main_2, "master");
}