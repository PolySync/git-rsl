#[cfg(test)]
#[macro_use] extern crate proptest;
extern crate git_rsl;
extern crate git2;
extern crate tempdir;
extern crate names;

mod utils;

use utils::model::{Repo, Branch, Commit, Action, State, Tool};

use proptest::sample::select;
use proptest::prelude::*;
use std::ops::Range;
use std::collections::HashMap;
use tempdir::TempDir;
use git_rsl::utils::test_helper::*;
use git_rsl::{BranchName, RemoteName};

pub fn repos(repo_count: Range<usize>) -> BoxedStrategy<Vec<Repo>> {
    prop::collection::vec(repo(), repo_count).boxed()
}

pub fn repo() -> BoxedStrategy<Repo> {
    let commits = vec![Commit { message: "Initial Commit".to_string() }];

    let mut branches = HashMap::new();
    branches.insert("master".to_string(), Branch { commits });

    Just(Repo {
        branches: branches,
        current_branch: "master".to_string()
    }).boxed()
}

pub fn arb_state() -> BoxedStrategy<State> {
    (repo(), repos(2..5))
    .prop_map(|state| {
        let (remote, locals) = state;
        State {
            remote,
            locals,
            action: None,
            prev_state: None,
        }
    }).boxed()
}

pub fn arb_valid_interactions(state: State, depth: usize) -> BoxedStrategy<State> {
    if depth == 0 {
        Just(state).boxed()
    } else {  
        (arb_valid_interactions(state, depth-1))
        .prop_flat_map(|state| {
            let target_repo = state.locals.len();

            (Just(state), 0..target_repo)
        })
        .prop_flat_map(|values| {
            let (state, target_repo) = values;

            let mut local_branches = Vec::new();
            let mut remote_branches = Vec::new();

            for branch in state.remote.branches.keys() {
                remote_branches.push(branch.clone());
            }
            for branch in state.locals[target_repo].branches.keys() {
                local_branches.push(branch.clone());
            }

            (Just(state), Just(target_repo), select(local_branches), select(remote_branches))
        })
        .prop_flat_map(move |values| {
            let (state, target_repo, target_local_branch, target_remote_branch) = values;

            let allowed_actions: Vec<Action> = state.allowable_actions(target_repo, target_local_branch, target_remote_branch);

            (select(allowed_actions), Just(state))
        })
        .prop_map(|mut values| {
            let (ref action, ref mut state) = values;
            state.apply(action)
        })
        .boxed()
    }
}

pub fn arb_attack(state: State) -> BoxedStrategy<State> {
    Just(state.clone())
    .prop_flat_map(|values| {
        let state = values;

        let attack_actions = state.allowable_attacks();

        (Just(state), select(attack_actions))
    })
    .prop_map(|values| {
        let (mut state, attack_action) = values;

        state.apply(&attack_action)
    })
    .boxed()
}

pub fn arb_verification_interaction(state: State, attack: Action) -> BoxedStrategy<State> {
    let attacked_branch = match attack {
        Action::Teleport(branch, _) => branch,
        Action::Deletion(branch) => branch,
        Action::Rollback(branch) => branch,
        _ => panic!("passed in action was not an attack...")
    };

    (Just(attacked_branch))
    .prop_flat_map(move |attacked_branch| {
        let remote_actions = state.allowable_verification_actions(attacked_branch);

        (Just(state.clone()), select(remote_actions))
    })
    .prop_map(|values| {
        let (mut state, action) = values;

        state.apply(&action)
    }).boxed()
}

fn arb_attacked_state_history() -> BoxedStrategy<(State, Action)> {
    (arb_state(), (utils::NUM_STARTING_ACTIONS_LOW..utils::NUM_STARTING_ACTIONS_HIGH))
    .prop_flat_map(|(state, depth)| arb_valid_interactions(state, depth))
    .prop_filter("Can't inject attack on remote with no history".to_string(), 
                    |state| utils::repo_has_unique_state(&state.remote))
    .prop_flat_map(|state| (arb_attack(state), (utils::NUM_INTERMEDIATE_ACTIONS_LOW..utils::NUM_INTERMEDIATE_ACTIONS_HIGH)))
    .prop_flat_map(|(state, depth)| (arb_valid_interactions(state.clone(), depth), Just(state.clone().action.unwrap())))
    .prop_flat_map(|(state, attack)| (arb_verification_interaction(state, attack.clone()),       
                    (utils::NUM_INTERMEDIATE_ACTIONS_LOW..utils::NUM_INTERMEDIATE_ACTIONS_HIGH),
                    Just(attack.clone())))
    .prop_flat_map(|(state, depth, attack)| (arb_valid_interactions(state.clone(), depth), Just(attack)))
    .no_shrink()
    .boxed()
}

/// These property-based tests create valid execution paths for a distributed git system setup of one remote and some number of local clones.
/// A model is used to determine which actions are valid at each state. Proptest strategies are then used to choose which action to apply, storing the history of actions that were applied to the model.
/// Using this set of states (and the actions used to transition to them), we can set up an actual git system in the same way as the model. This can then be used to test how well git or rsl methods can detect attacks.
/// These tests are ignored by default, since the recursive strategy used to manipulate the model and hence build up the action sequences can be time consuming. 
/// To run these tests, simply run `cargo test -- --ignored`. You may optionally add the `--nocapture` option for more verbose test failure information.
proptest!{
    #![proptest_config(ProptestConfig {
    cases: 5, .. ProptestConfig::default()
    })]
    #[test] 
    #[ignore]
    fn git_fails_to_detect_attack((ref state, ref attack) in arb_attacked_state_history()) 
    {
        let actions: Vec<Action> = utils::collect_actions(state);

        let mut context = setup_fresh();
        {
            let mut action_allowed = true;
            let mut num_allowed_actions = 0;

            let mut locals = vec![context.local];

            for i in 1..state.locals.len() {
                let mut local = utils::git::clone(&context.remote, i);
                locals.push(local)
            }

            for action in &actions {
                if action_allowed {
                        action_allowed = action.apply(&context.remote, &mut locals, Tool::Git);
                        num_allowed_actions += 1;
                } else {
                    break;
                }
            }

            prop_assert!(action_allowed == true, 
                            "git detected attack {:?} at {}: {:?}\n
                            command list: {:?}", 
                            attack,
                            num_allowed_actions, 
                            actions[num_allowed_actions-1], 
                            actions);
        }
    }

    #[test] 
    #[ignore]
    fn rsl_detects_attack((ref state, ref attack) in arb_attacked_state_history()) 
    {
        let actions: Vec<Action> = utils::collect_actions(state);

        let mut context = setup_fresh();
        {
            let mut action_allowed = true;

            let remote_name = RemoteName::new("origin");

            git_rsl::rsl_init_with_cleanup(&mut context.local, &remote_name).expect("failed to init rsl");
            git_rsl::secure_push_with_cleanup(&mut context.local, &remote_name, &BranchName::new("master")).expect("failed to secure push initial commit");


            let mut locals = vec![context.local];

            for i in 1..state.locals.len() {
                let mut local = utils::git::clone(&context.remote, i);
                locals.push(local)
            }

            for action in &actions {
                if action_allowed {
                        action_allowed = action.apply(&context.remote, &mut locals, Tool::RSL);
                } else {
                    break;
                }
            }

            prop_assert!(action_allowed == false, 
                            "rsl failed to detect attack {:?}\n
                            command list: {:?}", 
                            attack,
                            actions);
        }
    }

    #[test] 
    #[ignore]
    fn rsl_detects_before_git((ref state, ref attack) in arb_attacked_state_history()) 
    {
        let actions: Vec<Action> = utils::collect_actions(state);

        let mut git_context = setup_fresh();
        let mut rsl_context = setup_fresh();
        {
            let mut git_command_allowed = true;
            let mut git_num_allowed_actions = 0;
            utils::git::commit(&git_context.local, "Initial commit");
            utils::git::push(&git_context.local, "master");

            let mut git_locals = vec![git_context.local];

            for i in 1..state.locals.len() {
                let mut local = utils::git::clone(&git_context.remote, i);
                git_locals.push(local)
            }

            for action in &actions {
                if git_command_allowed {
                        git_command_allowed = action.apply(&git_context.remote, &mut git_locals, Tool::Git);
                    git_num_allowed_actions += 1;
                }
            }

            let remote_name = RemoteName::new("origin");

            git_rsl::rsl_init_with_cleanup(&mut rsl_context.local, &remote_name).expect("failed to init rsl");
            git_rsl::secure_push_with_cleanup(&mut rsl_context.local, &remote_name, &BranchName::new("master")).expect("failed to secure push initial commit");

            let mut rsl_locals = vec![rsl_context.local];

            for i in 1..state.locals.len() {
                let mut local = utils::git::clone(&rsl_context.remote, i);
                rsl_locals.push(local)
            }

            let mut rsl_command_allowed = true;
            let mut rsl_num_allowed_actions = 0;

            for action in &actions {
                if rsl_command_allowed {
                        rsl_command_allowed = action.apply(&rsl_context.remote, &mut rsl_locals, Tool::RSL);
                    rsl_num_allowed_actions += 1;
                }
            }

            prop_assert!(git_num_allowed_actions >= rsl_num_allowed_actions,
                "git detected attack faster than rsl: \n\t{:?}\n\t{:?}", attack, actions);
        }
    }
}