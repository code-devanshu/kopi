# Installing the Kopi alpha

Thanks for trying Kopi. This guide covers installing the alpha on a Mac, allowing the permissions it needs, updating it and removing it. On a Windows PC, skip to [Windows](#windows). If something goes wrong, [open an issue](https://github.com/code-devanshu/kopi/issues/new/choose).

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

Click **Download for macOS** on [kopi.devanshuverma.in](https://kopi.devanshuverma.in). It downloads `Kopi_<version>_universal.dmg`, the newest release. Every release's files are also on the [releases page](https://github.com/code-devanshu/kopi/releases).

To check the download isn't damaged, also download its `.sha256` file from the releases page and run this in Terminal, in the folder that has both:

```sh
shasum -a 256 -c Kopi_*_universal.dmg.sha256
```

It should print `OK`.

## 2. Install

1. Open the `.dmg`.
2. Drag **Kopi** onto **Applications**. The window also shows the steps for opening it the first time.
3. Eject the disk image.

Keep Kopi in Applications.

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

Kopi checks for updates every few hours and downloads them in the background. When one is ready, **Restart to Update** appears at the top of Kopi's menu bar menu. If you don't restart, choosing **Quit** installs the update, and it runs the next time you open Kopi. Choose **Check for Updates…** in the menu bar to check now.

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
4. If launch at login was on, remove Kopi from System Settings → General → Login Items. Versions up to 0.1.0-alpha.4 used `~/Library/LaunchAgents/Kopi.plist` instead, so delete that if it's there.
5. To clear Kopi from the permission lists, run `tccutil reset All in.devanshuverma.kopi` in Terminal.

## Windows

Kopi for Windows is new, so expect rough edges, and please report them. It needs **Windows 10 or 11** on a 64-bit PC.

### Install

1. Click **Download for Windows** on [kopi.devanshuverma.in](https://kopi.devanshuverma.in). It downloads `Kopi_<version>_x64-setup.exe`, the newest release. Every release's files are also on the [releases page](https://github.com/code-devanshu/kopi/releases).
2. Open it. The installer isn't signed yet, so Windows may say **Windows protected your PC**. Choose **More info**, then **Run anyway**.
3. The installer puts Kopi in your user account, so it doesn't ask for an administrator. When it's done, Kopi starts. Its icon is in the notification area, at the right of the taskbar. If you don't see it, it may be behind the **^** arrow.

To check the download isn't damaged, also download its `.sha256` file from the releases page and run this in PowerShell, in the folder that has both. It should print `True`.

```powershell
(Get-FileHash Kopi_*_x64-setup.exe).Hash -eq (Get-Content Kopi_*_x64-setup.exe.sha256).Split(' ')[0]
```

### Use it

Press **Ctrl+Shift+V** in any app to open Kopi. Ctrl does what ⌘ does on a Mac: **Ctrl+1** to **Ctrl+9** paste the first nine clips, **Ctrl+P** pins a clip and **Ctrl+K** opens its menu. Windows doesn't ask you to allow anything, unless you turn on [sync](#sync).

- Browsers and some other apps use Ctrl+Shift+V to paste without formatting. Kopi takes that shortcut while it runs, so if you use it, choose another in Kopi's Settings.
- Windows doesn't let Kopi paste into apps running as administrator. Kopi still copies the clip, so press Ctrl+V.

### Update

Kopi checks for updates and downloads them by itself. When one is ready, **Restart to Update** appears in Kopi's menu. Choosing it closes Kopi, installs the update and opens Kopi again.

### Remove

1. Open **Settings → Apps → Installed apps**, find **Kopi** and choose **Uninstall**.
2. To delete your clip history and settings too, delete the folder `%LOCALAPPDATA%\in.devanshuverma.kopi`.

## Sync

Kopi can sync your history, pins and pinboards between your Macs and Windows PCs, directly over your local network. It's off until you turn it on, and clips never go through a server.

1. On each computer, open Kopi's **Settings → Sync** and turn on **Sync with your other computers**.
   - **On a Mac**, macOS asks whether Kopi may find devices on your local network. Choose **Allow**. If you chose **Don't Allow**, Settings shows a button that opens **System Settings → Privacy & Security → Local Network**, where you can turn Kopi on.
   - **On Windows**, Windows Defender Firewall asks whether Kopi may communicate on networks. Allow it on **private networks**. On a standard account this needs an administrator's password. If you cancelled, open **Windows Security → Firewall & network protection → Allow an app through firewall** and tick Kopi for **Private**.
2. On both computers, choose **Add a computer…**. Each shows a 6-digit code and lists the other.
3. On one of them, pick the other computer and type the code it shows. Both say **Paired**, and your clips start syncing.

To add a third computer, pair it with any one of the others. It finds the rest by itself.

Both computers must be on the same network with Kopi running. A computer that was away catches up when it's back. If they can't find each other:

- On Windows, the network must be set to **Private**: **Settings → Network & internet**, choose the network, then **Private network**.
- Guest Wi-Fi and many office networks keep computers from seeing each other. Sync can't work there.
- Check that Kopi is allowed in the Mac's Local Network settings and in the Windows firewall, as above.

File clips don't sync, since a file's path means nothing on another computer. **What to sync** in Settings chooses the other kinds. Deleting a clip, or clearing history, deletes it on every paired computer. To stop syncing with a computer, choose **Remove…** next to it; it keeps the clips it already has.

macOS may ask about the local network again after an update. If sync stops finding your other computers after one, check Settings → Sync, and [let us know](#reporting-a-problem): it tells us whether every update will ask.

## Reporting a problem

[Open an issue](https://github.com/code-devanshu/kopi/issues/new/choose) with:

- the Kopi version from the release you downloaded
- your macOS version and whether your Mac has Apple silicon or Intel, or your Windows version
- what you did, what you expected and what happened

Don't paste clips with personal data into an issue. To report a security problem, follow [SECURITY.md](../SECURITY.md) instead.
