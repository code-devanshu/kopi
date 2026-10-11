# Contributing to Kopi

Thanks for wanting to help. Kopi is at an early stage, so you can shape it in a big way.

By taking part, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Where help is useful right now

The desktop app hasn't been started yet. For now, the most useful contributions are:

- **Design:** layouts, flows, icons and onboarding for the app. Share mockups in an issue or a discussion.
- **Architecture:** read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and weigh in on the open questions, especially if you know the macOS or Windows clipboard APIs, Tauri, Rust or encryption.
- **The website:** fixes, accessibility and performance improvements in `apps/web`.
- **Docs:** clearer READMEs, typo fixes and better explanations.
- **Testing:** try the website on different browsers and devices and report what's broken. Once alpha builds exist, testers on real machines will matter most.

You don't need to write code to make a real difference.

## Finding something to work on

- Browse issues labelled [`good first issue`](https://github.com/code-devanshu/kopi/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22). They're small and well scoped.
- Issues labelled [`help wanted`](https://github.com/code-devanshu/kopi/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22) are open to anyone.
- Comment on an issue before you start so two people don't work on the same thing.
- For anything big or new, open an issue or a [discussion](https://github.com/code-devanshu/kopi/discussions) first and agree on the approach. That saves you from writing code that might not be merged.

## Setup

You need:

- **Node.js 20.9 or newer.** CI uses Node 24, so that's the safest choice.
- **pnpm.** The version is pinned in the root `package.json`. Run `corepack enable` and the right version is used automatically.

```bash
git clone https://github.com/<your-username>/kopi.git
cd kopi
pnpm install
pnpm dev:web
```

The website runs at [http://localhost:3000](http://localhost:3000). You don't need any environment variables for local development. See [apps/web/README.md](apps/web/README.md) if you want to set them.

Before you open a pull request, run the same checks as CI from the repository root:

```bash
pnpm lint
pnpm build:web
```

## Project structure

```text
kopi/
├── apps/
│   ├── web/          # Website (Next.js, TypeScript, Tailwind CSS)
│   └── desktop/      # Desktop app (Tauri), not started yet
├── packages/         # Shared code, empty for now
├── docs/             # Architecture notes
└── .github/          # Issue templates, PR template and CI
```

This is a pnpm workspace. Install from the root, and run scripts for one package with `pnpm --filter <package-name> <script>`, for example `pnpm --filter @kopi/web lint`.

## Branches

Create a branch from `main` and name it after the kind of change:

| Prefix | Use it for |
| --- | --- |
| `feat/` | New features |
| `fix/` | Bug fixes |
| `docs/` | Documentation only |
| `design/` | Design work and assets |
| `chore/` | Tooling, dependencies and housekeeping |
| `refactor/` | Code changes that don't change behavior |

Use short, lowercase names with hyphens, such as `fix/waitlist-error-message` or `docs/architecture-sync`.

## Commit messages

We use [Conventional Commits](https://www.conventionalcommits.org):

```text
<type>(<optional scope>): <short summary>
```

Common types are `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci` and `chore`. Use the scope to name the area, such as `web`, `desktop` or `docs`.

```text
feat(web): add a link to the roadmap in the nav
fix(web): keep the FAQ open state on resize
docs: explain the clip data model
```

Write the summary in the imperative ("add", not "added"), keep it under about 72 characters and leave off the trailing period.

## Pull requests

1. Fork the repository and create a branch from `main`.
2. Keep each pull request focused on one change. Small pull requests get reviewed faster.
3. Run `pnpm lint` and `pnpm build:web`, and fix any errors.
4. Open the pull request and fill in the template. Link the issue it closes, such as `Closes #12`.
5. Add screenshots or a short recording for anything visual.
6. CI must pass. A maintainer will review, may ask for changes, and will merge when it's ready.

Draft pull requests are welcome if you want early feedback.

## Reporting bugs and requesting features

Use the [issue templates](https://github.com/code-devanshu/kopi/issues/new/choose). Questions and open-ended ideas belong in [Discussions](https://github.com/code-devanshu/kopi/discussions).

Never report a security problem in a public issue. Follow [SECURITY.md](SECURITY.md) instead.

## License

Kopi is released under the [MIT License](LICENSE). By contributing, you agree that your contributions are licensed under the MIT License too.
