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

mod push;
mod fetch;
mod rsl;
mod push_entry;
mod nonce;
mod nonce_bag;
mod utils;
mod errors;

use std::process;
use std::env;
use std::path::PathBuf;

pub use errors::*;
pub use utils::git;

use git2::{Repository, Oid};

fn main() {
    if let Err(ref e) = run() {
        report_error(e);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

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

    // TODO exit unless gpgtools are present

    let mut messy_repo = git::discover_repo()
        .chain_err(|| "You don't appear to be in a git project. Please check yourself and try again")?;

    let (original_branch_name, stash_id, original_dir) = prep_workspace(&mut messy_repo)?;

    let mut clean_repo = git::discover_repo().unwrap();

    let remote_name = matches.value_of("remote").unwrap().clone();
    let mut remote = (&clean_repo).find_remote(remote_name)
        .chain_err(|| format!("unable to find remote named {}", remote_name))?;

    let branches: Vec<&str> = matches.values_of("branch").unwrap().collect();

    let result = if program == "git-securefetch" || matches.is_present("fetch") {
        fetch::secure_fetch(&clean_repo, &mut remote, &branches)
    } else if program == "git-securepush" || matches.is_present("push") {
        push::secure_push(&clean_repo, &mut remote, &branches)
    } else {
        unreachable!();
    };

    // process results of operation
    let mut cleaner_repo = git::discover_repo()?;
    if let Err(e) = result {
        handle_error(&e, &mut cleaner_repo, &original_branch_name, stash_id, original_dir)?;
    } else {
        restore_workspace(&mut cleaner_repo, &original_branch_name, stash_id, original_dir)?;
        println!("Success!")
    }

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

fn handle_error(e: &Error,  mut clean_repo: &mut Repository, current_branch_name: &String, stash_id: Option<Oid>, original_dir: Option<PathBuf>) -> Result<()> {
    match *e {
        Error(ErrorKind::InvalidRSL, _) =>
            {
                report_error(&e);
                // unbork: reset remote RSL to local at last good state
                restore_workspace(&mut clean_repo, &current_branch_name, stash_id, original_dir);
                process::exit(-1)
            },
        Error(_,_) =>
            {
                report_error(&e);
                // unbork: reset remote RSL to local at last good state
                restore_workspace(&mut clean_repo, &current_branch_name, stash_id, original_dir);
                process::exit(-2)
            }
    }
}

// Returns a tuple containing the branch name, Some(stash_commit_id) if a stash took place or None if it was not necessary, and the path to the original working directory (if the user is not in the project root), in that order.
fn prep_workspace(mut repo: &mut Repository) -> Result<(String, Option<Oid>, Option<PathBuf>)> {
    let current_branch_name = repo.head()?
        .name()
        .ok_or("Not on a named branch. Please switch to one so we can put you back where you started when this is all through.")? // TODO allow this??
        .to_owned();

    let stash_id = git::stash_local_changes(&mut repo)
        .chain_err(|| "Couldn't stash local changes.")?;

    // save current working directory and cd to project root
    let cwd = env::current_dir()?;
    let project_root = repo.workdir().ok_or("RSL not supported for bare repos")?;
    let original_dir = if project_root != cwd {
        env::set_current_dir(&project_root)?;
        Some(cwd)
    } else {
        None
    };

    Ok((current_branch_name, stash_id, original_dir))
}

fn restore_workspace(mut repo: &mut Repository, original_branch_name: &String, stash_id: Option<Oid>, original_working_directory: Option<PathBuf>) -> Result<()> {
    git::checkout_branch(repo, original_branch_name)
        .chain_err(|| "Couldn't checkout starting branch. Sorry if we messed with your repo state. Ensure you are on the desired branch. It may be necessary to apply changes from the stash")?;

    if let Some(dir) = original_working_directory {
        env::set_current_dir(dir)?;

    }

    git::unstash_local_changes(&mut repo, stash_id)
        .chain_err(|| "Couldn't unstash local changes. Sorry if we messed with your repository state. It may be necessary to apply changes from the stash. {:?}")?;
    Ok(())
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
