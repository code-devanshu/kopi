<div align="center">

<img src="apps/web/src/app/icon.svg" alt="Kopi logo" width="88" height="88" />

# Kopi

**Everything you copy, on every computer.**

An open-source clipboard manager for macOS and Windows, coming soon.<br />
<a href="https://kopi.devanshuverma.in"><strong>kopi.devanshuverma.in</strong></a>

<p>
  <img alt="Status: coming soon" src="https://img.shields.io/badge/status-coming%20soon-d08a3a?style=flat-square" />
  <img alt="Platforms: macOS and Windows" src="https://img.shields.io/badge/platforms-macOS%20%C2%B7%20Windows-2b1d14?style=flat-square" />
  <a href="https://github.com/code-devanshu/kopi/issues"><img alt="Open issues" src="https://img.shields.io/github/issues/code-devanshu/kopi?style=flat-square&color=93561a" /></a>
</p>

<img src=".github/assets/preview.png" alt="Kopi preview card: Everything you copy, on every computer. Open-source clipboard manager for macOS and Windows." width="820" />

</div>

<br />

> [!NOTE]
> The desktop app is still in design and will live in `apps/desktop`. For now this repo holds the website, which has a waitlist for the first alphas.

## What Kopi will do

- **Unlimited clipboard history** for text, links, images, colors and code.
- **Instant search**, filtered by type or source app.
- **Pinboards** for the snippets you reuse.
- **Mac ↔ Windows sync**, optional and end-to-end encrypted.
- **Paste as plain text** and **keyboard-first** controls.

History stays on your own computer, works offline and needs no account. Copies from password managers are skipped.

## Repository structure

```text
kopi/
├── apps/
│   └── web/              # Website (Next.js): kopi.devanshuverma.in
├── packages/             # Shared packages (empty for now)
├── package.json          # Workspace scripts
└── pnpm-workspace.yaml
```

The desktop app (`apps/desktop`) is coming soon.

## Run the website locally

You need Node.js 20.9 or newer and pnpm. If you don't have pnpm, run `corepack enable`.

```bash
git clone https://github.com/code-devanshu/kopi.git
cd kopi
pnpm install
pnpm dev:web
```

Then open [http://localhost:3000](http://localhost:3000). See [apps/web/README.md](apps/web/README.md) for environment variables and website details.

| Command | What it does |
| --- | --- |
| `pnpm dev:web` | Start the website's dev server |
| `pnpm build:web` | Build the website |
| `pnpm build` | Build every package |
| `pnpm lint` | Lint every package |

## Contributing

Kopi is at an early stage, which makes it a good time to get involved. Developers, designers and testers are all welcome.

1. [Open an issue](https://github.com/code-devanshu/kopi/issues/new) to share an idea or report a problem.
2. For code changes, fork the repo, create a branch and open a pull request.
3. Run `pnpm lint` and `pnpm build` before you submit.

## License

A license hasn't been chosen yet. Until a `LICENSE` file is added, all rights are reserved by default.
