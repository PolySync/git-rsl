use std::process;
use std::vec::Vec;
use std::collections::HashSet;
use std::iter::FromIterator;


use git2::{Repository, Remote, Oid, BranchType};

use nonce_bag::{NonceBag, HasNonceBag};
use push_entry::PushEntry;
use rsl::{RSL, HasRSL};
use nonce::{Nonce, HasNonce};
use errors::*;
use utils::git;

pub fn secure_fetch<'repo>(repo: &Repository, mut remote: &mut Remote, ref_names: &[&str]) -> Result<()> {

    let mut remote_rsl: RSL;
    let mut local_rsl: RSL;
    let mut nonce_bag: NonceBag;
    let mut nonce: Nonce;

    repo.fetch_rsl(&mut remote);
    repo.init_rsl_if_needed(&mut remote);
    //let (remote_rsl, local_rsl, nonce_bag, nonce) = repo.read_rsl()?.chain_err(|| "couldn't read RSL");

    git::checkout_branch(&repo, "refs/heads/RSL");

    //TODO paper algo uses spin lock here, probably a better alternative

    let mut store_counter = 5;
    'store: loop {
        if store_counter == 0  {
            bail!("Couldn't store new fetch entry in RSL; check your connection and try again");
        }
        let mut counter = 5;
        'fetch: loop {
            if counter == 0 {
                bail!("Couldn't fetch; check your connection and try again");
            }
            repo.fetch_rsl(&mut remote)?;

            // TODO reject if one of the branches has no rsl push entry
            //for branch in ref_names {
            //    match last_push_entry_for(&branch) {
            //        branch.head.oid => ok
            //        _ => error
            //    }
            //}

            let (remote_rsl, local_rsl, nonce_bag, nonce) = repo.read_rsl().chain_err(|| "couldn't read RSL")?;


            match git::fetch(repo, &mut remote, ref_names, None) {
                Ok(_) => (),
                Err(e) => {
                    println!("Error: unable to fetch reference {} from remote {}", ref_names.clone().join(", "), &remote.name().unwrap());
                    println!("  {}", e);
                    process::exit(51);
                },
            };

            if all_push_entries_in_fetch_head(repo, &remote_rsl, ref_names) {
                break 'fetch;
            }
            counter -= 1;
        }

        let (remote_rsl, local_rsl, mut nonce_bag, nonce) = repo.read_rsl().chain_err(|| "couldn't read RSL")?;

        // update nonce bag
        if nonce_bag.bag.contains(&nonce) {
            nonce_bag.remove(&nonce);
        }

        let new_nonce = Nonce::new().unwrap();
        repo.write_nonce(&new_nonce).chain_err(|| "nonce write error")?;

        nonce_bag.insert(new_nonce);
        repo.write_nonce_bag(&nonce_bag).chain_err(|| "couldn't write to nonce baf file")?;
        repo.commit_nonce_bag().chain_err(|| "couldn't commit nonce bag")?;
        match repo.push_rsl(&mut remote) {
            Ok(()) => break 'store,
            _ => (),
        }
        store_counter -= 1;
    }

    repo.validate_rsl().chain_err(|| "Invalid remote RSL")?;

    // fast forward fetched refs
    reset_local_rsl_to_remote_rsl(repo);
    Ok(())
}


fn all_push_entries_in_fetch_head(repo: &Repository, remote_rsl: &RSL, ref_names: &[&str]) -> bool {
    // find the last push entry for each branch
    let mut latest_push_entries: Vec<Oid> = ref_names.clone().into_iter().filter_map(|ref_name| {
        match repo.find_last_push_entry_for_branch(remote_rsl, ref_name).ok() {
            Some(Some(pe)) => Some(pe.head),
            Some(None) => None,
            None => None,
        }
    }).collect();

    // find the Oid of the tip of each remote fetched branch
    let mut fetch_heads : Vec<Oid> = ref_names.clone().into_iter().filter_map(|ref_name| {
        println!("ref_name: {:?}", ref_name);
        match repo.find_branch(&format!("origin/{}", ref_name), BranchType::Remote) {
            Ok(branch) => branch.get().target(),
            Err(_) => None
        }
    }).collect();
    let push_entries: HashSet<&Oid> = HashSet::from_iter(&latest_push_entries);
    let fetch_head: HashSet<&Oid> = HashSet::from_iter(&fetch_heads);

    println!("latest push entries: {:?}", push_entries);
    println!("fetch_heads {:?}", fetch_head);
    push_entries.is_subset(&fetch_head)
}


// fn last_push_entry_for(repo: &Repository, reference: &str) -> Option<PushEntry> {
//     //TODO Actually walk the commits and look for the most recent for the branch we're interested
//     //in
//
//     // this is where it might come in yuseful to keep track of the last push entry for a branch...
//     // for each ref, try to parse into a pushentry
//     // if you can, check if that pushentry is for the branch
//     // if it is , return that pushentry. otherwise keep going
//     // if you get to then end of the walk, return false
//     Some(PushEntry::new(repo, reference, String::from(""), NonceBag::new()))
// }

//TODO implement
fn reset_local_rsl_to_remote_rsl(_repo: &Repository) {
}
