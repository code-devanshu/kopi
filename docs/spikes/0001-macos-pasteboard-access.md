# Spike 0001: macOS pasteboard access

- **Issue:** [#4](https://github.com/code-devanshu/kopi/issues/4)
- **Date:** 2026-10-11 to 2026-10-12
- **Code:** [spikes/macos-pasteboard-access](../../spikes/macos-pasteboard-access/README.md)
- **Feeds into:** [ADR 0003](../decisions/0003-clipboard-watching-approach.md), [PLATFORM_NOTES.md](../PLATFORM_NOTES.md#pasteboard-privacy-affects-the-whole-app)

## Question

Which `NSPasteboard` calls trigger the "Paste from Other Apps" privacy alert, what does a read return when the alert isn't answered, how long does an Allow last, and can the user grant access for good?

## Setup

| | |
| --- | --- |
| macOS | 26.6.2 (25G83) for rounds 1–3, then the same Mac upgraded to 27.0.1 (26A434) for round 4 |
| App | `in.devanshuverma.kopi.spike` 0.1.0 (1), universal, ad-hoc signed, not sandboxed |
| CDHash | `2eb502de…` for rounds 1–3. Round 4 rebuilt the spike twice (`4a99ca0d…`, then `ad9f1614…`). |
| Threading | Every call runs on the main thread and is timed. From round 4, a watchdog on its own queue logs every 5 s while the main thread is stuck. |

`EnablePasteboardPrivacyDeveloperPreview` is a per-app default that turns on the alert model before macOS enforces it. "Flag on" below means `defaults write in.devanshuverma.kopi.spike EnablePasteboardPrivacyDeveloperPreview -bool yes`, then a relaunch.

The tests:

| Test | Calls |
| --- | --- |
| A | `changeCount` every 0.5 s for 60 s |
| B | `types`, then `pasteboardItems.count` |
| C | `string(forType: .string)` |
| D | `data(forType: .png)`, falling back to `.tiff` |
| E | `types`, then the string only if no concealed, transient or auto-generated marker is present |

## Results

### Round 1: macOS 26.6.2, flag off

No alert for any call. `accessBehavior` stayed `alwaysAllow`, and every call took 0.2–3.5 ms. Every read returned content, and E skipped the concealed and transient samples without reading them.

### Round 2: macOS 26.6.2, flag on

`accessBehavior` was `default` at launch.

| Call | Alert? | Time | Notes |
| --- | --- | --- | --- |
| `changeCount` (A) | No | ~0.3 ms | 120 polls. Still `default` afterwards. |
| `types` (B, E) | No | 0.2–6.3 ms | |
| `pasteboardItems.count` (B) | No | 0.4 ms | |
| `string` / `data`, first read of a copy (C, D, E) | **Yes** | 1.4–11.5 s | The time is how long the user took to click. |
| Any read of a copy that was already allowed | No | 0.1–5.6 ms | Any type, not only the one first requested. |

The alert reads "“KopiSpike” is trying to access the pasteboard." Its buttons are **Allow Paste** (the default) and **Don't Allow Paste**.

What we saw in detail:

- **The first content read moves the app from `default` to `ask`.** It stays `ask` after Allow. The alert doesn't offer "always allow".
- **An Allow lasts for one copy.** At changeCount 64, C prompted and took 1,449.6 ms. Three seconds later, C on the same changeCount took 0.1 ms with no alert. Every new copy (changeCount 56, 59, 60, 61, 64, 65) prompted again.
- **The grant covers every type on that copy.** On the second D run, the user allowed, `data(forType: .png)` returned nil because the copy had no PNG, and `data(forType: .tiff)` returned 697,176 bytes in 5.6 ms with no second alert.
- **An unanswered read gives up after about 60 s.** The first C read blocked for 59,985.8 ms and returned nil. **The alert stayed on screen after the read gave up**, and answering it then didn't help that read. The spike's window was frozen for that whole minute.
- **The markers work without an alert.** E checked `types`, found `org.nspasteboard.ConcealedType` or `org.nspasteboard.TransientType`, and skipped the read. No alert, and the content was never touched.
- **One copy can bump `changeCount` twice.** A single copy in Chrome went 51 → 52 → 53.
- **Chrome adds `org.chromium.source-url`** (and `org.chromium.internal.source-rfh-token`) to text copies. That's a usable source hint.

### Round 3: System Settings grant (macOS 26.6.2, then 27.0.1)

- After the round 2 alerts, KopiSpike appeared under **System Settings → Privacy & Security** with three options: **Ask**, **Always Allow** and **Deny**.
- With **Always Allow** selected and the flag still on, the next launch read `alwaysAllow` instead of `default`.
- **The grant survived the upgrade to macOS 27.0.1 and a reboot.** Launches on 27.0.1 with the same CDHash still read `alwaysAllow`.
- **No content reads were logged under the grant on 26.6.2**, so "Always Allow means no alerts at all" is implied by `accessBehavior` but not observed.

### Round 4: macOS 27.0.1

**Flag on: pasteboard calls hang the app, with no alert.**

- C blocked the main thread for over 5 minutes on the first try. After the watchdog was added, C blocked for 79 s before the app was killed, and a launch-time `accessBehavior` call once blocked for 104 s.
- No alert appeared at any point.
- **A reboot didn't help.** After restarting, C hung again the same way.
- This happened even though `accessBehavior` read `alwaysAllow` at launch, so the round 3 grant didn't protect against it.
- The system log shows why. AppKit tries to connect to the Trusted UI service that shows the alert. The connection fails with `(ipc/send) invalid destination port`, meaning the service isn't registered, followed by `[com.apple.AppKit:TrustedUI] Error establishing Trusted UI connection.: NSCocoaErrorDomain (4099)`. The main thread then waits in `-[HIRunLoopSemaphore wait]` with no time limit. On 26.6.2 the same wait gave up after about 60 s.

**Calls can't be moved off the main thread.** Calling `accessBehavior` from a background queue raised `NSInternalInconsistencyException` in `-[HIRunLoopSemaphore wait:]`, and the app aborted (SIGABRT). Only `accessBehavior` was tried off the main thread. Content reads go through the same wait, so we expect the same crash.

**Flag off: nothing is enforced.**

- `accessBehavior` read `alwaysAllow`, and C returned in 5.3 ms with no alert.
- **The System Settings choice is ignored.** With KopiSpike set to **Ask** in System Settings, `accessBehavior` still read `alwaysAllow` before and after C, and C returned 22 characters in 14.9 ms with no alert.

**Grant and new builds (unconfirmed).** The rebuilt spike (CDHash `ad9f1614…`, never granted directly) launched with the flag on after the reboot and read `alwaysAllow`. That hints the grant isn't tied to the CDHash. But the same build's launch hung at 04:58 before the reboot, so the flag-on path on 27.0.1 is too unreliable to count this as proof.

## Answers

| Question | Answer |
| --- | --- |
| Does `changeCount` prompt? | **No.** |
| Does `types` prompt? | **No.** Nor does `pasteboardItems.count`. |
| Does reading content prompt? | **Yes**, once per copy, for the first `string`/`data` call (26.6.2, flag on). |
| Can Kopi skip passwords without an alert? | **Yes**, for apps that set the nspasteboard.org markers, because the check needs only `types`. |
| What does a denied or unanswered read return? | On 26.6.2: `nil`, after up to ~60 s, with the alert possibly still on screen. On 27.0.1 with the flag on: nothing. The call never returns. |
| Does clicking Allow in the alert persist? | **Only for that copy.** `accessBehavior` stays `ask`. |
| Can the user grant access for good? | **Yes.** System Settings → Privacy & Security offers Ask, Always Allow and Deny. Always Allow sets `alwaysAllow`, and it survived an OS upgrade and a reboot. |
| Is paste privacy enforced on 27.0.1? | **No.** Without the flag, `accessBehavior` is `alwaysAllow` and reads work even when System Settings says Ask. |
| Can pasteboard calls run off the main thread? | **No** on 27.0.1. `accessBehavior` off the main thread aborts the app. |

## What this means for Kopi

1. **On shipping macOS (26.x and 27.0.1), Kopi works without any permission.** Nothing is enforced, and no alert appears.
2. **Once enforcement arrives (reportedly 27.2), "Always Allow" is a required permission.** Without it, Kopi would show one alert per copy, which is unusable for a clipboard manager. Onboarding sends the user to System Settings → Privacy & Security and explains why.
3. **The read order in ADR 0003 is right and costs nothing.** Poll `changeCount`, then read `types`, then run the privacy filter. None of these prompt, so Kopi can detect copies and skip sensitive ones without ever triggering an alert.
4. **A content read can block the main thread, and moving it to a background thread doesn't work.** Under enforcement, a read blocks while the alert is up (60 s on 26.6.2). Under the 27.0.1 preview it blocked forever. Kopi's main thread is also Tauri's event loop, so a stuck read freezes the whole app. Kopi must not read content unless `accessBehavior` is `alwaysAllow`. If that turns out not to be enough on enforced macOS, the fallback is to do pasteboard reads in a small helper process that can be killed and restarted. That helper would need its own grant.
5. **Check `accessBehavior` before reading, on the main thread.** If it's `default`, `ask` or `alwaysDeny`, don't read in the background. Show the "Kopi can't see your clipboard" state and point to System Settings. Today `accessBehavior` reads `alwaysAllow` regardless of the setting, so this check only starts to matter once enforcement is on.
6. **Treat `nil` as "denied or not answered", not "empty".** Don't retry in a loop: each retry can raise another alert.
7. **Pick the format from `types` before reading.** Only the first read prompts, so reading several formats from one copy costs at most one alert.
8. **Dedupe `changeCount` bumps** that come close together, and keep ignoring Kopi's own writes (`org.nspasteboard.source`).
9. **Use `org.chromium.source-url`** where present to show which page a clip came from.
10. **Don't use the developer-preview flag** to test Kopi on 27.0.1. It hangs pasteboard calls instead of showing the alert. Test enforcement on a real 27.2 build.

## Not tested

- **Real macOS 27.2.** The enforced behaviour may differ from both the 26.6.2 preview and the broken 27.0.1 preview. **Most important follow-up**, before Kopi's macOS onboarding is built.
- **Reads under an Always Allow grant with enforcement working.** On 26.6.2 only the launch state was checked. On 27.0.1 the flag-on reads hung.
- **How long a grant survives an update.** A `BUILD=2` update and a Developer ID-signed build were never tried. The ad-hoc rebuild result above is unconfirmed.
- **Don't Allow Paste, and the Deny setting.** Never tried with enforcement working.
- **Reset.** Whether `tccutil reset Pasteboard in.devanshuverma.kopi.spike` returns the app to `default`.
- **Content reads off the main thread.** Only `accessBehavior` was tried, and only on 27.0.1.
- **`detectedPatterns(for:)` / `detectedMetadata(for:)`.** Apple says they don't notify the user. They could let Kopi classify a copy without reading it.
- **A real password manager.** The concealed and transient samples were written by the spike itself. 1Password, Bitwarden and others should be checked for which markers they set.
