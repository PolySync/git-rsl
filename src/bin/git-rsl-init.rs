#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;

extern crate git2;
extern crate git_rsl;

mod cli;

use clap::{App, Arg};
use cli::{collect_args, handle_error};
use git_rsl::rsl_init_with_cleanup;

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

    match collect_args(&matches) {
        Ok((remote, _, mut repo)) => {
            if let Err(ref e) = rsl_init_with_cleanup(&mut repo, &remote) {
                cli::handle_error(e);
            }
            println!("Success!")
        }
        Err(ref e) => handle_error(e),
    };
}
