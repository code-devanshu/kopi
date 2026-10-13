# 0006. Distribute on macOS without notarization

- **Status:** Accepted
- **Date:** 2026-10-13

## Context

Apple only lets an app open without a warning if it's signed with a Developer ID certificate and notarized. Both need the Apple Developer Program, which costs $99 a year. Kopi is free and open source, and won't pay that fee.

Kopi can't avoid the signing question, though. It needs the Accessibility and pasteboard permissions, and macOS ties those to the app's designated requirement. An ad-hoc signature's requirement names the build's own hash, so it changes on every build, and testers lose both permissions on every update ([INSTALL_ALPHA.md](../INSTALL_ALPHA.md)).

## Decision

1. **Don't join the Apple Developer Program.** Kopi isn't Developer ID signed or notarized.
2. **Sign releases with a self-signed code-signing certificate**, kept in the repository secrets. Its designated requirement is `identifier "in.devanshuverma.kopi" and certificate leaf = H"…"`, which is the same for every build, so permissions survive updates. The Release workflow imports it itself, because Tauri's `APPLE_CERTIFICATE` import only finds Apple-issued certificates, and refuses to build a release without it ([RELEASING.md](../RELEASING.md#signing)).
3. **Accept Gatekeeper's block on the first open**, and make it as easy as possible: the install guide, the release notes, a `.dmg` background that shows the steps and a one-line install from Terminal that doesn't quarantine the app ([#39](https://github.com/code-devanshu/kopi/issues/39)). The script only installs a Kopi signed with the certificate.
4. **Update in place with the Tauri updater** ([#38](https://github.com/code-devanshu/kopi/issues/38)), which signs updates with its own key, so testers don't download and approve every build.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Developer ID and notarization | $99 a year, every year. It's the only way to remove the first-open block, but Kopi can live with that block. |
| Ad-hoc signing only | Free, but testers lose their permissions on every update, which breaks pasting and history until they allow them again. |
| Mac App Store | Needs the same paid program, and Kopi can't run in the App Sandbox ([ADR 0004](0004-paste-simulation-approach.md)). |
| Homebrew cask | The official tap has dropped casks that fail Gatekeeper since Homebrew 5, and disabled them on 2026-09-01, and `--no-quarantine` is gone. A personal tap still works, but Homebrew quarantines what it downloads, so it would need to clear the flag itself, as the install script does without needing Homebrew. Worth adding if testers ask for it. |

## Consequences

- **Good:** no cost. Permissions survive updates: a build installed over another signed build kept both Accessibility and Paste from Other Apps on macOS 27.0.1. The updater installs updates without quarantine, so only the first install is blocked.
- **Bad:** every new user has to find **Open Anyway** in System Settings, which macOS 15 made harder. Some people won't install an app that macOS warns about. A build downloaded by hand is blocked again, which only matters for builds from before the updater.
- **Risk:** the certificate's private key can sign apps that macOS treats as Kopi, so it stays in the repository secrets and a password manager. Losing it means a new certificate, and testers allow the permissions once more.
- Windows signing is a separate decision, see [PLATFORM_NOTES.md](../PLATFORM_NOTES.md#code-signing-and-smartscreen).
