#[macro_use]
extern crate clap;
extern crate crypto;
extern crate git2;
extern crate libgit2_sys;
extern crate rand;
extern crate serde;
extern crate serde_json;
extern crate fs_extra;
#[macro_use]
extern crate serde_derive;

use std::{env, process};

use git2::Repository;

mod common;
mod push;
mod fetch;
mod utils;



fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut messy_repo = common::discover_repo().unwrap();

    let matches = clap_app!(git_rsl =>
                            (name: program.clone())
                            (version: "0.1")
                            (about: "Uses a reference state log to secure fetch and push")
                            (@group mode =>
                                (@arg fetch: --fetch "Securely fetch <branch> checking the reference state log")
                                (@arg push: --push "Securely push <branch> updating the reference state log")
                             )
                            (@arg remote: +required "Remote repository (example: origin)")
                            (@arg branch: ... +required "Branch(es) to securely fetch or push (example: master)")
                            ).get_matches();

    let current_branch_name: &str = &""; // yells at me that this is possibly uninitialized which is not true
    {
        let current_branch_ref = match messy_repo.head() {
            Ok(h) => h,
            Err(e) => panic!("not on a branch: {:?}", e),
        };
        let current_branch_name = match current_branch_ref.name() {
            Some(name) => name,
            None => panic!("lolwut: ur current branch has no object id"),
        };
    }

    let stash_id = match common::stash_local_changes(&mut messy_repo) {
        Ok(Some(id)) => Some(id),
        Ok(None) => None,
        Err(e) => panic!("couldn't stash local changes, or else there were no changes to stash. not sure what libgit2 returns when there is nothing to do"),
    };

    let mut clean_repo = common::discover_repo().unwrap();

    {
        let remote_name = matches.value_of("remote").unwrap().clone();
        let mut remote = match clean_repo.find_remote(remote_name) {
            Ok(r) => r,
            Err(e) => {
                println!("Error: unable to find remote named {}", remote_name);
                println!("  {}", e);
                process::exit(50);
            },
        };

        let branches: Vec<&str> = matches.values_of("branch").unwrap().collect();
        if program == "git-securefetch" || matches.is_present("fetch") {
            fetch::secure_fetch(&clean_repo, &mut remote, branches);
            return;
        } else if program == "git-securepush" || matches.is_present("push") {
            push::secure_push(&clean_repo, &mut remote, branches);
            return;
        }
    }

    match common::checkout_original_branch(&mut clean_repo, current_branch_name) {
        Ok(()) => (),
        Err(e) => panic!("Couldn't checkout starting branch. Sorry if we messed with your repo state. Ensure you are on the desired branch. It may be necessary to apply changes from the stash: {:?}", e),
    }

    match common::unstash_local_changes(&mut clean_repo, stash_id) {
        Ok(()) => (),
        Err(e) => panic!("Couldn't unstash local changes. Sorry if we messed with your repository state. It may be necessary to apply changes from the stash. {:?}", e),
    }


}
