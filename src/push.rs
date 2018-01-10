use git2::{Reference, Repository};

use std::process;

use common::{self, PushEntry};
use common::nonce_bag::{NonceBag, HasNonceBag};

pub fn secure_push<'repo>(repo: &Repository, remote_name: &str, ref_names: Vec<&str>) {
    let mut remote = match repo.find_remote(remote_name) {
        Ok(r) => r,
        Err(e) => {
            println!("Error: unable to find remote named {}", remote_name);
            println!("  {}", e);
            process::exit(50);
        },
    };

    let mut remote_rsl: Reference;
    let mut nonce_bag: NonceBag;

    //let mut refs = ref_names.iter().filter_map(|name| &repo.find_reference(name).ok());

    'push: loop {
        let (remote_rsl, nonce_bag) = match common::retrieve_rsl_and_nonce_bag_from_remote_repo(repo, &mut remote) {
            Some((rsl, bag)) => (rsl, bag),
            None => common::rsl_init(repo, &mut remote),
        };

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

        let latest_push_entry = PushEntry::from_oid(&repo, remote_oid).unwrap();
        let prev_hash = latest_push_entry.hash();
        //TODO change this to be all ref_names
        let _new_push_entry = PushEntry::new(repo, ref_names.first().unwrap(), prev_hash, nonce_bag.clone());

        if common::store_in_remote_repo(repo, &remote, &nonce_bag) {
            //TODO push local related_commits
            match common::push(repo, &mut remote, &ref_names) {
                Ok(_) => (),
                Err(e) => {
                    println!("Error: unable to push reference(s) {} to remote {}", &ref_names.clone().join(", "), &remote_name);
                    println!("  {}", e);
                    process::exit(51);
                },
            };
            //TODO localRSL = RemoteRSL
            break 'push;
        }

    }
}
