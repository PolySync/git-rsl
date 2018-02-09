

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






// pub fn retrieve_rsl_and_nonce_bag_from_remote_repo<'repo>(repo: &'repo Repository, mut remote: &mut Remote) -> Option<(Reference<'repo>, NonceBag)> {
//
//     fetch(repo, remote, &[RSL_BRANCH], Some(REFLOG_MSG));
//     let remote_rsl = match repo.find_branch(RSL_BRANCH, BranchType::Remote) {
//             Err(e) => return None,
//             Ok(rsl) => (rsl.into_reference())
//         };
//
//     let nonce_bag = match repo.read_nonce_bag(&remote_rsl) {
//         Ok(n) => n,
//         Err(_) => process::exit(10),
//     };
//
//     let repo_nonce = match repo.read_nonce() {
//         Ok(nonce) => nonce,
//         Err(e) => {
//             println!("Error: Couldn't read nonce: {:?}", e);
//             return false;
//         },
//     };
//     Some((remote_rsl, local_rsl, nonce_bag, repo_nonce))
// }


pub fn all_push_entries_in_fetch_head(repo: &Repository, ref_names: &Vec<&str>) -> bool {

    let mut latest_push_entries: &Vec<git2::Oid> = &ref_names.clone().into_iter().filter_map(|ref_name| {
        match last_push_entry_for(repo, ref_name) {
            Some(pe) => Some(pe.head),
            None => None,
        }
    }).collect();
    let mut fetch_heads : &Vec<git2::Oid> = &ref_names.clone().into_iter().filter_map(|ref_name| {
        match repo.find_branch(ref_name, BranchType::Remote) {
            Ok(branch) => branch.get().target(),
            Err(_) => None
        }
    }).collect();
    let h1: HashSet<&git2::Oid> = HashSet::from_iter(latest_push_entries);
    let h2: HashSet<&git2::Oid> = HashSet::from_iter(fetch_heads);

    h2.is_subset(&h1)
}

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



fn find_first_commit(repo: &Repository) -> Result<Commit> {
    let mut revwalk: Revwalk = repo.revwalk().expect("Failed to make revwalk");
    revwalk.push_head();
    let result = revwalk
        .last()  // option<result<oid, err>> => result<oid, err>
        .ok_or("revwalk empty?")? // result<oid, err>
        .chain_err(|| "revwalk gave error")?;
    let commit = repo.find_commit(result).chain_err(|| "first commit not in repo");
    commit
}





pub fn last_push_entry_for(repo: &Repository, reference: &str) -> Option<PushEntry> {
    //TODO Actually walk the commits and look for the most recent for the branch we're interested
    //in

    // this is where it might come in yuseful to keep track of the last push entry for a branch...
    // for each ref, try to parse into a pushentry
    /// if you can, check if that pushentry is for the branch
    // if it is , return that pushentry. otherwise keep going
    // if you get to then end of the walk, return false
    Some(PushEntry::new(repo, reference, String::from(""), NonceBag::new()))
}

//TODO implement
pub fn reset_local_rsl_to_remote_rsl(_repo: &Repository) {
}



#[cfg(test)]
mod tests {
    use super::*;
    use utils::test_helper::*;



}
