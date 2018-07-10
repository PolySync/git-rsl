#[macro_use]
extern crate clap;

extern crate git2;
extern crate git_rsl;

use std::process;

use clap::{App, Arg};
use git_rsl::errors::*;
use git_rsl::utils::git;
use git_rsl::{rsl_init_with_cleanup, RemoteName};

fn main() {
    let matches = App::new("git-rsl-init")
        .bin_name("git rsl-init")
        .about("Begin reference state log usage for this repository, using <REMOTE> for storage.")
        .arg(Arg::with_name("REMOTE")
            .help("The remote repository that will store reference state log usage in its 'rsl' branch.")
            .takes_value(false)
            .required(true))
        .version(crate_version!())
        .author(crate_authors!())
        .get_matches();

    let remote = match matches.value_of("REMOTE") {
        None => panic!("Must supply a REMOTE argument"),
        Some(v) => v.to_owned(),
    };

    let mut repo = git::discover_repo()
        .expect("You don't appear to be in a git project. Please check yourself and try again");

    if let Err(ref e) = rsl_init_with_cleanup(&mut repo, &RemoteName::new(&remote)) {
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
