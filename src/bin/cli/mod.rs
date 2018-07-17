use clap::ArgMatches;
use git2::Repository;
use git_rsl::errors::*;
use git_rsl::utils::git;
use git_rsl::{BranchName, RemoteName};
use std::process;

pub fn collect_args<'a>(
    matches: &'a ArgMatches,
) -> Result<(RemoteName<'a>, BranchName<'a>, Repository)> {
    let remote = match matches.value_of("REMOTE") {
        None => bail!("Must supply a REMOTE argument"),
        Some(v) => RemoteName::new(v),
    };

    let branch = match matches.value_of("BRANCH") {
        None => bail!("Must supply a BRANCH argument"),
        Some(v) => BranchName::new(v),
    };

    let repo = match git::discover_repo() {
        Err(_) => {
            bail!("You don't appear to be in a git project. Please check yourself and try again")
        }
        Ok(repo) => repo,
    };

    Ok((remote, branch, repo))
}

pub fn handle_error(e: &Error) -> () {
    report_error(&e);
    match *e {
        Error(ErrorKind::ReadError(_), _) => process::exit(1),
        Error(_, _) => process::exit(2),
    }
}
