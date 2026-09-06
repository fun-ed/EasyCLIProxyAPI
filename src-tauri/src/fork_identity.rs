//! Identifies this build as a fork and blocks upstream self-updates.
//!
//! The app checks `router-for-me/EasyCLIProxyAPI/releases` on every start and
//! offers a one-click update whenever its own version is older. A fork inherits
//! upstream's version number through rebasing, so it always looks out of date
//! the moment upstream publishes — and accepting the offer would download the
//! official build and overwrite every fork change with no way back.
//!
//! Rather than freezing or inflating the version, which would hide which
//! upstream base the fork sits on, the update path is refused outright.

/// Bundle identifier suffix that marks a fork build.
///
/// Set in `tauri.conf.json`, so no separate build flag has to be kept in sync.
const FORK_IDENTIFIER_SUFFIX: &str = ".fork";

pub(crate) fn is_fork_identifier(identifier: &str) -> bool {
    identifier.trim().ends_with(FORK_IDENTIFIER_SUFFIX)
}

pub(crate) fn is_fork_build(app: &tauri::AppHandle) -> bool {
    is_fork_identifier(&app.config().identifier)
}

/// Explains, in the field upstream already renders, why updating is unavailable.
pub(crate) fn self_update_blocked_reason() -> String {
    "这是 fork 版本，已停用应用内更新：更新会用官方版覆盖 fork 的全部改动。\
     请改用 ./sync-and-build.sh --app 从源码重建。"
        .to_string()
}

#[cfg(test)]
pub(crate) mod testing {
    pub(crate) use super::{is_fork_identifier, self_update_blocked_reason};
}
