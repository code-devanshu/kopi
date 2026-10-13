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

4. Wait for the workflow. It creates a **draft** release with the `.dmg`, its `.sha256`, a note on opening it the first time and notes generated from the merged PRs.
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

## What the workflow checks

Before it uploads anything, the workflow checks that:

- the tag matches the app version, and `package.json` and `Cargo.toml` agree
- `Kopi.app` holds both `arm64` and `x86_64` code
- the signature is valid (`codesign --verify --deep --strict`)
- for a signed build, the designated requirement names the certificate

## Not yet

- **Auto-update.** The [updater plugin](https://v2.tauri.app/plugin/updater/) is [#38](https://github.com/code-devanshu/kopi/issues/38). Until then, testers download each build.
- **Windows.** Windows builds come with the Windows alpha. See [PLATFORM_NOTES.md](PLATFORM_NOTES.md#code-signing-and-smartscreen).
