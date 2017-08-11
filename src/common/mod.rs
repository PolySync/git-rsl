extern crate crypto;
extern crate rand;

use std::collections::HashSet;
use std::vec::Vec;

use git2::{Oid, Reference, Remote, Repository};
use rand::Rng;

mod push_entry;
pub mod nonce;
pub use self::push_entry::PushEntry;
pub use self::nonce::Nonce;
pub use self::nonce::HasNonce;

const RSL_BRANCH: &'static str = "RSL";
const NONCE_BRANCH: &'static str = "RSL_NONCE";

pub fn retrieve_rsl_and_nonce_bag_from_remote_repo<'repo>(repo: &'repo Repository, remote: &mut Remote) -> (Reference<'repo>, HashSet<Nonce>) {

    remote.fetch(&[RSL_BRANCH, NONCE_BRANCH], None, None);
    let remote_name = remote.name().unwrap();
    let remote_rsl_ref_name = format!("{}/{}", remote_name, RSL_BRANCH);
    let remote_rsl = repo.find_reference(&remote_rsl_ref_name).unwrap();

    let remote_nonce_ref_name = format!("{}/{}", remote_name, NONCE_BRANCH);
    let remote_nonce = repo.find_reference(&remote_nonce_ref_name).unwrap();

    let nonce_bag = read_nonce_bag(&remote_nonce);

    (remote_rsl, nonce_bag)
}

pub fn store_in_remote_repo(repo: &Repository, remote: &Remote, nonce_bag: &HashSet<Nonce>) -> bool {
    false
}

pub fn validate_rsl(repo: &Repository, remote_rsl: &Reference, nonce_bag: &HashSet<Nonce>) -> bool {
    let repo_nonce = match repo.read_nonce() {
        Ok(nonce) => nonce,
        Err(e) => {
            //TODO Figure out what needs to happen when a nonce doeesn't exist because we're never
            //fetched
            println!("Error: Couldn't read nonce: {:?}", e);
            return false;
        },
    };
    if !nonce_bag.contains(&repo_nonce) /* TODO: && repo_nonce not in remote_rsl.push_after(local_rsl*/ {
        return false;
    }

    let local_rsl = local_rsl_from_repo(repo).unwrap();
    let mut current_push_entry = PushEntry::from(&local_rsl);


    true
}

fn local_rsl_from_repo(repo: &Repository) -> Option<Reference> {
    match repo.find_reference(RSL_BRANCH) {
        Ok(r) => Some(r),
        Err(_) => None,
    }
}

pub fn last_push_entry_for(repo: &Repository, remote: &Remote, reference: &str) -> Option<PushEntry> {
    let fully_qualified_ref_name = format!("{}/{}", remote.name().unwrap(), reference);
    //TODO Actually walk the commits and look for the most recent for the branch we're interested
    //in
    Some(PushEntry::new(repo, &fully_qualified_ref_name))
}

//TODO implement
pub fn reset_local_rsl_to_remote_rsl(repo: &Repository) {
}

//TODO implement
fn is_push_entry(nonce_branch: &Reference) -> bool {
    true
}

fn read_nonce_bag(remote_nonce: &Reference) -> HashSet<Nonce> {
    if is_push_entry(remote_nonce) {
        HashSet::new()
    } else {
        //TODO actually read the contents of the nonce bag from the commit
        let existing_nonce = rand::random::<Nonce>();
        let mut set = HashSet::new();
        set.insert(existing_nonce);
        set
    }

}

