use git2::{Remote, Repository};

use rsl::{HasRSL, RSL};
use errors::*;

use utils::git;

pub fn secure_push<'remote, 'repo: 'remote>(
    repo: &'repo Repository,
    mut remote: &'remote mut Remote<'repo>,
    ref_names: &[&str],
) -> Result<()> {
    repo.fetch_rsl(&mut remote)
        .chain_err(|| "Problem fetching Remote RSL. Check your connection or your SSH config")?;

    repo.init_local_rsl_if_needed(&mut remote)
        .chain_err(|| "Problem initializing RSL")?;

    // checkout RSL branch
    git::checkout_branch(repo, "refs/heads/RSL")?;

    'push: loop {
        repo.fetch_rsl(&mut remote)
            .chain_err(|| "Problem fetching Remote RSL. Check your connection or your SSH config")?;

        // TODO after fetching, make sure that the local branch is not out of date
        // print something like "fatal: Not possible to fast-forward, aborting."
        git::fetch(repo, &mut remote, ref_names, None)?;

        {
            let mut rsl = RSL::read(repo, &mut remote).chain_err(|| "couldn't read RSL")?;

            // reset to last trusted RSL if invalid
            if let Err(e) = rsl.validate() {
                rsl.reset_remote_to_local()?;
                return Err(e).chain_err(|| ErrorKind::InvalidRSL)?;
            }

            rsl.update_local()?;
            rsl.add_push_entry(ref_names)?;
            rsl.push()?;
        }
        match git::push(repo, &mut remote, ref_names) {
            Ok(_) => break 'push,
            Err(e) => {
                println!(
                    "Error: unable to push reference(s) {:?} to remote {:?}",
                    ref_names.clone().join(", "),
                    &remote.name().unwrap()
                );
                println!("  {}", e);
            }
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use utils::test_helper::*;
    use super::super::RemoteName;

    #[test]
    fn secure_push() {
        let mut context = setup_fresh();
        {
            assert_eq!((), super::super::rsl_init_with_cleanup(&mut context.local, &RemoteName::new("origin")).unwrap());
            let repo = &context.local;
            let mut rem = repo.find_remote("origin").unwrap().to_owned();

            let refs = vec!["master"];
            let res = super::secure_push(&repo, &mut rem, &refs).unwrap();
            assert_eq!(res, ());
        }
        teardown_fresh(context)
    }

    #[test]
    fn secure_push_twice() {
        let mut context = setup_fresh();
        {
            assert_eq!((), super::super::rsl_init_with_cleanup(&mut context.local, &RemoteName::new("origin")).unwrap());
            let repo = &context.local;
            let mut rem = repo.find_remote("origin").unwrap().to_owned();
            let refs = &["master"];
            super::secure_push(&repo, &mut rem, refs).unwrap();
            do_work_on_branch(&repo, "refs/heads/master");
            super::secure_push(&repo, &mut rem, refs).unwrap();
            // TODO add conditions
        }
        teardown_fresh(context)
    }
}
