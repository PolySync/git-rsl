use git2::{Repository, Remote};

use push_entry::PushEntry;
use rsl::{RSL, HasRSL};
use errors::*;

use utils::git;
pub fn secure_push<'remote, 'repo: 'remote>(repo: &'repo Repository, mut remote: &'remote mut Remote<'repo>, ref_names: &[&str]) -> Result<()> {

    //let mut refs = ref_names.iter().filter_map(|name| &repo.find_reference(name).ok());

    repo.fetch_rsl(&mut remote).chain_err(|| "Problem fetching Remote RSL. Check your connection or your SSH config")?;

    repo.init_rsl_if_needed(&mut remote).chain_err(|| "Problem initializing RSL")?;

    // checkout RSL branch
    git::checkout_branch(repo, "refs/heads/RSL")?;


    'push: loop {

        repo.fetch_rsl(&mut remote).chain_err(|| "Problem fetching Remote RSL. Check your connection or your SSH config")?;

        let mut rsl = RSL::read(repo, &mut remote).chain_err(|| "couldn't read RSL")?;

        rsl.validate().chain_err(|| "Invalid remote RSL")?;

        // TODO deal with no change necessary
        if !git::up_to_date(repo, "RSL", "origin/RSL")? {
            match git::fast_forward_possible(repo, "refs/remotes/origin/RSL") {
                Ok(true) => git::fast_forward_onto_head(repo, "refs/remotes/origin/RSL")?,
                Ok(false) => bail!("Local RSL cannot be fastforwarded to match remote. This may indicate that someone has tampered with the RSL history. Use caution before proceeding."),
                Err(e) => Err(e).chain_err(|| "Local RSL cannot be fastforwarded to match remote. This may indicate that someone has tampered with the RSL history. Use caution before proceeding.")?,
            }
        }

        rsl.add_push_entry(ref_names)?;

        // push RSL branch
        rsl.push()?;

        match git::push(repo, rsl.remote, ref_names) {
            Ok(_) => break 'push,
            Err(e) => {
                println!("Error: unable to push reference(s) {:?} to remote {:?}", ref_names.clone().join(", "), &rsl.remote.name().unwrap());
                println!("  {}", e);
            },
        };
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use utils::test_helper::*;

    #[test]
    fn secure_push() {
        let context = setup_fresh();
        {
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
        let context = setup_fresh();
        {
            let repo = &context.local;
            let mut rem = repo.find_remote("origin").unwrap().to_owned();
            let refs = &["master"];
            super::secure_push(&repo, &mut rem, refs).unwrap();
            do_work_on_branch(&repo, "master");
            super::secure_push(&repo, &mut rem, refs).unwrap();
            // TODO add conditions
        }
        teardown_fresh(context)
    }
}
