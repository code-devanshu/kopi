# Releasing Kopi

The [Release workflow](../.github/workflows/release.yml) builds a universal macOS `.dmg` (Apple silicon and Intel) and attaches it to a draft GitHub Release. This document is for maintainers. Testers should read [INSTALL_ALPHA.md](INSTALL_ALPHA.md).

## Cutting a release

1. Set the version in `apps/desktop/package.json` and in `[workspace.package]` in `Cargo.toml`. They must match. Run `cargo check` to update `Cargo.lock`.
2. Add the release to [CHANGELOG.md](../CHANGELOG.md).
3. Merge that to `main`, then tag the merge commit and push the tag:

   ```sh
   git tag v0.1.0-alpha.1
   git push origin v0.1.0-alpha.1
   ```

4. Wait for the workflow. It creates a **draft** release with the `.dmg`, its `.sha256`, a note on how it was signed and notes generated from the merged PRs.
5. Read the draft, download the `.dmg` and try it on a Mac. Then publish the release.

## Tags and versions

| Tag | Release |
|---|---|
| `v<version>`, such as `v0.1.0` | A normal release |
| `v<version>-<anything>`, such as `v0.1.0-alpha.1` | A prerelease |

`<version>` must be the app version, or the workflow fails before it builds. The suffix only names the tag, the release and the file (`Kopi_0.1.0-alpha.1_universal.dmg`). The app itself reports `0.1.0`, because macOS versions are three numbers.

Each build sets `CFBundleVersion` to the workflow's run number, so every build is newer than the last to macOS, even when the version stays the same.

If the workflow fails after it creates the release, fix the problem and rerun the failed job. A rerun replaces the files on the draft. To rebuild from a different commit, delete the draft release and the tag, then tag again.

## Trying the workflow without releasing

In **Actions → Release → Run workflow**, run it on a branch. It builds the `.dmg` and keeps it as a workflow artifact for 14 days. It doesn't create a release. Use this to test changes to the workflow or a new signing certificate before tagging.

## Signing and notarization

Without signing secrets, the app is **signed ad hoc**. That works, but macOS blocks the app until the tester approves it in System Settings, and testers lose their permissions on every update. [INSTALL_ALPHA.md](INSTALL_ALPHA.md) explains both. The build log and the release notes say which kind of build it is.

With a Developer ID certificate, Tauri signs the app with the hardened runtime, sends it to Apple to notarize and staples the ticket to it. Testers can open it at once, and their permissions should survive updates ([#21](https://github.com/code-devanshu/kopi/issues/21) checks this). This needs the [Apple Developer Program](https://developer.apple.com/programs/) ($99 a year).

Add these as repository secrets in **Settings → Secrets and variables → Actions**:

| Secret | What it is |
|---|---|
| `APPLE_CERTIFICATE` | Your **Developer ID Application** certificate and its private key, exported from Keychain Access as a `.p12` file and base64-encoded |
| `APPLE_CERTIFICATE_PASSWORD` | The password you set when you exported the `.p12` |
| `APPLE_SIGNING_IDENTITY` | The certificate's name, such as `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID` | The Apple Account email of the developer account |
| `APPLE_PASSWORD` | An [app-specific password](https://support.apple.com/102654) for that account, not its real password |
| `APPLE_TEAM_ID` | The 10-character team ID from [your developer account](https://developer.apple.com/account) |

The workflow signs when it has the first three secrets, and notarizes when it has all six. It fails if it has only one of `APPLE_CERTIFICATE` and `APPLE_SIGNING_IDENTITY`, so a typo can't silently fall back to an ad-hoc build.

To set them up:

1. In [Certificates, IDs & Profiles](https://developer.apple.com/account/resources/certificates/list), create a **Developer ID Application** certificate and install it in your login keychain.
2. In Keychain Access, find it under **My Certificates**, Control-click it and choose **Export**. Save it as a `.p12` with a password.
3. Find the identity's name:

   ```sh
   security find-identity -v -p codesigning
   ```

4. Add the secrets with the GitHub CLI, or paste them in the repository settings:

   ```sh
   base64 -i DeveloperID.p12 | gh secret set APPLE_CERTIFICATE
   gh secret set APPLE_CERTIFICATE_PASSWORD
   gh secret set APPLE_SIGNING_IDENTITY
   gh secret set APPLE_ID
   gh secret set APPLE_PASSWORD
   gh secret set APPLE_TEAM_ID
   ```

5. Delete the `.p12` file, then test with a manual run of the workflow.

The first notarized build has a different signature from the ad-hoc builds before it, so testers need to allow its permissions once more.

## What the workflow checks

Before it uploads anything, the workflow checks that:

- the tag matches the app version, and `package.json` and `Cargo.toml` agree
- `Kopi.app` holds both `arm64` and `x86_64` code
- the signature is valid (`codesign --verify --deep --strict`)
- for a notarized build, the ticket is stapled and Gatekeeper accepts the app (`stapler validate`, `spctl --assess`)

## Not yet

- **Auto-update.** The [updater plugin](https://v2.tauri.app/plugin/updater/) comes with the beta. Until then, testers download each build.
- **Windows.** Windows builds come with the Windows alpha. See [PLATFORM_NOTES.md](PLATFORM_NOTES.md#code-signing-and-smartscreen).
