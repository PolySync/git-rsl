#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#[macro_use] extern crate clap;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate error_chain;

extern crate crypto;
extern crate git2;
//extern crate libgit2_sys;
extern crate rand;
extern crate serde;
extern crate serde_json;
extern crate fs_extra;
extern crate tempdir;
extern crate tempfile;
extern crate hex;
extern crate gpgme;
extern crate regex;

use std::env;

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
        report_error(e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    // TODO exit unless gpgtools are present

    let mut messy_repo = git::discover_repo()
        .chain_err(|| "You don't appear to be in a git project. Please check yourself and try again")?;

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
        .ok_or("Not on a named branch. Please switch to one so we can put you back where you started when this is all through.")? // TODO allow this??
        .to_owned();

    // TODO save current working directory and cd to project root, hop back later

    let stash_id = git::stash_local_changes(&mut messy_repo)
        .chain_err(|| "Couldn't stash local changes.")?;

    let mut clean_repo = git::discover_repo().unwrap();

    {
        let remote_name = matches.value_of("remote").unwrap().clone();
        let mut remote = clean_repo.find_remote(remote_name)
            .chain_err(|| format!("unable to find remote named {}", remote_name))?;

        let branches: Vec<&str> = matches.values_of("branch").unwrap().collect();
        if program == "git-securefetch" || matches.is_present("fetch") {
            fetch::secure_fetch(&clean_repo, &mut remote, &branches).chain_err(|| "error fetching")?;
        } else if program == "git-securepush" || matches.is_present("push") {
            push::secure_push(&clean_repo, &mut remote, &branches).chain_err(|| "error pushing")?;
        }
    }

    git::checkout_branch(&clean_repo, current_branch_name)
        .chain_err(|| "Couldn't checkout starting branch. Sorry if we messed with your repo state. Ensure you are on the desired branch. It may be necessary to apply changes from the stash")?;

    git::unstash_local_changes(&mut clean_repo, stash_id)
        .chain_err(|| "Couldn't unstash local changes. Sorry if we messed with your repository state. It may be necessary to apply changes from the stash. {:?}")?;

    Ok(())
}

fn report_error(e: &Error) {
    println!("error: {}", e);
    for e in e.iter().skip(1) {
        println!("caused by: {}", e);
    }
    if let Some(backtrace) = e.backtrace() {
        println!("backtrace: {:?}", backtrace);
    }
}

#[cfg(test)]
mod tests {
    use push;
    use fetch;
    use utils::test_helper::*;

    #[test]
    fn push_and_fetch() {
        let context = setup_fresh();
        {
            let repo = &context.local;
            let mut rem = repo.find_remote("origin").unwrap().to_owned();
            let refs = &["master"];
            let res = push::secure_push(&repo, &mut rem, refs).unwrap();
            assert_eq!(res, ());
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
