

extern crate crypto;
extern crate rand;

use std::{env, process};
use std::vec::Vec;
use std::collections::HashSet;
use std::iter::FromIterator;


use git2;
use git2::{Error, FetchOptions, PushOptions, Oid, Reference, Branch, Commit, RemoteCallbacks, Remote, Repository, Revwalk, DiffOptions, RepositoryState};
use git2::build::CheckoutBuilder;
use git2::BranchType;

use git2::StashApplyOptions;
use git2::STASH_INCLUDE_UNTRACKED;


pub mod push_entry;
pub mod nonce;
pub mod nonce_bag;
pub mod rsl;

pub use self::push_entry::PushEntry;
pub use self::nonce::{Nonce, HasNonce};
pub use self::nonce_bag::{NonceBag, HasNonceBag};
pub use self::rsl::{RSL, HasRSL};

pub mod errors {
    error_chain!{
        foreign_links {
            Git(::git2::Error);
            Serde(::serde_json::Error);
            IO(::std::io::Error);
        }
    }
}

use self::errors::*;



pub fn validate_rsl(repo: &Repository, remote_rsl: &RSL, local_rsl: &RSL, nonce_bag: &NonceBag, repo_nonce: &Nonce) -> Result<()> {

    // Ensure remote RSL head is a descendant of local RSL head.
    let descendant = repo
        .graph_descendant_of(remote_rsl.head, local_rsl.head)
        .unwrap_or(false);
    let same = (remote_rsl.head == local_rsl.head);
    if !descendant && !same {
        bail!("RSL invalid: No path to get from Local RSL to Remote RSL");
    }

    // Walk through the commits from local RSL head, which we know is valid,
    // validating each additional pushentry since that point one by one.
    let last_hash = match local_rsl.last_push_entry {
        Some(ref push_entry) => Some(push_entry.hash()),
        None => None, // the first push entry will have None as last_push_entry
    };
    let mut revwalk: Revwalk = repo.revwalk().unwrap();
    revwalk.push(remote_rsl.head);
    revwalk.set_sorting(git2::SORT_REVERSE);
    revwalk.hide(local_rsl.head);

    let remaining = revwalk.map(|oid| oid.unwrap());

    let result = remaining.fold(last_hash, |prev_hash, oid| {
        match PushEntry::from_oid(&repo, &oid) {
            Some(current_push_entry) => {
                let current_prev_hash = current_push_entry.prev_hash();

                // if current prev_hash == local_rsl.head (that is, we have arrived at the first push entry after the last recorded one), then check if repo_nonce in PushEntry::from_oid(oid.parent_commit) or noncebag contains repo_nonce; return false if neither holds
                //if current_prev_hash == last_local_push_entry.hash() {

                    // validate nonce bag (lines 1-2):
                    // TODO does this take care of when there haven't been any new entries or only one new entry?
                    //if !nonce_bag.bag.contains(&repo_nonce) && !current_push_entry.nonce_bag.bag.contains(&repo_nonce) { // repo nonce not in remote nonce bag && repo_nonce not in remote_rsl.push_after(local_rsl){
                    //    None;
                    //}
                //}
                let current_hash = current_push_entry.hash();
                if prev_hash == Some(current_prev_hash) {
                    Some(current_hash)
                } else {
                    None
                }
            },
            None => prev_hash, // this was not a pushentry. continue with previous entry in hand
        }

    });

    if result != None { bail!("invalid RSL entry"); }


    verify_signature(remote_rsl.head).chain_err(|| "GPG signature of remote RSL head invalid")

}

fn verify_signature(_oid: Oid) -> Result<()> {
    return Ok(())
}




//TODO implement
pub fn reset_local_rsl_to_remote_rsl(_repo: &Repository) {
}



#[cfg(test)]
mod tests {
    use super::*;
    use utils::test_helper::*;



}
