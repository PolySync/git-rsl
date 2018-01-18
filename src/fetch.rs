use std::process;
use std::vec::Vec;

use git2::{Reference, Repository, Remote};

use common;
use common::{NonceBag, HasNonceBag};
use common::rsl::{RSL, HasRSL};
use common::nonce::{Nonce, HasNonce, NonceError};

pub fn secure_fetch<'repo>(repo: &Repository, mut remote: &mut Remote, ref_names: Vec<&str>) {

    let mut remote_rsl: RSL = unsafe { ::std::mem::uninitialized() };
    let mut local_rsl: RSL = unsafe { ::std::mem::uninitialized() };
    let mut nonce_bag: NonceBag = unsafe { ::std::mem::uninitialized() };
    let mut nonce: Nonce = unsafe { ::std::mem::uninitialized() };

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
            repo.init_rsl_if_needed(&mut remote);

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

            match common::fetch(repo, &mut remote, &ref_names, None) {
                Ok(_) => (),
                Err(e) => {
                    println!("Error: unable to fetch reference {} from remote {}", &ref_names.clone().join(", "), &remote.name().unwrap());
                    println!("  {}", e);
                    process::exit(51);
                },
            };

            if common::all_push_entries_in_fetch_head(&repo, &ref_names) {
                break 'fetch;
            }
            counter -= 1;
        }

        // update nonce bag
        if nonce_bag.bag.contains(&nonce) {
            nonce_bag.remove(&nonce);
        }

        let new_nonce = common::Nonce::new().unwrap();
        match repo.write_nonce(&new_nonce) {
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
        };

        nonce_bag.insert(new_nonce);
        repo.write_nonce_bag(&nonce_bag);
        repo.commit_nonce_bag();
        match repo.push_rsl(&mut remote) {
            Ok(()) => break 'store,
            _ => (),
        }
        store_counter -= 1;
    }

    if !common::validate_rsl(repo, &remote_rsl, &local_rsl, &nonce_bag, &nonce) {
        println!("Error: invalid remote RSL");
        process::exit(-1);
    }

    // fast forward fetched refs
    common::reset_local_rsl_to_remote_rsl(repo);
}
