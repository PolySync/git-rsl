extern crate crypto;
extern crate rand;

use std::collections::HashSet;
use std::process;
use std::vec::Vec;

use git2;
use git2::{Cred, FetchOptions, PushOptions, Oid, ProxyOptions, Reference, Branch, RemoteCallbacks, Remote, Repository, Revwalk};
use git2::BranchType;
use rand::Rng;

mod push_entry;
pub mod nonce;
pub use self::push_entry::PushEntry;
pub use self::nonce::Nonce;
pub use self::nonce::HasNonce;

const RSL_BRANCH: &'static str = "RSL";
const NONCE_BRANCH: &'static str = "RSL_NONCE";
const REFLOG_MSG: &'static str = "Retrieve RSL branchs from remote";

pub fn rsl_init<'repo>(repo: &'repo Repository, remote: &mut Remote) -> (Branch<'repo>, Branch<'repo>){
    // make branch
    // TODO: figure out a way to orphan branch; .branch() needs a commit ref.
    let initial_commit = repo.find_commit(Oid::from_str("134e253f2850089bb37607f21b619231c12f90d4").unwrap()).unwrap();
    let rsl = repo.branch("RSL", &initial_commit, false).unwrap();
    let nonce = repo.branch("RSL_NONCE", &initial_commit, false).unwrap();
    push(repo, remote, &[&rsl.name().unwrap().unwrap(), &nonce.name().unwrap().unwrap()]);
    (rsl, nonce)
}

pub fn fetch(repo: &Repository, remote: &mut Remote, ref_names: &[&str], reflog_msg: Option<&str>) -> Result<(), ::git2::Error> {
    let cfg = repo.config().unwrap();
    let remote_copy = remote.clone();
    let url = remote_copy.url().unwrap();

    with_authentication(url, &cfg, |f| {
        let mut cb = RemoteCallbacks::new();
        cb.credentials(f);
        let mut opts = FetchOptions::new();
        opts.remote_callbacks(cb);

        remote.fetch(&[RSL_BRANCH, NONCE_BRANCH], Some(&mut opts), Some(REFLOG_MSG))
    })
}

pub fn push(repo: &Repository, remote: &mut Remote, ref_names: &[&str]) -> Result<(), git2::Error> {
    let cfg = repo.config().unwrap();
    let remote_copy = remote.clone();
    let url = remote_copy.url().unwrap();

    with_authentication(url, &cfg, |f| {
        let mut cb = RemoteCallbacks::new();
        cb.credentials(f);
        let mut opts = PushOptions::new();
        opts.remote_callbacks(cb);

        //remote.connect(Direction::Push)?;
        remote.push(&[RSL_BRANCH, NONCE_BRANCH], Some(&mut opts))
    })
}

pub fn retrieve_rsl_and_nonce_bag_from_remote_repo<'repo>(repo: &'repo Repository, mut remote: &mut Remote) -> (Reference<'repo>, HashSet<Nonce>) {

    fetch(repo, remote, &[RSL_BRANCH, NONCE_BRANCH], Some(REFLOG_MSG));
    let (remote_rsl, remote_nonce) = match (
            repo.find_branch(RSL_BRANCH, BranchType::Remote),
            repo.find_branch(NONCE_BRANCH, BranchType::Remote),
            repo.find_branch(RSL_BRANCH, BranchType::Local),
            repo.find_branch(NONCE_BRANCH, BranchType::Local)
        ) {
            (Err(_), Err(_), Err(_), Err(_)) => rsl_init(repo, &mut remote),
            (Ok(rsl), Ok(nonce), Err(_), Err(_)) => (rsl, nonce),
            (Err(_), Err(_), Ok(_), Ok(_)) => process::exit(10), // something compromised. rsl got deleted remotely
            (Ok(rsl), Ok(nonce), Ok(_), Ok(_)) => (rsl, nonce),
            (Ok(_), Err(_), _, _) => process::exit(10),
            (Err(_), Ok(_), _, _) => process::exit(10),
            (_, _, Ok(_), Err(_)) => process::exit(10),
            (_, _, Err(_), Ok(_)) => process::exit(10)
    };

    let nonce_bag = read_nonce_bag(&remote_nonce.into_reference());
    (remote_rsl.into_reference(), nonce_bag)
}

pub fn store_in_remote_repo(repo: &Repository, remote: &Remote, nonce_bag: &HashSet<Nonce>) -> bool {
    false
}

pub fn validate_rsl(repo: &Repository, remote_rsl: &Reference, nonce_bag: &HashSet<Nonce>) -> bool {
    let repo_nonce = match repo.read_nonce() {
        Ok(nonce) => nonce,
        Err(e) => {
            //TODO Figure out what needs to happen when a nonce doeesn't exist because we're never
            //fetched
            println!("Error: Couldn't read nonce: {:?}", e);
            return false;
        },
    };
    if !nonce_bag.contains(&repo_nonce) /* TODO: && repo_nonce not in remote_rsl.push_after(local_rsl*/ {
        return false;
    }

    let local_rsl = match local_rsl_from_repo(repo) {
        Some(reference) => reference,
        None => {
            println!("Warning: No local RSL branch");
            return false;
        },
    };
    let local_oid = local_rsl.target().unwrap();
    let remote_oid = remote_rsl.target().unwrap();

    if !repo.graph_descendant_of(remote_oid, local_oid).unwrap_or(false) {
        println!("Error: No path to get from Local RSL to Remote RSL");
        return false;
    }

    let last_local_push_entry = PushEntry::from_ref(&local_rsl).unwrap();
    let mut last_hash = last_local_push_entry.hash();

    let mut revwalk: Revwalk = repo.revwalk().unwrap();
    revwalk.push(remote_oid);
    revwalk.set_sorting(git2::SORT_REVERSE);
    revwalk.hide(local_oid);
    let remaining = revwalk.map(|oid| oid.unwrap());

    let result = remaining.fold(Some(last_hash), |prev_hash, oid| {
        let current_push_entry = PushEntry::from_oid(oid).unwrap();
        let current_prev_hash = current_push_entry.prev_hash();
        let current_hash = current_push_entry.hash();
        if prev_hash == Some(current_prev_hash) {
            Some(current_hash)
        } else {
            None
        }
    });

    if result != None { return false; }


    verify_signature(remote_oid)

}

fn verify_signature(oid: Oid) -> bool {
    return false
}

fn for_each_commit_from<F>(repo: &Repository, local: Oid, remote: Oid, f: F)
    where F: Fn(Oid) -> ()
{
    let mut revwalk: Revwalk = repo.revwalk().unwrap();
    revwalk.push(remote);
    revwalk.set_sorting(git2::SORT_REVERSE);
    revwalk.hide(local);
    let remaining = revwalk.map(|oid| oid.unwrap());

    for oid in remaining {
        f(oid)
    }
}

pub fn local_rsl_from_repo(repo: &Repository) -> Option<Reference> {
    match repo.find_reference(RSL_BRANCH) {
        Ok(r) => Some(r),
        Err(_) => None,
    }
}

pub fn last_push_entry_for(repo: &Repository, remote: &Remote, reference: &str) -> Option<PushEntry> {
    let fully_qualified_ref_name = format!("{}/{}", remote.name().unwrap(), reference);
    //TODO Actually walk the commits and look for the most recent for the branch we're interested
    //in
    Some(PushEntry::new(repo, &fully_qualified_ref_name, String::from("")))
}

//TODO implement
pub fn reset_local_rsl_to_remote_rsl(repo: &Repository) {
}

//TODO implement
fn is_push_entry(nonce_branch: &Reference) -> bool {
    true
}

fn read_nonce_bag(remote_nonce: &Reference) -> HashSet<Nonce> {
    if is_push_entry(remote_nonce) {
        HashSet::new()
    } else {
        //TODO actually read the contents of the nonce bag from the commit
        let existing_nonce = rand::random::<Nonce>();
        let mut set = HashSet::new();
        set.insert(existing_nonce);
        set
    }

}

fn with_authentication<T, F>(url: &str, cfg: &git2::Config, mut f: F)
                             -> Result<T, ::git2::Error>
    where F: FnMut(&mut git2::Credentials) -> Result<T, ::git2::Error>
{
    let mut cred_helper = git2::CredentialHelper::new(url);
    cred_helper.config(cfg);

    let mut ssh_username_requested = false;
    let mut cred_helper_bad = None;
    let mut ssh_agent_attempts = Vec::new();
    let mut any_attempts = false;
    let mut tried_sshkey = false;

    let mut res = f(&mut |url, username, allowed| {
        any_attempts = true;
        // libgit2's "USERNAME" authentication actually means that it's just
        // asking us for a username to keep going. This is currently only really
        // used for SSH authentication and isn't really an authentication type.
        // The logic currently looks like:
        //
        //      let user = ...;
        //      if (user.is_null())
        //          user = callback(USERNAME, null, ...);
        //
        //      callback(SSH_KEY, user, ...)
        //
        // So if we're being called here then we know that (a) we're using ssh
        // authentication and (b) no username was specified in the URL that
        // we're trying to clone. We need to guess an appropriate username here,
        // but that may involve a few attempts. Unfortunately we can't switch
        // usernames during one authentication session with libgit2, so to
        // handle this we bail out of this authentication session after setting
        // the flag `ssh_username_requested`, and then we handle this below.
        if allowed.contains(git2::USERNAME) {
            debug_assert!(username.is_none());
            ssh_username_requested = true;
            return Err(git2::Error::from_str("gonna try usernames later"))
        }

        // An "SSH_KEY" authentication indicates that we need some sort of SSH
        // authentication. This can currently either come from the ssh-agent
        // process or from a raw in-memory SSH key. Cargo only supports using
        // ssh-agent currently.
        //
        // If we get called with this then the only way that should be possible
        // is if a username is specified in the URL itself (e.g. `username` is
        // Some), hence the unwrap() here. We try custom usernames down below.
        if allowed.contains(git2::SSH_KEY) && !tried_sshkey {
            // If ssh-agent authentication fails, libgit2 will keep
            // calling this callback asking for other authentication
            // methods to try. Make sure we only try ssh-agent once,
            // to avoid looping forever.
            tried_sshkey = true;
            let username = username.unwrap();
            debug_assert!(!ssh_username_requested);
            ssh_agent_attempts.push(username.to_string());
            return git2::Cred::ssh_key_from_agent(username)
        }

        // Sometimes libgit2 will ask for a username/password in plaintext. This
        // is where Cargo would have an interactive prompt if we supported it,
        // but we currently don't! Right now the only way we support fetching a
        // plaintext password is through the `credential.helper` support, so
        // fetch that here.
        if allowed.contains(git2::USER_PASS_PLAINTEXT) {
            let r = git2::Cred::credential_helper(cfg, url, username);
            cred_helper_bad = Some(r.is_err());
            return r
        }

        // I'm... not sure what the DEFAULT kind of authentication is, but seems
        // easy to support?
        if allowed.contains(git2::DEFAULT) {
            return git2::Cred::default()
        }

        // Whelp, we tried our best
        Err(git2::Error::from_str("no authentication available"))
    });

    // Ok, so if it looks like we're going to be doing ssh authentication, we
    // want to try a few different usernames as one wasn't specified in the URL
    // for us to use. In order, we'll try:
    //
    // * A credential helper's username for this URL, if available.
    // * This account's username.
    // * "git"
    //
    // We have to restart the authentication session each time (due to
    // constraints in libssh2 I guess? maybe this is inherent to ssh?), so we
    // call our callback, `f`, in a loop here.
    if ssh_username_requested {
        debug_assert!(res.is_err());
        let mut attempts = Vec::new();
        attempts.push("git".to_string());
        if let Ok(s) = ::std::env::var("USER").or_else(|_| ::std::env::var("USERNAME")) {
            attempts.push(s);
        }
        if let Some(ref s) = cred_helper.username {
            attempts.push(s.clone());
        }

        while let Some(s) = attempts.pop() {
            // We should get `USERNAME` first, where we just return our attempt,
            // and then after that we should get `SSH_KEY`. If the first attempt
            // fails we'll get called again, but we don't have another option so
            // we bail out.
            let mut attempts = 0;
            res = f(&mut |_url, username, allowed| {
                if allowed.contains(git2::USERNAME) {
                    return git2::Cred::username(&s);
                }
                if allowed.contains(git2::SSH_KEY) {
                    debug_assert_eq!(Some(&s[..]), username);
                    attempts += 1;
                    if attempts == 1 {
                        ssh_agent_attempts.push(s.to_string());
                        return git2::Cred::ssh_key_from_agent(&s)
                    }
                }
                Err(git2::Error::from_str("no authentication available"))
            });

            // If we made two attempts then that means:
            //
            // 1. A username was requested, we returned `s`.
            // 2. An ssh key was requested, we returned to look up `s` in the
            //    ssh agent.
            // 3. For whatever reason that lookup failed, so we were asked again
            //    for another mode of authentication.
            //
            // Essentially, if `attempts == 2` then in theory the only error was
            // that this username failed to authenticate (e.g. no other network
            // errors happened). Otherwise something else is funny so we bail
            // out.
            if attempts != 2 {
                break
            }
        }
    }

    if res.is_ok() || !any_attempts {
        return res.map_err(From::from)
    }

    // In the case of an authentication failure (where we tried something) then
    // we try to give a more helpful error message about precisely what we
    // tried.
    res
}
