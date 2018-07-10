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

sequential_test! {
    fn push_and_fetch_branch() {
        let mut context = setup_fresh();
        {
            assert_eq!((), git_rsl::rsl_init_with_cleanup(&mut context.local, &RemoteName::new("origin"))
                .expect("Could not rsl-init"));
            let res = git_rsl::secure_push_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("master")).expect("Could not run first push");
            assert_eq!(res, ());
            do_work_on_branch(&context.local, "refs/heads/master");

            let res2 = git_rsl::secure_push_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("master")).expect("Could not run second push");
            assert_eq!(res2, ());

            let res3 = git_rsl::secure_fetch_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("master")).expect("Could not run fetch");
            assert_eq!(res3, ());

            do_work_on_branch(&context.local, "refs/heads/master");
            let res4 = git_rsl::secure_push_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("master")).expect("Could not run third push");
            assert_eq!(res4, ());
            // TODO check that the git log of RSL looks how we want it to
        }
    }
}

sequential_test! {
    fn push_and_fetch_tag() {
        let mut context = setup_fresh();
        {
            assert_eq!((), git_rsl::rsl_init_with_cleanup(&mut context.local, &RemoteName::new("origin"))
                .expect("Could not rsl-init"));
            do_work_on_branch(&context.local, "refs/heads/master");
            tag_lightweight(&mut context.local, "v6.66");

            assert_eq!((), git_rsl::secure_push_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("v6.66")).expect("Could not push tag"));
            let remote_tag = &context.remote.find_reference("refs/tags/v6.66").expect("reference not found");
            assert!(remote_tag.is_tag());

            assert_eq!((), git_rsl::secure_fetch_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("v6.66")).expect("could not fetch tag"));
        }
    }
}

sequential_test! {
    fn error_handling() {
        let mut context = setup_fresh();
        {
            assert_eq!((), git_rsl::rsl_init_with_cleanup(&mut context.local, &RemoteName::new("origin"))
                .expect("Could not rsl-init"));
            let res = git_rsl::secure_push_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("master")).unwrap();
            assert_eq!(res, ());

            let nonce_file = context.repo_dir.join(".git/NONCE");
            Command::new("chmod")
            .arg("000")
            .arg(nonce_file.to_string_lossy().into_owned())
            .output()
            .expect("failed to change permissions");

            do_work_on_branch(&context.local, "refs/heads/master");
            let _res2 = git_rsl::secure_push_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("master")).unwrap_err();
            let head = context.local.head().unwrap().name().unwrap().to_owned();
            assert_eq!(head, "refs/heads/master");

        }
    }
}

sequential_test! {
    fn check_rsl() {
        let mut context = setup_fresh();
        {
            assert_eq!((), git_rsl::rsl_init_with_cleanup(&mut context.local, &RemoteName::new("origin"))
                .expect("Could not rsl-init"));
            let res = git_rsl::secure_push_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("master")).expect("First push failed");
            assert_eq!(res, ());
            do_work_on_branch(&context.local, "refs/heads/master");

            let res2 = git_rsl::secure_push_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("master")).expect("Second push failed");
            assert_eq!(res2, ());
        }
    }
}

sequential_test! {
    fn attack_detected_on_push() {
        let mut context = setup_fresh();
        {
            assert_eq!((), git_rsl::rsl_init_with_cleanup(&mut context.local, &RemoteName::new("origin"))
                .expect("Could not rsl-init"));
            let res = git_rsl::secure_push_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("master")).expect("First push failed");
            assert_eq!(res, ());
            do_work_on_branch(&context.local, "refs/heads/master");

            let res2 = git_rsl::secure_push_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("master")).expect("Second push failed");
            assert_eq!(res2, ());

            attack::rollback(&context.remote, "master");

            do_work_on_branch(&context.local, "refs/heads/master");
            let res3 = git_rsl::secure_push_with_cleanup(&mut context.local, &RemoteName::new("origin"), &ReferenceName::new("master")).expect_err("Checking for invalid RSL detection");
            assert_eq!(res3.description(), "invalid remote RSL");
        }
    }
}
