use std::process;
use std::vec::Vec;
use std::collections::HashSet;
use std::iter::FromIterator;


use git2::{Reference, Repository, Remote, Oid, BranchType};

use common;
use common::{NonceBag, HasNonceBag, PushEntry};
use common::rsl::{RSL, HasRSL};
use common::nonce::{Nonce, HasNonce};
use common::errors::*;

use utils::git;

pub fn secure_fetch<'repo>(repo: &Repository, mut remote: &mut Remote, ref_names: Vec<&str>) -> Result<()> {

    let mut remote_rsl: RSL = unsafe { ::std::mem::uninitialized() };
    let mut local_rsl: RSL = unsafe { ::std::mem::uninitialized() };
    let mut nonce_bag: NonceBag = unsafe { ::std::mem::uninitialized() };
    let mut nonce: Nonce = unsafe { ::std::mem::uninitialized() };


    repo.fetch_rsl(&mut remote);
    repo.init_rsl_if_needed(&mut remote);

    git::checkout_branch(&repo, "refs/heads/RSL");

    //TODO paper algo uses spin lock here, probably a better alternative

    let mut store_counter = 5;
    'store: loop {
        match store_counter {
            0 => panic!("Couldn't store new fetch entry in RSL; check your connection and try again"),
            _ => (),
        }
        let mut counter = 5;
        'fetch: loop {
            match counter {
                0 => panic!("Couldn't fetch; check your connection and try again"),
                _ => (),
            }
            //let original_branch = common::prep_workspace(&repo);
            repo.fetch_rsl(&mut remote);



            let (remote_rsl, local_rsl, nonce_bag, nonce) = match repo.read_rsl() {
                Ok((a,b,c,d)) => (a,b,c,d),
                Err(e) => panic!("Couldn't read RSL {:?}", e),
            };

            // TODO reject if one of the branches has no rsl push entry
            //for branch in ref_names {
            //    match last_push_entry_for(&branch) {
            //        branch.head.oid => ok
            //        _ => error
            //    }
            //}

            match git::fetch(repo, &mut remote, &ref_names, None) {
                Ok(_) => (),
                Err(e) => {
                    println!("Error: unable to fetch reference {} from remote {}", &ref_names.clone().join(", "), &remote.name().unwrap());
                    println!("  {}", e);
                    process::exit(51);
                },
            };

            if all_push_entries_in_fetch_head(&repo, &ref_names) {
                break 'fetch;
            }
            counter -= 1;
        }

        // update nonce bag
        if nonce_bag.bag.contains(&nonce) {
            nonce_bag.remove(&nonce);
        }

        let new_nonce = common::Nonce::new().unwrap();
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

    common::validate_rsl(repo, &remote_rsl, &local_rsl, &nonce_bag, &nonce).chain_err(|| "Invalid remote RSL")?;

    // fast forward fetched refs
    common::reset_local_rsl_to_remote_rsl(repo);
    Ok(())
}


fn all_push_entries_in_fetch_head(repo: &Repository, ref_names: &Vec<&str>) -> bool {

    let mut latest_push_entries: &Vec<Oid> = &ref_names.clone().into_iter().filter_map(|ref_name| {
        match last_push_entry_for(repo, ref_name) {
            Some(pe) => Some(pe.head),
            None => None,
        }
    }).collect();
    let mut fetch_heads : &Vec<Oid> = &ref_names.clone().into_iter().filter_map(|ref_name| {
        match repo.find_branch(ref_name, BranchType::Remote) {
            Ok(branch) => branch.get().target(),
            Err(_) => None
        }
    }).collect();
    let h1: HashSet<&Oid> = HashSet::from_iter(latest_push_entries);
    let h2: HashSet<&Oid> = HashSet::from_iter(fetch_heads);

    h2.is_subset(&h1)
}


fn last_push_entry_for(repo: &Repository, reference: &str) -> Option<PushEntry> {
    //TODO Actually walk the commits and look for the most recent for the branch we're interested
    //in

    // this is where it might come in yuseful to keep track of the last push entry for a branch...
    // for each ref, try to parse into a pushentry
    /// if you can, check if that pushentry is for the branch
    // if it is , return that pushentry. otherwise keep going
    // if you get to then end of the walk, return false
    Some(PushEntry::new(repo, reference, String::from(""), NonceBag::new()))
}
