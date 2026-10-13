# Installing the Kopi alpha

Thanks for trying Kopi. This guide covers installing the alpha on a Mac, allowing the permissions it needs, updating it and removing it. If something goes wrong, [open an issue](https://github.com/code-devanshu/kopi/issues/new/choose).

The alpha needs **macOS 14 Sonoma or later**, on Apple silicon or Intel. There's one download for both.

## The quick way: install from Terminal

Open Terminal and run:

```sh
curl -fsSL https://kopi.devanshuverma.in/install.sh | bash
```

It downloads the newest release, checks it isn't damaged and that it's signed with Kopi's certificate, copies Kopi to Applications and opens it. If Kopi is already running, it quits it first. It may ask for your password if your account can't write to Applications.

macOS doesn't block Kopi installed this way, because it only checks apps downloaded by a browser or another app that marks them. Skip to [4. Set it up](#4-set-it-up). To read the script before you run it, it's [scripts/install.sh](../scripts/install.sh).

Or install it by hand, in steps 1 to 3.

## 1. Download

Download `Kopi_<version>_universal.dmg` from the [releases page](https://github.com/code-devanshu/kopi/releases).

To check the download isn't damaged, also download the `.sha256` file next to it and run this in Terminal, in the folder that has both:

```sh
shasum -a 256 -c Kopi_*_universal.dmg.sha256
```

It should print `OK`.

## 2. Install

1. Open the `.dmg`.
2. Drag **Kopi** onto **Applications**. The window also shows the steps for opening it the first time.
3. Eject the disk image.

Keep Kopi in Applications. Launch at login points to where the app is, so it stops working if you move the app.

## 3. Open it the first time

Kopi isn't notarized by Apple, so macOS blocks it the first time. Kopi is open source and free, and notarization needs a paid Apple developer account.

- **macOS 15 Sequoia and later:**
  1. Open Kopi from Applications. macOS says "Kopi" wasn't opened. Click **Done**.
  2. Open **System Settings → Privacy & Security** and scroll down to **Security**. It says "Kopi" was blocked.
  3. Click **Open Anyway**, then enter your password.
  4. Click **Open Anyway** again in the dialog that follows.
- **macOS 14 Sonoma:** Control-click Kopi in Applications, choose **Open**, then click **Open**.

You do this once. Kopi installs its updates itself, and macOS doesn't block those.

## 4. Set it up

Kopi lives in the menu bar. It has no Dock icon. The first time it opens, a setup window walks you through the steps below. If you close it, open it again with **Set Up Kopi…** in Kopi's menu bar menu. Kopi's **Settings** show the same permissions.

### Accessibility

Lets Kopi paste into the app you're using. Without it, Kopi copies the clip and you press ⌘V yourself.

1. In the setup window, click **Allow…**. macOS shows its own request.
2. Click **Open System Settings**, then turn on **Kopi** under **Privacy & Security → Accessibility**.

The setup window shows **Allowed** as soon as it's on. Kopi doesn't need to restart.

### Paste from Other Apps

Lets Kopi see what you copy. Newer versions of macOS ask for this, and the setup window skips it on versions that don't.

1. Click **Open System Settings** next to **Paste from Other Apps**.
2. Set **Kopi** to **Always Allow**.

If you choose **Ask** instead, macOS asks every time you copy something, so Kopi can't keep your history.

### The shortcut

Press **⇧⌘V** in any app to open Kopi. You can choose another shortcut in the setup window or in Settings, and turn on launch at login there too.

## Updating

Kopi checks for updates every few hours and installs them in the background. When one is ready, **Restart to Update** appears at the top of Kopi's menu bar menu. If you don't restart, the update runs the next time Kopi starts. Choose **Check for Updates…** in the menu bar to check now.

To stop Kopi checking by itself, turn off **Update automatically** in Settings. **Check for Updates…** still works. Updates come from this project's GitHub releases, and Kopi sends nothing about you or your clips to check for them.

If you install your first build from before the updater, or want to update by hand, run the [install command](#the-quick-way-install-from-terminal) again. Or:

1. Quit Kopi from its menu bar menu.
2. Install the new build as in steps 1 to 3, and replace the old one in Applications.

Kopi keeps its permissions across updates. If Kopi stops pasting or stops saving what you copy after an update, even though System Settings shows it as allowed:

1. In **System Settings → Privacy & Security → Accessibility**, select **Kopi** and click **−** to remove it.
2. Open **Set Up Kopi…** from the menu bar and allow it again.

Or reset all of Kopi's permissions at once in Terminal, then allow them again:

```sh
tccutil reset All in.devanshuverma.kopi
```

You'll need to do this once if you used a build from before the first alpha release.

## Removing Kopi

1. Quit Kopi from its menu bar menu.
2. Delete Kopi from Applications.
3. To delete your clip history and settings too, delete `~/Library/Application Support/in.devanshuverma.kopi`.
4. If launch at login was on, delete `~/Library/LaunchAgents/Kopi.plist`.
5. To clear Kopi from the permission lists, run `tccutil reset All in.devanshuverma.kopi` in Terminal.

## Reporting a problem

[Open an issue](https://github.com/code-devanshu/kopi/issues/new/choose) with:

- the Kopi version from the release you downloaded
- your macOS version and whether your Mac has Apple silicon or Intel
- what you did, what you expected and what happened

Don't paste clips with personal data into an issue. To report a security problem, follow [SECURITY.md](../SECURITY.md) instead.
