#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;

extern crate crypto;
extern crate git2;
//extern crate libgit2_sys;
extern crate fs_extra;
extern crate gpgme;
extern crate hex;
extern crate rand;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate tempdir;
extern crate tempfile;

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
use clap::ArgMatches;

pub use errors::*;
pub use utils::git;

use git2::{Oid, Repository};

fn main() {
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

    let (branches, remote_name, mode) = parse_args(&matches, &program);
    let branch_refs: Vec<&str> = branches.iter().map(|x| x.as_str()).collect();
    let mut repo = init_repo();

    if let Err(ref e) = run(&mut repo, &branch_refs, &remote_name, &mode) {
        handle_error(e);
        process::exit(1);
    }
    println!("Success!")
}

fn parse_args(matches: &ArgMatches, program: &str) -> (Vec<String>, String, String) {
    let branches: Vec<String> = matches.values_of("branch").unwrap().map(|x| x.to_owned()).collect();
    let remote_name = matches.value_of("remote").unwrap().to_owned();
    let mode = if program == "git-securefetch" || matches.is_present("fetch") {
        "fetch".to_owned()
    } else if program == "git-securepush" || matches.is_present("push") {
        "push".to_owned()
    } else {
        unreachable!();
    };
    (branches, remote_name, mode)
}

fn init_repo() -> Repository {
    git::discover_repo().expect("You don't appear to be in a git project. Please check yourself and try again")
}

fn run(mut repo: &mut Repository, branches: &[&str], remote_name: &str, mode: &str) -> Result<()> {
    let (original_branch_name, stash_id, original_dir) = prep_workspace(&mut repo)?;

    let result = {
        let mut remote = (&repo)
            .find_remote(remote_name)
            .chain_err(|| format!("unable to find remote named {}", remote_name))?;


        let result = if mode == "fetch" {
            fetch::secure_fetch(&repo, &mut remote, &branches)
        } else if mode == "push" {
            push::secure_push(&repo, &mut remote, &branches)
        } else {
            panic!("this shouldn't happen");
        };
        result
    };

    restore_workspace(
        &mut repo,
        &original_branch_name,
        stash_id,
        original_dir,
    )?;

    result
}

fn handle_error(e: &Error) -> () {
    report_error(&e);
    match *e {
        Error(ErrorKind::ReadError(_), _) => {
            process::exit(-1)
        }
        Error(_, _) => {
            process::exit(-2)
        }
    }
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

// Returns a tuple containing the branch name, Some(stash_commit_id) if a stash took place or None if it was not necessary, and the path to the original working directory (if the user is not in the project root), in that order.
fn prep_workspace(mut repo: &mut Repository) -> Result<(String, Option<Oid>, Option<PathBuf>)> {
    let current_branch_name = repo.head()?
        .name()
        .ok_or("Not on a named branch. Please switch to one so we can put you back where you started when this is all through.")? // TODO allow this??
        .to_owned();

    let stash_id =
        git::stash_local_changes(&mut repo).chain_err(|| "Couldn't stash local changes.")?;

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

fn restore_workspace(
    mut repo: &mut Repository,
    original_branch_name: &String,
    stash_id: Option<Oid>,
    original_working_directory: Option<PathBuf>,
) -> Result<()> {
    println!("Returning to {} branch", original_branch_name);
    git::checkout_branch(repo, original_branch_name).chain_err(|| {
        "Couldn't checkout starting branch. Sorry if we messed with your repo state. Ensure you are on the desired branch. It may be necessary to apply changes from the stash"
    })?;

    if let Some(dir) = original_working_directory {
        env::set_current_dir(dir)?;
    }

    if let Some(_) = stash_id {
        println!("Unstashing local changes");
    }
    git::unstash_local_changes(&mut repo, stash_id).chain_err(|| {
        "Couldn't unstash local changes. Sorry if we messed with your repository state. It may be necessary to apply changes from the stash. {:?}"
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use push;
    use fetch;
    use errors::*;
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
            do_work_on_branch(&repo, "refs/heads/master");
            let res2 = push::secure_push(&repo, &mut rem, refs).unwrap();
            assert_eq!(res2, ());
            let res3 = fetch::secure_fetch(&repo, &mut rem, refs).unwrap();
            assert_eq!(res3, ());
            do_work_on_branch(&repo, "refs/heads/master");
            let res4 = push::secure_push(&repo, &mut rem, refs).unwrap();
            assert_eq!(res4, ());
            // TODO check that the git log of RSL looks how we want it to
        }
        teardown_fresh(context)
    }

    #[test]
    fn error_handling() {
        let mut context = setup_fresh();
        {
            let refs = &["master"];
            let res = super::run(&mut context.local, &[&"master"], &"origin", &"push").unwrap();
            assert_eq!(res, ());

            let nonce_file = context.repo_dir.join(".git/NONCE");
            Command::new("chmod")
            .arg("000")
            .arg(nonce_file.to_string_lossy().into_owned())
            .output()
            .expect("failed to change permissions");

            do_work_on_branch(&context.local, "refs/heads/master");
            //let res2 = push::secure_push(&repo, &mut rem, refs).unwrap_err();
            let res2 = super::run(&mut context.local, &[&"master"], &"origin", &"push").unwrap_err();
            // assert that we are on the right branch_head
            let head = context.local.head().unwrap().name().unwrap().to_owned();
            assert_eq!(head, "refs/heads/master");
            assert_eq!(res2.description(), "");

        }
        teardown_fresh(context)
    }
}
