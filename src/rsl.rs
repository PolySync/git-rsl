use git2::{Oid, Repository, Remote, Revwalk, BranchType, Sort};
use git2::build::CheckoutBuilder;

use nonce::{Nonce, HasNonce};
use nonce_bag::{NonceBag, HasNonceBag};
use push_entry::PushEntry;
use errors::*;
use utils::*;

const RSL_BRANCH: &'static str = "RSL";
const REFLOG_MSG: &'static str = "Retrieve RSL branchs from remote";


#[derive(Debug)]
pub enum RSLType {
    Local,
    Remote,
}

#[derive(Debug)]
pub struct RSL {
    pub kind: RSLType,
    //remote: &'repo Remote,
    pub head: Oid,
    pub last_push_entry: Option<PushEntry>,
}

impl RSL {


}

pub trait HasRSL {
    fn read_rsl(&self) -> Result<(RSL, RSL, NonceBag, Nonce)>;
    fn read_local_rsl(&self) -> Result<RSL>;
    fn read_remote_rsl(&self) -> Result<RSL>;
    fn init_rsl_if_needed(&self, remote: &mut Remote) -> Result<()>;
    fn rsl_init_global(&self, remote: &mut Remote) -> Result<()>;
    fn rsl_init_local(&self, remote: &mut Remote) -> Result<()>;
    fn fetch_rsl(&self, remote: &mut Remote) -> Result<()>;
    fn commit_push_entry(&self, push_entry: &PushEntry) -> Result<Oid>;
    fn push_rsl(&self, remote: &mut Remote) -> Result<()>;
    fn find_last_push_entry(&self, tree_tip: &Oid) -> Result<Option<PushEntry>>;
    fn find_last_push_entry_for_branch(&self, remote_rsl: &RSL, reference: &str) -> Result<Option<PushEntry>>;
    fn validate_rsl(&self) -> Result<()>;
}

impl HasRSL for Repository {

    fn find_last_push_entry_for_branch(&self, remote_rsl: &RSL, reference: &str) -> Result<Option<PushEntry>> {
        let mut revwalk: Revwalk = self.revwalk()?;
        revwalk.push(remote_rsl.head)?;
        let mut current = Some(remote_rsl.head.clone());
        while current != None {
            match PushEntry::from_oid(self, &current.unwrap())? {
                Some(pe) => {
                    if pe.branch == reference {
                        return Ok(Some(pe))
                    } else {
                        ()
                    }
                },
                None => (),
            }
            current = revwalk.next().and_then(|res| res.ok()); // .next returns Opt<Res<Oid>>
        }
        Ok(None)
    }

    // find the last commit on the branch pointed to by the given Oid that represents a push entry
    fn find_last_push_entry(&self, tree_tip: &Oid) -> Result<Option<PushEntry>> {
        let mut revwalk: Revwalk = self.revwalk().expect("Failed to make revwalk");
        revwalk.push(tree_tip.clone())?;
        let mut current = Some(tree_tip.clone());
        while current != None {
            if let Some(pe) = PushEntry::from_oid(self, &current.unwrap())? {
                return Ok(Some(pe))
            }
            current = revwalk.next().and_then(|res| res.ok()); // .next returns Opt<Res<Oid>>
        }
        Ok(None)
    }



    fn rsl_init_global(&self, remote: &mut Remote) -> Result<()> {
        println!("Initializing Reference State Log for this repository.");

        // get current branch name
        let head_name = self.head()?
            .name()
            .ok_or("not on a named branch")?
            .clone()
            .to_owned();

        // create new parentless orphan commit
        let mut index = self.index().chain_err(|| "could not find index")?;
        index.clear()?; // remove project files from index
        let oid = index.write_tree().chain_err(|| "could not write tree from index")?; // create empty tree
        let signature = self.signature().unwrap();
        let message = "Initialize RSL";
        let tree = self.find_tree(oid).chain_err(|| "could not find tree")?;
        let rsl_head = format!("refs/heads/{}", RSL_BRANCH);
        let _oid = git::commit_signed(
            self,
            &rsl_head, //  point HEAD to our new commit
            &signature, // author
            &signature, // committer
            &message, // commit message
            &tree, // tree
            &[] // parents
        ).chain_err(|| "could not create initial RSL commit")?;

        // TODO sign commit after making it
        // git::sign_commit(self, oid)?; // will return Result<Oid>

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
        self.checkout_tree(&tree, Some(&mut opts)).chain_err(|| "couldn't checkout tree")?; // Option<CheckoutBuilder>
        self.set_head(&head_name).chain_err(|| "couldn't switch head")?;

        // 2. checkout RSL again to continue init
        git::checkout_branch(self, "refs/heads/RSL")?;
        debug_assert!(&index.is_empty());

        // save random nonce locally
        let nonce = Nonce::new()?;
        self.write_nonce(&nonce).chain_err(|| "couldn't write local nonce")?;

        // create new nonce bag with initial nonce
        let mut nonce_bag = NonceBag::new();
        self.write_nonce_bag(&nonce_bag)?;
        self.commit_nonce_bag()?;
        nonce_bag.insert(nonce).chain_err(|| "couldn't add new nonce to bag")?;

        // create initial bootstrapping push entry
        let initial_pe = PushEntry::new(self, "RSL", String::from("First Push Entry"), nonce_bag);
        self.commit_push_entry(&initial_pe)?;

        // push new rsl branch
        self.push_rsl(remote)?;




        Ok(())


    }

    fn rsl_init_local(&self, remote: &mut Remote) -> Result<()> {
        println!("Initializing local Reference State Log based on existing remote RSL.");
        self.fetch_rsl(remote)?;

        let remote_rsl = self.read_remote_rsl()?;
        let latest_rsl_commit = self.find_commit(remote_rsl.head)?;
        // create local rsl branch
        self.branch(&"RSL", &latest_rsl_commit, false)?;

        git::checkout_branch(self, "refs/heads/RSL")?;

        let mut nonce_bag = self.read_nonce_bag()?;
        let new_nonce = Nonce::new().unwrap();
        self.write_nonce(&new_nonce).chain_err(|| "nonce write error")?;
        nonce_bag.insert(new_nonce)?;
        self.write_nonce_bag(&nonce_bag).chain_err(|| "couldn't write to nonce baf file")?;
        self.commit_nonce_bag().chain_err(|| "couldn't commit nonce bag")?;
        self.push_rsl(remote).chain_err(|| "rsl init error")?;

        Ok(())
    }

    fn read_rsl(&self) -> Result<(RSL, RSL, NonceBag, Nonce)> {
        let remote_rsl = self.read_remote_rsl().chain_err(|| "remote rsl read error")?;
        let local_rsl = self.read_local_rsl().chain_err(|| "local rsl read error")?;
        let nonce_bag = self.read_nonce_bag().chain_err(|| "nonce bag read error")?;
        let nonce = self.read_nonce().chain_err(|| "nonce read error")?;
        Ok((remote_rsl, local_rsl, nonce_bag, nonce))
    }

    fn read_local_rsl(&self) -> Result<RSL> {
        let kind = RSLType::Local;
        let branch = self.find_branch(RSL_BRANCH, BranchType::Local).chain_err(|| "couldnt find RSL branch")?;
        let reference = branch.into_reference();
        let head = reference.target().ok_or("could not find RSL branch tip OID")?;
        let last_push_entry = self.find_last_push_entry(&head)?;
        Ok(RSL {kind, head, last_push_entry})
    }

    fn read_remote_rsl(&self) -> Result<RSL> {
        let kind = RSLType::Remote;
        let branch = self.find_branch("origin/RSL", BranchType::Remote).chain_err(|| "could not find RSL branch")?;
        let reference = branch.into_reference();
        let head = reference.target().ok_or("could not find head reference")?;
        let last_push_entry = self.find_last_push_entry(&head)?;
        Ok(RSL {kind, head, last_push_entry})
    }

    fn commit_push_entry(&self, push_entry: &PushEntry) -> Result<Oid> {
        let mut index = self.index().chain_err(|| "could not find index")?;
        //index.add_path(self.path().join("NONCE_BAG"))?;
        let oid = index.write_tree().chain_err(|| "could not write tree")?;
        let signature = self.signature().unwrap();
        let message = push_entry.to_string();
        let parent_commit_ref = self.find_branch(RSL_BRANCH, BranchType::Local).chain_err(|| "RSL Branch not found: {:?}")?;
        let parent_commit = parent_commit_ref.get().peel_to_commit().chain_err(|| "could not find parent commit")?;
        let tree = self.find_tree(oid).chain_err(|| "could not find tree")?;
        let rsl_head = format!("refs/heads/{}", RSL_BRANCH);

        git::commit_signed(self,
            &rsl_head, //  point HEAD to our new commit
            &signature, // author
            &signature, // committer
            &message, // commit message
            &tree, // tree
            &[&parent_commit]
        ).chain_err(|| "could not commit push entry")

        // TODO sign commit after making it
        // git::sign_commit(self, oid)? // will return Result<Oid>
    }


    fn fetch_rsl(&self, remote: &mut Remote) -> Result<()> {
        // not sure the behavior here if the branch doesn't exist
        // should return Some(()) or Some(Reference) if remote exists and None if it doesn't exist and Err if it failed for some other reason.
        git::fetch(self, remote, &[RSL_BRANCH], Some(REFLOG_MSG)).chain_err(|| "could not fetch RSL")?;
        Ok(())
    }

    fn init_rsl_if_needed(&self, remote: &mut Remote) -> Result<()> {
        // validate that RSL does not exist locally or remotely
        match (self.find_branch("origin/RSL", BranchType::Remote), self.find_branch(RSL_BRANCH, BranchType::Local)) {
            (Err(_), Err(_)) => {self.rsl_init_global(remote).chain_err(|| "could not initialize remote RSL")?;
                                Ok(())}, // first use of git-rsl for repo
            (Ok(_), Err(_)) => {self.rsl_init_local(remote).chain_err(|| "could not initialize loxal rsl")?;
                                Ok(())}, // first use of git-rsl for this developer in this repo
            (Err(_), Ok(_)) => bail!("RSL exists locally but not globally"), // local exists but global not found
            (Ok(_), Ok(_)) => Ok(()), // RSL already set up
        }
    }

    fn push_rsl(&self, remote: &mut Remote) -> Result<()> {
        println!("gets here : )");
        git::push(self, remote, &[RSL_BRANCH]).chain_err(|| "could not push rsl")?;
        Ok(())
    }

    fn validate_rsl(&self) -> Result<()> {

        let (remote_rsl, local_rsl, _nonce_bag, _nonce) = self.read_rsl()?;

        // Ensure remote RSL head is a descendant of local RSL head.
        let descendant = self
            .graph_descendant_of(remote_rsl.head, local_rsl.head)
            .unwrap_or(false);
        let same = remote_rsl.head == local_rsl.head;
        if !descendant && !same {
            bail!("RSL invalid: No path to get from Local RSL to Remote RSL");
        }

        // Walk through the commits from local RSL head, which we know is valid,
        // validating each additional pushentry since that point one by one.
        let last_hash = match local_rsl.last_push_entry {
            Some(ref push_entry) => Some(push_entry.hash()),
            None => None, // the first push entry will have None as last_push_entry
        };
        let mut revwalk: Revwalk = self.revwalk()?;
        revwalk.push(remote_rsl.head)?;
        revwalk.set_sorting(Sort::REVERSE);
        revwalk.hide(local_rsl.head)?;

        let remaining = revwalk.map(|oid| oid.unwrap());
        println!("gets to validate");
        let result = remaining
            .inspect(|x| println!("about to fold: {}", x))
            .fold(last_hash, |prev_hash, oid| {
            //println!("last hash: {:?}", last_hash);
            println!("prev_hash: {:?}", prev_hash);
            println!("oid {:?}", oid);
            let current_push_entry = PushEntry::from_oid(self, &oid).unwrap_or(None);
            match current_push_entry {
                Some(entry) => {
                    println!("is push entry!!");
                    let current_prev_hash = entry.prev_hash();

                    // if current prev_hash == local_rsl.head (that is, we have arrived at the first push entry after the last recorded one), then check if repo_nonce in PushEntry::from_oid(oid.parent_commit) or noncebag contains repo_nonce; return false if neither holds
                    //if current_prev_hash == last_local_push_entry.hash() {

                        // validate nonce bag (lines 1-2):
                        // TODO does this take care of when there haven't been any new entries or only one new entry?
                        //if !nonce_bag.bag.contains(&repo_nonce) && !current_push_entry.nonce_bag.bag.contains(&repo_nonce) { // repo nonce not in remote nonce bag && repo_nonce not in remote_rsl.push_after(local_rsl){
                        //    None;
                        //}
                    //}
                    println!("current_prev_hash: {:?}", current_prev_hash);

                    let current_hash = entry.hash();
                    if prev_hash == Some(current_prev_hash) {
                        Some(current_hash)
                    } else {
                        None
                    }
                },
                None => {
                    println!("this was not a pushentry. continue with previous entry in hand");
                    prev_hash
                },
            }
        });

        if result == None { bail!("invalid RSL entry"); }

        // TODO really verify
        // let (sig, data) = extract signature(commit_oid)
        // gpg::verify_detached_signature(sig, data)
        gpg::verify_commit_signature(remote_rsl.head).chain_err(|| "GPG signature of remote RSL head invalid")

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
            assert!(&context.local.find_branch("origin/RSL", BranchType::Remote).is_ok());
            assert!(&context.local.find_branch("RSL", BranchType::Local).is_ok());
            assert!(context.local.state() == RepositoryState::Clean);
            assert_eq!(context.local.diff_index_to_workdir(None, None).unwrap().deltas().count(), 0);
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
            let _commit_id = git::add_and_commit(&repo, Some(&Path::new(".gitignore")), "Add gitignore", "master").unwrap();

            // add foo.txt
            let foo_path = repo.path().parent().unwrap().join("foo.txt");
            create_file_with_text(&foo_path, &"some ignored text");

            // init RSL
            let result = &repo.rsl_init_global(&mut remote).unwrap();

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
            let result = &context.local.rsl_init_global(&mut remote).unwrap();

            // delete local RSL
            repo.find_reference("refs/heads/RSL").unwrap().delete().unwrap();
            repo.find_reference("refs/remotes/origin/RSL").unwrap().delete().unwrap();

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
            let entry = PushEntry {
                    //related_commits: vec![oid.to_owned(), oid.to_owned()],
                    branch: String::from("branch_name"),
                    head: repo.head().unwrap().target().unwrap(),
                    prev_hash: String::from("hash_of_last_pushentry"),
                    nonce_bag: NonceBag::new(),
                    signature: String::from("gpg signature"),
            };
            let oid = repo.commit_push_entry(&entry).unwrap();

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
}
