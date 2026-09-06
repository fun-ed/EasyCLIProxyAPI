#!/usr/bin/env bash
#
# Sync all three CLI Proxy API repositories from upstream and rebuild everything locally.
#
#   CLIProxyAPI                      Go core, pristine upstream, fast-forward only
#   Cli-Proxy-API-Management-Center  official web panel, pristine upstream, built locally
#   EasyCLIProxyAPI                  our fork, feature branch rebased onto upstream
#
# See EasyCLIProxyAPI/FORK.md for the maintenance contract this script implements.

set -euo pipefail

# Resolve through the workspace-root symlink so both entry points behave the same.
SCRIPT_PATH="${BASH_SOURCE[0]}"
while [ -L "$SCRIPT_PATH" ]; do
  SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" && pwd)"
  SCRIPT_PATH="$(readlink "$SCRIPT_PATH")"
  case "$SCRIPT_PATH" in
    /*) ;;
    *) SCRIPT_PATH="$SCRIPT_DIR/$SCRIPT_PATH" ;;
  esac
done
APP_DIR="$(cd "$(dirname "$SCRIPT_PATH")/.." && pwd)"
ROOT_DIR="$(cd "$APP_DIR/.." && pwd)"
CORE_DIR="$ROOT_DIR/CLIProxyAPI"
PANEL_DIR="$ROOT_DIR/Cli-Proxy-API-Management-Center"

FORK_BRANCH="${FORK_BRANCH:-feature/request-archive}"
CORE_INSTALL_DIR="${CPA_CORE_INSTALL_DIR:-$HOME/Library/Application Support/com.cpa.gui/cpa-core}"

DO_SYNC=1
DO_CORE=1
DO_PANEL=1
DO_APP=1
DO_VERIFY=1
DO_PACKAGE=1
DO_CHECK=0

declare -a SUMMARY=()

usage() {
  cat <<'USAGE'
Usage: ./sync-and-build.sh [options]

Options:
  --check         Only report whether upstream has new commits, then exit.
                  Fetches but changes nothing. Exit 0 = up to date, 10 = updates.
  --no-sync       Skip all git fetch/rebase; build what is already checked out.
  --skip-core     Skip the Go core (sync + build).
  --skip-panel    Skip the management panel (sync + build + install).
  --skip-app      Skip the EasyCLIProxyAPI fork (sync + build).
  --no-verify     Skip the fork test suites (tsc, bun test, cargo test).
  --no-package    Run the fork test suites but skip the slow ./build.sh release build.
  -h, --help      Show this help.

Environment:
  FORK_BRANCH            Fork branch to rebase (default: feature/request-archive)
  CPA_CORE_INSTALL_DIR   Where the GUI installed the core
                         (default: ~/Library/Application Support/com.cpa.gui/cpa-core)
USAGE
}

while [ $# -gt 0 ]; do
  case "$1" in
    --check) DO_CHECK=1 ;;
    --no-sync) DO_SYNC=0 ;;
    --skip-core) DO_CORE=0 ;;
    --skip-panel) DO_PANEL=0 ;;
    --skip-app) DO_APP=0 ;;
    --no-verify) DO_VERIFY=0 ;;
    --no-package) DO_PACKAGE=0 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

step() { printf '\n\033[1;34m==> %s\033[0m\n' "$*"; }
info() { printf '    %s\n' "$*"; }
warn() { printf '\033[1;33m    WARN: %s\033[0m\n' "$*"; }
die()  { printf '\n\033[1;31mFAILED: %s\033[0m\n' "$*" >&2; exit 1; }
note() { SUMMARY+=("$1"); }

require_tool() {
  command -v "$1" >/dev/null 2>&1 || die "required tool not found in PATH: $1"
}

require_clean() {
  local dir="$1" name="$2"
  [ -d "$dir/.git" ] || die "$name is not a git checkout: $dir"
  if [ -n "$(git -C "$dir" status --porcelain --untracked-files=no)" ]; then
    git -C "$dir" --no-pager status --short --untracked-files=no
    die "$name has uncommitted changes; commit or stash them before syncing"
  fi
}

# Fast-forward a pristine upstream checkout. Refuses to touch local commits.
sync_pristine() {
  local dir="$1" name="$2" branch="$3"
  step "$name: sync upstream ($branch)"
  require_clean "$dir" "$name"
  git -C "$dir" fetch --quiet origin
  local before after ahead
  before="$(git -C "$dir" rev-parse --short HEAD)"
  ahead="$(git -C "$dir" rev-list --count "origin/$branch..HEAD")"
  if [ "$ahead" != "0" ]; then
    die "$name has $ahead local commit(s) not in origin/$branch; it must stay pristine (FORK.md invariant)"
  fi
  git -C "$dir" checkout --quiet "$branch"
  git -C "$dir" merge --quiet --ff-only "origin/$branch"
  after="$(git -C "$dir" rev-parse --short HEAD)"
  if [ "$before" = "$after" ]; then
    info "already up to date at $after"
    note "$name: unchanged ($after)"
  else
    info "$before -> $after"
    note "$name: updated $before -> $after"
  fi
}

sync_fork() {
  step "EasyCLIProxyAPI: rebase $FORK_BRANCH onto origin/main"
  require_clean "$APP_DIR" "EasyCLIProxyAPI"
  git -C "$APP_DIR" fetch --quiet origin
  git -C "$APP_DIR" checkout --quiet "$FORK_BRANCH"
  local base_before base_after head_before
  base_before="$(git -C "$APP_DIR" merge-base HEAD origin/main)"
  head_before="$(git -C "$APP_DIR" rev-parse --short HEAD)"
  if [ "$base_before" = "$(git -C "$APP_DIR" rev-parse origin/main)" ]; then
    info "already based on the newest origin/main"
    note "EasyCLIProxyAPI: no rebase needed ($head_before)"
    return
  fi
  if ! git -C "$APP_DIR" rebase origin/main; then
    cat >&2 <<EOF

Rebase stopped on a conflict. Resolve it using the patch inventory in
EasyCLIProxyAPI/FORK.md section 3, then:

    git -C "$APP_DIR" add <file>
    git -C "$APP_DIR" rebase --continue
    ./sync-and-build.sh --no-sync

To abandon:  git -C "$APP_DIR" rebase --abort
EOF
    exit 1
  fi
  base_after="$(git -C "$APP_DIR" rev-parse --short origin/main)"
  info "rebased onto $base_after"
  note "EasyCLIProxyAPI: rebased onto $base_after"
}

build_core() {
  step "CLIProxyAPI: build"
  local unformatted
  unformatted="$( cd "$CORE_DIR" && gofmt -l . 2>/dev/null | grep -v '^vendor/' | head -5 || true )"
  if [ -n "$unformatted" ]; then
    while IFS= read -r f; do
      warn "not gofmt-clean: $f"
    done <<<"$unformatted"
  fi
  ( cd "$CORE_DIR" && go build -o /tmp/cpa-core-build-check ./cmd/server )
  rm -f /tmp/cpa-core-build-check
  info "compile check passed"
  note "CLIProxyAPI: build OK"
}

build_panel() {
  step "Cli-Proxy-API-Management-Center: build"
  ( cd "$PANEL_DIR" && bun install --frozen-lockfile >/dev/null && bun run build >/dev/null )
  local built="$PANEL_DIR/dist/index.html"
  [ -f "$built" ] || die "panel build produced no dist/index.html"
  info "built $(wc -c <"$built" | tr -d ' ') bytes"

  if [ ! -d "$CORE_INSTALL_DIR" ]; then
    warn "core install dir not found, skipping panel install: $CORE_INSTALL_DIR"
    note "panel: built but NOT installed (core install dir missing)"
    return
  fi

  local target_dir="$CORE_INSTALL_DIR/static"
  local target="$target_dir/management.html"
  mkdir -p "$target_dir"
  if [ -f "$target" ] && cmp -s "$built" "$target"; then
    info "installed copy already identical"
  else
    cp "$built" "$target"
    info "installed -> $target"
  fi

  pin_panel_auto_update
  note "panel: installed to $target"
}

# The core replaces the asset from GitHub every 3 hours unless this is set.
pin_panel_auto_update() {
  local config="$CORE_INSTALL_DIR/config.yaml"
  if [ ! -f "$config" ]; then
    warn "core config not found, cannot pin the panel: $config"
    return
  fi
  python3 - "$config" <<'PY'
import re
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as handle:
    lines = handle.read().splitlines()

key = "disable-auto-update-panel"
for index, line in enumerate(lines):
    if re.match(r"^\s+%s\s*:" % re.escape(key), line):
        indent = line[: len(line) - len(line.lstrip())]
        desired = "%s%s: true" % (indent, key)
        if line.rstrip() == desired:
            print("    panel auto-update already pinned")
        else:
            lines[index] = desired
            with open(path, "w", encoding="utf-8") as handle:
                handle.write("\n".join(lines) + "\n")
            print("    pinned panel auto-update (updated existing key)")
        sys.exit(0)

for index, line in enumerate(lines):
    if re.match(r"^remote-management\s*:", line):
        lines.insert(index + 1, "  %s: true" % key)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write("\n".join(lines) + "\n")
        print("    pinned panel auto-update (inserted key)")
        sys.exit(0)

print("    WARN: no remote-management block found; panel may be overwritten")
PY
}

verify_app() {
  step "EasyCLIProxyAPI: verify"
  ( cd "$APP_DIR" && bun install >/dev/null )
  ( cd "$APP_DIR" && bun run check )
  info "tsc clean"
  # The i18n suite has a known upstream failure, so report instead of aborting.
  if ( cd "$APP_DIR" && bun test >/tmp/cpa-bun-test.log 2>&1 ); then
    info "bun test: all passed"
  else
    # bun prints "(fail)" twice per failure (inline plus summary), so read the
    # authoritative "N fail" counter instead of counting markers.
    local failed
    failed="$(awk '/^[[:space:]]*[0-9]+ fail$/ { count = $1 } END { print count + 0 }' /tmp/cpa-bun-test.log)"
    if [ "$failed" = "1" ] && grep -q 'keeps both locale dictionaries structurally aligned' /tmp/cpa-bun-test.log; then
      warn "bun test: only the known upstream i18n failure (see FORK.md section 8)"
      note "EasyCLIProxyAPI: bun test had the known i18n failure only"
    else
      tail -40 /tmp/cpa-bun-test.log >&2
      die "bun test failed with $failed failure(s); full log at /tmp/cpa-bun-test.log"
    fi
  fi
  # tests::instance_lock::* shares a fixed directory and occasionally loses a race
  # with OS lock release, so one retry separates that flake from a real failure.
  if ( cd "$APP_DIR/src-tauri" && cargo test --quiet >/tmp/cpa-cargo-test.log 2>&1 ); then
    info "cargo test clean"
  else
    warn "cargo test failed once, retrying to rule out the known instance_lock flake"
    if ( cd "$APP_DIR/src-tauri" && cargo test --quiet >/tmp/cpa-cargo-test.log 2>&1 ); then
      warn "cargo test passed on retry (flake, see FORK.md section 8)"
      note "EasyCLIProxyAPI: cargo test needed a retry"
    else
      tail -40 /tmp/cpa-cargo-test.log >&2
      die "cargo test failed twice; full log at /tmp/cpa-cargo-test.log"
    fi
  fi
  note "EasyCLIProxyAPI: verify OK"
}

package_app() {
  step "EasyCLIProxyAPI: release build"
  ( cd "$APP_DIR" && ./build.sh )
  local out="$APP_DIR/bin-work/EasyCLIProxyAPI-fork"
  [ -x "$out" ] || die "build.sh finished but $out is missing"
  info "built $out"
  note "EasyCLIProxyAPI: packaged $out"
}

# Reports upstream drift without touching any working tree.
check_updates() {
  local pending=0
  for entry in "$CORE_DIR:CLIProxyAPI" \
               "$PANEL_DIR:Cli-Proxy-API-Management-Center" \
               "$APP_DIR:EasyCLIProxyAPI"; do
    local dir="${entry%%:*}" name="${entry##*:}"
    if [ ! -d "$dir/.git" ]; then
      warn "$name: not a git checkout, skipped"
      continue
    fi
    git -C "$dir" fetch --quiet origin
    local behind ahead
    behind="$(git -C "$dir" rev-list --count HEAD..origin/main)"
    ahead="$(git -C "$dir" rev-list --count origin/main..HEAD)"
    if [ "$behind" = "0" ]; then
      printf '    %-32s up to date (%s local commit(s))\n' "$name" "$ahead"
    else
      printf '    \033[1;33m%-32s %s new upstream commit(s)\033[0m\n' "$name" "$behind"
      git -C "$dir" --no-pager log --oneline --max-count=5 "HEAD..origin/main" | sed 's/^/        /'
      pending=1
    fi
  done
  if [ "$pending" = "0" ]; then
    printf '\n\033[1;32mEverything is up to date.\033[0m\n\n'
    return 0
  fi
  printf '\nRun ./sync-and-build.sh to apply and rebuild.\n\n'
  return 10
}

main() {
  require_tool git

  if [ "$DO_CHECK" = 1 ]; then
    step "Checking upstream"
    check_updates
    exit $?
  fi
  if [ "$DO_CORE" = 1 ]; then require_tool go; fi
  if [ "$DO_PANEL" = 1 ] || [ "$DO_APP" = 1 ]; then require_tool bun; fi
  if [ "$DO_APP" = 1 ]; then require_tool cargo; fi
  if [ "$DO_PANEL" = 1 ]; then require_tool python3; fi

  if [ "$DO_SYNC" = 1 ]; then
    if [ "$DO_CORE" = 1 ]; then sync_pristine "$CORE_DIR" "CLIProxyAPI" main; fi
    if [ "$DO_PANEL" = 1 ]; then sync_pristine "$PANEL_DIR" "Cli-Proxy-API-Management-Center" main; fi
    if [ "$DO_APP" = 1 ]; then sync_fork; fi
  else
    step "sync skipped (--no-sync)"
  fi

  if [ "$DO_CORE" = 1 ]; then build_core; fi
  if [ "$DO_PANEL" = 1 ]; then build_panel; fi
  if [ "$DO_APP" = 1 ]; then
    if [ "$DO_VERIFY" = 1 ]; then verify_app; fi
    if [ "$DO_PACKAGE" = 1 ]; then package_app; fi
  fi

  step "Summary"
  for line in "${SUMMARY[@]}"; do
    info "$line"
  done
  printf '\n\033[1;32mDone.\033[0m Run the app with ./EasyCLIProxyAPI/run.sh\n'
  printf 'Emergency rollback of the fork feature: CPA_FORK_ARCHIVE=0 ./EasyCLIProxyAPI/run.sh\n\n'
}

main "$@"
