# Changelog

This file records fork-specific behavior changes and regressions. Upstream
release notes remain in the upstream repository history.

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
