# Spike: macOS pasteboard access

A tiny AppKit app for [issue #4](https://github.com/code-devanshu/kopi/issues/4). Each button touches the general pasteboard in exactly one way, so we can see which call, if any, makes macOS ask for "Paste from Other Apps" permission. Results are written up in [docs/spikes/0001-macos-pasteboard-access.md](../../docs/spikes/0001-macos-pasteboard-access.md).

It needs only the Command Line Tools (`swiftc`, `codesign`). There's no Xcode project.

## Build and run

```sh
./build.sh --open           # build/KopiSpike.app, bundle ID in.devanshuverma.kopi.spike, ad-hoc signed
BUILD=2 ./build.sh --open   # same, with CFBundleVersion 2, to mimic an update
```

The build is universal (arm64 + x86_64) with a minimum of macOS 14. Every rebuild is ad-hoc signed, so the CDHash changes. The script prints it.

## Tests

| Button | What it calls |
| --- | --- |
| A: Poll changeCount | `changeCount` every 0.5 s for 60 s, nothing else |
| B: Read types | `types`, then `pasteboardItems.count` |
| C: Read string | `string(forType: .string)` |
| D: Read image | `data(forType: .png)`, falling back to `.tiff` |
| E: Privacy filter | `types`, then reads the string only if no concealed, transient or auto-generated marker is present |

The helper buttons check `accessBehavior` (macOS 15.4+) and put a concealed or transient sample on the pasteboard for Test E.

Every call is timed. A call that blocks on a system alert shows up as a long duration.

All pasteboard calls run on the main thread. On macOS 27.0.1, calling `accessBehavior` from a background queue makes AppKit raise `NSInternalInconsistencyException`, and the app aborts. A call can also block the main thread for a long time, so a watchdog on its own queue writes `main thread still blocked after N s` to the log every 5 s. The window freezes while that happens, but the log file keeps going.

## Log

Everything goes to the window and to `~/Library/Logs/KopiSpike.log`. At launch, the app logs the macOS version and build, the bundle version, the signing identifier and CDHash, the `EnablePasteboardPrivacyDeveloperPreview` default and `accessBehavior`. Clip content is never logged, only lengths, type names, sizes and timings.

## Developer-preview flag

On macOS 15.4 and later, the privacy alert can be switched on for one app (the macOS 27.2 betas reportedly remove this flag):

```sh
defaults write in.devanshuverma.kopi.spike EnablePasteboardPrivacyDeveloperPreview -bool yes
defaults delete in.devanshuverma.kopi.spike EnablePasteboardPrivacyDeveloperPreview   # undo
```

Quit and relaunch the app after changing it.

**Don't turn it on under macOS 27.0.1.** There, the flag makes content reads (and sometimes `accessBehavior`) hang with no alert and no time limit, because the system's Trusted UI service isn't available. Quit a hung spike with `pkill KopiSpike`.

## Reset

```sh
tccutil reset Pasteboard in.devanshuverma.kopi.spike   # untested; the service name may differ by macOS version
rm ~/Library/Logs/KopiSpike.log
```
