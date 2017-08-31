use git2::Repository;

use std::process;

use common;
use common::{Nonce, PushEntry};

pub fn secure_push<'repo>(repo: &Repository, remote_name: &str, ref_names: Vec<&str>) {
    let mut remote = match repo.find_remote(remote_name) {
        Ok(r) => r,
        Err(e) => {
            println!("Error: unable to find remote named {}", remote_name);
            println!("  {}", e);
            process::exit(50);
        },
    };

    'push: loop {
        let (remote_rsl, nonce_bag) = common::retrieve_rsl_and_nonce_bag_from_remote_repo(repo, &mut remote);

        if !common::validate_rsl(repo, &remote_rsl, &nonce_bag) {
            println!("Error: invalid remote RSL");
            process::exit(-1);
        }

        let local_rsl = match common::local_rsl_from_repo(repo) {
            Some(branch) => branch,
            None => {
                println!("Warning: No local RSL branch");
                return;
            },
        };

        if local_rsl.target() != remote_rsl.target() {
            println!("Error: You don't have the latest RSL, please fetch first");
            return;
        }

        let remote_oid = remote_rsl.target().unwrap();

        let latest_push_entry = PushEntry::from_oid(remote_oid).unwrap();
        let prev_hash = latest_push_entry.hash();
        //TODO change this to be all ref_names
        let new_push_entry = PushEntry::new(repo, ref_names.first().unwrap(), prev_hash);

        if common::store_in_remote_repo(repo, &remote, &nonce_bag) {
            //TODO push local related_commits
            //TODO localRSL = RemoteRSL
            break 'push;
        }

    }
}
