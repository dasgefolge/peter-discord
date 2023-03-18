function ThrowOnNativeFailure {
    if (-not $?)
    {
        throw 'Native Failure'
    }
}

git push
ThrowOnNativeFailure

# copy the tree to the WSL file system to improve compile times
wsl rsync --delete -av /mnt/c/Users/fenhl/git/github.com/dasgefolge/peter-discord/stage/ /home/fenhl/wslgit/github.com/dasgefolge/peter-discord/ --exclude target
ThrowOnNativeFailure

wsl env -C /home/fenhl/wslgit/github.com/dasgefolge/peter-discord cargo build --release --target=x86_64-unknown-linux-musl
ThrowOnNativeFailure

wsl cp /home/fenhl/wslgit/github.com/dasgefolge/peter-discord/target/x86_64-unknown-linux-musl/release/peter /mnt/c/Users/fenhl/git/github.com/dasgefolge/peter-discord/stage/target/wsl/release/peter
ThrowOnNativeFailure

ssh gefolge.org sudo systemctl stop peter
ThrowOnNativeFailure

ssh gefolge.org env -C /opt/git/github.com/dasgefolge/peter-discord/master git pull
ThrowOnNativeFailure

scp .\target\wsl\release\peter gefolge.org:bin/peter
ThrowOnNativeFailure

ssh gefolge.org sudo systemctl start peter
ThrowOnNativeFailure
