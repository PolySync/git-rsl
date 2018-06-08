use git2::{Repository, BranchType, Oid, Reference};

pub fn teleport(repo: &Repository, target_branch_name: &str, teleport_target: &str) -> bool {
    let teleport_branch_ref: Reference = repo.find_reference(&format!("refs/heads/{}", teleport_target)).expect("failed to get branch ref");

    let mut target_branch_ref = repo.find_reference(&format!("refs/heads/{}", target_branch_name)).expect("failed to get teleport branch ref");

    target_branch_ref.set_target(teleport_branch_ref.peel_to_commit().expect("failed to peel to commit").id(), "").is_ok()
}

pub fn rollback(repo: &Repository, target_branch_name: &str) -> bool {
    let mut branch_ref: Reference = repo.find_reference(&format!("refs/heads/{}", target_branch_name)).expect("failed to get branch ref");

    let latest_commit_parent_oid: Oid = branch_ref.peel_to_commit().expect("failed to peel to commit").parent_id(0).expect("failed to get parent oid");

    branch_ref.set_target(latest_commit_parent_oid, "").is_ok()
}

pub fn deletion(repo: &Repository, target_branch_name: &str) -> bool {
    let mut target_branch = repo.find_branch(target_branch_name, BranchType::Local).expect("failed to get target branch");

    target_branch.delete().is_ok()
}