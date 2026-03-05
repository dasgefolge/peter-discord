git push
if (-not $?)
{
    throw 'Native Failure'
}

# copy the tree to the WSL file system to improve compile times
wsl -d ubuntu-m2 rsync --delete -av /mnt/c/Users/fenhl/git/github.com/dasgefolge/peter-discord/stage/ /home/fenhl/wslgit/github.com/dasgefolge/peter-discord/ --exclude target
if (-not $?)
{
    throw 'Native Failure'
}

wsl -d ubuntu-m2 env -C /home/fenhl/wslgit/github.com/dasgefolge/peter-discord /home/fenhl/.cargo/bin/cargo build --release --target=x86_64-unknown-linux-musl
if (-not $?)
{
    throw 'Native Failure'
}

wsl -d ubuntu-m2 mkdir -p /mnt/c/Users/fenhl/git/github.com/dasgefolge/peter-discord/stage/target/wsl/release
if (-not $?)
{
    throw 'Native Failure'
}

wsl -d ubuntu-m2 cp /home/fenhl/wslgit/github.com/dasgefolge/peter-discord/target/x86_64-unknown-linux-musl/release/peter /mnt/c/Users/fenhl/git/github.com/dasgefolge/peter-discord/stage/target/wsl/release/peter
if (-not $?)
{
    throw 'Native Failure'
}

ssh gefolge.org sudo systemctl stop peter
if (-not $?)
{
    throw 'Native Failure'
}

ssh gefolge.org env -C /opt/git/github.com/dasgefolge/peter-discord/main git pull
if (-not $?)
{
    throw 'Native Failure'
}

scp .\target\wsl\release\peter gefolge.org:bin/peter
if (-not $?)
{
    throw 'Native Failure'
}

ssh gefolge.org sudo systemctl start peter
if (-not $?)
{
    throw 'Native Failure'
}
