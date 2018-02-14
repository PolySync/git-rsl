#[macro_use] extern crate clap;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate error_chain;

extern crate crypto;
extern crate git2;
extern crate libgit2_sys;
extern crate rand;
extern crate serde;
extern crate serde_json;
extern crate fs_extra;
extern crate tempdir;

use std::{env, process};

mod push;
mod fetch;
mod rsl;
mod push_entry;
mod nonce;
mod nonce_bag;
mod utils;
mod errors;

pub use errors::*;
pub use utils::git;

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

    let mut messy_repo = git::discover_repo().unwrap();

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

    let stash_id = match git::stash_local_changes(&mut messy_repo) {
        Ok(Some(id)) => Some(id),
        Ok(None) => None,
        Err(e) => panic!("couldn't stash local changes, or else there were no changes to stash. not sure what libgit2 returns when there is nothing to do"),
    };

    let mut clean_repo = git::discover_repo().unwrap();

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
            fetch::secure_fetch(&clean_repo, &mut remote, &branches).chain_err(|| "error fetching")?;
        } else if program == "git-securepush" || matches.is_present("push") {
            push::secure_push(&clean_repo, &mut remote, &branches).chain_err(|| "error pushing")?;
        }
    }

    match git::checkout_branch(&mut clean_repo, current_branch_name) {
        Ok(()) => (),
        Err(e) => panic!("Couldn't checkout starting branch. Sorry if we messed with your repo state. Ensure you are on the desired branch. It may be necessary to apply changes from the stash: {:?}", e),
    }

    match git::unstash_local_changes(&mut clean_repo, stash_id) {
        Ok(()) => (),
        Err(e) => panic!("Couldn't unstash local changes. Sorry if we messed with your repository state. It may be necessary to apply changes from the stash. {:?}", e),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use push;
    use fetch;
    use utils::test_helper::*;

    #[test]
    fn push_and_fetch() {
        let mut context = setup_fresh();
        {
            let repo = &context.local;
            let mut rem = repo.find_remote("origin").unwrap().to_owned();
            let refs = &["master"];
            let res = push::secure_push(&repo, &mut rem, refs).unwrap();
            do_work_on_branch(&repo, "master");
            let res2 = push::secure_push(&repo, &mut rem, refs).unwrap();
            assert_eq!(res2, ());
            let res3 = fetch::secure_fetch(&repo, &mut rem, refs).unwrap();
            assert_eq!(res3, ());
            do_work_on_branch(&repo, "master");
            let res4 = push::secure_push(&repo, &mut rem, refs).unwrap();
            assert_eq!(res4, ());
            // TODO check that the git log of RSL looks how we want it to
        }
        teardown_fresh(context)
    }

}
