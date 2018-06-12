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

pub enum FailureType {
    Detection,
    Other,
}

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