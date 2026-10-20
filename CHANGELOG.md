# Changelog

This file records fork-specific behavior changes and regressions. Upstream
release notes remain in the upstream repository history.

## 2026-10-20

### Changed

- Rebased all 28 fork commits onto upstream `aee871a`, tagged `v0.3.47`, producing fork HEAD `2b65086`. Fast-forwarded the pristine core to `3f0cef01` (`v8.0.35-3-g3f0cef01`) and the management panel to `fe29633` (`v1.26.5`). GUI Cargo version remains `0.3.6`; the source core pin is `8.0.34`.
- Preserved upstream's shared usage types, Select import, and native window decorations. Moved the archive's `request_id` field into `src/services/usage.ts`. Restored all four pre-existing modified files and retained the recovery branch and stash.
- Go compilation and both frontend production builds passed. With Bun 1.3.14, panel tests passed all 1,635 cases; GUI tests passed 926, skipped 32, and failed 4. All four failures reproduced in a clean upstream snapshot: Copilot localization, two quota-priority assertions, and the WindowFrame alert-role check. Rust 1.90.0 tests passed 949, ignored 9, and failed only the documented macOS symlink cleanup test. No test allowlists changed.
- The initial sync stopped before packaging. Follow-up release packaging produced the arm64 `.app` and `EasyCLIProxyAPI-fork_0.3.6_upstream-0.3.47_20261020_aarch64.dmg` with Rust 1.90.0 and Bun 1.3.14. The App was ad-hoc signed after Tauri left only a linker signature; strict codesign verification, DMG checksum verification, and a mounted-DMG comparison against the built App passed. This is not an Apple-notarized release. No new portable bundle was produced.
- Browser verification was unavailable because Firefox MCP exited during startup and the Vite preview did not become ready. The 50 focused archive and shared-control integration tests passed.
- Committed the archive adaptation and maintenance records as `193bd8b`, then published [fork-v0.3.47-20261020](https://github.com/fun-ed/EasyCLIProxyAPI/releases/tag/fork-v0.3.47-20261020). The release tag points to that source commit. The downloaded DMG passed its uploaded SHA-256 checksum. With explicit approval, the rebased branch was pushed using an exact `--force-with-lease`; the previous remote tip remains on `backup/pre-sync-20261020-6375f31`.
- Installed the verified App into `/Applications/EasyCLIProxyAPI-fork.app`, retaining the prior App as `EasyCLIProxyAPI-fork.backup-20261020-163812.app`. Installed files match the signed build. After relaunch, the App and existing core remained running, the root endpoint returned HTTP 200, and the running core reported `8.0.35`. The DMG contains the GUI only, not an offline core archive; its source pin does not establish the running core version. Existing profile archives produced a non-fatal offline-selection warning and were left untouched.
- Project-local continuation notes are in Serena `core`, `easycliproxyapi/core`, and `easycliproxyapi/release-workflow`. The screened evidence source is KB-relative `projects/cliproxy-api/session-notes/2026-10-20-sync-release-install-serena.md`; its receipt sidecar records Cognee/Hindsight completion and source hashes. These external notes are not bundled with the App.

### Fixed

- Adapted the archive UI to upstream's shared Dialog, Switch, Tabs, surface, and button contracts. Removed superseded private control styling, routed core operations through services, and added isolated empty-archive browser mock state with regression coverage.

## 2026-10-15

### Changed

- Rebased all 28 fork commits onto upstream `d64dda6`, tagged `v0.3.38`, producing fork HEAD `7216771`. Fast-forwarded the pristine core to `5fc580b1`, tagged `v8.0.29`, and the management panel to `13f94b4`, tagged `v1.26.4`. GUI Cargo version remains `0.3.6`; the bundled core pin is `8.0.29`.
- Kept upstream's expandable error rows, error-copy action, and sidebar layout while retaining the fork's archive-detail action, kill switch, and translated product name. Restored the pre-existing local documentation and archive test fixture changes; kept the recovery branch and stash.
- Go compilation, panel build, and TypeScript checking passed. Bun tests passed 827 cases, skipped 32, and failed only the Copilot status localization check, which also failed in a clean upstream snapshot. Rust tests passed 881 cases, ignored 8, and failed only the previously recorded local `agy` path and macOS symlink cleanup cases. Their source files are unchanged from upstream. The test failure allowlist was not expanded.
- Built the arm64 portable binary, `.app`, and `EasyCLIProxyAPI-fork_0.3.6_aarch64.dmg` with Rust 1.90.0 and Bun 1.3.14 after separate verification. `hdiutil verify` passed; the portable core archive is `CLIProxyAPI_8.0.29_darwin_aarch64.tar.gz`. No remote push, app launch with the real profile, or `/Applications` installation was performed.
- Built the management panel locally without installing it into the running profile or changing its configuration.

### Fixed

- Changed the archive danger-button hover background to the existing `--danger-color` role so it passes upstream's new theme-background check.

## 2026-10-07

### Changed

- Rebased 28 fork commits onto upstream `aba62fa`, tagged `v0.3.26`. Fast-forwarded the pristine core to `57bde351`, tagged `v8.0.17`, and the management panel to `6abace9`, tagged `v1.25.4`. GUI Cargo version remains `0.3.6`; the bundled core pin is `8.0.16`.
- Resolved event-list conflicts without dropping upstream row-height settings, export controls, or date grouping. Retained the fork's archive-detail action and `CPA_FORK_ARCHIVE` gate.
- Go compilation, panel build and installation, and TypeScript checking passed. Bun tests passed 723 cases, skipped 32, and failed 3; all three failures reproduced in a clean upstream worktree. Rust tests passed 747 cases, ignored 6, and failed 2 due to the local `agy` path and the documented macOS symlink cleanup case. All 22 archive tests passed after the fixture correction below.
- Built the portable binary, `.app`, and `EasyCLIProxyAPI-fork_0.3.6_aarch64.dmg` with Rust 1.90.0 and Bun 1.3.14. Packaging used `--no-verify` after separate verification; the script's failure allowlist remains unchanged. No remote push or `/Applications` installation was performed.

### Fixed

- Disabled age-based retention in three archive tests that exercise scanning or size limits. Their fixed September 6 transcripts had exceeded the default 30-day retention window. The dedicated age-retention test still checks expiration.

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
