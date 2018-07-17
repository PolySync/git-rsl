#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;

extern crate git2;
extern crate git_rsl;

mod cli;

use clap::{App, Arg};
use cli::{collect_args, handle_error};
use git_rsl::secure_push_with_cleanup;

fn main() {
    let matches = App::new("git-secure-push")
        .bin_name("git secure-push")
        .about("Securely push <BRANCH> to <REMOTE> while checking and updating the reference state log to protect against metadata attacks")
        .arg(Arg::with_name("REMOTE")
            .help("The remote repository that is the target of the push operation. (example: origin)")
            .takes_value(false)
            .required(true))
        .arg(Arg::with_name("BRANCH")
            .help("The target branch to push. (example: master)")
            .takes_value(false)
            .required(true))
        .version(crate_version!())
        .author(crate_authors!())
        .get_matches();

    match collect_args(&matches) {
        Ok((remote, branch, mut repo)) => {
            if let Err(ref e) = secure_push_with_cleanup(&mut repo, &remote, &branch) {
                cli::handle_error(e);
            }
            println!("Success!")
        }
        Err(ref e) => handle_error(e),
    };
}
