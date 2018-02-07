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
#[macro_use]
extern crate error_chain;


use std::{env, process};

use git2::Repository;

mod common;
mod push;
mod fetch;
mod utils;



use common::errors::*;

fn main() {
    if let Err(ref e) = run() {
        println!("error: {}", e);
        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }
        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
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

    let current_branch_name = &messy_repo.head()?
        .name()
        .ok_or("Not on a named branch.")? // TODO allow this??
        .to_owned();

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
            fetch::secure_fetch(&clean_repo, &mut remote, branches).chain_err(|| "error fetching")?;
        } else if program == "git-securepush" || matches.is_present("push") {
            push::secure_push(&clean_repo, &mut remote, branches).chain_err(|| "error pushing")?;
        }
    }

    match common::checkout_branch(&mut clean_repo, current_branch_name) {
        Ok(()) => (),
        Err(e) => panic!("Couldn't checkout starting branch. Sorry if we messed with your repo state. Ensure you are on the desired branch. It may be necessary to apply changes from the stash: {:?}", e),
    }

    match common::unstash_local_changes(&mut clean_repo, stash_id) {
        Ok(()) => (),
        Err(e) => panic!("Couldn't unstash local changes. Sorry if we messed with your repository state. It may be necessary to apply changes from the stash. {:?}", e),
    }

    Ok(())
}
