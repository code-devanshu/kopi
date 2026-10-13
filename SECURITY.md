# Security policy

A clipboard manager sees almost everything you copy: passwords, tokens, addresses and private messages. We treat security reports as a top priority and will put them ahead of feature work.

## Reporting a vulnerability

**Please don't report security problems in public issues, discussions or pull requests.**

Report them privately instead, in either of these ways:

- Email **<code.devanshu@gmail.com>** with "Kopi security" in the subject.
- Use GitHub's [private vulnerability reporting](https://github.com/code-devanshu/kopi/security/advisories/new).

Please include:

- What the problem is and what an attacker could do with it
- Steps to reproduce it, or a proof of concept
- The affected part of the project (website, desktop app or sync) and the version or commit
- Your operating system, if it's relevant

## What happens next

- We'll confirm we've received your report within **3 working days**.
- We'll look into it, keep you updated, and let you know whether we can confirm the problem.
- Once a fix is ready, we'll release it and publish an advisory. We'll credit you unless you'd rather stay anonymous.

Please give us a reasonable chance to fix the problem before you disclose it publicly.

## Scope

Kopi is in alpha on macOS. Reports about the desktop app in `apps/desktop` and `crates/`, its releases and updates, and the website at [kopi.devanshuverma.in](https://kopi.devanshuverma.in) are all welcome.

We're most interested in issues that could:

- Expose clipboard history to other apps, other users or the network
- Capture copies that should be skipped, such as from password managers
- Let an attacker run code through content that gets copied or pasted
- Get Kopi to install an update that this project didn't sign
- Weaken the encryption or key handling used by sync, once it exists

## Supported versions

Until v1.0, only the newest release and the latest code on `main` get security fixes. Kopi updates itself, so a fix reaches testers in the next release.
