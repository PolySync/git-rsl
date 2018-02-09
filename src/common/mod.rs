

extern crate crypto;
extern crate rand;

use std::{env, process};
use std::vec::Vec;
use std::collections::HashSet;
use std::iter::FromIterator;


use git2;
use git2::{Error, FetchOptions, PushOptions, Oid, Reference, Branch, Commit, RemoteCallbacks, Remote, Repository, Revwalk, DiffOptions, RepositoryState};
use git2::build::CheckoutBuilder;
use git2::BranchType;

use git2::StashApplyOptions;
use git2::STASH_INCLUDE_UNTRACKED;


pub mod push_entry;
pub mod nonce;
pub mod nonce_bag;
pub mod rsl;

pub use self::push_entry::PushEntry;
pub use self::nonce::{Nonce, HasNonce};
pub use self::nonce_bag::{NonceBag, HasNonceBag};
pub use self::rsl::{RSL, HasRSL};


use errors::*;


//TODO implement
pub fn reset_local_rsl_to_remote_rsl(_repo: &Repository) {
}
