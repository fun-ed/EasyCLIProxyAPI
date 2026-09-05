use super::*;

use crate::request_archive::{testing, RequestArchiveQuery, RequestArchiveSettings};

fn archive_test_root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "cpa-gui-archive-{name}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

/// Mirrors the layout produced by the core's FileRequestLogger: sections are
/// separated by three newlines and each marker sits on its own line.
fn sample_log(request_id: &str) -> String {
    [
        "=== REQUEST INFO ===",
        "Version: 7.2.149",
        "URL: http://127.0.0.1:8317/v1/messages?beta=true",
        "Method: POST",
        &format!("Timestamp: 2026-09-06T01:21:39.123456789+08:00"),
        "",
        "",
        "=== HEADERS ===",
        "Content-Type: application/json",
        &format!("X-Request-Id: {request_id}"),
        "",
        "",
        "=== REQUEST BODY ===",
        r#"{"model":"claude-opus-5","stream":true,"system":[{"type":"text","text":"You are a careful assistant."}],"messages":[{"role":"user","content":"hello"}],"tools":[{"name":"read_file"}]}"#,
        "",
        "",
        "=== API REQUEST ===",
        r#"{"model":"claude-opus-5","stream":true,"system":[{"type":"text","text":"You are a careful assistant."}],"messages":[{"role":"user","content":"hello"}],"tools":[{"name":"read_file"}]}"#,
        "",
        "",
        "=== API RESPONSE ===",
        r#"data: {"type":"message_start","message":{"usage":{"input_tokens":10,"output_tokens":0}}}"#,
        r#"data: {"type":"message_delta","usage":{"input_tokens":268945,"output_tokens":1020,"cache_read_input_tokens":267409,"cache_creation_input_tokens":1200}}"#,
        "data: [DONE]",
        "",
        "",
        "=== RESPONSE ===",
        "Status: 200",
        "Content-Type: text/event-stream",
        "",
        r#"data: {"type":"message_stop"}"#,
        "",
    ]
    .join("\n")
}

#[test]
fn parses_core_request_log_into_structured_fields() {
    let parsed = testing::parse(&sample_log("req-abc"));

    assert_eq!(parsed.core_version, "7.2.149");
    assert_eq!(parsed.method, "POST");
    assert_eq!(parsed.url, "http://127.0.0.1:8317/v1/messages?beta=true");
    assert_eq!(parsed.model, "claude-opus-5");
    assert!(parsed.stream);
    assert_eq!(parsed.response_status, 200);
    assert_eq!(parsed.system_prompt, "You are a careful assistant.");
    assert!(parsed.messages_json.contains("\"role\":\"user\""));
    assert!(parsed.tools_json.contains("read_file"));
    assert_eq!(
        parsed
            .request_headers
            .get("X-Request-Id")
            .and_then(|values| values.first())
            .map(String::as_str),
        Some("req-abc")
    );
    // The final streaming chunk carries the authoritative totals.
    assert_eq!(parsed.usage.input_tokens, 268_945);
    assert_eq!(parsed.usage.output_tokens, 1_020);
    assert_eq!(parsed.usage.cache_read_tokens, 267_409);
    assert_eq!(parsed.usage.cache_creation_tokens, 1_200);
    assert_eq!(parsed.usage.total_tokens, 269_965);
}

#[test]
fn parses_openai_error_response_status() {
    let contents = [
        "=== REQUEST INFO ===",
        "URL: http://127.0.0.1:8317/v1/chat/completions",
        "Method: POST",
        "",
        "",
        "=== REQUEST BODY ===",
        r#"{"model":"gpt-x","messages":[{"role":"system","content":"be terse"},{"role":"user","content":"hi"}]}"#,
        "",
        "",
        "=== API ERROR RESPONSE ===",
        "HTTP Status: 429",
        "",
        "rate limit exceeded",
        "",
    ]
    .join("\n");

    let parsed = testing::parse(&contents);
    assert_eq!(parsed.api_error_status, 429);
    assert!(parsed.api_error_text.contains("rate limit exceeded"));
    assert_eq!(parsed.system_prompt, "be terse");
    assert_eq!(parsed.model, "gpt-x");
}

#[test]
fn ingests_each_log_once_and_reingests_after_change() {
    let root = archive_test_root("ingest");
    let logs = root.join("logs");
    fs::create_dir_all(&logs).unwrap();
    let log_path = logs.join("v1-messages-2026-09-06T012139-req-abc.log");
    fs::write(&log_path, sample_log("req-abc")).unwrap();

    let connection = testing::open_database(&root).unwrap();
    let settings = RequestArchiveSettings {
        enabled: true,
        ..RequestArchiveSettings::default()
    };

    assert!(testing::ingest(&connection, &log_path, settings).unwrap());
    assert!(!testing::ingest(&connection, &log_path, settings).unwrap());

    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let (request_id, endpoint, tokens): (String, String, i64) = connection
        .query_row(
            "SELECT request_id, endpoint, total_tokens FROM request_records",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(request_id, "req-abc");
    assert_eq!(endpoint, "/v1/messages");
    assert_eq!(tokens, 269_965);

    // A rewritten transcript replaces the stored row instead of duplicating it.
    fs::write(&log_path, sample_log("req-zzz")).unwrap();
    assert!(testing::ingest(&connection, &log_path, settings).unwrap());
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn scan_skips_application_log_and_spilled_bodies() {
    let root = archive_test_root("scan");
    let logs = root.join("logs");
    fs::create_dir_all(&logs).unwrap();
    fs::write(logs.join("main.log"), "level=info msg=started").unwrap();
    fs::write(logs.join("response-body-123.tmp.log"), "partial").unwrap();
    fs::write(
        logs.join("v1-messages-2026-09-06T012139-req-abc.log"),
        sample_log("req-abc"),
    )
    .unwrap();

    // Backdate the transcripts so the write-settle guard does not skip them.
    let old = SystemTime::now() - Duration::from_secs(60);
    for entry in fs::read_dir(&logs).unwrap().flatten() {
        let file = fs::File::options().write(true).open(entry.path()).unwrap();
        file.set_modified(old).unwrap();
    }

    let connection = testing::open_database(&root).unwrap();
    let settings = RequestArchiveSettings {
        enabled: true,
        ..RequestArchiveSettings::default()
    };
    let ingested = testing::scan(&connection, &root, &logs, settings).unwrap();
    assert_eq!(ingested, 1);

    let sources: Vec<String> = connection
        .prepare("SELECT source_file FROM request_records")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(sources, vec!["v1-messages-2026-09-06T012139-req-abc.log"]);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn retention_removes_records_older_than_the_configured_window() {
    let root = archive_test_root("retention");
    let connection = testing::open_database(&root).unwrap();
    let now = chrono::Local::now().timestamp_millis();
    for (index, age_days) in [1_i64, 45_i64].into_iter().enumerate() {
        connection
            .execute(
                "INSERT INTO request_records (source_file, captured_at_ms, created_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    format!("record-{index}.log"),
                    now - age_days * 24 * 60 * 60 * 1_000,
                    "2026-09-06T00:00:00+08:00"
                ],
            )
            .unwrap();
    }

    let settings = RequestArchiveSettings {
        enabled: true,
        retention_days: 30,
        max_total_mb: 0,
        ..RequestArchiveSettings::default()
    };
    let removed = testing::retention(&connection, &root, settings).unwrap();
    assert_eq!(removed, 1);

    let remaining: i64 = connection
        .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
        .unwrap();
    assert_eq!(remaining, 1);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn settings_round_trip_and_clamp_body_limit() {
    let root = archive_test_root("settings");
    let connection = testing::open_database(&root).unwrap();

    assert!(!testing::settings(&connection).enabled);

    testing::save(
        &connection,
        RequestArchiveSettings {
            enabled: true,
            retention_days: 7,
            max_total_mb: 100,
            max_body_kb: 1,
        },
    )
    .unwrap();

    let loaded = testing::settings(&connection);
    assert!(loaded.enabled);
    assert_eq!(loaded.retention_days, 7);
    assert_eq!(loaded.max_total_mb, 100);
    // A one-kilobyte cap would truncate every payload; the floor keeps it usable.
    assert_eq!(loaded.max_body_kb, 16);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn query_filters_translate_to_bound_sql() {
    let (clause, values) = testing::filters(&RequestArchiveQuery {
        start_ms: Some(10),
        end_ms: Some(20),
        model: Some("claude-opus-5".to_string()),
        result: Some("failed".to_string()),
        search: Some("prompt".to_string()),
        ..RequestArchiveQuery::default()
    });

    assert!(clause.starts_with(" WHERE "));
    assert!(clause.contains("captured_at_ms >= ?"));
    assert!(clause.contains("failed = 1"));
    // Two range bounds, one model and five search placeholders.
    assert_eq!(values.len(), 8);

    let (empty_clause, empty_values) = testing::filters(&RequestArchiveQuery::default());
    assert!(empty_clause.is_empty());
    assert!(empty_values.is_empty());
}
