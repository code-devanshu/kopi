# Releasing Kopi

The [Release workflow](../.github/workflows/release.yml) builds a universal macOS `.dmg` (Apple silicon and Intel), a Windows installer (x64) and the updates the app installs, and attaches them to a draft GitHub Release. When the release is published, the [Updater workflow](../.github/workflows/updater.yml) offers it to the app. This document is for maintainers. Testers should read [INSTALL_ALPHA.md](INSTALL_ALPHA.md).

## Cutting a release

1. Set the version in `apps/desktop/package.json` and in `[workspace.package]` in `Cargo.toml`. They must match. Run `cargo check` to update `Cargo.lock`.
2. Add the release to [CHANGELOG.md](../CHANGELOG.md).
3. Merge that to `main`, then tag the merge commit and push the tag:

   ```sh
   git tag v0.1.0-alpha.1
   git push origin v0.1.0-alpha.1
   ```

4. Wait for the workflow. It creates a **draft** release with the `.dmg`, its `.sha256`, the macOS update (`.app.tar.gz` and its `.sig`), the Windows installer (`_x64-setup.exe`, its `.sha256` and its `.sig`), `latest.json`, a note on opening it the first time and notes generated from the merged PRs.
5. Read the draft, download the `.dmg` and the `.exe`, and try them on a Mac and on Windows. Then publish the release.
6. Publishing runs the Updater workflow, which offers the release to everyone's Kopi. Check that it passed. Within a few hours, Kopi installs the update in the background, or straight away from **Check for Updates…**.

## Tags and versions

| Tag | Release |
|---|---|
| `v<version>`, such as `v0.1.0` | A normal release |
| `v<version>-<anything>`, such as `v0.1.0-alpha.1` | A prerelease |

`<version>` must be the app version, or the workflow fails before it builds. The suffix only names the tag, the release and the file (`Kopi_0.1.0-alpha.1_universal.dmg`). The app itself reports `0.1.0`, because macOS versions are three numbers.

Each build sets `CFBundleVersion` to the workflow's run number, so every build is newer than the last to macOS and to the updater, even when the version stays the same.

If the workflow fails after it creates the release, fix the problem and rerun the failed job. A rerun replaces the files on the draft. To rebuild from a different commit, delete the draft release and the tag, then tag again.

## The `.dmg` window

The `.dmg` opens to a window that shows the drag to Applications and how to open Kopi the first time. The background is `apps/desktop/src-tauri/dmg/background.tiff`, drawn by `make-background.swift` next to it, and the window size and icon positions are `bundle.macOS.dmg` in `tauri.conf.json`. To change it, edit the script, run `swift apps/desktop/src-tauri/dmg/make-background.swift` and keep the positions in both files the same.

Finder lays out the window when the `.dmg` is built, so building one on your Mac (`pnpm --filter @kopi/desktop tauri build --bundles dmg`) opens a Finder window for a few seconds. `pnpm build:desktop` only builds the app. Tauri skips the layout on CI unless `TAURI_BUNDLER_DMG_IGNORE_CI` is set, which the workflow does, and the workflow fails if the `.dmg` has no background.

## The install script

[`scripts/install.sh`](../scripts/install.sh) installs the newest release from Terminal: `curl -fsSL https://kopi.devanshuverma.in/install.sh | bash`. The website redirects `/install.sh` to the script on `main`, so a change to it is live once it's merged.

It reads the newest release from the `updater` release's `latest.json`, so it offers a release once the Updater workflow has. It downloads that release's `.dmg`, checks the `.sha256` and checks that the app's designated requirement names the signing certificate, then copies Kopi to `/Applications`. Files curl downloads aren't quarantined, so Gatekeeper doesn't block Kopi.

The script names the certificate by its hash. **With a new certificate, update `requirement` in the script** in the same change, or it refuses every release.

## Trying the workflow without releasing

In **Actions → Release → Run workflow**, run it on a branch. It builds the `.dmg`, the Windows installer and the updates, and keeps them as workflow artifacts for 14 days. It doesn't create a release or a `latest.json`. Use this to test changes to the workflow or a new signing certificate before tagging, or to try a branch on Windows:

1. Run the workflow on the branch, from the Actions page or with `gh workflow run release.yml --ref <branch>`.
2. When it's done, open the run on a Windows PC, signed in to GitHub, and download the `Kopi_<version>-dev.<run>_x64-setup` artifact at the bottom of the page. GitHub only lets signed-in users download artifacts.
3. Unzip it and open the `.exe`. A dev build installs over the release you have, and the next release replaces it.

## Signing

Kopi isn't in the [Apple Developer Program](https://developer.apple.com/programs/), so releases aren't notarized and macOS blocks them the first time a tester opens one. [INSTALL_ALPHA.md](INSTALL_ALPHA.md) shows testers how to open it. [ADR 0006](decisions/0006-macos-distribution-without-notarization.md) explains the decision.

Releases are signed with a **self-signed certificate** instead. macOS ties permissions such as Accessibility to the app's designated requirement. With the certificate, that names the certificate (`certificate leaf = H"…"`), so it's the same for every build and testers keep their permissions across updates. An ad-hoc signature names the build itself, so it changes every time.

The workflow reads the certificate from two repository secrets:

| Secret | What it is |
|---|---|
| `SIGNING_CERTIFICATE` | The certificate and its private key as a `.p12` file, base64-encoded |
| `SIGNING_CERTIFICATE_PASSWORD` | The `.p12` file's password |

Without them, a manual run signs ad hoc and a tag build fails, so every release keeps permissions across updates. The build log and the job summary show the signing and the designated requirement.

### Making the certificate

Do this once. Use the same certificate for every release: a new one changes the designated requirement, and testers have to allow Kopi's permissions once more.

1. Create the certificate and its key, valid for 20 years. Use the `openssl` that comes with macOS (`/usr/bin/openssl`), since its `.p12` files import on every version of macOS.

   ```sh
   cat > kopi-signing.cnf <<'EOF'
   [req]
   distinguished_name = dn
   prompt = no
   x509_extensions = ext
   [dn]
   CN = Kopi Release Signing
   [ext]
   basicConstraints = critical,CA:false
   keyUsage = critical,digitalSignature
   extendedKeyUsage = critical,codeSigning
   subjectKeyIdentifier = hash
   EOF
   /usr/bin/openssl req -x509 -newkey rsa:2048 -nodes -days 7305 \
     -config kopi-signing.cnf -keyout kopi-signing.key -out kopi-signing.crt
   /usr/bin/openssl pkcs12 -export -name "Kopi Release Signing" \
     -inkey kopi-signing.key -in kopi-signing.crt -out kopi-signing.p12
   ```

   The last command asks for a password for the `.p12`.

2. Add the secrets:

   ```sh
   base64 -i kopi-signing.p12 | gh secret set SIGNING_CERTIFICATE
   gh secret set SIGNING_CERTIFICATE_PASSWORD
   ```

3. Keep `kopi-signing.p12` and its password somewhere safe, such as a password manager, then delete the files. Anyone with the key can sign apps that macOS treats as Kopi.
4. Test with a manual run of the workflow.

The first build signed with the certificate has a different designated requirement from the ad-hoc builds before it, so testers allow its permissions once more.

To sign a local build the same way, import `kopi-signing.p12` into your login keychain and build with `APPLE_SIGNING_IDENTITY="Kopi Release Signing"`.

## Updates

Kopi updates itself with the [Tauri updater](https://v2.tauri.app/plugin/updater/). It checks 30 seconds after launch and then every 6 hours, unless the user turns off **Update automatically** in Settings, and whenever they choose **Check for Updates…** in the menu bar. It downloads the update, checks its signature, replaces Kopi.app and offers **Restart to Update**. If the user waits, the update runs the next time Kopi starts. Development builds only check from the menu, and don't install what they find.

On Windows the installer can't run while Kopi does, and installing quits Kopi. So there Kopi only downloads the update by itself, and **Restart to Update** quits Kopi and runs the installer, which opens Kopi again. The update is the installer itself, signed with the same updater key.

- **Where the app looks:** `latest.json` on the [`updater` release](https://github.com/code-devanshu/kopi/releases/tag/updater), set in `tauri.conf.json`. It's a fixed address because GitHub's `releases/latest` skips prereleases, and every alpha is one. The release is a prerelease that's never marked latest, so it doesn't show as the newest Kopi.
- **What moves it:** publishing a release. The Updater workflow copies that release's `latest.json` to the `updater` release. To offer a release again, run **Actions → Updater → Run workflow** with its tag.
- **Which update wins:** a greater version, or the same version with a greater build number (`build` in `latest.json`, the run number). Windows builds carry the same number: the workflow sets `bundle.macOS.bundleVersion` on both, and the app reads it from its built-in config. Kopi never installs an older build, so to undo a bad release, publish a fixed one.
- **What's signed:** the update archive, with the updater key. Kopi checks the signature against the public key in `tauri.conf.json`, and that the signature names the version `latest.json` announces. The app inside is signed with the certificate as usual, so permissions survive the update.
- **What it sends:** two requests to GitHub, for `latest.json` and the update, with no clip data and no ID. See [PRIVACY_DESIGN.md](PRIVACY_DESIGN.md#local-only-by-default).

An update installed this way isn't quarantined, so Gatekeeper doesn't block it the way it blocks a download.

### The updater key

The workflow signs updates with two repository secrets:

| Secret | What it is |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | The updater's private key, as `tauri signer generate` writes it |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | The key's password |

Without them, a manual run builds no update and a tag build fails.

The key was made once with `pnpm --filter @kopi/desktop tauri signer generate -w kopi-updater.key`, and its public key is `plugins.updater.pubkey` in `tauri.conf.json`. Keep the private key and its password in a password manager. **Don't lose it:** every copy of Kopi only accepts updates signed with it. A new key means a release with the new public key that testers download and install by hand once, like the first install.

## What the workflow checks

Before it uploads anything, the workflow checks that:

- the tag matches the app version, and `package.json` and `Cargo.toml` agree
- `Kopi.app` holds both `arm64` and `x86_64` code
- the signature is valid (`codesign --verify --deep --strict`)
- for a signed build, the designated requirement names the certificate

Before it offers a release to the app, the Updater workflow checks that the release is published and that its update downloads without signing in.

## Not yet

- **Signing on Windows.** The installer isn't signed, so SmartScreen warns about it and testers choose **More info → Run anyway**. See [PLATFORM_NOTES.md](PLATFORM_NOTES.md#code-signing-and-smartscreen).
