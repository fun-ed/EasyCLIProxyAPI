mod parser;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Local, TimeZone};
use rusqlite::{params, params_from_iter, types::Value as SqlValue, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tokio_util::sync::CancellationToken;

use super::{auth_dir_path_for_core, core_base_dir, core_install_dir, GuiConfigState};
use parser::{endpoint_from_url, header_value, parse_request_log, request_id_from_filename};

const ARCHIVE_DIR_NAME: &str = "request-records";
const ARCHIVE_DATABASE_FILE: &str = "requests.db";
const ARCHIVE_UPDATED_EVENT: &str = "request-archive-updated";
const SQLITE_BUSY_TIMEOUT_SECONDS: u64 = 30;
const SCAN_INTERVAL_SECONDS: u64 = 5;
/// A log file is only ingested once the core has stopped writing to it.
const WRITE_SETTLE_SECONDS: u64 = 2;
const MAX_FILES_PER_SCAN: usize = 200;

const DEFAULT_RETENTION_DAYS: u32 = 30;
const DEFAULT_MAX_TOTAL_MB: u32 = 5_120;
const DEFAULT_MAX_BODY_KB: u32 = 1_024;

const SETTING_ENABLED: &str = "enabled";
const SETTING_RETENTION_DAYS: &str = "retention_days";
const SETTING_MAX_TOTAL_MB: &str = "max_total_mb";
const SETTING_MAX_BODY_KB: &str = "max_body_kb";

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestArchiveSettings {
    pub enabled: bool,
    pub retention_days: u32,
    pub max_total_mb: u32,
    pub max_body_kb: u32,
}

impl Default for RequestArchiveSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            retention_days: DEFAULT_RETENTION_DAYS,
            max_total_mb: DEFAULT_MAX_TOTAL_MB,
            max_body_kb: DEFAULT_MAX_BODY_KB,
        }
    }
}

impl RequestArchiveSettings {
    fn normalized(mut self) -> Self {
        // 0 means "no limit" for retention and size; the body cap always keeps a floor
        // so a single oversized request cannot exhaust the disk.
        self.retention_days = self.retention_days.min(3_650);
        self.max_total_mb = self.max_total_mb.min(1_024 * 1_024);
        self.max_body_kb = self.max_body_kb.clamp(16, 1_024 * 64);
        self
    }
}

/// Kill switch for every fork-local request archive behaviour.
///
/// Setting `CPA_FORK_ARCHIVE` to `0`, `false`, `off` or `no` stops the ingester and
/// hides the UI, which restores upstream behaviour without reverting any commit.
pub(crate) const FORK_ARCHIVE_ENV: &str = "CPA_FORK_ARCHIVE";

pub(crate) fn fork_archive_disabled_in(value: Option<&str>) -> bool {
    matches!(
        value.map(|raw| raw.trim().to_ascii_lowercase()).as_deref(),
        Some("0") | Some("false") | Some("off") | Some("no")
    )
}

pub(crate) fn fork_archive_disabled() -> bool {
    fork_archive_disabled_in(std::env::var(FORK_ARCHIVE_ENV).ok().as_deref())
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestArchiveStatus {
    pub fork_disabled: bool,
    pub settings_enabled: bool,
    pub retention_days: u32,
    pub max_total_mb: u32,
    pub max_body_kb: u32,
    pub request_log_enabled: bool,
    pub record_count: i64,
    pub database_bytes: i64,
    pub database_path: String,
    pub logs_directory: String,
    pub last_ingested_at: String,
    pub last_error: String,
    pub oldest_record_at: String,
    pub newest_record_at: String,
}

#[derive(Default)]
struct RequestArchiveRuntime {
    token: Option<CancellationToken>,
    last_ingested_at: String,
    last_error: String,
}

#[derive(Default)]
pub(crate) struct RequestArchiveState {
    runtime: Mutex<RequestArchiveRuntime>,
}

impl RequestArchiveState {
    fn start(&self) -> Option<CancellationToken> {
        let mut runtime = self.runtime.lock().ok()?;
        if runtime.token.is_some() {
            return None;
        }
        let token = CancellationToken::new();
        runtime.token = Some(token.clone());
        Some(token)
    }

    fn stop(&self) {
        if let Ok(mut runtime) = self.runtime.lock() {
            if let Some(token) = runtime.token.take() {
                token.cancel();
            }
        }
    }

    fn record_success(&self, timestamp: String) {
        if let Ok(mut runtime) = self.runtime.lock() {
            runtime.last_ingested_at = timestamp;
            runtime.last_error.clear();
        }
    }

    fn record_error(&self, error: String) {
        if let Ok(mut runtime) = self.runtime.lock() {
            runtime.last_error = error;
        }
    }

    fn snapshot(&self) -> (String, String) {
        match self.runtime.lock() {
            Ok(runtime) => (runtime.last_ingested_at.clone(), runtime.last_error.clone()),
            Err(_) => (String::new(), String::new()),
        }
    }
}

fn archive_root_dir() -> Result<PathBuf, String> {
    Ok(core_base_dir()?.join(ARCHIVE_DIR_NAME))
}

fn archive_database_path() -> Result<PathBuf, String> {
    Ok(archive_root_dir()?.join(ARCHIVE_DATABASE_FILE))
}

/// The core resolves its log directory to `<cwd>/logs` when that path is
/// writable and falls back to `<auth-dir>/logs` otherwise, so both candidates
/// are scanned. The core is always spawned with the install directory as cwd.
pub(crate) fn core_logs_directories(auth_dir: &str) -> Result<Vec<PathBuf>, String> {
    let install_dir = core_install_dir()?;
    let mut candidates = vec![
        auth_dir_path_for_core(auth_dir, &install_dir).join("logs"),
        install_dir.join("logs"),
    ];
    candidates.dedup();
    Ok(candidates)
}

fn primary_logs_directory(auth_dir: &str) -> Result<PathBuf, String> {
    let candidates = core_logs_directories(auth_dir)?;
    Ok(candidates
        .iter()
        .find(|path| path.is_dir())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone()))
}

fn open_archive_database_at(root: &Path) -> Result<Connection, String> {
    fs::create_dir_all(root).map_err(|error| format!("创建请求归档目录失败: {error}"))?;
    let path = root.join(ARCHIVE_DATABASE_FILE);
    let connection = Connection::open(&path)
        .map_err(|error| format!("打开 SQLite 请求归档数据库失败 {}: {error}", path.display()))?;
    connection
        .busy_timeout(Duration::from_secs(SQLITE_BUSY_TIMEOUT_SECONDS))
        .map_err(|error| format!("设置 SQLite busy timeout 失败: {error}"))?;
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|error| format!("启用 SQLite WAL 模式失败: {error}"))?;
    connection
        .pragma_update(None, "synchronous", "NORMAL")
        .map_err(|error| format!("设置 SQLite synchronous 模式失败: {error}"))?;
    initialize_archive_schema(&connection)?;
    Ok(connection)
}

fn open_archive_database() -> Result<Connection, String> {
    open_archive_database_at(&archive_root_dir()?)
}

fn initialize_archive_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS archive_metadata (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS request_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                request_id TEXT NOT NULL DEFAULT '',
                source_file TEXT NOT NULL,
                source_fingerprint TEXT NOT NULL DEFAULT '',
                captured_at TEXT NOT NULL DEFAULT '',
                captured_at_ms INTEGER NOT NULL DEFAULT 0,
                core_version TEXT NOT NULL DEFAULT '',
                url TEXT NOT NULL DEFAULT '',
                endpoint TEXT NOT NULL DEFAULT '',
                method TEXT NOT NULL DEFAULT '',
                downstream_transport TEXT NOT NULL DEFAULT '',
                upstream_transport TEXT NOT NULL DEFAULT '',
                model TEXT NOT NULL DEFAULT '',
                stream INTEGER NOT NULL DEFAULT 0,
                http_status INTEGER NOT NULL DEFAULT 0,
                api_error_status INTEGER NOT NULL DEFAULT 0,
                failed INTEGER NOT NULL DEFAULT 0,
                system_prompt TEXT NOT NULL DEFAULT '',
                messages_json TEXT NOT NULL DEFAULT '',
                tools_json TEXT NOT NULL DEFAULT '',
                usage_json TEXT NOT NULL DEFAULT '',
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                reasoning_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                request_headers_json TEXT NOT NULL DEFAULT '',
                response_headers_json TEXT NOT NULL DEFAULT '',
                request_body TEXT NOT NULL DEFAULT '',
                api_request TEXT NOT NULL DEFAULT '',
                api_response TEXT NOT NULL DEFAULT '',
                response_body TEXT NOT NULL DEFAULT '',
                api_error_text TEXT NOT NULL DEFAULT '',
                websocket_timeline TEXT NOT NULL DEFAULT '',
                truncated INTEGER NOT NULL DEFAULT 0,
                byte_size INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_request_records_source_file
                ON request_records(source_file);
            CREATE INDEX IF NOT EXISTS idx_request_records_captured
                ON request_records(captured_at_ms DESC, id DESC);
            CREATE INDEX IF NOT EXISTS idx_request_records_request_id
                ON request_records(request_id);
            CREATE INDEX IF NOT EXISTS idx_request_records_model
                ON request_records(model, captured_at_ms DESC);
            CREATE INDEX IF NOT EXISTS idx_request_records_failed
                ON request_records(failed, captured_at_ms DESC);
            "#,
        )
        .map_err(|error| format!("初始化 SQLite 请求归档结构失败: {error}"))
}

fn read_setting(connection: &Connection, key: &str) -> Option<String> {
    connection
        .query_row(
            "SELECT value FROM archive_metadata WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
}

fn write_setting(connection: &Connection, key: &str, value: &str) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO archive_metadata (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map(|_| ())
        .map_err(|error| format!("写入请求归档设置失败: {error}"))
}

fn load_settings(connection: &Connection) -> RequestArchiveSettings {
    let defaults = RequestArchiveSettings::default();
    RequestArchiveSettings {
        enabled: read_setting(connection, SETTING_ENABLED)
            .map(|value| value == "true")
            .unwrap_or(defaults.enabled),
        retention_days: read_setting(connection, SETTING_RETENTION_DAYS)
            .and_then(|value| value.parse().ok())
            .unwrap_or(defaults.retention_days),
        max_total_mb: read_setting(connection, SETTING_MAX_TOTAL_MB)
            .and_then(|value| value.parse().ok())
            .unwrap_or(defaults.max_total_mb),
        max_body_kb: read_setting(connection, SETTING_MAX_BODY_KB)
            .and_then(|value| value.parse().ok())
            .unwrap_or(defaults.max_body_kb),
    }
    .normalized()
}

fn store_settings(
    connection: &Connection,
    settings: RequestArchiveSettings,
) -> Result<(), String> {
    write_setting(connection, SETTING_ENABLED, &settings.enabled.to_string())?;
    write_setting(
        connection,
        SETTING_RETENTION_DAYS,
        &settings.retention_days.to_string(),
    )?;
    write_setting(
        connection,
        SETTING_MAX_TOTAL_MB,
        &settings.max_total_mb.to_string(),
    )?;
    write_setting(
        connection,
        SETTING_MAX_BODY_KB,
        &settings.max_body_kb.to_string(),
    )
}

fn truncate_text(value: &str, max_bytes: usize) -> (String, bool) {
    if value.len() <= max_bytes {
        return (value.to_string(), false);
    }
    let mut boundary = max_bytes;
    while boundary > 0 && !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    (value[..boundary].to_string(), true)
}

fn file_fingerprint(metadata: &fs::Metadata) -> String {
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("{}:{modified}", metadata.len())
}

fn timestamp_to_millis(timestamp: &str, fallback: &fs::Metadata) -> (String, i64) {
    if !timestamp.is_empty() {
        if let Ok(parsed) = DateTime::parse_from_rfc3339(timestamp) {
            return (
                parsed.with_timezone(&Local).to_rfc3339(),
                parsed.timestamp_millis(),
            );
        }
    }
    let millis = fallback
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_else(|| Local::now().timestamp_millis());
    let rendered = Local
        .timestamp_millis_opt(millis)
        .single()
        .map(|value| value.to_rfc3339())
        .unwrap_or_default();
    (rendered, millis)
}

fn ingest_file(
    connection: &Connection,
    path: &Path,
    settings: RequestArchiveSettings,
) -> Result<bool, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("读取日志文件信息失败 {}: {error}", path.display()))?;
    let filename = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let fingerprint = file_fingerprint(&metadata);

    let existing: Option<String> = connection
        .query_row(
            "SELECT source_fingerprint FROM request_records WHERE source_file = ?1",
            params![filename],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("查询请求归档记录失败: {error}"))?;
    if existing.as_deref() == Some(fingerprint.as_str()) {
        return Ok(false);
    }

    let contents = fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(|error| format!("读取日志文件失败 {}: {error}", path.display()))?;
    let parsed = parse_request_log(&contents);

    let max_bytes = settings.max_body_kb as usize * 1024;
    let mut truncated = false;
    let mut cap = |value: &str| {
        let (text, was_truncated) = truncate_text(value, max_bytes);
        truncated = truncated || was_truncated;
        text
    };

    let request_body = cap(&parsed.request_body);
    let api_request = cap(&parsed.api_request);
    let api_response = cap(&parsed.api_response);
    let response_body = cap(&parsed.response_body);
    let websocket_timeline = cap(&parsed.websocket_timeline);
    let system_prompt = cap(&parsed.system_prompt);
    let messages_json = cap(&parsed.messages_json);
    let tools_json = cap(&parsed.tools_json);
    let api_error_text = cap(&parsed.api_error_text);

    let (captured_at, captured_at_ms) = timestamp_to_millis(&parsed.timestamp, &metadata);
    let mut request_id = header_value(&parsed.request_headers, "X-Request-Id");
    if request_id.is_empty() {
        request_id = request_id_from_filename(&filename);
    }
    let http_status = if parsed.response_status > 0 {
        parsed.response_status
    } else {
        parsed.api_error_status
    };
    let failed = i64::from(
        parsed.api_error_status > 0 || !(200..400).contains(&http_status) && http_status != 0,
    );
    let usage_json = parsed
        .usage
        .raw
        .as_ref()
        .map(|value| value.to_string())
        .unwrap_or_default();
    let request_headers_json = serde_json::to_string(&parsed.request_headers).unwrap_or_default();
    let response_headers_json = serde_json::to_string(&parsed.response_headers).unwrap_or_default();

    connection
        .execute(
            "INSERT INTO request_records (
                request_id, source_file, source_fingerprint, captured_at, captured_at_ms,
                core_version, url, endpoint, method, downstream_transport, upstream_transport,
                model, stream, http_status, api_error_status, failed,
                system_prompt, messages_json, tools_json, usage_json,
                input_tokens, output_tokens, reasoning_tokens, cache_read_tokens,
                cache_creation_tokens, total_tokens,
                request_headers_json, response_headers_json,
                request_body, api_request, api_response, response_body, api_error_text,
                websocket_timeline, truncated, byte_size, created_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
                ?31, ?32, ?33, ?34, ?35, ?36, ?37
             )
             ON CONFLICT(source_file) DO UPDATE SET
                request_id = excluded.request_id,
                source_fingerprint = excluded.source_fingerprint,
                captured_at = excluded.captured_at,
                captured_at_ms = excluded.captured_at_ms,
                core_version = excluded.core_version,
                url = excluded.url,
                endpoint = excluded.endpoint,
                method = excluded.method,
                downstream_transport = excluded.downstream_transport,
                upstream_transport = excluded.upstream_transport,
                model = excluded.model,
                stream = excluded.stream,
                http_status = excluded.http_status,
                api_error_status = excluded.api_error_status,
                failed = excluded.failed,
                system_prompt = excluded.system_prompt,
                messages_json = excluded.messages_json,
                tools_json = excluded.tools_json,
                usage_json = excluded.usage_json,
                input_tokens = excluded.input_tokens,
                output_tokens = excluded.output_tokens,
                reasoning_tokens = excluded.reasoning_tokens,
                cache_read_tokens = excluded.cache_read_tokens,
                cache_creation_tokens = excluded.cache_creation_tokens,
                total_tokens = excluded.total_tokens,
                request_headers_json = excluded.request_headers_json,
                response_headers_json = excluded.response_headers_json,
                request_body = excluded.request_body,
                api_request = excluded.api_request,
                api_response = excluded.api_response,
                response_body = excluded.response_body,
                api_error_text = excluded.api_error_text,
                websocket_timeline = excluded.websocket_timeline,
                truncated = excluded.truncated,
                byte_size = excluded.byte_size",
            params![
                request_id,
                filename,
                fingerprint,
                captured_at,
                captured_at_ms,
                parsed.core_version,
                parsed.url,
                endpoint_from_url(&parsed.url),
                parsed.method,
                parsed.downstream_transport,
                parsed.upstream_transport,
                parsed.model,
                i64::from(parsed.stream),
                http_status,
                parsed.api_error_status,
                failed,
                system_prompt,
                messages_json,
                tools_json,
                usage_json,
                parsed.usage.input_tokens,
                parsed.usage.output_tokens,
                parsed.usage.reasoning_tokens,
                parsed.usage.cache_read_tokens,
                parsed.usage.cache_creation_tokens,
                parsed.usage.total_tokens,
                request_headers_json,
                response_headers_json,
                request_body,
                api_request,
                api_response,
                response_body,
                api_error_text,
                websocket_timeline,
                i64::from(truncated),
                metadata.len() as i64,
                Local::now().to_rfc3339(),
            ],
        )
        .map_err(|error| format!("写入请求归档记录失败: {error}"))?;
    Ok(true)
}

fn database_bytes(root: &Path) -> i64 {
    let mut total = 0_i64;
    for suffix in ["", "-wal", "-shm"] {
        let path = root.join(format!("{ARCHIVE_DATABASE_FILE}{suffix}"));
        if let Ok(metadata) = fs::metadata(&path) {
            total += metadata.len() as i64;
        }
    }
    total
}

fn apply_retention(
    connection: &Connection,
    root: &Path,
    settings: RequestArchiveSettings,
) -> Result<usize, String> {
    let mut removed = 0_usize;
    if settings.retention_days > 0 {
        let cutoff = Local::now().timestamp_millis()
            - i64::from(settings.retention_days) * 24 * 60 * 60 * 1_000;
        removed += connection
            .execute(
                "DELETE FROM request_records WHERE captured_at_ms < ?1",
                params![cutoff],
            )
            .map_err(|error| format!("清理过期请求归档失败: {error}"))?;
    }

    if settings.max_total_mb > 0 {
        let limit = i64::from(settings.max_total_mb) * 1024 * 1024;
        let mut guard = 0;
        while database_bytes(root) > limit && guard < 200 {
            let deleted = connection
                .execute(
                    "DELETE FROM request_records WHERE id IN (
                        SELECT id FROM request_records ORDER BY captured_at_ms ASC, id ASC LIMIT 200
                     )",
                    [],
                )
                .map_err(|error| format!("按容量清理请求归档失败: {error}"))?;
            if deleted == 0 {
                break;
            }
            removed += deleted;
            connection
                .execute("PRAGMA wal_checkpoint(TRUNCATE)", [])
                .map_err(|error| format!("请求归档 WAL 检查点失败: {error}"))?;
            guard += 1;
        }
    }

    if removed > 0 {
        connection
            .execute_batch("VACUUM")
            .map_err(|error| format!("压缩请求归档数据库失败: {error}"))?;
    }
    Ok(removed)
}

fn scan_once(
    connection: &Connection,
    root: &Path,
    logs_dir: &Path,
    settings: RequestArchiveSettings,
) -> Result<usize, String> {
    if !logs_dir.is_dir() {
        return Ok(0);
    }
    let entries = fs::read_dir(logs_dir)
        .map_err(|error| format!("读取日志目录失败 {}: {error}", logs_dir.display()))?;
    let now = SystemTime::now();
    let mut candidates: Vec<(i64, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().map(|value| value.to_string_lossy().to_string()) else {
            continue;
        };
        // main.log is the application log, not a per-request transcript.
        if !name.ends_with(".log") || name == "main.log" || name.starts_with("response-body-") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if let Ok(modified) = metadata.modified() {
            if now
                .duration_since(modified)
                .map(|elapsed| elapsed < Duration::from_secs(WRITE_SETTLE_SECONDS))
                .unwrap_or(false)
            {
                continue;
            }
        }
        let sort_key = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default();
        candidates.push((sort_key, path));
    }
    candidates.sort_by_key(|(sort_key, _)| *sort_key);
    candidates.truncate(MAX_FILES_PER_SCAN);

    let mut ingested = 0_usize;
    for (_, path) in candidates {
        match ingest_file(connection, &path, settings) {
            Ok(true) => ingested += 1,
            Ok(false) => {}
            Err(error) => return Err(error),
        }
    }
    if ingested > 0 {
        apply_retention(connection, root, settings)?;
    }
    Ok(ingested)
}

pub(crate) fn start_request_archive_ingester(app: tauri::AppHandle) {
    if fork_archive_disabled() {
        eprintln!("请求归档已通过 {FORK_ARCHIVE_ENV} 关闭，跳过采集器启动");
        return;
    }
    let state = app.state::<RequestArchiveState>();
    let Some(token) = state.start() else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        ingester_loop(app, token).await;
    });
}

pub(crate) fn stop_request_archive_ingester(app: &tauri::AppHandle) {
    app.state::<RequestArchiveState>().stop();
}

async fn ingester_loop(app: tauri::AppHandle, token: CancellationToken) {
    let root = match archive_root_dir() {
        Ok(root) => root,
        Err(error) => {
            app.state::<RequestArchiveState>().record_error(error);
            return;
        }
    };
    loop {
        if token.is_cancelled() {
            return;
        }
        let outcome = (|| -> Result<usize, String> {
            let connection = open_archive_database_at(&root)?;
            let settings = load_settings(&connection);
            if !settings.enabled {
                return Ok(0);
            }
            let auth_dir = app
                .state::<GuiConfigState>()
                .snapshot()
                .map(|config| config.auth_dir)
                .unwrap_or_default();
            let mut ingested = 0;
            for logs_dir in core_logs_directories(&auth_dir)? {
                ingested += scan_once(&connection, &root, &logs_dir, settings)?;
            }
            Ok(ingested)
        })();

        match outcome {
            Ok(ingested) => {
                let state = app.state::<RequestArchiveState>();
                if ingested > 0 {
                    let timestamp = Local::now().to_rfc3339();
                    state.record_success(timestamp.clone());
                    let _ = app.emit(ARCHIVE_UPDATED_EVENT, timestamp);
                }
            }
            Err(error) => {
                app.state::<RequestArchiveState>().record_error(error);
            }
        }

        tokio::select! {
            _ = token.cancelled() => return,
            _ = tokio::time::sleep(Duration::from_secs(SCAN_INTERVAL_SECONDS)) => {}
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestArchiveQuery {
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub model: Option<String>,
    pub result: Option<String>,
    pub search: Option<String>,
    pub request_id: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestArchiveSummary {
    pub id: i64,
    pub request_id: String,
    pub captured_at: String,
    pub captured_at_ms: i64,
    pub url: String,
    pub endpoint: String,
    pub method: String,
    pub model: String,
    pub stream: bool,
    pub http_status: i64,
    pub api_error_status: i64,
    pub failed: bool,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub has_system_prompt: bool,
    pub has_tools: bool,
    pub truncated: bool,
    pub byte_size: i64,
    pub source_file: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestArchivePage {
    pub total: i64,
    pub records: Vec<RequestArchiveSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestArchiveDetail {
    #[serde(flatten)]
    pub summary: RequestArchiveSummary,
    pub core_version: String,
    pub downstream_transport: String,
    pub upstream_transport: String,
    pub system_prompt: String,
    pub messages_json: String,
    pub tools_json: String,
    pub usage_json: String,
    pub reasoning_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub request_headers_json: String,
    pub response_headers_json: String,
    pub request_body: String,
    pub api_request: String,
    pub api_response: String,
    pub response_body: String,
    pub api_error_text: String,
    pub websocket_timeline: String,
}

const SUMMARY_COLUMNS: &str = "id, request_id, captured_at, captured_at_ms, url, endpoint, method, \
     model, stream, http_status, api_error_status, failed, input_tokens, output_tokens, \
     total_tokens, length(system_prompt) > 0, length(tools_json) > 0, truncated, byte_size, \
     source_file";

fn summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RequestArchiveSummary> {
    Ok(RequestArchiveSummary {
        id: row.get(0)?,
        request_id: row.get(1)?,
        captured_at: row.get(2)?,
        captured_at_ms: row.get(3)?,
        url: row.get(4)?,
        endpoint: row.get(5)?,
        method: row.get(6)?,
        model: row.get(7)?,
        stream: row.get::<_, i64>(8)? != 0,
        http_status: row.get(9)?,
        api_error_status: row.get(10)?,
        failed: row.get::<_, i64>(11)? != 0,
        input_tokens: row.get(12)?,
        output_tokens: row.get(13)?,
        total_tokens: row.get(14)?,
        has_system_prompt: row.get::<_, i64>(15)? != 0,
        has_tools: row.get::<_, i64>(16)? != 0,
        truncated: row.get::<_, i64>(17)? != 0,
        byte_size: row.get(18)?,
        source_file: row.get(19)?,
    })
}

fn build_filters(query: &RequestArchiveQuery) -> (String, Vec<SqlValue>) {
    let mut clauses = Vec::new();
    let mut values: Vec<SqlValue> = Vec::new();

    if let Some(start) = query.start_ms {
        clauses.push("captured_at_ms >= ?".to_string());
        values.push(SqlValue::Integer(start));
    }
    if let Some(end) = query.end_ms {
        clauses.push("captured_at_ms <= ?".to_string());
        values.push(SqlValue::Integer(end));
    }
    if let Some(model) = query.model.as_ref().filter(|value| !value.is_empty()) {
        clauses.push("model = ?".to_string());
        values.push(SqlValue::Text(model.clone()));
    }
    if let Some(request_id) = query.request_id.as_ref().filter(|value| !value.is_empty()) {
        clauses.push("request_id = ?".to_string());
        values.push(SqlValue::Text(request_id.clone()));
    }
    match query.result.as_deref() {
        Some("success") => clauses.push("failed = 0".to_string()),
        Some("failed") => clauses.push("failed = 1".to_string()),
        _ => {}
    }
    if let Some(search) = query.search.as_ref().filter(|value| !value.is_empty()) {
        clauses.push(
            "(system_prompt LIKE ? OR messages_json LIKE ? OR url LIKE ? OR request_id LIKE ? OR model LIKE ?)"
                .to_string(),
        );
        let pattern = format!("%{search}%");
        for _ in 0..5 {
            values.push(SqlValue::Text(pattern.clone()));
        }
    }

    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };
    (where_clause, values)
}

#[tauri::command]
pub(crate) fn get_request_archive_status(
    app: tauri::AppHandle,
    gui_config_state: tauri::State<'_, GuiConfigState>,
) -> Result<RequestArchiveStatus, String> {
    if fork_archive_disabled() {
        return Ok(RequestArchiveStatus {
            fork_disabled: true,
            ..RequestArchiveStatus::default()
        });
    }
    let root = archive_root_dir()?;
    let connection = open_archive_database_at(&root)?;
    let settings = load_settings(&connection);
    let record_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
        .map_err(|error| format!("统计请求归档记录失败: {error}"))?;
    let (oldest, newest): (Option<String>, Option<String>) = connection
        .query_row(
            "SELECT MIN(captured_at), MAX(captured_at) FROM request_records",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap_or((None, None));

    let config = gui_config_state.snapshot()?;
    let (last_ingested_at, last_error) = app.state::<RequestArchiveState>().snapshot();

    Ok(RequestArchiveStatus {
        fork_disabled: false,
        settings_enabled: settings.enabled,
        retention_days: settings.retention_days,
        max_total_mb: settings.max_total_mb,
        max_body_kb: settings.max_body_kb,
        request_log_enabled: config.request_log,
        record_count,
        database_bytes: database_bytes(&root),
        database_path: archive_database_path()?.to_string_lossy().to_string(),
        logs_directory: primary_logs_directory(&config.auth_dir)?
            .to_string_lossy()
            .to_string(),
        last_ingested_at,
        last_error,
        oldest_record_at: oldest.unwrap_or_default(),
        newest_record_at: newest.unwrap_or_default(),
    })
}

#[tauri::command]
pub(crate) fn save_request_archive_settings(
    app: tauri::AppHandle,
    settings: RequestArchiveSettings,
) -> Result<(), String> {
    let normalized = settings.normalized();
    let root = archive_root_dir()?;
    let connection = open_archive_database_at(&root)?;
    store_settings(&connection, normalized)?;
    apply_retention(&connection, &root, normalized)?;
    // Restart the loop so a freshly enabled archive starts scanning immediately.
    stop_request_archive_ingester(&app);
    start_request_archive_ingester(app);
    Ok(())
}

#[tauri::command]
pub(crate) fn query_request_archive_records(
    query: RequestArchiveQuery,
) -> Result<RequestArchivePage, String> {
    let connection = open_archive_database()?;
    let (where_clause, values) = build_filters(&query);

    let total: i64 = connection
        .query_row(
            &format!("SELECT COUNT(*) FROM request_records{where_clause}"),
            params_from_iter(values.iter()),
            |row| row.get(0),
        )
        .map_err(|error| format!("统计请求归档记录失败: {error}"))?;

    let limit = query.limit.unwrap_or(50).clamp(1, 500) as i64;
    let offset = i64::from(query.offset.unwrap_or(0));
    let mut page_values = values.clone();
    page_values.push(SqlValue::Integer(limit));
    page_values.push(SqlValue::Integer(offset));

    let statement = format!(
        "SELECT {SUMMARY_COLUMNS} FROM request_records{where_clause} \
         ORDER BY captured_at_ms DESC, id DESC LIMIT ? OFFSET ?"
    );
    let mut prepared = connection
        .prepare(&statement)
        .map_err(|error| format!("准备请求归档查询失败: {error}"))?;
    let records = prepared
        .query_map(params_from_iter(page_values.iter()), summary_from_row)
        .map_err(|error| format!("查询请求归档记录失败: {error}"))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| format!("读取请求归档记录失败: {error}"))?;

    Ok(RequestArchivePage { total, records })
}

fn detail_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RequestArchiveDetail> {
    Ok(RequestArchiveDetail {
        summary: summary_from_row(row)?,
        core_version: row.get(20)?,
        downstream_transport: row.get(21)?,
        upstream_transport: row.get(22)?,
        system_prompt: row.get(23)?,
        messages_json: row.get(24)?,
        tools_json: row.get(25)?,
        usage_json: row.get(26)?,
        reasoning_tokens: row.get(27)?,
        cache_read_tokens: row.get(28)?,
        cache_creation_tokens: row.get(29)?,
        request_headers_json: row.get(30)?,
        response_headers_json: row.get(31)?,
        request_body: row.get(32)?,
        api_request: row.get(33)?,
        api_response: row.get(34)?,
        response_body: row.get(35)?,
        api_error_text: row.get(36)?,
        websocket_timeline: row.get(37)?,
    })
}

const DETAIL_COLUMNS: &str = "core_version, downstream_transport, upstream_transport, \
     system_prompt, messages_json, tools_json, usage_json, reasoning_tokens, cache_read_tokens, \
     cache_creation_tokens, request_headers_json, response_headers_json, request_body, \
     api_request, api_response, response_body, api_error_text, websocket_timeline";

#[tauri::command]
pub(crate) fn get_request_archive_record(id: i64) -> Result<Option<RequestArchiveDetail>, String> {
    let connection = open_archive_database()?;
    connection
        .query_row(
            &format!("SELECT {SUMMARY_COLUMNS}, {DETAIL_COLUMNS} FROM request_records WHERE id = ?1"),
            params![id],
            detail_from_row,
        )
        .optional()
        .map_err(|error| format!("读取请求归档详情失败: {error}"))
}

#[tauri::command]
pub(crate) fn get_request_archive_record_by_request_id(
    request_id: String,
) -> Result<Option<RequestArchiveDetail>, String> {
    if request_id.trim().is_empty() {
        return Ok(None);
    }
    let connection = open_archive_database()?;
    connection
        .query_row(
            &format!(
                "SELECT {SUMMARY_COLUMNS}, {DETAIL_COLUMNS} FROM request_records \
                 WHERE request_id = ?1 ORDER BY captured_at_ms DESC, id DESC LIMIT 1"
            ),
            params![request_id.trim()],
            detail_from_row,
        )
        .optional()
        .map_err(|error| format!("读取请求归档详情失败: {error}"))
}

#[tauri::command]
pub(crate) fn clear_request_archive(app: tauri::AppHandle) -> Result<(), String> {
    let root = archive_root_dir()?;
    let connection = open_archive_database_at(&root)?;
    connection
        .execute("DELETE FROM request_records", [])
        .map_err(|error| format!("清空请求归档失败: {error}"))?;
    connection
        .execute_batch("VACUUM")
        .map_err(|error| format!("压缩请求归档数据库失败: {error}"))?;
    let _ = app.emit(ARCHIVE_UPDATED_EVENT, Local::now().to_rfc3339());
    Ok(())
}

#[tauri::command]
pub(crate) fn get_request_archive_models() -> Result<Vec<String>, String> {
    let connection = open_archive_database()?;
    let mut prepared = connection
        .prepare(
            "SELECT DISTINCT model FROM request_records WHERE model <> '' ORDER BY model ASC",
        )
        .map_err(|error| format!("准备请求归档模型查询失败: {error}"))?;
    let models = prepared
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("查询请求归档模型失败: {error}"))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| format!("读取请求归档模型失败: {error}"))?;
    Ok(models)
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    pub(crate) fn open_database(root: &Path) -> Result<Connection, String> {
        open_archive_database_at(root)
    }

    pub(crate) fn ingest(
        connection: &Connection,
        path: &Path,
        settings: RequestArchiveSettings,
    ) -> Result<bool, String> {
        ingest_file(connection, path, settings)
    }

    pub(crate) fn scan(
        connection: &Connection,
        root: &Path,
        logs_dir: &Path,
        settings: RequestArchiveSettings,
    ) -> Result<usize, String> {
        scan_once(connection, root, logs_dir, settings)
    }

    pub(crate) fn settings(connection: &Connection) -> RequestArchiveSettings {
        load_settings(connection)
    }

    pub(crate) fn save(
        connection: &Connection,
        value: RequestArchiveSettings,
    ) -> Result<(), String> {
        store_settings(connection, value)
    }

    pub(crate) fn retention(
        connection: &Connection,
        root: &Path,
        value: RequestArchiveSettings,
    ) -> Result<usize, String> {
        apply_retention(connection, root, value)
    }

    pub(crate) fn filters(query: &RequestArchiveQuery) -> (String, Vec<SqlValue>) {
        build_filters(query)
    }

    pub(crate) fn parse(contents: &str) -> super::parser::ParsedRequestLog {
        super::parser::parse_request_log(contents)
    }

    pub(crate) fn request_id(filename: &str) -> String {
        super::parser::request_id_from_filename(filename)
    }

    pub(crate) fn model_from_url(url: &str) -> String {
        super::parser::model_from_url(url)
    }
}
