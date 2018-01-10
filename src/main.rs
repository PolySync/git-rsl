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

use std::env;

use git2::Repository;

mod common;
mod push;
mod fetch;
mod utils;

fn discover_repo() -> Result<Repository, git2::Error> {
    let current_dir = env::current_dir().unwrap();
    Repository::discover(current_dir)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let repo = discover_repo().unwrap();

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

    let remote = matches.value_of("remote").unwrap().clone();
    let branches: Vec<&str> = matches.values_of("branch").unwrap().collect();
    if program == "git-securefetch" || matches.is_present("fetch") {
        fetch::secure_fetch(&repo, remote, branches);
        return;
    } else if program == "git-securepush" || matches.is_present("push") {
        push::secure_push(&repo, remote, branches);
        return;
    }
}
