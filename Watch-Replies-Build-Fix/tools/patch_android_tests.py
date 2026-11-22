#!/usr/bin/env python3
"""Pin Robolectric unit tests to API 35 and run Android-dependent WatchLink tests under Robolectric."""
from pathlib import Path
import sys

ROOT = Path(sys.argv[1]).resolve()
OUT = Path(sys.argv[2]).resolve()
BASE = ROOT / "WatchMenuShell/app/src/test/java/com/example"
FILES = {
    "ContactsSyncTest.kt": (
        "import org.junit.Test\n",
        "import org.junit.Test\nimport org.junit.runner.RunWith\nimport org.robolectric.RobolectricTestRunner\nimport org.robolectric.annotation.Config\n",
        "class ContactsSyncTest {",
        "@RunWith(RobolectricTestRunner::class)\n@Config(sdk = [35])\nclass ContactsSyncTest {",
    ),
    "WeatherSyncTest.kt": (
        "import org.robolectric.RobolectricTestRunner\n",
        "import org.robolectric.RobolectricTestRunner\nimport org.robolectric.annotation.Config\n",
        "@RunWith(RobolectricTestRunner::class)\nclass WeatherSyncTest {",
        "@RunWith(RobolectricTestRunner::class)\n@Config(sdk = [35])\nclass WeatherSyncTest {",
    ),
    "RepliesUtf8Test.kt": (
        "import org.robolectric.RobolectricTestRunner\n",
        "import org.robolectric.RobolectricTestRunner\nimport org.robolectric.annotation.Config\n",
        "@RunWith(RobolectricTestRunner::class)\nclass RepliesUtf8Test {",
        "@RunWith(RobolectricTestRunner::class)\n@Config(sdk = [35])\nclass RepliesUtf8Test {",
    ),
}
for filename, (old_import, new_import, old_class, new_class) in FILES.items():
    src = BASE / filename
    if not src.is_file():
        raise SystemExit(f"Missing expected unit test: {src}")
    text = src.read_text(encoding="utf-8")
    if text.count(old_import) != 1:
        raise SystemExit(f"Expected one import anchor in {src}; found {text.count(old_import)}")
    if text.count(old_class) != 1:
        raise SystemExit(f"Expected one class annotation anchor in {src}; found {text.count(old_class)}")
    # Do not double-apply if the user already made this fix.
    if "@Config(sdk = [35])" in text:
        raise SystemExit(f"{src} already has an SDK override; review it instead of overwriting it")
    text = text.replace(old_import, new_import, 1).replace(old_class, new_class, 1)
    dest = OUT / filename
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(text, encoding="utf-8")
print("Prepared Robolectric API-35 test files (no project files changed yet).")
