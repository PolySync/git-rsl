use std::process;
use std::vec::Vec;
use std::collections::HashSet;
use std::iter::FromIterator;

use git2::{BranchType, Oid, Remote, Repository};

use nonce_bag::HasNonceBag;
use rsl::{HasRSL, RSL};
use nonce::{HasNonce, Nonce};
use errors::*;
use utils::git;

pub fn secure_fetch<'remote, 'repo: 'remote>(
    repo: &'repo Repository,
    mut remote: &'remote mut Remote<'repo>,
    ref_names: &[&str],
) -> Result<()> {
    repo.fetch_rsl(&mut remote)?;
    repo.init_rsl_if_needed(&mut remote)?;
    git::checkout_branch(repo, "refs/heads/RSL")?;

    //TODO paper algo uses spin lock here, probably a better alternative

    let mut store_counter = 5;
    let mut err: Result<()> = Err("".into());
    'store: loop {
        if store_counter == 0 {
            err.chain_err(|| "Couldn't store new fetch entry in RSL; check your connection and try again")?;
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

            let mut rsl = RSL::read(repo, &mut remote).chain_err(|| "couldn't read RSL")?;

            match git::fetch(repo, &mut rsl.remote, ref_names, None) {
                Ok(_) => (),
                Err(e) => {
                    println!(
                        "Error: unable to fetch reference {} from remote {}",
                        ref_names.clone().join(", "),
                        &rsl.remote.name().unwrap()
                    );
                    println!("  {}", e);
                }
            };

            if all_push_entries_in_fetch_head(repo, &rsl, ref_names) {
                break 'fetch;
            }
            counter -= 1;
        }

        let mut rsl = RSL::read(&repo, &mut remote).chain_err(|| "couldn't read RSL")?;

        // validate remote RSL
        rsl.validate().chain_err(|| ErrorKind::InvalidRSL)?;

        // Fastforward valid remote RSL onto local branch
        // TODO deal with no change necessary
        if !git::up_to_date(repo, "RSL", "origin/RSL")? {
            match git::fast_forward_possible(repo, "refs/remotes/origin/RSL") {
                Ok(true) => git::fast_forward_onto_head(repo, "refs/remotes/origin/RSL")?,
                Ok(false) => bail!(
                    "Local RSL cannot be fastforwarded to match /
                remote. This may indicate that someone has tampered with the /
                RSL history. Use caution before proceeding."
                ),
                Err(e) => Err(e).chain_err(|| {
                    "Local RSL cannot be /
                fastforwarded to match remote. This may indicate that someone /
                has tampered with the RSL history. Use caution before /
                proceeding."
                })?,
            }
        }

        rsl.update_nonce_bag()?;

        match rsl.push() {
            Ok(()) => break 'store,
            Err(e) => {
                err = Err(e);
                ()
            }
        }
        store_counter -= 1;
    }

    Ok(())
}

fn all_push_entries_in_fetch_head(repo: &Repository, rsl: &RSL, ref_names: &[&str]) -> bool {
    // find the last push entry for each branch
    let latest_push_entries: Vec<Oid> = ref_names
        .clone()
        .into_iter()
        .filter_map(|ref_name| {
            match repo.find_last_remote_push_entry_for_branch(rsl, ref_name)
                .ok()
            {
                Some(Some(pe)) => Some(pe.head()),
                Some(None) | None => None,
            }
        })
        .collect();

    // find the Oid of the tip of each remote fetched branch
    let fetch_heads: Vec<Oid> = ref_names
        .clone()
        .into_iter()
        .filter_map(|ref_name| {
            println!("ref_name: {:?}", ref_name);
            match repo.find_branch(&format!("origin/{}", ref_name), BranchType::Remote) {
                Ok(branch) => branch.get().target(),
                Err(_) => None,
            }
        })
        .collect();
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
