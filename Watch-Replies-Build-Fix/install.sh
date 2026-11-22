#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ALLOW=0
ROOT_ARG=""
for arg in "$@"; do
  case "$arg" in
    --allow-local-changes) ALLOW=1 ;;
    -h|--help) echo "Usage: ./install.sh [PROJECT_ROOT] [--allow-local-changes]"; exit 0 ;;
    *) ROOT_ARG="$arg" ;;
  esac
done
if [[ -n "$ROOT_ARG" ]]; then ROOT="$(realpath "$ROOT_ARG")"; else ROOT="$(pwd)"; fi
[[ -f "$ROOT/Watch-Replies-1.2.6/watch-replies/src/fonts.rs" && -f "$ROOT/Watch-Replies-1.2.6/watch-replies/src/arabic.rs" && -f "$ROOT/WatchMenuShell/gradlew" ]] || {
  echo "Project root must contain Watch-Replies-1.2.6/ and WatchMenuShell/." >&2; exit 2;
}
WATCH="$ROOT/Watch-Replies-1.2.6/watch-replies"
ANDROID_TEST="$ROOT/WatchMenuShell/app/src/test/java/com/example"
# These hashes identify the exact outputs from the previously installed Arabic patch.
check_sha() {
  local path="$1" expected="$2" actual
  actual="$(sha256sum "$path" | awk '{print $1}')"
  if [[ "$actual" != "$expected" ]]; then
    echo "Local source differs from the expected Arabic-patch version: $path" >&2
    echo "Expected SHA-256: $expected" >&2
    echo "Found SHA-256:    $actual" >&2
    if [[ "$ALLOW" -ne 1 ]]; then
      echo "Nothing changed. Review your file and backup, then pass --allow-local-changes only if replacement is intended." >&2
      exit 3
    fi
  fi
}
check_sha "$WATCH/src/font.rs" "50e498ac9668ac5896283b064141e364ba903e722e836ae45dcc3a2de5ad8e1c"
check_sha "$WATCH/src/fonts.rs" "8b25d1c6ebe6f113e021ea2a43dfdaf550844e0c78d09518186ca241ab14a9dc"
check_sha "$WATCH/fonts/mkfont.py" "71223f88f08f3bdb12303800ed50da0fe28178f1a3f0ca2cdd4e75e7d5095f3c"

STAGE="$(mktemp -d "$ROOT/.watch-replies-build-fix.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
python3 "$HERE/tools/validate_fonts.py" "$HERE"
python3 "$HERE/tools/patch_android_tests.py" "$ROOT" "$STAGE/tests"

for f in ContactsSyncTest.kt WeatherSyncTest.kt RepliesUtf8Test.kt; do
  [[ -f "$ANDROID_TEST/$f" ]] || { echo "Missing $ANDROID_TEST/$f" >&2; exit 4; }
done
BACKUP="$ROOT/.backups/Watch-Replies-Build-$(date +%Y%m%d-%H%M%S)-$$"
mkdir -p "$BACKUP/Watch-Replies-1.2.6/watch-replies/src" "$BACKUP/Watch-Replies-1.2.6/watch-replies/fonts" "$BACKUP/WatchMenuShell/app/src/test/java/com/example"
for rel in \
  Watch-Replies-1.2.6/watch-replies/src/font.rs \
  Watch-Replies-1.2.6/watch-replies/src/fonts.rs \
  Watch-Replies-1.2.6/watch-replies/fonts/mkfont.py \
  WatchMenuShell/app/src/test/java/com/example/ContactsSyncTest.kt \
  WatchMenuShell/app/src/test/java/com/example/WeatherSyncTest.kt \
  WatchMenuShell/app/src/test/java/com/example/RepliesUtf8Test.kt; do
  mkdir -p "$BACKUP/$(dirname "$rel")"
  cp -a "$ROOT/$rel" "$BACKUP/$rel"
done
cp -a "$HERE/Watch-Replies-1.2.6/watch-replies/src/font.rs" "$WATCH/src/font.rs"
cp -a "$HERE/Watch-Replies-1.2.6/watch-replies/src/fonts.rs" "$WATCH/src/fonts.rs"
cp -a "$HERE/Watch-Replies-1.2.6/watch-replies/fonts/mkfont.py" "$WATCH/fonts/mkfont.py"
cp -a "$STAGE/tests/"*.kt "$ANDROID_TEST/"

FULL="$ROOT/Watch-Replies-Build-Fixed-Full-Source"
if [[ -e "$FULL" ]]; then FULL="$ROOT/Watch-Replies-Build-Fixed-Full-Source-$(date +%Y%m%d-%H%M%S)-$$"; fi
mkdir -p "$FULL/Watch-Replies-1.2.6/watch-replies/src" "$FULL/Watch-Replies-1.2.6/watch-replies/fonts" "$FULL/WatchMenuShell/app/src/test/java/com/example"
cp -a "$WATCH/src/font.rs" "$WATCH/src/fonts.rs" "$FULL/Watch-Replies-1.2.6/watch-replies/src/"
cp -a "$WATCH/fonts/mkfont.py" "$FULL/Watch-Replies-1.2.6/watch-replies/fonts/"
cp -a "$ANDROID_TEST/"*.kt "$FULL/WatchMenuShell/app/src/test/java/com/example/"
cat <<EOF

Watch font compression and Android test configuration installed.
Backup: $BACKUP
Complete replacement files: $FULL

Watch: 16-level bitmaps were reduced to 4 anti-aliased coverage levels (2 bits/pixel),
and unused Arabic presentation glyphs were removed. The drawing dimensions and glyph metrics remain unchanged.
Android tests: pinned to Robolectric API 35; ContactsSyncTest now runs under Robolectric
so WatchLink can initialize its Android main-thread Handler.

Next, run these separately and paste the full output if either fails:
  cd "$ROOT"
  ./Watch-Replies-1.2.6/build_watch_app.sh
  cd "$ROOT/WatchMenuShell"
  ./gradlew --no-daemon :app:compileDebugKotlin :app:testDebugUnitTest
EOF
