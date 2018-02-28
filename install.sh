
#!/bin/bash

echo "Installing kevlar-laces-rs to ~/.local/bin ..."
echo ""
echo "Checking dependencies ..."

if command -v cargo >/dev/null 2>&1 ; then
    echo "    cargo found"
    echo "    version: $(cargo -V)"
else
    echo "Error: cargo not found"
    exit -1
fi

if command -v git >/dev/null 2>&1 ; then
    echo "    git found"
    echo "    version: $(git --version)"
else
    echo "Error: git not found"
    echo "kevlar-laces is a git plugin and as such will not work without it."
    exit -1
fi

if command -v gpg2 >/dev/null 2>&1 ; then
    echo "    gpg2 found"
    echo "    version: $(gpg2 --version | sed -n 1p)"
else
    echo "Error: gpg2 not found"
    if command -v gpg >/dev/null 2>&1 ; then
        echo "    gpg found"
        echo "    version: $(gpg --version | sed -n 1p)"
    else
        echo "Error: gpg not found"
        echo "kevlar-laces-rs utlizes GPG to verify the author of a commit."
        echo "Please install it before proceeding."
        echo "    $ apt-get install gpg2"
    fi
fi
echo "All dependencies present. Proceeding with install."
echo ""

DEST_DIR=$HOME/.cargo/bin
echo "Ensuring $DEST_DIR exists ..."
mkdir -p $DEST_DIR

echo "Running tests ..."
cargo test
if [ $? != 0 ]
then
  echo "I cannot install kevlar-laces-rs in good conscience when tests are failing."
  exit -1
fi
echo "All tests pass. Proceeding with install ..."

echo "Building binaries ..."
cargo build

echo "Creating symlink for kevlar-laces-rs binary ..."
ln -sf `pwd`/target/debug/kevlar-laces-rs $DEST_DIR

output=$(echo $PATH | grep -F $DEST_DIR)
if [ $? != 0 ]
then
  echo "Could not find $DEST_DIR in PATH. Adding to ~/.bashrc"
  echo ""
  echo "export PATH=\$PATH:$DEST_DIR"
  echo "export PATH=\$PATH:$DEST_DIR" >> $HOME/.bashrc
  eval "export PATH=$PATH:$DEST_DIR"
fi

echo "Creating git aliases for secure-fetch and secure-push ..."
git config --global alias.secure-push '!kevlar-laces-rs --push'
git config --global alias.secure-fetch '!kevlar-laces-rs --fetch'

echo "Installation successful. To learn about usage of this tool, run any of
the subcommands with the '-h' flag, e.g. 'git secure-push -h'.
For even more information, please consult the README."
