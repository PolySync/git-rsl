extern crate getopts;
extern crate git2;

use std::env;

use getopts::Options;
use git2::Repository;

mod common;
mod push;
mod fetch;


fn discover_repo() -> Result<Repository, git2::Error> {
    let current_dir = env::current_dir().unwrap();
    Repository::discover(current_dir)
}
fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let repo = discover_repo().unwrap();

    let mut opts = Options::new();

    opts.optflag("h", "help", "print this help menu");
    opts.optflag("", "fetch", "securely fetch, adding new entry to RSL nonce bag");
    opts.optflag("", "push", "securely push, logging this push in the RSL");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    if program == "git-securefetch" || matches.opt_present("fetch") {
        fetch::secure_fetch(&repo);
        return;
    } else if program == "git-securepush" || matches.opt_present("push") {
        push::secure_push(&repo);
        return;
    } else {
        print_usage(&program, opts);
        return;
    }
}
