 use std::path::Path;
use std::env;

use git2;
use git2::{Error, FetchOptions, PushOptions, Oid, Reference, Signature, Branch, Commit, RemoteCallbacks, Remote, Repository, Revwalk, DiffOptions, RepositoryState};
use git2::build::CheckoutBuilder;
use git2::BranchType;

use git2::StashApplyOptions;
use git2::STASH_INCLUDE_UNTRACKED;

use errors::*;


pub fn checkout_branch(repo: &Repository, ref_name: &str) -> Result<()> {
    let tree = repo.find_reference(ref_name)
        .chain_err(|| "couldn't find branch")?
        .peel_to_commit()
        .chain_err(|| "couldnt find latest RSSL commit")?
        .into_object();

    let mut opts = CheckoutBuilder::new();
    opts.force();
    opts.remove_untracked(true); // this should be fine since we stash untracked at the beginning
    repo.checkout_tree(&tree, Some(&mut opts)).chain_err(|| "couldn't checkout tree")?; // Option<CheckoutBuilder>
    repo.set_head(&ref_name).chain_err(|| "couldn't switch head to RSL")?;
    Ok(())
}

pub fn discover_repo() -> Result<Repository> {
    let current_dir = env::current_dir()?;
    Repository::discover(current_dir).chain_err(|| "cwd is not a git repo")
}

pub fn stash_local_changes(repo: &mut Repository) -> Result<(Option<Oid>)> {
    let signature = repo.signature()?;
    let message = "Stashing local changes for RSL business";

    // check that there are indeed changes in index or untracked to stash
    {
        let is_clean = repo.state() == RepositoryState::Clean;
        let mut diff_options = DiffOptions::new();
        diff_options.include_untracked(true);
        let  diff = repo.diff_index_to_workdir(
            None, // defaults to head index,
            Some(&mut diff_options),
        )?;

        let num_deltas = diff.deltas().count();
        if is_clean && (num_deltas == 0) {
            return Ok(None)
        }
    }
    let oid = repo.stash_save(
        &signature,
        &message,
        Some(STASH_INCLUDE_UNTRACKED),
    )?;
    Ok(Some(oid))
}

pub fn unstash_local_changes(repo: &mut Repository, stash_id: Option<Oid>) -> Result<()> {
    if stash_id == None {
        return Ok(());
    }
    let mut options = StashApplyOptions::new();
    options.reinstantiate_index();
    repo.stash_pop(
        0, // TODO validate SHA of stash commit
        Some(&mut options),
    )?;
    Ok(())
}

pub fn add_and_commit(repo: &Repository, path: Option<&Path>, message: &str, branch: &str) -> Result<Oid> {
    let mut index = repo.index()?;
    if path.is_some() {
        index.add_path(path.unwrap())?;
    }
    let oid = index.write_tree()?;
    let signature = repo.signature()?;
    let ref_name = format!("refs/heads/{}", branch);
    let parent = repo.find_reference(&ref_name).and_then(|x| x.peel_to_commit()).ok();
    let tree = repo.find_tree(oid)?;

    // stupid duplication because &[&T] is a terrible type to mess with
    if let Some(parent_commit) = parent {
        let oid = repo.commit(Some(&ref_name), //  point HEAD to our new commit
                    &signature, // author
                    &signature, // committer
                    message, // commit message
                    &tree, // tree
                    &[&parent_commit])?; // parents
        Ok(oid)
    } else {
        let oid = repo.commit(Some(&ref_name), //  point HEAD to our new commit
                    &signature, // author
                    &signature, // committer
                    message, // commit message
                    &tree, // tree
                    &[])?; // parents
        Ok(oid)
    }
}

pub fn fetch(repo: &Repository, remote: &mut Remote, ref_names: &[&str], _reflog_msg: Option<&str>) -> Result<()> {
    let cfg = repo.config().unwrap();
    let remote_copy = remote.clone();
    let url = remote_copy.url().unwrap();

    with_authentication(url, &cfg, |f| {

        let mut cb = RemoteCallbacks::new();
        cb.credentials(f);
        let mut opts = FetchOptions::new();
        opts.remote_callbacks(cb);

        let reflog_msg = "Retrieve RSL branch from remote";

        remote.fetch(&ref_names, Some(&mut opts), Some(&reflog_msg)).chain_err(|| "could not fetch ref")
    })
}

pub fn push(repo: &Repository, remote: &mut Remote, ref_names: &[&str]) -> Result<()> {
    let cfg = repo.config().unwrap();
    let remote_copy = remote.clone();
    let url = remote_copy.url().unwrap();

    with_authentication(url, &cfg, |f| {
        let mut cb = RemoteCallbacks::new();
        cb.credentials(|a,b,c| f(a,b,c));
        let mut opts = PushOptions::new();
        opts.remote_callbacks(cb);

        let mut refs: Vec<String> = ref_names
            .to_vec()
            .iter()
            .map(|name: &&str| format!("refs/heads/{}:refs/heads/{}", name.to_string(), name.to_string()))
            .collect();

        let mut refs_ref: Vec<&str> = vec![];
        for name in &refs {
            refs_ref.push(&name)
        }

        remote.push(&refs_ref, Some(&mut opts))?;
        Ok(())
    })
}


fn with_authentication<T, F>(url: &str, cfg: &git2::Config, mut f: F)
                             -> Result<T>
    where F: FnMut(&mut git2::Credentials) -> Result<T>
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
            bail!(git2::Error::from_str("gonna try usernames later"))
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
        bail!(git2::Error::from_str("no authentication available"))
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
                    println!("username: {}", &s);

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
                bail!(git2::Error::from_str("no authentication available"));
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

mod test {
    use utils::test_helper::*;

    #[test]
    fn checkout_branch() {
        let context = setup();
        {
            let repo = &context.local;
            assert!(repo.head().unwrap().name().unwrap() == "refs/heads/devel");
            super::checkout_branch(&repo, "RSL").unwrap();
            assert!(repo.head().unwrap().name().unwrap() == "refs/heads/RSL");
        }
        teardown(context)
    }
}
