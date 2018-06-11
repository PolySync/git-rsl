extern crate git_rsl;
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;
use std::fmt;
use git2::Repository;
use self::git_rsl::BranchName;
use names::Generator;

use super::{git, attack, rsl};

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Commit {
    pub message: String
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Branch {
    pub commits: Vec<Commit>
}

#[derive(PartialEq, Debug, Clone)]
pub struct Repo {
    pub branches: HashMap<String, Branch>,
    pub current_branch: String,
}

impl Repo {  
    pub fn add_branch(&mut self, name: &str) {
        let new_branch = Branch {
            commits: self.get_current_branch().commits.clone()
        };
        self.branches.insert(name.to_string(), new_branch);
    }

    pub fn commit(&mut self, message: &str) {
        let current_branch = self.current_branch.clone();
        let branch = self.branches.get_mut(&current_branch).expect("failed to get mutable ref to branch");
        branch.commits.push(Commit { message: message.to_string() });
    }

    pub fn get_current_branch(&self) -> &Branch {
        &self.branches[&self.current_branch]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tool {
    Git,
    RSL,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Commit(usize, String),
    Push(usize, String),
    Pull(usize, String),
    Checkout(usize, String),
    Branch(usize, String),
    Teleport(String, String),
    Rollback(String),
    Deletion(String),
}

impl Action {
    pub fn apply(&self, remote: &Repository, locals: &mut Vec<Repository>, tool: Tool) -> bool {
        match self {
           &Action::Commit(repo_num, ref message) => {
                git::commit(&locals[repo_num], message)
           },
           &Action::Push(repo_num, ref branch) => {
                match tool {
                    Tool::Git => git::push(&locals[repo_num], branch),
                    Tool::RSL => rsl::push(&mut locals[repo_num], &BranchName::new(branch)),
                }
           },
           &Action::Pull(repo_num, ref branch) => {
                match tool {
                    Tool::Git => git::pull(&locals[repo_num], branch),
                    Tool::RSL => rsl::pull(&mut locals[repo_num], &BranchName::new(branch)),
                }
           },
           &Action::Branch(repo_num, ref name) => {
                git::branch(&locals[repo_num], name)
           },
           &Action::Checkout(repo_num, ref branch) => {
                git::checkout(&locals[repo_num], branch)
           },
           &Action::Teleport(ref target_branch, ref teleport_target) => {
                attack::teleport(remote, target_branch, teleport_target)
           },
           &Action::Rollback(ref target_branch) => {
                attack::rollback(remote, target_branch)
           },
           &Action::Deletion(ref target_branch) => {
                attack::deletion(remote, target_branch)
           }
        }
    }
}

pub fn commits_as_hash_set(commits: &Vec<Commit>) -> HashSet<Commit> {
    HashSet::from_iter(commits.iter().cloned())
}

pub fn ff_possible(from: &Vec<Commit>, onto: &Vec<Commit>) -> bool {
    let f_set = commits_as_hash_set(from);
    let o_set = commits_as_hash_set(onto);

    f_set.is_subset(&o_set)
}

pub fn branch_contains_unique_commit(from: &Vec<Commit>, onto: &Vec<Commit>) -> bool {
    let f_set = commits_as_hash_set(from);
    let o_set = commits_as_hash_set(onto);

    o_set.difference(&f_set).count() > 0
}

#[derive(PartialEq, Clone)]
pub struct State {
    pub remote: Repo,
    pub locals: Vec<Repo>,
    pub action: Option<Action>,
    pub prev_state: Option<Box<State>>,
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prev_action = &self.action;
        write!(f, "State {{ num_locals: {}, prev_action: {:?} }}", self.locals.len(), prev_action)
    }
}

impl State {
    pub fn allowable_verification_actions(&self, target_branch: String) -> Vec<Action> {
        let mut actions = Vec::new();

        let repos = &self.locals;

        for i in 0..repos.len() {
            let repo = &repos[i];

            if !repo.branches.contains_key(&target_branch) {
                actions.push(Action::Pull(i, target_branch.clone()));
            } else {
                let local_branch = &repo.branches[&target_branch];
                let remote_branch = &self.remote.branches[&target_branch];

                if local_branch == remote_branch {
                    actions.push(Action::Pull(i, target_branch.clone()));
                    actions.push(Action::Push(i, target_branch.clone()));
                } else if ff_possible(&local_branch.commits, &remote_branch.commits) {
                    actions.push(Action::Pull(i, target_branch.clone()));
                } else if ff_possible(&remote_branch.commits, &local_branch.commits) {
                    actions.push(Action::Push(i, target_branch.clone()));
                }
            }
        }

        actions
    }

    pub fn allowable_attacks(&self) -> Vec<Action> {
        let mut attacks = Vec::new();

        let branches = &self.remote.branches;

        for (name, branch) in branches {
            if name != "master" {
                attacks.push(Action::Deletion(name.to_string()));
            }

            if branch.commits.len() > 1 {
                attacks.push(Action::Rollback(name.to_string()));
            }
            
            let mut teleport_branches = branches.clone();

            teleport_branches.retain(|k, _| k != name);

            for (teleport_name, teleport_branch) in teleport_branches {
                if branch_contains_unique_commit(&branch.commits, &teleport_branch.commits) {
                    attacks.push(Action::Teleport(name.to_string(), teleport_name.to_string()));
                }
            }
        }

        attacks
    }

    pub fn allowable_actions(&self, repo: usize, local_branch: String, remote_branch: String) -> Vec<Action> {
        let mut actions = Vec::new();

        let random_name = Generator::default().next().expect("failed to get a name");

        let branch_name = format!("{}-repo-{}", random_name.clone(), repo);
        let commit_msg = format!("{} -- from repo {}", random_name.clone(), repo);

        actions.push(Action::Commit(repo, commit_msg));
        actions.push(Action::Branch(repo, branch_name));

        let potential_actions = vec![Action::Push(repo, local_branch.clone()), Action::Pull(repo, remote_branch.clone()), Action::Checkout(repo, local_branch.clone())];

        for action in potential_actions {
            if self.allows(&action) {
                actions.push(action);
            }
        }

        actions
    }

    pub fn allows(&self, action: &Action) -> bool {
        match action {
            &Action::Push(repo, ref branch) => {
                if !self.remote.branches.contains_key(branch) {
                    true
                } else if !self.locals[repo].branches.contains_key(branch) {
                    false
                } else {
                    let remote_branch = self.remote.branches.get(branch).expect("remote doesn't have this branch..."); 
                    let local_branch = self.locals[repo].branches.get(branch).expect("local doesn't have this branch?");

                    if remote_branch == local_branch {
                        false
                    } else {
                        ff_possible(&remote_branch.commits, &local_branch.commits)
                    }
                }
            },
            &Action::Pull(repo, ref branch) => {
                if !self.locals[repo].branches.contains_key(branch) {
                    true
                } else if !self.remote.branches.contains_key(branch) {
                    false
                } else {
                    let remote_branch = self.remote.branches.get(branch).expect("remote doesn't have this branch..."); 
                    let local_branch = self.locals[repo].branches.get(branch).expect("local doesn't have this branch?");

                    if local_branch == remote_branch {
                        false
                    } else {
                        ff_possible(&local_branch.commits, &remote_branch.commits)
                    }
                }
            }
            &Action::Checkout(repo, ref branch_name) => {
                self.locals[repo].branches.contains_key(branch_name) && self.locals[repo].current_branch != branch_name.to_string()
            },
            _ => true,
        }
    }

    pub fn apply(&mut self, action: &Action) -> State {
        let old_state = self.clone();

        match action {
            &Action::Branch(repo, ref name) => {
                self.locals[repo].add_branch(name);
            },
            &Action::Commit(repo, ref message) => {
                self.locals[repo].commit(message);
            },
            &Action::Checkout(repo, ref branch) => {
                self.locals[repo].current_branch = branch.to_string();
            },
            &Action::Push(repo, ref branch) => {
                let local_branch = self.locals[repo].branches.get(branch).expect("failed to get local branch");
                self.remote.branches.insert(branch.to_string(), local_branch.clone());
            },
            &Action::Pull(repo, ref branch) => {
                let remote_branch = self.remote.branches.get(branch).expect("failed to get remote branch");
                self.locals[repo].branches.insert(branch.to_string(), remote_branch.clone());
            },
            _ => ()
        };

        State {
            remote: self.remote.clone(),
            locals: self.locals.clone(),
            prev_state: Some(Box::new(old_state)),
            action: Some(action.clone())
        }
    }
}