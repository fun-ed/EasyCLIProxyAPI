# Changelog

This file records fork-specific behavior changes and regressions. Upstream
release notes remain in the upstream repository history.

## 2026-10-01

### Changed

- Rebased all 25 fork commits onto upstream `90364e9` (v0.3.12) without conflicts. Fast-forwarded the pristine core to `97f244b8` (v8.0.7) and the management panel to `a7ec312` (v1.25.1). GUI Cargo version remains `0.3.6`; the bundled core pin is now `8.0.6`.
- Preserved and restored pre-existing local documentation and script changes. Go compilation, panel build and installation, TypeScript checking, and Bun tests passed. Rust tests passed 678 cases with 2 ignored; only the documented macOS symlink cleanup test failed.
- Built the portable binary, `.app`, and `EasyCLIProxyAPI-fork_0.3.6_aarch64.dmg` with Rust 1.90.0. The portable package includes the core `8.0.6` archive. Packaging skipped repeated verification after the documented Rust test failure.

### Fixed

- Updated the archive log-directory resolver to propagate errors from upstream's now-fallible `auth_dir_path_for_core` API instead of calling `join` on a `Result`.

## 2026-09-27

### Changed

- Rebased `feature/request-archive` onto upstream `cb7fbbe` (v0.3.7 release-notes follow-up), fast-forwarded the pristine core to `4a2c8186` (v8.0.2), and kept the management panel at `4530da2` (v1.24.2). The GUI source is now `0.3.6` and bundles core `8.0.2`.
- Kept upstream's cross-platform tray guard, usage tab panel container, and explicit model-pricing source selection while retaining the fork's archive entry point. Fixed panel pinning to update the v8 `management` block instead of inserting invalid YAML after a legacy flow mapping; repaired and validated the installed configuration.
- Built the portable binary, `.app`, and `EasyCLIProxyAPI-fork_0.3.6_aarch64.dmg`. Go compilation, the panel build, TypeScript checking, and Bun tests passed. The full Rust suite passed 635 of 636 tests; the documented macOS symlink cleanup test still fails, so packaging used `--no-verify`.

## 2026-09-25

### Changed

- Rebased `feature/request-archive` onto upstream `6c64fc0` (v0.3.2 release-notes follow-up), fast-forwarded the pristine core to `c404af96` (v7.3.16), and kept the already-current management panel at `4530da2` (v1.24.2). The GUI source remains `0.2.97` and now bundles core `7.3.12`.
- Resolved two event-list rebase conflicts by retaining upstream's unique `row_id` key and the fork's request-archive row action and kill switch. Restored pre-existing uncommitted documentation and script changes after rebasing.
- Built the portable binary, `.app`, and `EasyCLIProxyAPI-fork_0.2.97_aarch64.dmg`. Go compilation and logging tests, the panel build, TypeScript checking, Bun tests, and 22 focused request-archive Rust tests passed. The full Rust suite passed 607 of 608 tests; the documented macOS symlink cleanup test still fails, so packaging used `--no-verify`.

## 2026-09-21

### Changed

- Rebased `feature/request-archive` onto upstream `7b72d97` (tag `v0.2.101`), fast-forwarded the pristine core to `a5ab6952` (tag `v7.3.10`), and updated the management panel to `bbac79d` (tag `v1.24.1`).
- The desktop source remains `cpa-gui` `0.2.97`; upstream currently pins the bundled core at `7.3.9`. The standalone core checkout is newer at `7.3.10`.
- Updated `scripts/fork-sync-and-build.sh` so both the workspace-root symlink and the repository script locate the workspace root before syncing or building.
- Built the portable binary, `.app`, and `EasyCLIProxyAPI-fork_0.2.97_aarch64.dmg`. Go compilation, the management-panel build, TypeScript checking, and Bun tests passed. macOS Rust tests still stop at the documented symlink cleanup test, so the release build used `--no-verify`.

## 2026-09-09

### Changed

- Rebased the GUI fork onto upstream `5548324`, updated the pristine core to
  `7fac6b15`, and updated the management panel to `ed5f1c4`. The resulting GUI
  version is `0.2.80` and the bundled core version is `7.2.154`.

## 2026-09-07

### Fixed

- Fixed request archive scan starvation after changing archive or log-directory
  limits. The usage event list continued to show new requests and the matching
  transcripts existed under `oauth/logs`, but request detail returned "not
  found".
- The scanner used to sort every transcript by age, truncate the list to 200
  files, and only then ask whether each file was already archived. Once the log
  directory held more than 200 retained transcripts, the same unchanged files
  occupied every scan and newer files were never processed.
- `scan_once()` now removes files whose `source_file` and
  `source_fingerprint` already match the archive before sorting and applying the
  200-file batch limit. Changed files are still reingested.
- Added
  `scan_reaches_new_logs_after_the_first_batch_is_already_archived`, which
  creates one full archived batch plus a newer transcript and requires the next
  scan to ingest it.
- Archive settings now submit the form's current values, refresh status after
  saving, and report success only when every stored value matches the submitted
  payload.
- Routed the fork product name and archive HTTP status through i18n so the
  upstream UI localization boundary test remains green.

### Changed

- Rebased the GUI fork onto upstream `75fac19` and updated the pristine core to
  `934fb792`. The resulting GUI version is `0.2.76` and the bundled core version
  is `7.2.152`.
- `scripts/fork-sync-and-build.sh --app` now treats the DMG as a required build
  output. It checks that the file exists, prints its path, and records it in the
  final Summary.
- Updated `AGENTS.md`, `CLAUDE.md`, and `FORK.md` so a new agent can locate the
  archive pipeline, reproduce missing-detail failures, sync upstream safely,
  and build the installable DMG without reconstructing the workflow from git
  history.
