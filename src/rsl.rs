use git2::{BranchType, Oid, Remote, Repository, Revwalk, Sort};
use git2::build::CheckoutBuilder;

use nonce::{HasNonce, Nonce};
use nonce_bag::{HasNonceBag, NonceBag};
use push_entry::PushEntry;
use errors::*;
use utils::*;

const RSL_BRANCH: &str = "RSL";
const REFLOG_MSG: &str = "Retrieve RSL branches from remote";

#[derive(Debug)]
pub enum RSLType {
    Local,
    Remote,
}

//#[derive(Debug)]
pub struct RSL<'remote, 'repo: 'remote> {
    remote: &'remote mut Remote<'repo>,
    repo: &'remote Repository,
    local_head: Oid,
    remote_head: Oid,
    last_local_push_entry: PushEntry,
    last_remote_push_entry: PushEntry,
    nonce_bag: NonceBag,
    nonce: Nonce,
    username: String,
}

impl<'remote, 'repo> RSL<'remote, 'repo> {
    pub fn read(
        repo: &'repo Repository,
        remote: &'remote mut Remote<'repo>,
    ) -> Result<RSL<'remote, 'repo>> {
        let remote_head = git::oid_from_long_name(repo, "refs/remotes/origin/RSL")?;
        let local_head = git::oid_from_long_name(repo, "refs/heads/RSL")?;
        let last_local_push_entry = find_last_push_entry(repo, &local_head)?;
        let last_remote_push_entry = find_last_push_entry(repo, &remote_head)?;
        let nonce_bag = repo.read_nonce_bag().chain_err(|| "nonce bag read error")?;
        let nonce = repo.read_nonce()?;
        let username = git::username(repo)?;

        let rsl: RSL<'remote, 'repo> = RSL {
            repo,
            remote,
            local_head,
            remote_head,
            last_local_push_entry,
            last_remote_push_entry,
            nonce_bag,
            nonce,
            username,
        };
        Ok(rsl)
    }

    pub fn validate(&self) -> Result<()> {
        // Ensure remote RSL head is a descendant of local RSL head.
        let descendant = self.repo
            .graph_descendant_of(self.remote_head, self.local_head)
            .unwrap_or(false);
        let same = self.local_head == self.remote_head;
        if !descendant && !same {
            bail!("RSL invalid: No path to get from Local RSL to Remote RSL");
        }

        // Walk through the commits from local RSL head, which we know is valid,
        // validating each additional pushentry since that point one by one.
        let last_hash = Some(self.last_local_push_entry.hash());

        let mut revwalk: Revwalk = self.repo.revwalk()?;
        revwalk.push(self.remote_head)?;
        revwalk.set_sorting(Sort::REVERSE);
        revwalk.hide(self.local_head)?;

        let remaining = revwalk.map(|oid| oid.unwrap());
        println!("gets to validate");
        let result = remaining
            .inspect(|x| println!("about to fold: {}", x))
            .fold(last_hash, |prev_hash, oid| {
                //println!("last hash: {:?}", last_hash);
                println!("prev_hash: {:?}", prev_hash);
                println!("oid {:?}", oid);
                let current_push_entry = PushEntry::from_oid(self.repo, &oid).unwrap_or(None);
                match current_push_entry {
                    Some(entry) => {
                        println!("is push entry!!");
                        let current_prev_hash = entry.prev_hash();

                        // validate nonce bag (lines 1-2):
                        // if we have arrived at the first *new* push entry after the last local one recorded one), then check if local nonce is either a) in the nonce bag, or b) in the first new push entry. If not, then someone may have tampered with the RSL
                        // TODO does this take care of when there haven't been any new entries or only one new entry?
                        if current_prev_hash == self.last_local_push_entry.hash() {

                            if !self.nonce_bag.contains(&self.nonce) && !entry.get_nonce_bag().contains(&self.nonce) {
                                //bail!(ErrorKind::MismatchedNonce)
                                return None
                            }
                        }
                        println!("current_prev_hash: {:?}", current_prev_hash);

                        let current_hash = entry.hash();
                        if prev_hash == Some(current_prev_hash) {
                            Some(current_hash)
                        } else {
                            None
                        }
                    }
                    None => {
                        println!("this was not a pushentry. continue with previous entry in hand");
                        prev_hash
                    }
                }
            });

        if result == None {
            // TODO would be nice to bubble up what kind of invalidity we are dealing with
            bail!("invalid RSL entry");
        }

        match verify_commit_signature(self.repo, self.remote_head)? {
            true => Ok(()),
            false => bail!("GPG signature of remote RSL head invalid")
        }
    }


    pub fn push(&mut self) -> Result<()> {
        println!("Pushing updated RSL to remote : )");
        git::push(self.repo, &mut self.remote, &[RSL_BRANCH]).chain_err(|| "could not push rsl")?;
        Ok(())
    }

    pub fn add_push_entry(&self, ref_names: &[&str]) -> Result<Oid> {
        let prev_hash = self.last_remote_push_entry.hash();
        let new_push_entry = PushEntry::new(
            self.repo,
            ref_names.first().unwrap(),
            prev_hash,
            self.nonce_bag.clone(),
        );

        // commit new pushentry (TODO commit to detached HEAD instead of local RSL branch, in case someone else has updated and a fastforward is not possible)
        self.repo
            .commit_push_entry(&new_push_entry, "refs/heads/RSL")
            .chain_err(|| "Couldn't commit new push entry")
    }

    pub fn update_nonce_bag(&mut self) -> Result<()> {
        // if nonce bag contains a nonce for current developer, remove it
        if !self.nonce_bag.remove(&self.nonce) {
            // if nonce in bag does not match local nonce, stop and warn user of possible tampering
            bail!("Your local '.git/NONCE' does not match the one fetched from the remote reference state log. Someone may have tampered with the remote repo.");
        }

        // save new random nonce locally
        let new_nonce = Nonce::new()?;
        self.nonce = new_nonce;
        self.repo
            .write_nonce(&new_nonce)
            .chain_err(|| "nonce write error")?;

        // add new nonce to nonce bag
        self.nonce_bag.insert(new_nonce);
        self.repo
            .write_nonce_bag(&self.nonce_bag)
            .chain_err(|| "couldn't write to nonce bag file")?;
        self.repo
            .commit_nonce_bag()
            .chain_err(|| "couldn't commit nonce bag")?;
        Ok(())
    }

    pub fn update_local(&mut self) -> Result<()> {
        if !git::up_to_date(self.repo, "RSL", "origin/RSL")? {
            match git::fast_forward_possible(self.repo, "refs/remotes/origin/RSL") {
                Ok(true) => git::fast_forward_onto_head(self.repo, "refs/remotes/origin/RSL")?,
                Ok(false) => bail!("Local RSL cannot be fastforwarded to match remote. This may indicate that someone has tampered with the RSL history. Use caution before proceeding."),
                Err(e) => Err(e).chain_err(|| "Local RSL cannot be fastforwarded to match remote. This may indicate that someone has tampered with the RSL history. Use caution before proceeding.")?,
            }
        }
        self.local_head = self.remote_head;
        self.last_local_push_entry = find_last_push_entry(self.repo, &self.local_head)?;
        Ok(())
    }

    // If we have detected a problem with the RSL, we need to reset the fetched origin/RSL to the last trusted revision of our local RSL.
    pub fn reset_remote_to_local(&mut self) -> Result<()> {
        // ensure that the remote is ahead of the locall
        self.repo.graph_descendant_of(self.local_head, self.remote_head)?;

        // find reference of origin/RSL
        let mut reference = self.repo.find_reference("refs/remotes/origin/RSL")?;
        let msg = "Resetting RSL to last trusted state";
        reference.set_target(self.local_head, &msg)?;

        self.remote_head = self.local_head;
        Ok(())
    }

    pub fn find_last_remote_push_entry_for_branch(
        &self,
        reference: &str,
    ) -> Result<Option<PushEntry>> {
        let mut revwalk: Revwalk = self.repo.revwalk()?;
        revwalk.push(self.remote_head)?;
        let mut current = Some(self.remote_head);
        while current != None {
            if let Some(pe) = PushEntry::from_oid(self.repo, &current.unwrap())? {
                if pe.branch() == reference {
                    return Ok(Some(pe));
                }
            }
            // .next() returns Opt<Res<Oid>>
            current = revwalk.next().and_then(|res| res.ok());
        }
        Ok(None)
    }
}

fn verify_commit_signature(repo: &Repository, oid: Oid) -> Result<bool> {
    let (sig, content) = repo.extract_signature(&oid, None)?;
    gpg::verify_detached_signature(sig.as_str().ok_or("")?, content.as_str().ok_or("")?, None)
}

// find the last commit on the branch pointed to by the given Oid that represents a push entry
fn find_last_push_entry(repo: &Repository, oid: &Oid) -> Result<PushEntry> {
    let tree_tip = oid;
    let mut revwalk: Revwalk = repo.revwalk().expect("Failed to make revwalk");
    revwalk.push(tree_tip.clone())?;
    let mut current = Some(tree_tip.clone());
    while current != None {
        if let Some(pe) = PushEntry::from_oid(repo, &current.unwrap())? {
            return Ok(pe);
        }
        current = revwalk.next().and_then(|res| res.ok()); // .next returns Opt<Res<Oid>>
    }
    bail!("no push entries on this branch")
}

pub trait HasRSL<'repo> {
    fn init_rsl_if_needed(&self, remote: &mut Remote) -> Result<()>;
    fn rsl_init_global(&self, remote: &mut Remote) -> Result<()>;
    fn rsl_init_local(&self, remote: &mut Remote) -> Result<()>;
    fn fetch_rsl(&self, remote: &mut Remote) -> Result<()>;
    fn commit_push_entry(&self, push_entry: &PushEntry, branch: &str) -> Result<Oid>;
}

impl<'repo> HasRSL<'repo> for Repository {

    fn rsl_init_global(&self, remote: &mut Remote) -> Result<()> {
        println!("Initializing Reference State Log for this repository.");
        println!("You will be prompted for your gpg pin and/or touch sig in order to sign RSL entries.");

        // get current branch name
        let head_name = self.head()?
            .name()
            .ok_or("Not on a named branch")?
            .clone()
            .to_owned();

        // create new parentless orphan commit
        let mut index = self.index().chain_err(|| "could not find index")?;
        index.clear()?; // remove project files from index
        let oid = index
            .write_tree()
            .chain_err(|| "Could not write tree from index.")?; // create empty tree
        let signature = self.signature().unwrap();
        let message = "Initialize RSL";
        let tree = self.find_tree(oid).chain_err(|| "could not find tree")?;
        let rsl_head = format!("refs/heads/{}", RSL_BRANCH);
        let _oid = git::commit_signed(
            self,
            Some(&rsl_head), //  point HEAD to our new commit
            &signature,      // author
            &signature,      // committer
            message,         // commit message
            &tree,           // tree
            &[],             // parents
        ).chain_err(|| "could not create initial RSL commit")?;

        // checkout unborn orphan branch of parentless commit
        git::checkout_branch(self, "refs/heads/RSL")?;

        /// after checking out an orphan branch, the index and work tree match
        /// the original branch. Meaning in git cli terms that alllll of our
        /// project files are staged in green in the index. We want to get rid
        /// of them so we don't commit them with the nonce bag. However, we
        /// want to keep the ignored files, so we don't lose compiled binaries
        /// when switching back to a regular branch. For this reason, we want
        /// to run the equivalent of
        ///
        /// 1: > git checkout --force master
        /// 2: > git checkout RSL
        ///
        /// to remove all untracked files from the working directory. To do
        /// this we checkout the
        /// original branch, allowing conflicts (not a problem because all the
        /// files are the exact same. And then we come back to the
        /// RSL branch to continue initialization.
        debug_assert!(&index.is_empty());

        // 1. perform the custom checkout
        let tree = self.find_reference(&head_name)
            .chain_err(|| "couldn't find branch")?
            .peel_to_commit()
            .chain_err(|| "couldnt find latest RSL commit")?
            .into_object();
        let mut opts = CheckoutBuilder::new();
        opts.allow_conflicts(true);
        self.checkout_tree(&tree, Some(&mut opts))
            .chain_err(|| "couldn't checkout tree")?; // Option<CheckoutBuilder>
        self.set_head(&head_name)
            .chain_err(|| "couldn't switch head")?;

        // 2. checkout RSL again to continue init
        git::checkout_branch(self, "refs/heads/RSL")?;
        debug_assert!(&index.is_empty());

        // save random nonce locally
        let nonce = Nonce::new()?;
        self.write_nonce(&nonce)
            .chain_err(|| "couldn't write local nonce")?;

        // create new nonce bag with initial nonce
        let mut nonce_bag = NonceBag::new();
        nonce_bag.insert(nonce);

        self.write_nonce_bag(&nonce_bag)?;
        self.commit_nonce_bag()?;

        // create initial bootstrapping push entry
        let initial_pe = PushEntry::new(self, "RSL", String::from("First Push Entry"), nonce_bag);
        self.commit_push_entry(&initial_pe, "refs/heads/RSL")?;

        // push new rsl branch
        git::push(self, remote, &["RSL"]).chain_err(|| "rsl init error")?;

        Ok(())
    }

    fn rsl_init_local(&self, remote: &mut Remote) -> Result<()> {
        println!("Initializing local Reference State Log based on existing remote RSL.");
        self.fetch_rsl(remote)?;

        let remote_rsl_tip = git::oid_from_long_name(self, "refs/remotes/origin/RSL")?;
        let latest_rsl_commit = self.find_commit(remote_rsl_tip)?;
        // create local rsl branch
        self.branch("RSL", &latest_rsl_commit, false)?;

        git::checkout_branch(self, "refs/heads/RSL")?;

        let mut nonce_bag = self.read_nonce_bag()?;

        // save random nonce locally
        let new_nonce = Nonce::new().unwrap();
        self.write_nonce(&new_nonce)
            .chain_err(|| "nonce write error")?;

        // add that nonce to the bag
        let username = git::username(self)?;
        nonce_bag.insert(new_nonce);
        self.write_nonce_bag(&nonce_bag)
            .chain_err(|| "couldn't write to nonce baf file")?;
        self.commit_nonce_bag()
            .chain_err(|| "couldn't commit nonce bag")?;

        git::push(self, remote, &["refs/heads/RSL"]).chain_err(|| "rsl init error")?;

        Ok(())
    }

    fn commit_push_entry(&self, push_entry: &PushEntry, branch: &str) -> Result<Oid> {
        let message = push_entry.to_string();
        git::add_and_commit_signed(self, None, &message, branch)
            .chain_err(|| "could not commit push entry")
    }

    fn fetch_rsl(&self, remote: &mut Remote) -> Result<()> {
        // not sure the behavior here if the branch doesn't exist
        // should return Some(()) or Some(Reference) if remote exists and None if it doesn't exist and Err if it failed for some other reason.
        git::fetch(self, remote, &[RSL_BRANCH], Some(REFLOG_MSG))
            .chain_err(|| "could not fetch RSL")?;
        Ok(())
    }

    fn init_rsl_if_needed(&self, remote: &mut Remote) -> Result<()> {
        // validate that RSL does not exist locally or remotely
        match (
            self.find_branch("origin/RSL", BranchType::Remote),
            self.find_branch("RSL", BranchType::Local),
        ) {
            (Err(_), Err(_)) => {
                self.rsl_init_global(remote)
                    .chain_err(|| "could not initialize remote RSL")?;
                Ok(())
            } // first use of git-rsl for repo
            (Ok(_), Err(_)) => {
                self.rsl_init_local(remote)
                    .chain_err(|| "could not initialize local rsl")?;
                Ok(())
            } // first use of git-rsl for this developer in this repo
            (Err(_), Ok(_)) => bail!("RSL exists locally but not globally. Somebody has deleted the remote RSL for this project, or else you are interacting with a remote that does not have an RSL."), // local exists but global not found
            (Ok(_), Ok(_)) => Ok(()),                                        // RSL already set up
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::test_helper::*;
    use std::path::Path;
    use std::process::Command;
    use git2::RepositoryState;

    #[test]
    fn rsl_init_global() {
        let context = setup_fresh();
        {
            let mut remote = context.local.find_remote("origin").unwrap().to_owned();
            let result = &context.local.rsl_init_global(&mut remote).unwrap();
            assert_eq!(result, &()); // returns successfully
                                     // local rsl branch exists
                                     // local nonce exists
                                     // remote rsl branch exists
            assert!(&context
                .local
                .find_branch("origin/RSL", BranchType::Remote)
                .is_ok());
            assert!(&context.local.find_branch("RSL", BranchType::Local).is_ok());
            assert!(context.local.state() == RepositoryState::Clean);
            assert_eq!(
                context
                    .local
                    .diff_index_to_workdir(None, None)
                    .unwrap()
                    .deltas()
                    .count(),
                0
            );
            // TODO to test that the repo does not contain untracked NONCE_BAG file and simultaneously show deleted NONCE_BAG file?? git gets confused??? Open git2rs issue about needing to reset after commit.
        }
        teardown_fresh(context);
    }

    #[test]
    fn rsl_init_with_gitignore() {
        let context = setup_fresh();
        {
            let repo = &context.local;
            let mut remote = context.local.find_remote("origin").unwrap().to_owned();

            // add foo.txt to gitignore & commit
            let ignore_path = repo.path().parent().unwrap().join(".gitignore");
            create_file_with_text(&ignore_path, &"foo.txt");
            let _commit_id = git::add_and_commit(
                &repo,
                Some(&Path::new(".gitignore")),
                "Add gitignore",
                "refs/heads/master",
            ).unwrap();

            // add foo.txt
            let foo_path = repo.path().parent().unwrap().join("foo.txt");
            create_file_with_text(&foo_path, &"some ignored text");

            // init RSL
            &repo.rsl_init_global(&mut remote).unwrap();

            // switch back to master branch
            git::checkout_branch(&repo, "refs/heads/master").unwrap();

            // check that foo.txt is still there
            assert_eq!(foo_path.is_file(), true);

            // checkout RSL and ensure wwork.txt is not there and foo.txt is and is untracked
            git::checkout_branch(&repo, "refs/heads/RSL").unwrap();
            assert!(!repo.workdir().unwrap().join("work.txt").is_file());
        }
        teardown_fresh(context);
    }

    #[test]
    fn rsl_fetch() {
        // test that RSL fetch gets the remote branch but doesnt create a local branch if it doesn't yet exist. if it does, we need to change how we decide whether to init.
        let context = setup_fresh();
        {
            let repo = &context.local;
            let mut remote = context.local.find_remote("origin").unwrap().to_owned();
            &context.local.rsl_init_global(&mut remote).unwrap();

            // delete local RSL
            repo.find_reference("refs/heads/RSL")
                .unwrap()
                .delete()
                .unwrap();
            repo.find_reference("refs/remotes/origin/RSL")
                .unwrap()
                .delete()
                .unwrap();

            &repo.fetch_rsl(&mut remote).unwrap();

            assert!(&repo.find_branch("origin/RSL", BranchType::Remote).is_ok());

            assert!(&repo.find_branch("RSL", BranchType::Local).is_err());
        }
        teardown_fresh(context)
    }

    #[test]
    fn commit_push_entry() {
        let context = setup_fresh();
        {
            let repo = &context.local;

            // RSL commit only works on RSL branch; we have to initialize it and check it out
            let mut rem = repo.find_remote("origin").unwrap().to_owned();
            repo.rsl_init_global(&mut rem).unwrap();
            git::checkout_branch(repo, "refs/heads/RSL").unwrap();

            // try commit
            let entry = PushEntry::new(
                repo,
                &"master", // branch
                String::from("hash_of_last_pushentry"), // prev
                NonceBag::new()
            );
            let oid = repo.commit_push_entry(&entry, "refs/heads/RSL").unwrap();

            // we are on the correct branch with new commit at the tip
            let head = repo.head().unwrap();
            let ref_name = head.name().unwrap();
            let tip = head.target().unwrap();
            assert_eq!(ref_name, "refs/heads/RSL");
            assert_eq!(oid, tip);

            // check text of commit
            let obj = repo.find_commit(oid).unwrap();
            let result = PushEntry::from_str(&obj.message().unwrap()).unwrap();
            assert_eq!(result, entry);

            // commit is signed and we are on the right branch
            let status = Command::new("git")
                //.env("GNUPGHOME", "./fixtures/fixture.gnupghome")
                .args(&["--exec-path", &context.repo_dir.to_str().unwrap()])
                .args(&["verify-commit", "HEAD"])
                .status()
                .unwrap();
            assert!(status.success());
        }
        teardown_fresh(context);
    }

    #[test]
    fn reset_remote_to_local() {
        let context = setup_fresh();
        {
            let repo = &context.local;
            let mut remote = repo.find_remote("origin").unwrap();
            repo.rsl_init_global(&mut remote).unwrap();
            {
                let rsl = RSL::read(&repo, &mut remote).unwrap();

                // checkout remote RSL and add some commits
                git::checkout_branch(rsl.repo, "refs/remotes/origin/RSL").unwrap();

                // create push entry manuallly and commit it to the remote rsl branch
                let prev_hash = rsl.last_remote_push_entry.hash();
                let push_entry = PushEntry::new(&repo, &"master", prev_hash, rsl.nonce_bag);
                let oid = repo.commit_push_entry(&push_entry, "refs/remotes/origin/RSL").unwrap();

                // remote rsl head is this latest commit
                let remote_head = rsl.repo.find_reference("refs/remotes/origin/RSL").unwrap().target().unwrap();
                assert_eq!(oid, remote_head);

                // remote and local rsl branches differ
                let local_head = rsl.repo.find_reference("refs/heads/RSL").unwrap().target().unwrap();
                assert_ne!(local_head, remote_head);
            }
            {
                let mut rsl = RSL::read(&repo, &mut remote).unwrap();

                // do reset
                rsl.reset_remote_to_local().unwrap();

                let remote_head = rsl.repo.find_reference("refs/remotes/origin/RSL").unwrap().target().unwrap();
                let local_head = rsl.repo.find_reference("refs/heads/RSL").unwrap().target().unwrap();
                assert_eq!(remote_head, local_head);
                assert_eq!(rsl.remote_head, rsl.local_head);
            }

        }
        teardown_fresh(context)
    }
}
