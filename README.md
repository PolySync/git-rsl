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

