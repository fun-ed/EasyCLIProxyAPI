# AGENTS.md

Guidance for coding agents working in this repository.

> **This is a fork.** Read [`FORK.md`](FORK.md) before changing anything. It is the
> single source of truth for what this fork adds, why, and how to keep it
> mergeable with upstream `router-for-me/EasyCLIProxyAPI`.

## Before you start

1. Read `FORK.md` §2 (architecture zones) and §3 (patch inventory).
2. Run `git status --short`. Never start work on a dirty tree.
3. The working branch is `feature/request-archive`, rebased onto `origin/main`.

## The one rule that matters

**New code goes in new files.** Every line added to a file that upstream also
maintains becomes a rebase conflict forever. The fork currently touches only ~68
lines across upstream files; keep it that way.

| Zone | Meaning | Rule |
| --- | --- | --- |
| A | Fork-owned new files | Put new logic here |
| B | Upstream files with fork edits | Only unavoidable wiring |
| C | Rename to `EasyCLIProxyAPI-fork` | Do not extend |

Fork-owned files: `src-tauri/src/request_archive.rs`, `src-tauri/src/request_archive/`,
`src-tauri/src/tests/request_archive.rs`, `src/services/requestArchive.ts`,
`src/pages/RequestArchivePanel.tsx`, `src/i18n/locales/requestArchive.ts`,
`src/styles/requestArchive.css`, `tests/requestArchive.test.ts`,
`scripts/fork-sync-and-build.sh`, `FORK.md`, `AGENTS.md`.

Practical consequences:

- New i18n keys go in `src/i18n/locales/requestArchive.ts`, never in `locales/zh-CN.ts`.
- New styles go in `src/styles/requestArchive.css`, never in `src/styles.css`.
- New test fixtures use `.txt`, never `.log` (`.gitignore` excludes `*.log`).

## CSS specificity trap

`src/styles.css` is emitted *after* the fork stylesheet in the bundle, so at equal
specificity upstream wins on source order. When overriding an upstream class, use
a compound selector, as `.config-dialog.usage-archive-dialog` does.

## Commands

```bash
bun install
bun run check                      # tsc --noEmit
bun test                           # one known upstream i18n failure is expected
bun test tests/requestArchive.test.ts
cd src-tauri && cargo test

bun tauri build                    # .app bundle: shares the real profile
./build.sh                         # portable: keeps its OWN blank profile
```

Full sync and build in one command, from the workspace root:

```bash
./sync-and-build.sh --check        # report upstream drift only
./sync-and-build.sh --app          # sync, build, verify, package, bundle .app
./sync-and-build.sh --app-only     # just rebuild the .app, about a minute
```

## Verification expectations

- `bun test` fails exactly one i18n assertion. That gap predates the fork; confirm
  the missing-key count is still 50 rather than trying to fix it.
- `cargo test` occasionally loses a race in `tests::instance_lock::*`. Re-run once
  before treating it as a regression.
- Do not claim a check passed without running it.

## Hard boundaries

- `CLIProxyAPI/` (the Go core) stays at pristine upstream state. Never patch it.
- Never commit credentials. The archive database stores request bodies and headers;
  treat it as sensitive.
- Do not delete a user's archive database or source logs without being asked.

## Feature context

The fork adds a full request archive: it parses the core's `request-log`
transcripts and stores them in a separate SQLite database at
`<core base dir>/request-records/requests.db`. See `FORK.md` §9 for the schema,
the three switches required to make it produce data, and the `commercial-mode`
interaction that silently disables request logging entirely.

Emergency rollback without reverting commits: `CPA_FORK_ARCHIVE=0`.

<!-- OPENWIKI:START -->

## OpenWiki

This repository has a generated `openwiki/` evidence index. It is optional just-in-time context, not required startup reading.

- Treat source code and tests as authoritative. A brief's unknowns and review items are verification gaps, not automatic requirements.
- Prefer the narrowest quiet validation that proves the changed behavior. Preserve complete failure output.

The scheduled OpenWiki GitHub Actions workflow refreshes the repository wiki. Do not hand-edit generated OpenWiki pages unless explicitly asked; prefer updating source code/docs and letting OpenWiki regenerate.

<!-- OPENWIKI:END -->
