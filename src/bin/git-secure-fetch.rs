#[macro_use]
extern crate clap;

extern crate git2;
extern crate git_rsl;

use std::process;

use clap::{App, Arg};
use git_rsl::errors::*;
use git_rsl::utils::git;
use git_rsl::{secure_fetch_with_cleanup, BranchName, RemoteName};

fn main() {
    let matches = App::new("git-secure-fetch")
        .bin_name("git secure-fetch")
        .about("Securely fetch <BRANCH> from <REMOTE> checking the reference state log to protect against metadata attacks")
        .arg(Arg::with_name("REMOTE")
            .help("The remote repository that is the source of the fetch operation.")
            .takes_value(false)
            .required(true))
        .arg(Arg::with_name("BRANCH")
            .help("The target branch to fetch.")
            .takes_value(false)
            .required(true))
        .version(crate_version!())
        .author(crate_authors!())
        .get_matches();

    let remote = match matches.value_of("REMOTE") {
        None => panic!("Must supply a REMOTE argument"),
        Some(v) => v.to_owned(),
    };

    let branch = match matches.value_of("BRANCH") {
        None => panic!("Must supply a BRANCH argument"),
        Some(v) => v.to_owned(),
    };
    // TODO - reduce code duplication across the top level of the binaries
    let mut repo = git::discover_repo()
        .expect("You don't appear to be in a git project. Please check yourself and try again");

    if let Err(ref e) = secure_fetch_with_cleanup(
        &mut repo,
        &RemoteName::new(&remote),
        &BranchName::new(&branch),
    ) {
        handle_error(e);
        process::exit(1);
    }
    println!("Success!")
}

fn handle_error(e: &Error) -> () {
    report_error(&e);
    match *e {
        Error(ErrorKind::ReadError(_), _) => process::exit(1),
        Error(_, _) => process::exit(2),
    }
}
