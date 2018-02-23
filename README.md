# kevlar-laces-rs

kevlar-laces-rs is a Rust implementation of the Reference State Log (RSL) detailed in [On omitting
commits and committing omissions: Preventing Git metadata tampering that
(re)introduces software
vulnerabilities](https://www.usenix.org/system/files/conference/usenixsecurity16/sec16_paper_torres-arias.pdf).
The paper authors are actively working to get RSL incorporated into
[git](https://github.com/git/git), at which point this project is moot.

### Warning

Correct usage of the RSL depends on merging PRs from the command line. Using GitHub's or some other UI to merge will invalidate the RSL. Merging changes the tip of the target branch, requiring a new RSL entry for that branch. In order to take advantage of the extra security afforded by Kevlar Laces, secure push and pull must be used in conjunction with `secure-merge`, which fetches the branches to be merged, and uses `secure-push` to send back to the remote, creating a new RSL entry for the target branch in the process. `secure-merge`, which is not yet implemented, enables repositories to have signed merge commits, which is impossible merging from the UI.

## Installation

kevlar-laces-rs is intended to be a git helper application installed anywhere in your
path. When running `git-secure-push`, git will look for the `kevlar-laces-rs` helper application
and invoke it in the `--push` mode.

This crate is currently private. Until it is hosted on crates.io or an internal cargo server, the easiest way to install is to clone this repository and run the install script. The script ensures that all the prerequisites are installed, installs them if necessary, runs the tests, builds the latest version of the project, and installs the compiled binary to `~/.cargo/bin/kevlar-laces-rs`. Then, it creates symlinks from `git-secure-push` and `git-secure-fetch`. kevlar-laces-rs has two modes: fetch and push. If you prefer to install manually, we recommend either creating symlinks as in the install script OR establishing git aliases for these specific modes,\ as shown below.

### Prerequisites

* Rust
* git
* Cargo
* gnupg2

### Steps

```
git clone git@github.com:PolySync/kevlar-laces-rs.git
cd kevlar-laces-rs
./install.sh
```

The above example explicitly runs the install script with a preceding `.`. This is solely for convenience in the current terminal session, because `install.sh` may need to modify `$PATH` to include `$HOME/.cargo/bin`. If you already have `$HOME/.cargo/bin` in `$PATH` or you do not need to run the various kevlar-laces git subcommands in the _current_ terminal session, then `./install.sh` will suffice.

### Git Alias

```
$ git config --global alias.secure-push '!kevlar-laces-rs --push'
$ git config --global alias.secure-fetch '!kevlar-laces-rs --fetch'
```

The `!` before the executable name tells git that we are not aliasing a git subcommand but running an external application. NB single quotes around the command are essential to properly escape the bang character.

### Symlinks

```
$ ln -s kevlar-laces-rs git-secure-fetch
$ ln -s kevlar-laces-rs git-secure-push
```

`git-secure-fetch` and `git-secure-push` are specifically searched for by kevlar-laces-rs
and will automatically invoke `--fetch` and `--push`, respectively.

### For developers

```
git clone git@github.com:PolySync/kevlar-laces-rs.git
cd kevlar-laces-rs
cargo build
ln -s target/debug/kevlar-laces-rs ~/.cargo/bin
```

## Usage

If you have installed the tool successfully using the provided install script, you should be able to push and fetch securely.
```
git secure-push origin <branch>
git secure-fetch origin <branch>
```
### Limitations

* Branch names must be stipulated (no using `HEAD` =/).
* Must be run in the top level of the git project (i.e., the one containing the `.git` dir).
* No pushing or fetching multiple branches.
* Only supports a single remote, which must be named `origin`.

## Exit Codes

When kevlar-laces-rs encounters an unrecoverable error, the process will exit with a
unique code to help diagnose the situation.

| Code | Description |
| ---- | ----------- |
| 0    | Success     |
| -1   | The reference state log in the remote repository is not valid. May be compromised. |
| 10   | The remote doesn't have an `RSL` branch. If you've already run `kevlar-laces-rs --push <remote> <branches>` in the past, then this remote is likely compromised since the `RSL` branch is now missing. |
| 50   | The remote you specified doesn't exist in your configuration. |
| 51   | The ref or branch name you tried to fetch doesn't exist on the remote. |
| 52   | We were unable to find a nonce file for writing. This indicates that the `.git` directory is perhaps not writable. |
| 53   | We found a nonce file in `.git/NONCE`, but we could not write to it.  This is likely a permissions problem in the `.git` directory. |
| 60   | We were unable to get a random number generator from the operating system for creating the nonce. |
| 61   | We were unable to open `.git/NONCE` for creation or reading. This is a permissions problem. |
| 62   | We were unable to write the nonce to `.git/NONCE`. This is a permissions problem. |
| 99   | A bug. Unexpected situation. Please open an issue. |
