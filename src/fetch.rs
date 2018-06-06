use std::vec::Vec;
use std::collections::HashSet;
use std::iter::FromIterator;

use git2::{BranchType, Oid, Remote, Repository};

use rsl::{HasRSL, RSL};
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
                bail!("Couldn't fetch; No push entry for latest commit on target branch. It is likely that someone pushed without using git-rsl. Please have that developer secure-push the branch and try again.");
            }
            repo.fetch_rsl(&mut remote)?;


            let mut remote_2 = remote.clone();
            let mut rsl = RSL::read(repo, &mut remote_2).chain_err(|| "couldn't read RSL")?;

            // reject if one of the branches has no rsl push entry
            for branch in ref_names {
                match rsl.find_last_remote_push_entry_for_branch(&branch) {
                    Ok(None) => bail!("no push records for the ref you are attempting to fetch"),
                    Err(e) => return Err(e.chain_err(|| "couldn't check that provided refs are valid")),
                    Ok(_) => (),
                }
            }

            match git::fetch(repo, &mut remote, ref_names, None) {
                Ok(_) => (),
                Err(e) => {
                    println!(
                        "Error: unable to fetch reference {} from remote {}",
                        ref_names.clone().join(", "),
                        &remote.name().unwrap()
                    );
                    println!("  {}", e);
                }
            };

            // paper algorithm:
            // 9    C <- RemoteRSL.latestPush(X).refPointer
            // 10   id (C == FETCH_HEAD) and_then
            // 11   fetch_success <- true
            if all_push_entries_in_fetch_head(repo, &rsl, ref_names) {
                break 'fetch;
            } else {
                rsl.reset_remote_to_local()?;
            }
            counter -= 1;
        }

        let mut rsl = RSL::read(&repo, &mut remote).chain_err(|| "couldn't read RSL")?;


        // reset to last trusted RSL if invalid
        if let Err(e) = rsl.validate() {
            rsl.reset_remote_to_local()?;
            // TODO reset remote fetchspec(s) to local as well
            return Err(e).chain_err(|| ErrorKind::InvalidRSL)?;
        }


        // Fastforward valid remote RSL onto local branch
        rsl.update_local()?;

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
            match rsl.find_last_remote_push_entry_for_branch(ref_name)
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
    println!("push_entries: {:?}", &push_entries);
    println!("fetch_head: {:?}", &fetch_head);
    push_entries.is_subset(&fetch_head)
}
