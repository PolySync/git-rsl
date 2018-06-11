extern crate git_rsl as kevlar_laces;
extern crate git2;
use std::process::Command;
use kevlar_laces::utils::test_helper::*;


#[test]
fn push_and_fetch() {
    let mut context = setup_fresh();
    {
        assert_eq!((), kevlar_laces::rsl_init_with_cleanup(&mut context.local, "origin")
            .expect("Could not rsl-init"));
        let res = kevlar_laces::run(&mut context.local, &[&"master"], &"origin", &"push").unwrap();
        assert_eq!(res, ());
        do_work_on_branch(&context.local, "refs/heads/master");

        let res2 = kevlar_laces::run(&mut context.local, &[&"master"], &"origin", &"push").unwrap();
        assert_eq!(res2, ());

        let res3 = kevlar_laces::run(&mut context.local, &[&"master"], &"origin", &"fetch").unwrap();
        assert_eq!(res3, ());

        do_work_on_branch(&context.local, "refs/heads/master");
        let res4 = kevlar_laces::run(&mut context.local, &[&"master"], &"origin", &"push").unwrap();
        assert_eq!(res4, ());
        // TODO check that the git log of RSL looks how we want it to
    }
    teardown_fresh(context)
}

#[test]
fn error_handling() {

    let mut context = setup_fresh();
    {
        let res = kevlar_laces::run(&mut context.local, &[&"master"], &"origin", &"push").unwrap();
        assert_eq!(res, ());

        let nonce_file = context.repo_dir.join(".git/NONCE");
        Command::new("chmod")
        .arg("000")
        .arg(nonce_file.to_string_lossy().into_owned())
        .output()
        .expect("failed to change permissions");

        do_work_on_branch(&context.local, "refs/heads/master");
        //let res2 = push::secure_push(&repo, &mut rem, refs).unwrap_err();
        let res2 = kevlar_laces::run(&mut context.local, &[&"master"], &"origin", &"push").unwrap_err();
        // TODO - analyse this test and find out what res2 here should be, then add an assert
        // assert that we are on the right branch_head
        let head = context.local.head().unwrap().name().unwrap().to_owned();
        assert_eq!(head, "refs/heads/master");
        //assert_eq!(res2.description(), "");

    }
    teardown_fresh(context)
}

#[test]
fn check_rsl() {
    let mut context = setup_fresh();
    {
        let res = kevlar_laces::secure_push_with_cleanup(&mut context.local, &"master", &"origin").unwrap();
        assert_eq!(res, ());
        do_work_on_branch(&context.local, "refs/heads/master");

        let res2 = kevlar_laces::secure_push_with_cleanup(&mut context.local, &"master", &"origin").unwrap();
        assert_eq!(res2, ());
    }
    teardown_fresh(context)
}
