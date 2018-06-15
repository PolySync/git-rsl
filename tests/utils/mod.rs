pub mod attack;
pub mod git;
pub mod model;
pub mod rsl;

use git2::Repository;
use git_rsl::utils::test_helper::*;
use utils::model::{Action, Repo, State, Tool};

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

pub fn setup_local_repos(context: &Context, num_clones: usize) -> Vec<Repository> {
    let first_clone = Repository::open(context.local.path())
        .expect("failed to open local repository from context");

    git::commit(&first_clone, "Initial commit");
    git::push(&first_clone, "master");

    let mut locals = vec![first_clone];

    for i in 1..num_clones {
        let clone = git::clone(&context.remote, i);
        locals.push(clone);
    }

    locals
}

pub fn apply_actions_to_system(
    remote: &Repository,
    locals: &mut Vec<Repository>,
    actions: &Vec<Action>,
    tool: Tool,
) -> usize {
    let mut action_allowed = true;
    let mut num_allowed_actions = 0;

    for action in actions {
        if action_allowed {
            action_allowed = action.apply(remote, locals, tool);
            num_allowed_actions += 1;
        } else {
            break;
        }
    }

    num_allowed_actions
}
