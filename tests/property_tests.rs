#[cfg(test)]
#[macro_use]
extern crate proptest;
extern crate git2;
extern crate git_rsl;
extern crate names;

#[macro_use]
extern crate lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref SEQUENTIAL_TEST_MUTEX: Mutex<()> = Mutex::new(());
}

mod utils;

use utils::model::{Action, Branch, Commit, Repo, State, Tool};

use git_rsl::utils::test_helper::*;
use git_rsl::{BranchName, RemoteName};
use proptest::prelude::*;
use proptest::sample::select;
use std::collections::HashMap;
use std::ops::Range;

pub fn repos(repo_count: Range<usize>) -> BoxedStrategy<Vec<Repo>> {
    prop::collection::vec(repo(), repo_count).boxed()
}

pub fn repo() -> BoxedStrategy<Repo> {
    let commits = vec![
        Commit {
            message: "Initial Commit".to_string(),
        },
    ];

    let mut branches = HashMap::new();
    branches.insert("master".to_string(), Branch { commits });

    Just(Repo {
        branches: branches,
        current_branch: "master".to_string(),
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
        })
        .boxed()
}

pub fn arb_valid_interactions(state: State, depth: usize) -> BoxedStrategy<State> {
    if depth == 0 {
        Just(state).boxed()
    } else {
        (arb_valid_interactions(state, depth - 1))
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

                (
                    Just(state),
                    Just(target_repo),
                    select(local_branches),
                    select(remote_branches),
                )
            })
            .prop_flat_map(move |values| {
                let (state, target_repo, target_local_branch, target_remote_branch) = values;

                let allowed_actions: Vec<Action> =
                    state.allowable_actions(target_repo, target_local_branch, target_remote_branch);

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
        _ => panic!("passed in action was not an attack..."),
    };

    (Just(attacked_branch))
        .prop_flat_map(move |attacked_branch| {
            let remote_actions = state.allowable_verification_actions(attacked_branch);

            (Just(state.clone()), select(remote_actions))
        })
        .prop_map(|values| {
            let (mut state, action) = values;

            state.apply(&action)
        })
        .boxed()
}

fn arb_attacked_state_history() -> BoxedStrategy<(State, Action)> {
    (
        arb_state(),
        (utils::NUM_STARTING_ACTIONS_LOW..utils::NUM_STARTING_ACTIONS_HIGH),
    ).prop_flat_map(|(state, depth)| arb_valid_interactions(state, depth))
        .prop_filter(
            "Can't inject attack on remote with no history".to_string(),
            |state| utils::repo_has_unique_state(&state.remote),
        )
        .prop_flat_map(|state| {
            (
                arb_attack(state),
                (utils::NUM_INTERMEDIATE_ACTIONS_LOW..utils::NUM_INTERMEDIATE_ACTIONS_HIGH),
            )
        })
        .prop_flat_map(|(state, depth)| {
            (
                arb_valid_interactions(state.clone(), depth),
                Just(state.clone().action.unwrap()),
            )
        })
        .prop_flat_map(|(state, attack)| {
            (
                arb_verification_interaction(state, attack.clone()),
                (utils::NUM_INTERMEDIATE_ACTIONS_LOW..utils::NUM_INTERMEDIATE_ACTIONS_HIGH),
                Just(attack.clone()),
            )
        })
        .prop_flat_map(|(state, depth, attack)| {
            (arb_valid_interactions(state.clone(), depth), Just(attack))
        })
        .no_shrink()
        .boxed()
}

/// These property-based tests create valid execution paths for a distributed
/// git system setup of one remote and some number of local clones. A model is
/// used to determine which actions are valid at each state. Proptest
/// strategies are then used to choose which action to apply, storing the
/// history of actions that were applied to the model. Using this set of states
/// (and the actions used to transition to them), we can set up an actual git
/// system in the same way as the model. This can then be used to test how well
/// git or rsl methods can detect attacks. These tests are ignored by default,
/// since the recursive strategy used to manipulate the model and hence build
/// up the action sequences can be time consuming. To run these tests, simply
/// run `cargo test -- --ignored`. You may optionally add the `--nocapture`
/// option for more verbose test failure information.
proptest!{
    #![proptest_config(ProptestConfig {
        cases: 5,
        .. ProptestConfig::default()
    })]

    #[test]
    #[ignore]
    fn git_fails_to_detect_attack((ref state, ref attack) in arb_attacked_state_history())
    {
        let _guard = SEQUENTIAL_TEST_MUTEX.lock();
        let actions: Vec<Action> = utils::collect_actions(state);

        let context = setup_fresh();
        {
            let mut locals = utils::setup_local_repos(&context, state.locals.len());

            let num_successful_actions = utils::apply_actions_to_system(
                &context.remote, &mut locals, &actions, Tool::Git);

            prop_assert!(num_successful_actions == actions.len(),
                            "git detected attack {:?} at {}: {:?}\n
                            command list: {:?}",
                            attack,
                            num_successful_actions,
                            actions[num_successful_actions-1],
                            actions);
        }
    }

    #[test]
    #[ignore]
    fn rsl_detects_attack((ref state, ref attack) in arb_attacked_state_history())
    {
        let _guard = SEQUENTIAL_TEST_MUTEX.lock();
        let actions: Vec<Action> = utils::collect_actions(state);

        let mut context = setup_fresh();
        {
            let remote_name = RemoteName::new("origin");

            git_rsl::rsl_init_with_cleanup(&mut context.local, &remote_name).expect("failed to init rsl");
            git_rsl::secure_push_with_cleanup(&mut context.local, &remote_name, &BranchName::new("master")).expect("failed to secure push initial commit");

            let mut locals = utils::setup_local_repos(&context, state.locals.len());

            let num_successful_actions = utils::apply_actions_to_system(
                &context.remote, &mut locals, &actions, Tool::RSL);

            prop_assert!(num_successful_actions < actions.len(),
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
        let _guard = SEQUENTIAL_TEST_MUTEX.lock();
        let actions: Vec<Action> = utils::collect_actions(state);

        let mut context = setup_fresh();
        let num_successful_git_actions = {
            let mut locals = utils::setup_local_repos(&context, state.locals.len());

            utils::apply_actions_to_system(
                &context.remote, &mut locals, &actions, Tool::Git)
        };

        context = setup_fresh();
        let num_successful_rsl_actions = {
            let remote_name = RemoteName::new("origin");
            git_rsl::rsl_init_with_cleanup(&mut context.local, &remote_name).expect("failed to init rsl");
            git_rsl::secure_push_with_cleanup(&mut context.local, &remote_name, &BranchName::new("master")).expect("failed to secure push initial commit");

            let mut locals = utils::setup_local_repos(&context, state.locals.len());

            utils::apply_actions_to_system(
                &context.remote, &mut locals, &actions, Tool::RSL)
        };

        prop_assert!(num_successful_git_actions >= num_successful_rsl_actions,
                "git detected attack faster than rsl: \n\t{:?}\n\t{:?}", attack, actions);
    }
}
