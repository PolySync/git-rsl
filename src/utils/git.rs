use std::path::Path;
use std::env;

use git2;
use git2::{Error, FetchOptions, PushOptions, Oid, Reference, Signature, Branch, Commit, RemoteCallbacks, Remote, Repository, Revwalk, DiffOptions, RepositoryState};
use git2::build::CheckoutBuilder;
use git2::BranchType;

use git2::StashApplyOptions;
use git2::STASH_INCLUDE_UNTRACKED;

use ::common::errors::*;


pub fn checkout_branch(repo: &Repository, ref_name: &str) -> Result<()> {
    let tree = repo.find_reference(ref_name)
        .chain_err(|| "couldn't find branch")?
        .peel_to_commit()
        .chain_err(|| "couldnt find latest RSSL commit")?
        .into_object();

    let mut opts = CheckoutBuilder::new();
    opts.force();
    opts.remove_untracked(true); // this should be fine since we stash untracked at the beginning
    repo.checkout_tree(&tree, Some(&mut opts)).chain_err(|| "couldn't checkout tree")?; // Option<CheckoutBuilder>
    repo.set_head(&ref_name).chain_err(|| "couldn't switch head to RSL")?;
    Ok(())
}

pub fn discover_repo() -> Result<Repository> {
    let current_dir = env::current_dir().unwrap();
    Repository::discover(current_dir).chain_err(|| "cwd is not a git repo")
}

pub fn stash_local_changes(repo: &mut Repository) -> Result<(Option<Oid>)> {
    let signature = repo.signature()?;
    let message = "Stashing local changes for RSL business";

    // check that there are indeed changes in index or untracked to stash
    {
        let is_clean = repo.state() == RepositoryState::Clean;
        let mut diff_options = DiffOptions::new();
        diff_options.include_untracked(true);
        let  diff = repo.diff_index_to_workdir(
            None, // defaults to head index,
            Some(&mut diff_options),
        )?;

        let num_deltas = diff.deltas().count();
        if is_clean && (num_deltas == 0) {
            return Ok(None)
        }
    }
    let oid = repo.stash_save(
        &signature,
        &message,
        Some(STASH_INCLUDE_UNTRACKED),
    )?;
    Ok(Some(oid))
}

pub fn unstash_local_changes(repo: &mut Repository, stash_id: Option<Oid>) -> Result<()> {
    if stash_id == None {
        return Ok(());
    }
    let mut options = StashApplyOptions::new();
    options.reinstantiate_index();
    repo.stash_pop(
        0, // TODO validate SHA of stash commit
        Some(&mut options),
    )?;
    Ok(())
}

pub fn add_and_commit(repo: &Repository, path: Option<&Path>, message: &str, branch: &str) -> Result<Oid> {
    let mut index = repo.index()?;
    if path.is_some() {
        index.add_path(path.unwrap());
    }
    let oid = index.write_tree()?;
    let signature = repo.signature()?;
    let ref_name = format!("refs/heads/{}", branch);
    let parent = repo.find_reference(&ref_name).and_then(|x| x.peel_to_commit()).ok();
    let tree = repo.find_tree(oid)?;

    // stupid duplication because &[&T] is a terrible type to mess with
    if let Some(parent_commit) = parent {
        let oid = repo.commit(Some(&ref_name), //  point HEAD to our new commit
                    &signature, // author
                    &signature, // committer
                    message, // commit message
                    &tree, // tree
                    &[&parent_commit])?; // parents
        Ok(oid)
    } else {
        let oid = repo.commit(Some(&ref_name), //  point HEAD to our new commit
                    &signature, // author
                    &signature, // committer
                    message, // commit message
                    &tree, // tree
                    &[])?; // parents
        Ok(oid)
    }

    #[test]
    fn checkout_branch() {
        let context = setup();
        {
            let repo = &context.local;
            assert!(repo.head().unwrap().name().unwrap() == "refs/heads/devel");
            super::checkout_branch(&repo, "RSL").unwrap();
            assert!(repo.head().unwrap().name().unwrap() == "refs/heads/RSL");
        }
        teardown(context)
    }

}
