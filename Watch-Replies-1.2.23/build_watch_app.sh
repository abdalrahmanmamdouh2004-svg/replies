#!/usr/bin/env bash
# Builds Watch Replies for the watch: watch-replies.wp2app
# Needs: git, make, uv, Rust (rustup), resvg. Run it from this folder (next to the watch-replies folder).
# The build itself happens in ~/.cache/watch-replies-build, because the apps repo's Makefile breaks when the folder's path
# has spaces or brackets in it (like "Downloads/Watch-Replies-1.2.14 (1)"). Your folder is not touched except for the result.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
APP=watch-replies
WORK="${XDG_CACHE_HOME:-$HOME/.cache}/watch-replies-build"

command -v git >/dev/null    || { echo "Install git first.";  exit 1; }
command -v make >/dev/null   || { echo "Install make first."; exit 1; }
command -v cargo >/dev/null  || { echo "Install Rust first: https://rustup.rs"; exit 1; }
command -v uv >/dev/null     || { echo "Install uv first: https://docs.astral.sh/uv/"; exit 1; }
command -v resvg >/dev/null  || { echo "Install resvg first (cargo install resvg)"; exit 1; }
case "$WORK" in *[[:space:]\(\)]*) echo "The build folder $WORK has spaces or brackets in it; set XDG_CACHE_HOME to a plainer path."; exit 1;; esac

rustup target add thumbv7em-none-eabi
rustup component add llvm-tools

mkdir -p "$WORK"
cd "$WORK"
[ -d apps ] || git clone --depth 1 https://github.com/cmf-tech-community/apps apps
rm -rf apps/apps/$APP
cp -R "$HERE/$APP" apps/apps/$APP
# the store tool in the apps repo does not know the 'published' key (it is for the store listing, not the build): leave it
# out of the copy that is built; your own folder is not touched
sed -i.bak '/^published[[:space:]]*=/d' apps/apps/$APP/store.toml && rm -f apps/apps/$APP/store.toml.bak

cd apps
make test APP=$APP
make build APP=$APP
cp target/$APP.wp2app "$HERE/$APP.wp2app"
echo
echo "Done: $HERE/$APP.wp2app"
