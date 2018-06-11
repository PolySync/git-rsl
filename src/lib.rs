#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
#[macro_use] extern crate proptest;

extern crate crypto;
extern crate git2;
extern crate fs_extra;
extern crate gpgme;
extern crate hex;
extern crate rand;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate tempdir;
extern crate tempfile;

pub mod push;
pub mod fetch;
pub mod rsl;
pub mod push_entry;
pub mod nonce;
pub mod nonce_bag;
pub mod utils;
pub mod errors;

use errors::*;
use utils::git;

use std::env;
use git2::{Oid, Repository};
use std::path::PathBuf;

pub fn rsl_init_with_cleanup(repo: &mut Repository, remote_name: &str) -> Result<()> {
    let ws = Workspace::new(repo)?;
    let mut remote = ws.repo.find_remote(remote_name)
        .chain_err(|| format!("unable to find remote named {}", remote_name))?;
    rsl::HasRSL::rsl_init_global(ws.repo, &mut remote)
}

pub fn secure_fetch_with_cleanup(repo: &mut Repository, branch: &str, remote_name: &str) -> Result<()> {
    let ws = Workspace::new(repo)?;
    let mut remote = ws.repo.find_remote(remote_name)
        .chain_err(|| format!("unable to find remote named {}", remote_name))?;
    fetch::secure_fetch(ws.repo, &mut remote, &[branch])
}

pub fn secure_push_with_cleanup(repo: &mut Repository, branch: &str, remote_name: &str) -> Result<()> {
    let ws = Workspace::new(repo)?;
    let mut remote = ws.repo.find_remote(remote_name)
        .chain_err(|| format!("unable to find remote named {}", remote_name))?;
    push::secure_push(ws.repo, &mut remote, &[branch])
}

// Returns a tuple containing the branch name, Some(stash_commit_id) if a stash took place or None if it was not necessary, and the path to the original working directory (if the user is not in the project root), in that order.
fn prep_workspace(mut repo: &mut Repository) -> Result<WorkspaceSnapshot> {
    let current_branch_name = repo.head()?
        .name()
        .ok_or("Not on a named branch. Please switch to one so we can put you back where you started when this is all through.")? // TODO allow this??
        .to_owned();

    let stash_id =
        git::stash_local_changes(&mut repo).chain_err(|| "Couldn't stash local changes.")?;

    // save current working directory and cd to project root
    let cwd = env::current_dir()?;
    let project_root = repo.workdir().ok_or("RSL not supported for bare repos")?;
    let original_dir = if project_root != cwd {
        env::set_current_dir(&project_root)?;
        Some(cwd)
    } else {
        Some(cwd)
    };

    Ok(WorkspaceSnapshot { original_branch_name: current_branch_name.to_string(), stash_commit_id: stash_id, original_working_dir: original_dir })
}

struct Workspace<'repo> {
    pub repo: &'repo mut Repository,
    pub old_state: WorkspaceSnapshot
}

/// An informal wrapper around workspace state with metadata for state prior to an operation for later restoration
struct WorkspaceSnapshot {
    original_branch_name: String,
    stash_commit_id: Option<Oid>,
    original_working_dir: Option<PathBuf>
}

impl <'repo> Workspace<'repo> {
    pub fn new(repo: &'repo mut Repository) -> Result<Workspace> {
        let snapshot = prep_workspace(repo)?;
        Ok(Workspace {
            repo,
            old_state: snapshot
        })
    }
}

impl <'repo> Drop for Workspace<'repo> {
    fn drop(&mut self) {
        restore_workspace(&mut self.repo, &self.old_state)
            .expect("Could not restore workspace to original configuration");
    }
}

fn restore_workspace(
    mut repo: &mut Repository,
    WorkspaceSnapshot {
    original_branch_name,
    stash_commit_id,
    original_working_dir,
    }: &WorkspaceSnapshot
) -> Result<()> {
    println!("Returning to {} branch", original_branch_name);
    git::checkout_branch(repo, &original_branch_name).chain_err(|| {
        "Couldn't checkout starting branch. Sorry if we messed with your repo state. Ensure you are on the desired branch. It may be necessary to apply changes from the stash"
    })?;

    if let Some(dir) = original_working_dir {
        env::set_current_dir(dir)?;
    }

    if let Some(_) = stash_commit_id {
        println!("Unstashing local changes");
    }
    git::unstash_local_changes(&mut repo, *stash_commit_id).chain_err(|| {
        "Couldn't unstash local changes. Sorry if we messed with your repository state. It may be necessary to apply changes from the stash. {:?}"
    })?;
    Ok(())
}
