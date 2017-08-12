use std::collections::HashSet;
use std::process;
use std::vec::Vec;

use git2::{Reference, Repository};

use common;
use common::Nonce;
use common::nonce::{HasNonce, NonceError};

pub fn secure_fetch<'repo>(repo: &Repository, remote_name: &str, ref_names: Vec<&str>) {
    let mut remote = match repo.find_remote(remote_name) {
        Ok(r) => r,
        Err(e) => {
            println!("Error: unable to find remote named {}", remote_name);
            println!("  {}", e);
            process::exit(50);
        },
    };

    let mut remote_rsl: Reference;
    let mut nonce_bag: HashSet<Nonce>;

    //TODO paper algo uses spin lock here, probably a better alternative

    'store: loop {
        'fetch: loop {
            let (fetch_local_remote_rsl, fetch_local_nonce_bag) = common::retrieve_rsl_and_nonce_bag_from_remote_repo(repo, &mut remote);
            remote_rsl = fetch_local_remote_rsl;
            nonce_bag = fetch_local_nonce_bag;

            match common::fetch(repo, &mut remote, &ref_names, None) {
                Ok(_) => (),
                Err(e) => {
                    println!("Error: unable to fetch reference {} from remote {}", &ref_names.clone().join(", "), &remote_name);
                    println!("  {}", e);
                    process::exit(51);
                },
            };
            let latest_push_entries = &ref_names.clone().into_iter().map(|ref_name| {
                common::last_push_entry_for(repo, &remote, ref_name)
            }).collect();

            if all_push_entries_in_fetch_head(repo, latest_push_entries) {
                break 'fetch;
            }
        }

        match repo.read_nonce() {
            Ok(current_nonce) => {
                if nonce_bag.contains(&current_nonce) {
                    nonce_bag.remove(&current_nonce);
                }
            },
            _ => (),
        };

        let new_nonce = common::Nonce::new().unwrap();
        match repo.write_nonce(new_nonce) {
            Ok(_) => (),
            Err(NonceError::NoNonceFile(e)) => {
                println!("Error: unable to create nonce file.");
                println!("  {}", e);
                process::exit(52);
            },
            Err(NonceError::NonceWriteError(e)) => {
                println!("Error: unable to write to nonce file.");
                println!("  {}", e);
                process::exit(53);
            },
            Err(e) => {
                println!("Unexpected error encountered. This is a bug. Please open an issue.");
                println!("  {:?}", e);
                process::exit(99);
            },
        }
        nonce_bag.insert(new_nonce);

        if common::store_in_remote_repo(repo, &remote, &nonce_bag) {
            break 'store;
        }

    }

    if !common::validate_rsl(repo, &remote_rsl, &nonce_bag) {
        println!("Error: invalid remote RSL");
        process::exit(-1);
    }

    common::reset_local_rsl_to_remote_rsl(repo);
}

fn all_push_entries_in_fetch_head(repo: &Repository, push_entries: &Vec<Option<common::PushEntry>>) -> bool {
    false
}

