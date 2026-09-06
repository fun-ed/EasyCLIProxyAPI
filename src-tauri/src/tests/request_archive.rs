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

#[test]
fn fork_kill_switch_only_trips_on_explicit_negative_values() {
    use crate::request_archive::fork_archive_disabled_in;

    for value in ["0", "false", "off", "no", "FALSE", " Off ", "NO"] {
        assert!(
            fork_archive_disabled_in(Some(value)),
            "{value:?} must disable the fork feature"
        );
    }

    for value in ["1", "true", "on", "yes", "", "maybe"] {
        assert!(
            !fork_archive_disabled_in(Some(value)),
            "{value:?} must leave the fork feature enabled"
        );
    }

    assert!(
        !fork_archive_disabled_in(None),
        "an unset variable must leave the fork feature enabled"
    );
}


/// Guards the parser against a transcript captured from a real CLIProxyAPI 7.2.151
/// process rather than a hand-written sample, so a change to the core's writer
/// shows up here instead of silently producing empty archive rows.
#[test]
fn parses_a_transcript_captured_from_a_live_core() {
    let raw = include_str!("fixtures/real-core-claude-error.txt");
    let parsed = testing::parse(raw);

    assert_eq!(parsed.core_version, "7.2.151");
    assert_eq!(parsed.method, "POST");
    assert_eq!(parsed.url, "/v1/messages");
    assert_eq!(parsed.downstream_transport, "http");
    assert_eq!(parsed.upstream_transport, "http");
    assert_eq!(parsed.model, "claude-sonnet-4-20250514");
    assert!(!parsed.stream);
    assert_eq!(parsed.response_status, 400);

    assert_eq!(
        parsed.system_prompt,
        "You are a terse assistant used for an end to end archive test."
    );
    assert!(parsed.messages_json.contains("Reply with the single word OK."));
    assert!(parsed.tools_json.contains("get_weather"));

    // The header wins over the request id embedded in the log file name.
    assert_eq!(
        parsed
            .request_headers
            .get("X-Request-Id")
            .and_then(|values| values.first())
            .map(String::as_str),
        Some("e2e-claude-0001")
    );
    assert_eq!(
        crate::request_archive::testing::request_id("v1-messages-2026-09-06T115125-60d64dd7.log"),
        "60d64dd7"
    );

    // The core only partially masks Authorization (util.MaskAuthorizationHeader
    // keeps the leading and trailing characters), so the archive never sees the
    // whole credential. Assert the property rather than the exact mask shape.
    let authorization = parsed
        .request_headers
        .get("Authorization")
        .and_then(|values| values.first())
        .expect("the Authorization header must survive as a masked value");
    assert!(
        authorization.contains("..."),
        "Authorization must carry the core's mask marker, got {authorization:?}"
    );
    assert!(
        !raw.contains("e2e-test-key"),
        "the unmasked credential must never reach the archive"
    );

    assert!(parsed.api_response.contains("unknown provider"));
    assert!(parsed.response_body.contains("invalid_request_error"));
}

/// Drives the whole ingest path with transcripts captured from a live core in the
/// three provider shapes the parser branches on, so a regression in system prompt
/// or message extraction cannot pass as an empty-but-present row.
#[test]
fn ingests_live_core_transcripts_across_provider_shapes() {
    let root = archive_test_root("live-shapes");
    let logs = root.join("logs");
    fs::create_dir_all(&logs).unwrap();

    let cases: [(&str, &str, &str, &str); 3] = [
        (
            "v1-messages-2026-09-06T115125-60d64dd7.log",
            include_str!("fixtures/real-core-claude-error.txt"),
            "e2e-claude-0001",
            "You are a terse assistant used for an end to end archive test.",
        ),
        (
            "v1-chat-completions-2026-09-06T115407-61346789.log",
            include_str!("fixtures/real-core-openai-error.txt"),
            "e2e-openai-0002",
            "sys prompt for openai shape",
        ),
        (
            "v1beta-models-gemini-3-pro-generateContent-2026-09-06T115407-e831c26d.log",
            include_str!("fixtures/real-core-gemini-error.txt"),
            "e2e-gemini-0003",
            "gemini system text",
        ),
    ];

    let connection = testing::open_database(&root).unwrap();
    let settings = RequestArchiveSettings {
        enabled: true,
        ..RequestArchiveSettings::default()
    };

    for (name, contents, _, _) in cases {
        let path = logs.join(name);
        fs::write(&path, contents).unwrap();
        assert!(
            testing::ingest(&connection, &path, settings).unwrap(),
            "{name} must be ingested"
        );
    }

    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 3);

    for (name, _, request_id, system_prompt) in cases {
        let (stored_prompt, status, model): (String, i64, String) = connection
            .query_row(
                "SELECT system_prompt, http_status, model FROM request_records WHERE request_id = ?1",
                [request_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap_or_else(|error| panic!("{name} produced no row for {request_id}: {error}"));

        assert_eq!(stored_prompt, system_prompt, "{name} system prompt");
        assert_eq!(status, 400, "{name} http status");
        assert!(!model.is_empty(), "{name} must record a model");
    }

    fs::remove_dir_all(&root).ok();
}

#[test]
fn model_is_recovered_from_gemini_style_paths_only() {
    use crate::request_archive::testing::model_from_url;

    assert_eq!(
        model_from_url("/v1beta/models/gemini-3-pro:generateContent"),
        "gemini-3-pro"
    );
    assert_eq!(
        model_from_url("http://127.0.0.1:8317/v1beta/models/gemini-3-flash:streamGenerateContent?alt=sse"),
        "gemini-3-flash"
    );
    assert_eq!(model_from_url("/v1beta/models/gemini-3-pro"), "gemini-3-pro");

    // Surfaces that carry the model in the body must not be guessed at.
    assert_eq!(model_from_url("/v1/messages"), "");
    assert_eq!(model_from_url("/v1/chat/completions"), "");
    assert_eq!(model_from_url(""), "");
    assert_eq!(model_from_url("/v1beta/models/"), "");
}

#[test]
fn codex_attribution_is_dropped_while_the_totals_survive() {
    let log = [
        "=== REQUEST INFO ===",
        "Version: 7.2.151",
        "URL: /v1/responses",
        "Method: POST",
        "Timestamp: 2026-09-06T12:29:10.613903+08:00",
        "",
        "",
        "=== REQUEST BODY ===",
        r#"{"model":"gpt-5.6-terra","input":[]}"#,
        "",
        "",
        "=== API RESPONSE ===",
        r#"{"usage":{"attribution":{"items":{"ctc_call_a":{"input_tokens":525},"ctc_call_b":{"input_tokens":1799}}},"input_tokens":141335,"output_tokens":461,"total_tokens":141796}}"#,
        "",
        "",
        "=== RESPONSE ===",
        "Status: 200",
        "",
        "{}",
    ]
    .join("\n");

    let parsed = testing::parse(&log);

    assert_eq!(parsed.usage.input_tokens, 141335);
    assert_eq!(parsed.usage.output_tokens, 461);
    assert_eq!(parsed.usage.total_tokens, 141796);

    let raw = parsed.usage.raw.expect("usage payload must be retained");
    assert!(
        raw.get("attribution").is_none(),
        "the bulky attribution block must be dropped"
    );
    assert_eq!(raw.get("input_tokens").and_then(|v| v.as_i64()), Some(141335));
    assert_eq!(raw.get("total_tokens").and_then(|v| v.as_i64()), Some(141796));
}


#[test]
fn messages_are_derived_from_the_stored_body_instead_of_duplicated() {
    let root = archive_test_root("derive-messages");
    let logs = root.join("logs");
    fs::create_dir_all(&logs).unwrap();
    let log_path = logs.join("v1-messages-2026-09-06T012139-derive-1.log");
    fs::write(&log_path, sample_log("derive-1")).unwrap();

    let connection = testing::open_database(&root).unwrap();
    let settings = RequestArchiveSettings {
        enabled: true,
        ..RequestArchiveSettings::default()
    };
    assert!(testing::ingest(&connection, &log_path, settings).unwrap());

    // The conversation is kept once, in the raw body.
    let (stored_messages, body_len): (String, i64) = connection
        .query_row(
            "SELECT messages_json, LENGTH(request_body) FROM request_records",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(stored_messages, "", "messages must not be persisted");
    assert!(body_len > 0, "the raw body must still be stored");

    // Reading a record reconstructs them.
    let body: String = connection
        .query_row("SELECT request_body FROM request_records", [], |row| {
            row.get(0)
        })
        .unwrap();
    let derived = crate::request_archive::testing::messages_from_payload(&body);
    assert!(
        derived.contains("\"role\":\"user\""),
        "derived messages must round-trip the conversation, got {derived:?}"
    );

    fs::remove_dir_all(&root).ok();
}

#[test]
fn size_retention_stops_at_the_limit_instead_of_emptying_the_archive() {
    let root = archive_test_root("size-retention");
    let logs = root.join("logs");
    fs::create_dir_all(&logs).unwrap();

    let connection = testing::open_database(&root).unwrap();
    let settings = RequestArchiveSettings {
        enabled: true,
        max_body_kb: 65536,
        ..RequestArchiveSettings::default()
    };
    for index in 0..12 {
        let path = logs.join(format!("v1-messages-2026-09-06T0121{index:02}-cap-{index}.log"));
        fs::write(&path, sample_log(&format!("cap-{index}"))).unwrap();
        assert!(testing::ingest(&connection, &path, settings).unwrap());
    }

    // A cap far above the data must not delete anything, and must not error on
    // the WAL checkpoint, which previously aborted before VACUUM and left the
    // loop measuring an unchanged file size until every row was gone.
    let generous = RequestArchiveSettings {
        max_total_mb: 512,
        ..settings
    };
    assert_eq!(testing::retention(&connection, &root, generous).unwrap(), 0);

    let remaining: i64 = connection
        .query_row("SELECT COUNT(*) FROM request_records", [], |row| row.get(0))
        .unwrap();
    assert_eq!(remaining, 12, "nothing may be deleted while under the cap");

    fs::remove_dir_all(&root).ok();
}

#[test]
fn logs_directory_size_counts_nested_spill_directories() {
    let root = archive_test_root("logs-size");
    let logs = root.join("logs");
    let spill = logs.join("request-log-parts-api-request-1");
    fs::create_dir_all(&spill).unwrap();

    assert_eq!(crate::request_archive::testing::logs_bytes(&logs), 0);

    fs::write(logs.join("v1-messages-2026-09-06T012139-a.log"), vec![b'x'; 1500]).unwrap();
    // The core spills oversized bodies into nested temp directories, so a flat
    // listing would under-report the directory by exactly the largest payloads.
    fs::write(spill.join("part-0"), vec![b'y'; 2500]).unwrap();

    assert_eq!(crate::request_archive::testing::logs_bytes(&logs), 4000);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn only_per_request_transcripts_are_treated_as_archive_sources() {
    use crate::request_archive::testing::is_log_candidate;

    assert!(is_log_candidate("v1-messages-2026-09-06T012139-abc.log"));
    assert!(is_log_candidate("v1beta-models-gemini-3-pro-generateContent-x-def.log"));

    // The application log and spilled body fragments must never be ingested,
    // and therefore must never be deleted as "already archived" either.
    assert!(!is_log_candidate("main.log"));
    assert!(!is_log_candidate("response-body-123.log"));
    assert!(!is_log_candidate("notes.txt"));
}
