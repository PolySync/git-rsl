#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#[macro_use]
extern crate clap;
extern crate kevlar_laces;
extern crate git2;

use std::process;
use std::env;
use clap::ArgMatches;
pub use kevlar_laces::errors::*;
pub use kevlar_laces::utils::git;

use git2::Repository;

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let matches = clap_app!(kevlar_laces =>
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

    if let Err(ref e) = kevlar_laces::run(&mut repo, &branch_refs, &remote_name, &mode) {
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

fn handle_error(e: &Error) -> () {
    report_error(&e);
    match *e {
        Error(ErrorKind::ReadError(_), _) => {
            process::exit(1)
        }
        Error(_, _) => {
            process::exit(2)
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
