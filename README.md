# git-rsl

git-rsl is an implementation of the Reference State Log (RSL) detailed in [On omitting
commits and committing omissions: Preventing Git metadata tampering that
(re)introduces software
vulnerabilities](https://www.usenix.org/system/files/conference/usenixsecurity16/sec16_paper_torres-arias.pdf).
The paper authors are actively working to get RSL incorporated into
[git](https://github.com/git/git), at which point this project is moot.

## Usage

git-rsl is intended to be a git helper application installed anywhere in your
path. When running `git rsl`, git will look for the `git-rsl` helper application
and invoke it.

git-rsl has two modes: fetch and push. We recommend either establishing git
aliases or symlinks for these specific modes.

### Git Alias

```
$ git config --global securepush "rsl --push"
$ git config --global securefetch "rsl --fetch"
```

### Symlinks

```
$ ln -s git-rsl git-securefetch
$ ln -s git-rsl git-securepush
```

`git-securefetch` and `git-securepush` are specifically searched for by git-rsl
and will automatically invoke `--fetch` and `--push`, respectively.


## Exit Codes

When git-rsl encounters an unrecoverable error, the process will exit with a
unique code to help diagnosis the situation.

| Code | Description |
| ---- | ----------- |
| 0    | Success     |
| -1   | The reference state log in the remote repository is not valid. May be compromised. |
| 10   | The remote doesn't have an `RSL` branch. If you've already run `git-rsl
--push <remote> <branches>` in the past, then this remote is likely compromised
since the `RSL` branch is now missing. |
| 11   | The remote doesn't have an `RSL_NONCE` branch. If you've already run
`git-rsl --push <remote> <branches>` in the past, this this remote is likely
compromised since the `RSL_NONCE` branch is missing. The nonce branch is
mutable, so this is less critical than if the `RSL` branch were missing, but
still should involve investigation. |
| 50   | The remote you specified doesn't exist in your configuration. |
| 51   | The ref or branch name you tried to fetch doesn't exist on the remote. |
| 52   | We were unable to find a nonce file for writing. This indicates that
the `.git` directory is perhaps not writable. |
| 53   | We found a nonce file in `.git/NONCE`, but we could not write to it.
This is likely a permissions problem in the `.git` directory. |
| 60   | We were unable to get a random number generator from the operating
system for creating the nonce. |
| 61   | We were unable to open `.git/NONCE` for creation or reading. This is a
permissions problem. |
| 62   | We were unable to write the nonce to `.git/NONCE`. This is a
permissions problem. |
| 99   | A bug. Unexpected situation. Please open an issue. |
