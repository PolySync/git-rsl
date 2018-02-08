use git2::{Oid, Signature, Repository};
use std::path::Path;

use ::common::errors::*;

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


}
