# Sync, documentation, and macOS release runbook

Use this runbook when updating the three CLI Proxy API repositories, rebasing the `feature/request-archive` fork, refreshing release documentation, and producing the portable binary, `.app`, and DMG.

Run commands from the workspace root unless a command names a repository with `git -C`.

## Scope

The workspace contains three independent repositories:

| Directory | Role | Sync rule |
| --- | --- | --- |
| `CLIProxyAPI/` | Proxy core | Keep pristine. Fast-forward `main` only. |
| `Cli-Proxy-API-Management-Center/` | Management panel | Keep pristine. Fast-forward `main` only. |
| `EasyCLIProxyAPI/` | Desktop fork | Rebase `feature/request-archive` onto `origin/main`. |

`EasyCLIProxyAPI/core-version.txt` controls the core archive bundled with the desktop application. Do not replace it with the standalone core checkout's tag unless upstream EasyCLIProxyAPI also ships the matching archive.

## Prerequisites

Use the working Rust toolchain and a real temporary directory on Apple Silicon macOS:

```bash
export RUSTUP_TOOLCHAIN=1.90.0-aarch64-apple-darwin
export TMPDIR=/private/tmp
```

Rust 1.98.1 crashes while compiling `cpa-gui` on this workstation. The default macOS temporary path also makes the backup-path validator reject `/var` symlinks.

## 1. Inspect and protect local work

Check every repository before moving a branch:

```bash
git -C CLIProxyAPI status --short --branch
git -C Cli-Proxy-API-Management-Center status --short --branch
git -C EasyCLIProxyAPI status --short --branch
```

The core and panel must be clean. Preserve tracked desktop changes with a named stash when the fork needs a rebase:

```bash
git -C EasyCLIProxyAPI stash push -m 'sync-update-release temporary local changes'
```

Do not reset or discard another person's work. `AGENTS.md` may contain local OpenWiki output. Keep it out of commits unless the task explicitly changes its maintained instructions.

## 2. Sync upstream

The normal command performs all three sync operations, builds the core and panel, runs verification, packages the portable application, and creates the `.app` and DMG:

```bash
./sync-and-build.sh --app
```

If the fork rebase conflicts, resolve each conflict before continuing. For locale conflicts, retain both the upstream entries and the fork's `requestArchive` spread. Then run:

```bash
git -C EasyCLIProxyAPI add <resolved-files>
git -C EasyCLIProxyAPI rebase --continue
```

After the rebase completes, restore the temporary stash:

```bash
git -C EasyCLIProxyAPI stash pop
```

The sync script finds the workspace root through either the root symlink or `EasyCLIProxyAPI/scripts/fork-sync-and-build.sh`. Run its `--check` mode before a release when only upstream drift is needed:

```bash
./sync-and-build.sh --check
```

## 3. Handle the known macOS Rust test result

With `TMPDIR=/private/tmp`, `cargo test` should avoid the large `/var/folders` failure set. One known cleanup test can still fail on macOS:

```text
linked_configuration_and_backup_directories_are_rejected
```

That test calls `fs::remove_dir()` on a symlink and macOS returns `ENOTDIR`. It does not indicate an application build failure.

If the normal `--app` command has completed synchronization and stops only on that documented test, finish the release without resyncing or repeating verification:

```bash
TMPDIR=/private/tmp ./sync-and-build.sh --no-sync --app --no-verify
```

Any other test failure requires investigation before packaging.

## 4. Verify release artifacts

The script checks these outputs before it exits successfully:

```text
EasyCLIProxyAPI/bin-work/EasyCLIProxyAPI-fork
EasyCLIProxyAPI/src-tauri/target/release/bundle/macos/EasyCLIProxyAPI-fork.app
EasyCLIProxyAPI/src-tauri/target/release/bundle/dmg/EasyCLIProxyAPI-fork_<version>_aarch64.dmg
```

Launch the `.app` when testing with the real profile:

```bash
open EasyCLIProxyAPI/src-tauri/target/release/bundle/macos/EasyCLIProxyAPI-fork.app
```

The portable binary uses its own blank profile. It is not suitable for checking existing credentials, usage records, or request archives.

## 5. Refresh Markdown after a successful sync

Read facts from the checked-out repositories. Do not infer a bundled-core version from the standalone core tag.

```bash
git -C CLIProxyAPI rev-parse --short HEAD
git -C CLIProxyAPI describe --tags --always
git -C Cli-Proxy-API-Management-Center rev-parse --short HEAD
git -C Cli-Proxy-API-Management-Center describe --tags --always
git -C EasyCLIProxyAPI rev-parse --short origin/main
git -C EasyCLIProxyAPI describe --tags --always origin/main
cat EasyCLIProxyAPI/core-version.txt
```

Update these maintained records:

- `EasyCLIProxyAPI/FORK.md`: current upstream base, current GUI Cargo version, standalone core checkout, bundled core pin, sync outcome, and verification result.
- `EasyCLIProxyAPI/CHANGELOG.md`: dated entry for the rebase, core and panel updates, script behavior changes, built artifacts, and any verification exception.
- `EasyCLIProxyAPI/README.fork.md`: commands and platform-specific build guidance when they changed.
- `.github/copilot-instructions.md`: workspace layout and the current bundled-core pin.

OpenWiki files are generated reference material. Regenerate them with their source tool when it is available, or state that the generated documentation remains out of scope. Do not edit fixture provenance merely because it names the core version that produced the fixture.

## 6. Record the result

Before reporting completion, run:

```bash
git -C EasyCLIProxyAPI diff --check
git -C CLIProxyAPI status --short --branch
git -C Cli-Proxy-API-Management-Center status --short --branch
git -C EasyCLIProxyAPI status --short --branch
```

Report the three revision identifiers, the bundled core version, the portable binary path, the `.app` path, the DMG path, and any known verification exception. State which Markdown files changed. Preserve unrelated local changes.
