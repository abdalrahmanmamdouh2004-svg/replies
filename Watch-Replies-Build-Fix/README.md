# Watch Replies build fixes

This follow-up fixes the two blockers found after installing the Arabic/English keyboard patch.

## Watch memory overflow

The Arabic patch initially stored each font pixel as a 4-bit coverage value (16 shades) and embedded an entire Arabic presentation-form range. The watch font tables are now stored at 2 bits per pixel (four coverage levels) and retain only the Arabic presentation glyphs referenced by `arabic.rs`, plus the Arabic punctuation and digit glyphs displayed directly. Glyph dimensions, positions, advances, keyboard layouts, and screen geometry are unchanged. Anti-aliasing remains enabled, with four levels instead of sixteen.

The font generator `mkfont.py` was updated to produce the same compact table if it is regenerated later.

## Android unit tests

The app targets API 36, while its pinned Robolectric 4.14.1 supports through API 35. The tests now declare `@Config(sdk = [35])`, which is sufficient for these serializer/formatting tests. `ContactsSyncTest` additionally uses `RobolectricTestRunner` because `WatchLink` initializes a main-thread `Handler` and requires Android's `Looper` environment.

## Install

Extract this archive, then run from the extracted folder:

```bash
./install.sh ~/Downloads
```

Change `~/Downloads` only if your project root is elsewhere. The installer checks that the font files match the files installed by the earlier Arabic fix, stages all changes before copying them, and backs up every replaced file under `.backups/`. It refuses to overwrite a file that differs from the expected version unless `--allow-local-changes` is supplied.

After installation, run the watch build and Android tests separately as printed by the installer. This bundle was statically validated in the packaging environment; a real watch build and physical-device test still need to be performed on your machine.
