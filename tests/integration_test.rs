#[macro_use]
extern crate lazy_static;

extern crate git2;
extern crate git_rsl;
extern crate names;
extern crate tempdir;

mod utils;

use git_rsl::utils::test_helper::*;
use git_rsl::{ReferenceName, RemoteName};
use std::process::Command;
use std::sync::Mutex;
use utils::attack;

lazy_static! {
    static ref SEQUENTIAL_TEST_MUTEX: Mutex<()> = Mutex::new(());
}
macro_rules! sequential_test {
    (fn $name:ident() $body:block) => {
        #[test]
        fn $name() {
            let _guard = $crate::SEQUENTIAL_TEST_MUTEX.lock();
            {
                $body
            }
        }
    };
}

macro_rules! assert_ok {
    ($fn:expr, $msg:expr) => {
        assert_eq!((), $fn.expect($msg))
    }
}

sequential_test! {
    fn push_and_fetch_branch() {
        let mut context = setup_fresh();
        {
            let remote = RemoteName::new("origin");
            let master = ReferenceName::new("master");

            assert_ok!(
                git_rsl::rsl_init_with_cleanup(&mut context.local, &remote), 
                "Could not rsl-init");

            assert_ok!(
                git_rsl::secure_push_with_cleanup(&mut context.local, &remote, &master), 
                "Could not run first push");
            do_work_on_branch(&context.local, "refs/heads/master");

            assert_ok!(
                git_rsl::secure_push_with_cleanup(&mut context.local, &remote, &master),
                "Could not run second push");

            assert_ok!(
                git_rsl::secure_fetch_with_cleanup(&mut context.local, &remote, &master),
                "Could not run fetch");

            do_work_on_branch(&context.local, "refs/heads/master");
            assert_ok!(
                git_rsl::secure_push_with_cleanup(&mut context.local, &remote, &master),
                "Could not run third push");
            // TODO check that the git log of RSL looks how we want it to
        }
    }
}

sequential_test! {
    fn push_and_fetch_tag() {
        let mut context = setup_fresh();
        {
            let remote = RemoteName::new("origin");
            let tag = ReferenceName::new("v6.66");

            assert_ok!(git_rsl::rsl_init_with_cleanup(&mut context.local, &remote),
                "Could not rsl-init");
            do_work_on_branch(&context.local, "refs/heads/master");
            tag_lightweight(&mut context.local, "v6.66");

            assert_ok!(
                git_rsl::secure_push_with_cleanup(&mut context.local, &remote, &tag), 
                "Could not push tag");
            let remote_tag = &context.remote.find_reference("refs/tags/v6.66").expect("reference not found");
            assert!(remote_tag.is_tag());

            assert_ok!(
                git_rsl::secure_fetch_with_cleanup(&mut context.local, &remote, &tag),
                "could not fetch tag");
        }
    }
}

sequential_test! {
    fn error_handling() {
        let mut context = setup_fresh();
        {
            let remote = RemoteName::new("origin");
            let master = ReferenceName::new("master");

            assert_ok!(git_rsl::rsl_init_with_cleanup(&mut context.local, &remote),
                "Could not rsl-init");
            assert_ok!(
                git_rsl::secure_push_with_cleanup(&mut context.local, &remote, &master),
                "failed to secure push");

            let nonce_file = context.repo_dir.join(".git/NONCE");
            Command::new("chmod")
            .arg("000")
            .arg(nonce_file.to_string_lossy().into_owned())
            .output()
            .expect("failed to change permissions");

            do_work_on_branch(&context.local, "refs/heads/master");
            git_rsl::secure_push_with_cleanup(&mut context.local, &remote, &master)
                .unwrap_err();
            let head = context.local.head().unwrap().name().unwrap().to_owned();
            assert_eq!(head, "refs/heads/master");

        }
    }
}

sequential_test! {
    fn check_rsl() {
        let mut context = setup_fresh();
        {
            let remote = RemoteName::new("origin");
            let master = ReferenceName::new("master");

            assert_ok!(
                git_rsl::rsl_init_with_cleanup(&mut context.local, &remote),
                "Could not rsl-init");
            assert_ok!(
                git_rsl::secure_push_with_cleanup(&mut context.local, &remote, &master),
                "First push failed");
            do_work_on_branch(&context.local, "refs/heads/master");

            assert_ok!(
                git_rsl::secure_push_with_cleanup(&mut context.local, &remote, &master), 
                "Second push failed");
        }
    }
}

sequential_test! {
    fn attack_detected_on_push() {
        let mut context = setup_fresh();
        {
            let remote = RemoteName::new("origin");
            let master = ReferenceName::new("master");

            assert_ok!(git_rsl::rsl_init_with_cleanup(&mut context.local, &remote),
                "Could not rsl-init");
            assert_ok!(
                git_rsl::secure_push_with_cleanup(&mut context.local, &remote, &master),
                "First push failed");
            do_work_on_branch(&context.local, "refs/heads/master");

            assert_ok!(
                git_rsl::secure_push_with_cleanup(&mut context.local, &remote, &master), 
                "Second push failed");

            attack::rollback(&context.remote, "master");

            do_work_on_branch(&context.local, "refs/heads/master");
            let res3 = git_rsl::secure_push_with_cleanup(&mut context.local, &remote, &master)
                .expect_err("Checking for invalid RSL detection");
            assert_eq!(res3.description(), "invalid remote RSL");
        }
    }
}
