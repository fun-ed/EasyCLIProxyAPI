# AGENTS.md

This repository is a fork of `router-for-me/EasyCLIProxyAPI`. The working branch
is `feature/request-archive`, rebased onto `origin/main`.

Read [`FORK.md`](FORK.md) before changing code. It records the fork contract,
patch inventory, sync procedure, packaging behavior, and request archive design.
Read [`CHANGELOG.md`](CHANGELOG.md) when investigating a regression that may have
already happened.

## Fast orientation

| Need | Start here |
| --- | --- |
| Fork boundaries and rebase conflicts | `FORK.md` §2 and §3 |
| Upstream sync and release build | `FORK.md` §6 and `scripts/fork-sync-and-build.sh` |
| Request archive behavior and troubleshooting | `FORK.md` §10 |
| Rust archive backend | `src-tauri/src/request_archive.rs` and `src-tauri/src/request_archive/` |
| React archive UI | `src/pages/RequestArchivePanel.tsx` |
| Frontend archive types and Tauri calls | `src/services/requestArchive.ts` |
| Archive translations | `src/i18n/locales/requestArchive.ts` |
| Regression history | `CHANGELOG.md` |
| Bundled core release | `core-version.txt` |

The workspace-root `sync-and-build.sh` is a symlink to
`EasyCLIProxyAPI/scripts/fork-sync-and-build.sh`. Edit the repository file, not
the symlink.

## Before changing code

1. Run `git status --short`. Preserve every existing change.
2. Read the relevant `FORK.md` section and inspect the current tests.
3. Decide whether the target is fork-owned or maintained by upstream.
4. Build a targeted failing test before fixing a request archive regression.

The sync script requires clean tracked worktrees. If upstream sync is part of a
task and this repository is dirty, stash the tracked changes with a descriptive
name, rebase, then restore the stash. Resolve conflicts with `FORK.md` §3. Do not
discard, reset, or overwrite the user's work.

## Fork boundaries

New logic belongs in fork-owned files. Every extra line in an upstream file is a
future rebase conflict.

| Zone | Meaning | Rule |
| --- | --- | --- |
| A | Fork-owned files | Put implementation, tests, translations, styles, and docs here |
| B | Upstream files with fork wiring | Keep only imports, spreads, command registration, and UI mounting |
| C | Product rename | Keep the existing `EasyCLIProxyAPI-fork` substitutions, do not extend them |

Fork-owned files include:

```text
src-tauri/src/request_archive.rs
src-tauri/src/request_archive/
src-tauri/src/tests/request_archive.rs
src/services/requestArchive.ts
src/pages/RequestArchivePanel.tsx
src/styles/requestArchive.css
src/i18n/locales/requestArchive.ts
tests/requestArchive.test.ts
scripts/fork-sync-and-build.sh
FORK.md
CHANGELOG.md
AGENTS.md
CLAUDE.md
```

The stylesheet path is `src/styles/requestArchive.css`. New styles go there,
never in `src/styles.css`. New archive i18n keys go in
`src/i18n/locales/requestArchive.ts`, never in upstream locale dictionaries.
Test fixtures use `.txt`, because `.gitignore` excludes `*.log`.

`src/styles.css` is emitted after the fork stylesheet. Use a compound selector
when overriding an upstream class, such as
`.config-dialog.usage-archive-dialog`.

## Request archive model

The usage event list and request archive use different SQLite databases:

```text
usage-records/usage.db
  usage_events.request_id
          |
          v
request-records/requests.db
  request_records.request_id
          ^
          |
oauth/logs/v1-*.log
```

Clicking an event row looks up `request_records` by `request_id`. A listed event
can therefore have no detail if its transcript has not been archived.

The ingester runs every five seconds. `scan_once()`:

1. reads transcript candidates from the core log directory;
2. removes files whose stored `source_fingerprint` still matches;
3. sorts the remaining new or changed files by modification time;
4. processes at most 200 files;
5. applies archive retention and the fork log-directory cap.

Do not move the 200-file limit before fingerprint filtering. That caused the
2026-09-07 starvation bug: the same 200 archived files occupied every batch, so
newer transcripts existed on disk but never reached `requests.db`.

## Missing request detail playbook

First confirm the app uses the real profile. A bundled `.app` uses
`~/Library/Application Support/com.cpa.gui`; `bin-work/EasyCLIProxyAPI-fork`
uses its own blank profile.

Then check the three archive gates:

1. archive setting `enabled=true`;
2. core `request-log=true`;
3. core `commercial-mode=false`.

Use metadata only unless request contents are explicitly needed. The archive
contains sensitive headers and bodies.

```bash
BASE="$HOME/Library/Application Support/com.cpa.gui"
sqlite3 "$BASE/request-records/requests.db" \
  "SELECT key,value FROM archive_metadata ORDER BY key;
   SELECT COUNT(*),MAX(captured_at) FROM request_records;"
sqlite3 "$BASE/usage-records/usage.db" \
  "SELECT timestamp,request_id,model FROM usage_events
   ORDER BY timestamp_ms DESC LIMIT 20;"
find "$BASE/oauth/logs" -maxdepth 1 -type f -name 'v1-*.log' | wc -l
```

If a usage event has a request ID and the matching log exists, but the archive's
latest timestamp is stale, run:

```bash
cd src-tauri
cargo test scan_reaches_new_logs_after_the_first_batch_is_already_archived
```

Never delete `requests.db`, `usage.db`, or source logs while diagnosing.

## Commands

Run from `EasyCLIProxyAPI/` unless noted:

```bash
bun install
bun run check
bun test
bun test tests/requestArchive.test.ts tests/uiLocalization.test.ts
cd src-tauri && RUSTUP_TOOLCHAIN=1.90.0-aarch64-apple-darwin cargo test request_archive
```

Run the complete workflow from the workspace root:

```bash
export RUSTUP_TOOLCHAIN=1.90.0-aarch64-apple-darwin
./sync-and-build.sh --check
./sync-and-build.sh --app
./sync-and-build.sh --app-only
./sync-and-build.sh --no-sync --app
```

`--app` syncs all repositories, verifies the fork, builds the portable binary,
and produces both:

```text
EasyCLIProxyAPI/src-tauri/target/release/bundle/macos/EasyCLIProxyAPI-fork.app
EasyCLIProxyAPI/src-tauri/target/release/bundle/dmg/EasyCLIProxyAPI-fork_<version>_<arch>.dmg
```

`--app-only` skips git, the core, and the management panel. Use it only when
those are already current. `./build.sh` produces the portable build with a
separate blank profile.

On this Apple Silicon macOS workstation, use Rust 1.90.0 for Rust-backed commands. Rust 1.98.1 reproducibly crashes rustc with SIGBUS while compiling `cpa-gui`, including after `cargo clean`.

For local Rust tests on macOS, use `TMPDIR=/private/tmp` to avoid the default `/var/folders/...` path; it otherwise causes 66 `agents::backups` failures because the validator rejects `/var` as a symlink. One separate known test cleanup failure remains: `linked_configuration_and_backup_directories_are_rejected` uses `fs::remove_dir()` on a symlink and receives `ENOTDIR`.

Current upstream passes `bun test`. `cargo test` may hit the known
`tests::instance_lock::*` timing flake; rerun it once before calling it a
regression. Do not claim a command passed unless it ran successfully.

## Hard boundaries

- Keep `CLIProxyAPI/` pristine. Never patch the core from this fork task.
- Never commit credentials, request bodies, archive databases, or captured logs.
- Do not delete a user's database or source logs without explicit approval.
- Do not add archive code to upstream-owned files when a fork-owned file can hold it.
- Emergency rollback: start with `CPA_FORK_ARCHIVE=0`.

<!-- OPENWIKI:START -->

## OpenWiki

This repository has a generated `openwiki/` evidence index. It is optional just-in-time context, not required startup reading.

- Treat source code and tests as authoritative. A brief's unknowns and review items are verification gaps, not automatic requirements.
- Prefer the narrowest quiet validation that proves the changed behavior. Preserve complete failure output.

The scheduled OpenWiki GitHub Actions workflow refreshes the repository wiki. Do not hand-edit generated OpenWiki pages unless explicitly asked; prefer updating source code/docs and letting OpenWiki regenerate.

<!-- OPENWIKI:END -->
