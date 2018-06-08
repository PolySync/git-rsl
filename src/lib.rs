#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
#[macro_use] extern crate proptest;

extern crate crypto;
extern crate git2;
//extern crate libgit2_sys;
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

pub fn secure_fetch_with_cleanup(mut repo: &mut Repository, branch: &str, remote_name: &str) -> Result<()> {
    let (original_branch_name, stash_id, original_dir) = prep_workspace(&mut repo)?;

    let result = {
        let mut remote = (&repo)
            .find_remote(remote_name)
            .chain_err(|| format!("unable to find remote named {}", remote_name))?;
        fetch::secure_fetch(&repo, &mut remote, &[branch])
    };

    restore_workspace(
        &mut repo,
        &original_branch_name,
        stash_id,
        original_dir,
    )?;

    result
}

pub fn secure_push_with_cleanup(mut repo: &mut Repository, branch: &str, remote_name: &str) -> Result<()> {
    let (original_branch_name, stash_id, original_dir) = prep_workspace(&mut repo)?;

    let result = {
        let mut remote = (&repo)
            .find_remote(remote_name)
            .chain_err(|| format!("unable to find remote named {}", remote_name))?;
        push::secure_push(&repo, &mut remote, &[branch])
    };

    restore_workspace(
        &mut repo,
        &original_branch_name,
        stash_id,
        original_dir,
    )?;

    result
}

// TODO - deprecate run when we remove the old kevlar-laces-rs interface
pub fn run(mut repo: &mut Repository, branches: &[&str], remote_name: &str, mode: &str) -> Result<()> {
    let (original_branch_name, stash_id, original_dir) = prep_workspace(&mut repo)?;

    let result = {
        let mut remote = (&repo)
            .find_remote(remote_name)
            .chain_err(|| format!("unable to find remote named {}", remote_name))?;


        let result = if mode == "fetch" {
            fetch::secure_fetch(&repo, &mut remote, &branches)
        } else if mode == "push" {
            push::secure_push(&repo, &mut remote, &branches)
        } else {
            panic!("this shouldn't happen");
        };
        result
    };

    restore_workspace(
        &mut repo,
        &original_branch_name,
        stash_id,
        original_dir,
    )?;

    result
}

// Returns a tuple containing the branch name, Some(stash_commit_id) if a stash took place or None if it was not necessary, and the path to the original working directory (if the user is not in the project root), in that order.
fn prep_workspace(mut repo: &mut Repository) -> Result<(String, Option<Oid>, Option<PathBuf>)> {
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
        None
    };

    Ok((current_branch_name, stash_id, original_dir))
}

fn restore_workspace(
    mut repo: &mut Repository,
    original_branch_name: &String,
    stash_id: Option<Oid>,
    original_working_directory: Option<PathBuf>,
) -> Result<()> {
    println!("Returning to {} branch", original_branch_name);
    git::checkout_branch(repo, original_branch_name).chain_err(|| {
        "Couldn't checkout starting branch. Sorry if we messed with your repo state. Ensure you are on the desired branch. It may be necessary to apply changes from the stash"
    })?;

    if let Some(dir) = original_working_directory {
        env::set_current_dir(dir)?;
    }

    if let Some(_) = stash_id {
        println!("Unstashing local changes");
    }
    git::unstash_local_changes(&mut repo, stash_id).chain_err(|| {
        "Couldn't unstash local changes. Sorry if we messed with your repository state. It may be necessary to apply changes from the stash. {:?}"
    })?;
    Ok(())
}
