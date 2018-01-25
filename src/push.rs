use git2::{Reference, Repository, Remote};

use std::process;

use common::{self, PushEntry};
use common::rsl::{RSL, HasRSL};
use common::nonce_bag::{NonceBag, HasNonceBag};
use common::nonce::{Nonce, HasNonce, NonceError};

pub fn secure_push<'repo>(repo: &Repository, mut remote: &mut Remote, ref_names: Vec<&str>) {

    let mut remote_rsl: RSL;
    let mut local_rsl: RSL;
    let mut nonce_bag: NonceBag;
    let mut nonce: Nonce;

    //let mut refs = ref_names.iter().filter_map(|name| &repo.find_reference(name).ok());

    'push: loop {

        repo.fetch_rsl(&mut remote).expect("Problem fetching Remote RSL. Check your connection or your SSH config");

        repo.init_rsl_if_needed(&mut remote).expect("Problem initializing RSL");

        let (remote_rsl, local_rsl, nonce_bag, nonce) = match repo.read_rsl() {
            Ok((a,b,c,d)) => (a,b,c,d),
            Err(e) => panic!("Couldn't read RSL: {:?}", e),
        };


        if !common::validate_rsl(repo, &remote_rsl, &local_rsl, &nonce_bag, &nonce) {
            println!("Error: invalid remote RSL");
            process::exit(-1);
        }

        // validate that fast forward is possible

        // checkout remote rsl detached
        // make new push entry
        let prev_hash = match remote_rsl.last_push_entry {
            Some(pe) => pe.hash(),
            None => String::from(""),
        };
        //TODO change this to be all ref_names
        let new_push_entry = PushEntry::new(repo, ref_names.first().unwrap(), prev_hash, nonce_bag.clone());
        // TODO commit new pushentry
        repo.commit_push_entry(&new_push_entry).expect("Couldn't commit new push entry");

        match common::push(repo, &mut remote, &ref_names) {
            Ok(_) => break 'push,
            Err(e) => {
                println!("Error: unable to push reference(s) {:?} to remote {:?}", &ref_names.clone().join(", "), &remote.name().unwrap());
                println!("  {}", e);
                process::exit(51);
            },
        };
    }
    //TODO localRSL = RemoteRSL (fastforward)
}
