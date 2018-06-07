# git-rsl

## Overview

git-rsl is a Rust implementation of the Reference State Log (RSL) detailed in [On omitting
commits and committing omissions: Preventing Git metadata tampering that
(re)introduces software
vulnerabilities](https://www.usenix.org/system/files/conference/usenixsecurity16/sec16_paper_torres-arias.pdf).
The paper authors are actively working to get RSL incorporated into
[git](https://github.com/git/git), at which point this project is moot.

git-rsl currently provides two binaries, `git-secure-fetch` and `git-secure-push` which
work as `git` plugins.

## Getting Started

### Dependencies

`git-rsl` is a Rust project with some system dependencies.

* [rust](https://github.com/rust-lang-nursery/rustup.rs)
* [git](https://git-scm.com/)
* [gnupg2](https://gnupg.org/)

### Building

These instructions include the installation instructions for the dependencies,
assuming some flavor of Linux. `git-rsl` was developed on Ubuntu, and while
the build instructions assume the same, anywhere the dependencies can be installed
should work just as well.


* Install Rust dependency
  ```bash
  curl https://sh.rustup.rs -sSf | sh
  ```
* Install git dependency
  ```bash
  sudo apt-get update
  sudo apt-get install git-core
  ```
* Install gnupg2 dependency
  ```bash
  sudo apt-get update
  sudo apt-get install gnupg2
  ```
* Download and build `git-rsl` itself:
  ```bash
  git clone https://github.com/PolySync/git-rsl.git
  cd git-rsl
  cargo build
  ```

### Installation

After obtaining the dependencies listed in the Building section above,
one can install the `git-rsl` binaries (`git-secure-fetch` and `git-secure-push`)
by doing the following:

* Install from git (does not require a local clone):
  ```bash
  cargo install --force --git https://github.com/PolySync/git-rsl.git
  ```
* Alternatively, if you have cloned the `git-rsl` repo and want to install
  binaries based on your local development code, you can instead run from
  the `git-rsl` repository's root directory:
  ```bash
  cargo install --force
  ```

## Usage

Correct usage of the RSL depends on merging PRs from the command line.
Using GitHub's or some other UI to merge will invalidate the RSL.

Merging changes the tip of the target branch, requiring a new RSL entry
for that branch. In order to take advantage of the extra security afforded
by git-rsl, `secure-push` and `secure-fetch` must be used in conjunction
with `secure-merge`, which fetches the branches to be merged, and uses
`secure-push` to send back to the remote, creating a new RSL entry for
the target branch in the process.

`secure-merge`, which is not yet implemented, enables repositories to
have signed merge commits, which is impossible merging from the UI.

If you have installed the tools successfully using the provided install script, you should be able to fetch and push securely.

```bash
# Note that the branch name must be exact and not aliased (no using `HEAD`)
# Note that only a single remote and a single branch may be specified at a time
git secure-fetch <REMOTE> <BRANCH>

# Similarly, only a single remote and branch at a time may be specified for pushing
git secure-push <REMOTE> <BRANCH>
```


### Examples

* Example assuming a pre-existing git repository with a remote named "origin"
  and a branch named "master"
  ```bash
  # Within the context of a git repository directory
  # These commands must be run in the top level of the git project
  # (i.e. the directory containing the `.git` dir)
  cd my_git_repo
  git checkout master

  # Securely fetch content from the remote named `origin` for the branch `master`
  git secure-fetch origin master

  # Make a new commit
  echo 'Hello git-rsl' > git_rsl_greeting.txt
  git add git_rsl_greeting.txt
  git commit -m 'Trivial commit example'

  # Securely push the commit to your remote
  git secure-push origin master
  ```

## Tests

`git-rsl` manages its tests with the standard Rust test framework, plus `proptest`
for property-based testing.

### Building

Tests can be built from the `git-rsl` repository directory with:

```bash
cargo build --tests
```

### Running

The standard tests available can be run from the `git-rsl` repository directory:

```bash
cargo test

# Additional long-running tests are ignored by default, but can be run using:
cargo test -- --ignored
```

# License

© 2018, PolySync Technologies, Inc.

* Jeff Weiss [email:](mailto:jeffweiss@polysync.io)
* Gabriella Chronis [email:](mailto:gchronis@polysync.io)
* Katie Cleary [email:](mailto:kcleary@polysync.io)
* Zack Pierce [email:](mailto:zpierce@polysync.io)

Please see the [LICENSE](./LICENSE) file for more details
