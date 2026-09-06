//! Tests for the fork build guard.

use crate::fork_identity::testing::{is_fork_identifier, self_update_blocked_reason};

#[test]
fn only_the_fork_bundle_identifier_disables_self_update() {
    // tauri.conf.json ships com.cpa.gui.fork, upstream ships com.cpa.gui.
    assert!(is_fork_identifier("com.cpa.gui.fork"));
    assert!(is_fork_identifier("  com.cpa.gui.fork  "));

    assert!(!is_fork_identifier("com.cpa.gui"));
    assert!(
        !is_fork_identifier("com.cpa.gui.forked"),
        "a longer suffix must not be mistaken for the fork marker"
    );
    assert!(!is_fork_identifier("fork.com.cpa.gui"));
    assert!(!is_fork_identifier(""));
}

#[test]
fn the_blocked_reason_points_at_the_rebuild_command() {
    let reason = self_update_blocked_reason();
    assert!(
        reason.contains("sync-and-build.sh"),
        "the message must tell the reader how to update instead, got {reason:?}"
    );
}
