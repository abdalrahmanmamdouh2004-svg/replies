#!/usr/bin/env bash
# Builds Watch Replies for the watch: watch-replies.wp2app
# Needs: git, make, uv, Rust (rustup), resvg. Run it from this folder (next to the watch-replies folder).
set -euo pipefail
cd "$(dirname "$0")"
APP=watch-replies

command -v git >/dev/null    || { echo "Install git first.";  exit 1; }
command -v make >/dev/null   || { echo "Install make first."; exit 1; }
command -v cargo >/dev/null  || { echo "Install Rust first: https://rustup.rs"; exit 1; }
command -v uv >/dev/null     || { echo "Install uv first: https://docs.astral.sh/uv/"; exit 1; }
command -v resvg >/dev/null  || { echo "Install resvg first (cargo install resvg)"; exit 1; }

rustup target add thumbv7em-none-eabi
rustup component add llvm-tools

[ -d apps ] || git clone --depth 1 https://github.com/cmf-tech-community/apps apps
rm -rf apps/apps/$APP
cp -R $APP apps/apps/$APP
# the store tool in the apps repo does not know the 'published' key (it is for the store listing, not the build): leave it
# out of the copy that is built; your own folder is not touched
sed -i '/^published[[:space:]]*=/d' apps/apps/$APP/store.toml

cd apps
make test APP=$APP
make build APP=$APP
cp target/$APP.wp2app ../$APP.wp2app
echo
echo "Done: $(cd .. && pwd)/$APP.wp2app"
