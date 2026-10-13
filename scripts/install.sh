#!/bin/bash
# Installs Kopi on macOS, or updates it to the newest release:
#
#   curl -fsSL https://kopi.devanshuverma.in/install.sh | bash
#
# It downloads the newest release's .dmg, checks its SHA-256 and that the app is signed with
# Kopi's certificate, copies Kopi to /Applications and opens it. Files curl downloads aren't
# quarantined, so macOS doesn't block Kopi the first time, as it does a .dmg from a browser.
# Set KOPI_RELEASE to install another release, such as KOPI_RELEASE=0.1.0-alpha.2.
#
# See docs/INSTALL_ALPHA.md.

# Everything runs from main, at the end, so a download cut short runs nothing.
set -euo pipefail

repository=https://github.com/code-devanshu/kopi
# The app's designated requirement: its bundle ID and Kopi's signing certificate. A Kopi.app
# signed with anything else isn't installed. See docs/RELEASING.md#signing.
requirement='identifier "in.devanshuverma.kopi" and certificate leaf = H"3483010475485455fab4e2ed1a0f434b7b969a3c"'
destination=/Applications/Kopi.app
work=
admin=false

# Runs a command, with sudo if /Applications needs it.
as_admin() {
  if [ "$admin" = true ]; then sudo "$@"; else "$@"; fi
}
say() { printf '\033[1m==>\033[0m %s\n' "$*"; }
fail() {
  printf '\033[31mError:\033[0m %s\n' "$*" >&2
  exit 1
}

main() {
  [ "$(uname -s)" = Darwin ] || fail "Kopi's installer is for macOS. Windows is next."
  local major
  major=$(sw_vers -productVersion | cut -d. -f1)
  [ "$major" -ge 14 ] || fail "Kopi needs macOS 14 Sonoma or later. This Mac has $(sw_vers -productVersion)."

  # The newest release, prereleases included, from the file the app checks for updates.
  local release=${KOPI_RELEASE:-}
  if [ -z "$release" ]; then
    release=$(curl -fsSL "$repository/releases/download/updater/latest.json" \
      | plutil -extract release raw -o - -) || fail "Couldn't find the newest release."
  fi

  work=$(mktemp -d)
  trap 'hdiutil detach "$work/volume" -quiet 2> /dev/null || true; rm -rf "$work"' EXIT

  local dmg=Kopi_${release}_universal.dmg
  say "Downloading Kopi $release"
  curl -fL --progress-bar -o "$work/$dmg" "$repository/releases/download/v$release/$dmg" \
    || fail "Couldn't download $dmg."
  curl -fsSL -o "$work/$dmg.sha256" "$repository/releases/download/v$release/$dmg.sha256" \
    || fail "Couldn't download $dmg.sha256."
  (cd "$work" && shasum -a 256 -c "$dmg.sha256" > /dev/null) \
    || fail "The download is damaged. Run the installer again."

  say "Checking the signature"
  hdiutil attach "$work/$dmg" -quiet -readonly -nobrowse -noautoopen -mountpoint "$work/volume" \
    || fail "Couldn't open $dmg."
  codesign --verify --deep --strict -R="$requirement" "$work/volume/Kopi.app" 2> /dev/null \
    || fail "Kopi.app in $dmg isn't signed with Kopi's certificate, so it wasn't installed."

  # Quits Kopi, if it's running, so it can be replaced.
  if pgrep -xq kopi-desktop; then
    say "Quitting Kopi"
    pkill -x kopi-desktop || true
    local waited=0
    while pgrep -xq kopi-desktop && [ "$waited" -lt 10 ]; do
      sleep 1
      waited=$((waited + 1))
    done
  fi

  # Most Macs let the user write to /Applications. If not, ask for an administrator's password.
  if [ ! -w /Applications ] || { [ -e "$destination" ] && [ ! -w "$destination" ]; }; then
    say "Your password is needed to install Kopi in /Applications"
    admin=true
  fi

  say "Installing Kopi in /Applications"
  as_admin rm -rf "$destination"
  as_admin ditto "$work/volume/Kopi.app" "$destination"
  as_admin xattr -dr com.apple.quarantine "$destination" 2> /dev/null || true

  hdiutil detach "$work/volume" -quiet || true
  open "$destination"

  say "Kopi $release is installed and open in the menu bar."
  echo "    Press ⇧⌘V in any app to open it. It installs its updates itself."
  echo "    Setting it up: $repository/blob/main/docs/INSTALL_ALPHA.md#4-set-it-up"
}

main "$@"
