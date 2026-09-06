use std::collections::BTreeMap;

use serde_json::{Map, Value};

/// Section markers emitted by the core's FileRequestLogger.
const SECTION_REQUEST_INFO: &str = "REQUEST INFO";
const SECTION_HEADERS: &str = "HEADERS";
const SECTION_REQUEST_BODY: &str = "REQUEST BODY";
const SECTION_API_REQUEST: &str = "API REQUEST";
const SECTION_API_RESPONSE: &str = "API RESPONSE";
const SECTION_API_ERROR_RESPONSE: &str = "API ERROR RESPONSE";
const SECTION_RESPONSE: &str = "RESPONSE";
const SECTION_WEBSOCKET_TIMELINE: &str = "WEBSOCKET TIMELINE";
const SECTION_API_WEBSOCKET_TIMELINE: &str = "API WEBSOCKET TIMELINE";

#[derive(Debug, Default, Clone)]
pub(crate) struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub total_tokens: i64,
    pub raw: Option<Value>,
}

impl TokenUsage {
    fn is_empty(&self) -> bool {
        self.raw.is_none()
            && self.input_tokens == 0
            && self.output_tokens == 0
            && self.total_tokens == 0
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct ParsedRequestLog {
    pub core_version: String,
    pub url: String,
    pub method: String,
    pub downstream_transport: String,
    pub upstream_transport: String,
    pub timestamp: String,
    pub request_headers: BTreeMap<String, Vec<String>>,
    pub response_headers: BTreeMap<String, Vec<String>>,
    pub request_body: String,
    pub api_request: String,
    pub api_response: String,
    pub api_error_text: String,
    pub api_error_status: i64,
    pub response_body: String,
    pub response_status: i64,
    pub websocket_timeline: String,

    pub model: String,
    pub stream: bool,
    pub system_prompt: String,
    pub messages_json: String,
    pub tools_json: String,
    pub usage: TokenUsage,
}

/// Splits a core request log into its `=== NAME ===` sections.
///
/// Repeated markers are concatenated because the core may emit both an inline
/// payload and a spilled file body under the same marker.
fn split_sections(contents: &str) -> BTreeMap<String, String> {
    let mut sections: BTreeMap<String, String> = BTreeMap::new();
    let mut current: Option<String> = None;
    let mut buffer = String::new();

    for line in contents.split_inclusive('\n') {
        if let Some(name) = section_name(line) {
            if let Some(previous) = current.take() {
                push_section(&mut sections, previous, std::mem::take(&mut buffer));
            }
            current = Some(name);
            continue;
        }
        if current.is_some() {
            buffer.push_str(line);
        }
    }
    if let Some(previous) = current.take() {
        push_section(&mut sections, previous, buffer);
    }
    sections
}

fn push_section(sections: &mut BTreeMap<String, String>, name: String, body: String) {
    let trimmed = body.trim_matches('\n').to_string();
    match sections.get_mut(&name) {
        Some(existing) if !trimmed.is_empty() => {
            if !existing.is_empty() {
                existing.push('\n');
            }
            existing.push_str(&trimmed);
        }
        Some(_) => {}
        None => {
            sections.insert(name, trimmed);
        }
    }
}

fn section_name(line: &str) -> Option<String> {
    let trimmed = line.trim_end_matches(['\n', '\r']);
    let inner = trimmed.strip_prefix("=== ")?.strip_suffix(" ===")?;
    if inner.is_empty() || inner.contains("===") {
        return None;
    }
    Some(inner.to_string())
}

fn parse_key_values(body: &str) -> BTreeMap<String, Vec<String>> {
    let mut values: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in body.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        values
            .entry(key.to_string())
            .or_default()
            .push(value.trim().to_string());
    }
    values
}

fn first_value(values: &BTreeMap<String, Vec<String>>, key: &str) -> String {
    values
        .get(key)
        .and_then(|entries| entries.first())
        .cloned()
        .unwrap_or_default()
}

/// Splits a leading `Key: value` block from a payload body. The core writes
/// `Status:` / `HTTP Status:` / header lines before the body, separated from it
/// by a blank line.
fn split_leading_headers(body: &str) -> (BTreeMap<String, Vec<String>>, String) {
    match body.split_once("\n\n") {
        Some((head, rest)) if !head.is_empty() && head.lines().all(is_header_line) => (
            parse_key_values(head),
            rest.trim_start_matches('\n').to_string(),
        ),
        _ if !body.is_empty() && body.lines().all(is_header_line) => {
            (parse_key_values(body), String::new())
        }
        _ => (BTreeMap::new(), body.to_string()),
    }
}

fn is_header_line(line: &str) -> bool {
    if line.trim().is_empty() {
        return true;
    }
    if line.starts_with(' ') || line.starts_with('\t') {
        return false;
    }
    match line.split_once(':') {
        Some((key, _)) => {
            !key.is_empty()
                && key.chars().all(|value| {
                    value.is_ascii_alphanumeric() || value == '-' || value == '_' || value == ' '
                })
        }
        None => false,
    }
}

pub(crate) fn parse_request_log(contents: &str) -> ParsedRequestLog {
    let sections = split_sections(contents);
    let mut parsed = ParsedRequestLog::default();

    let info = parse_key_values(sections.get(SECTION_REQUEST_INFO).map_or("", String::as_str));
    parsed.core_version = first_value(&info, "Version");
    parsed.url = first_value(&info, "URL");
    parsed.method = first_value(&info, "Method");
    parsed.downstream_transport = first_value(&info, "Downstream Transport");
    parsed.upstream_transport = first_value(&info, "Upstream Transport");
    parsed.timestamp = first_value(&info, "Timestamp");

    if let Some(headers) = sections.get(SECTION_HEADERS) {
        parsed.request_headers = parse_key_values(headers);
    }
    parsed.request_body = sections
        .get(SECTION_REQUEST_BODY)
        .cloned()
        .unwrap_or_default();
    parsed.api_request = sections
        .get(SECTION_API_REQUEST)
        .cloned()
        .unwrap_or_default();
    parsed.api_response = sections
        .get(SECTION_API_RESPONSE)
        .cloned()
        .unwrap_or_default();
    parsed.websocket_timeline = sections
        .get(SECTION_WEBSOCKET_TIMELINE)
        .or_else(|| sections.get(SECTION_API_WEBSOCKET_TIMELINE))
        .cloned()
        .unwrap_or_default();

    if let Some(error) = sections.get(SECTION_API_ERROR_RESPONSE) {
        let (headers, body) = split_leading_headers(error);
        parsed.api_error_status = first_value(&headers, "HTTP Status").parse().unwrap_or(0);
        parsed.api_error_text = if body.trim().is_empty() {
            error.trim().to_string()
        } else {
            body.trim().to_string()
        };
    }

    if let Some(response) = sections.get(SECTION_RESPONSE) {
        let (headers, body) = split_leading_headers(response);
        parsed.response_status = first_value(&headers, "Status").parse().unwrap_or(0);
        parsed.response_body = body;
        parsed.response_headers = headers;
        parsed.response_headers.remove("Status");
    }

    enrich_from_request_body(&mut parsed);
    parsed.usage = extract_usage(&parsed);
    parsed
}

fn enrich_from_request_body(parsed: &mut ParsedRequestLog) {
    // The upstream payload is the richest source; fall back to the downstream body.
    let mut merged: Option<Map<String, Value>> = None;
    for candidate in [parsed.api_request.as_str(), parsed.request_body.as_str()] {
        if let Some(Value::Object(object)) = parse_embedded_json(candidate) {
            match merged.as_mut() {
                Some(existing) => {
                    for (key, value) in object {
                        existing.entry(key).or_insert(value);
                    }
                }
                None => merged = Some(object),
            }
        }
    }
    let Some(body) = merged else {
        return;
    };

    parsed.model = body
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if parsed.model.is_empty() {
        // The Gemini native surface names the model in the URL instead of the body.
        parsed.model = model_from_url(&parsed.url);
    }
    parsed.stream = body.get("stream").and_then(Value::as_bool).unwrap_or(false);
    parsed.system_prompt = extract_system_prompt(&body);
    parsed.messages_json = extract_messages(&body);
    parsed.tools_json = extract_tools(&body);
}

/// Recovers the model from Gemini-style paths such as
/// `/v1beta/models/gemini-3-pro:generateContent`, where the model never appears
/// in the request body.
pub(crate) fn model_from_url(url: &str) -> String {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let Some(rest) = path.split("/models/").nth(1) else {
        return String::new();
    };
    let segment = rest.split('/').next().unwrap_or_default();
    segment
        .split(':')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Extracts a JSON document from a payload that may carry a leading header
/// block, since the core prefixes spilled bodies with `Timestamp:` lines.
fn parse_embedded_json(payload: &str) -> Option<Value> {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Some(value);
    }
    let start = trimmed.find(['{', '['])?;
    let candidate = &trimmed[start..];
    if let Ok(value) = serde_json::from_str::<Value>(candidate) {
        return Some(value);
    }
    last_sse_json(candidate)
}

fn last_sse_json(payload: &str) -> Option<Value> {
    let mut last = None;
    for line in payload.lines() {
        let Some(data) = line.trim().strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(data) {
            last = Some(value);
        }
    }
    last
}

fn extract_system_prompt(body: &Map<String, Value>) -> String {
    // Claude native: top-level `system`.
    if let Some(system) = body.get("system") {
        let text = collect_text(system);
        if !text.is_empty() {
            return text;
        }
    }
    // OpenAI Responses / Codex: `instructions`.
    if let Some(instructions) = body.get("instructions").and_then(Value::as_str) {
        if !instructions.trim().is_empty() {
            return instructions.to_string();
        }
    }
    // Gemini: `systemInstruction` / `system_instruction`.
    for key in ["systemInstruction", "system_instruction"] {
        if let Some(instruction) = body.get(key) {
            let text = collect_text(instruction);
            if !text.is_empty() {
                return text;
            }
        }
    }
    // OpenAI chat completions: system/developer roles inside `messages`.
    if let Some(messages) = body.get("messages").and_then(Value::as_array) {
        let mut collected = Vec::new();
        for message in messages {
            let role = message
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if role == "system" || role == "developer" {
                let text = collect_text(message.get("content").unwrap_or(&Value::Null));
                if !text.is_empty() {
                    collected.push(text);
                }
            }
        }
        if !collected.is_empty() {
            return collected.join("\n\n");
        }
    }
    String::new()
}

/// Re-derives the message list from a stored raw request body.
///
/// Messages are no longer persisted alongside the body they come from: the two
/// held near-identical content and together accounted for most of the archive's
/// size. Reconstructing them when a record is opened costs one JSON parse and
/// halves the database.
pub(crate) fn messages_from_payload(payload: &str) -> String {
    match parse_embedded_json(payload) {
        Some(Value::Object(body)) => extract_messages(&body),
        _ => String::new(),
    }
}

fn extract_messages(body: &Map<String, Value>) -> String {
    for key in ["messages", "contents", "input"] {
        if let Some(value) = body.get(key) {
            if value.is_array() {
                return value.to_string();
            }
        }
    }
    String::new()
}

fn extract_tools(body: &Map<String, Value>) -> String {
    for key in ["tools", "functions", "toolConfig", "tool_config"] {
        if let Some(value) = body.get(key) {
            if !value.is_null() {
                return value.to_string();
            }
        }
    }
    String::new()
}

/// Recursively collects human-readable text from a prompt-shaped value.
fn collect_text(value: &Value) -> String {
    let mut parts = Vec::new();
    collect_text_into(value, &mut parts);
    parts.join("\n").trim().to_string()
}

fn collect_text_into(value: &Value, parts: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            if !text.trim().is_empty() {
                parts.push(text.clone());
            }
        }
        Value::Array(entries) => {
            for entry in entries {
                collect_text_into(entry, parts);
            }
        }
        Value::Object(object) => {
            if let Some(Value::String(text)) = object.get("text") {
                if !text.trim().is_empty() {
                    parts.push(text.clone());
                }
                return;
            }
            for key in ["parts", "content"] {
                if let Some(nested) = object.get(key) {
                    collect_text_into(nested, parts);
                }
            }
        }
        _ => {}
    }
}

fn extract_usage(parsed: &ParsedRequestLog) -> TokenUsage {
    for payload in [
        parsed.api_response.as_str(),
        parsed.response_body.as_str(),
        parsed.websocket_timeline.as_str(),
    ] {
        let usage = usage_from_payload(payload);
        if !usage.is_empty() {
            return usage;
        }
    }
    TokenUsage::default()
}

fn usage_from_payload(payload: &str) -> TokenUsage {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return TokenUsage::default();
    }
    let mut best = TokenUsage::default();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if let Some(usage) = find_usage_object(&value, 0) {
            best = normalize_usage(&usage);
        }
    }
    if !best.is_empty() {
        return best;
    }
    // Streaming responses: the final chunk carries the authoritative totals.
    for line in trimmed.lines() {
        let candidate = line.trim().strip_prefix("data:").unwrap_or(line.trim());
        let candidate = candidate.trim();
        if candidate.is_empty() || candidate == "[DONE]" || !candidate.starts_with('{') {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(candidate) else {
            continue;
        };
        if let Some(usage) = find_usage_object(&value, 0) {
            let normalized = normalize_usage(&usage);
            if !normalized.is_empty() {
                best = normalized;
            }
        }
    }
    best
}

fn find_usage_object(value: &Value, depth: usize) -> Option<Value> {
    if depth > 6 {
        return None;
    }
    let Value::Object(object) = value else {
        if let Value::Array(entries) = value {
            for entry in entries {
                if let Some(found) = find_usage_object(entry, depth + 1) {
                    return Some(found);
                }
            }
        }
        return None;
    };
    for key in ["usage", "usageMetadata", "usage_metadata"] {
        if let Some(usage @ Value::Object(_)) = object.get(key) {
            return Some(usage.clone());
        }
    }
    for nested in object.values() {
        if let Some(found) = find_usage_object(nested, depth + 1) {
            return Some(found);
        }
    }
    None
}

fn normalize_usage(usage: &Value) -> TokenUsage {
    let input = first_number(
        usage,
        &[
            "input_tokens",
            "prompt_tokens",
            "promptTokenCount",
            "inputTokens",
        ],
    );
    let output = first_number(
        usage,
        &[
            "output_tokens",
            "completion_tokens",
            "candidatesTokenCount",
            "outputTokens",
        ],
    );
    let reasoning = nested_number(
        usage,
        &["output_tokens_details", "completion_tokens_details"],
        "reasoning_tokens",
    )
    .or_else(|| first_number(usage, &["thoughtsTokenCount", "reasoning_tokens"]))
    .unwrap_or(0);
    let cache_read = first_number(
        usage,
        &[
            "cache_read_input_tokens",
            "cached_content_token_count",
            "cachedContentTokenCount",
        ],
    )
    .or_else(|| {
        nested_number(
            usage,
            &["prompt_tokens_details", "input_tokens_details"],
            "cached_tokens",
        )
    })
    .unwrap_or(0);
    let cache_creation = first_number(
        usage,
        &["cache_creation_input_tokens", "cache_creation_tokens"],
    )
    .unwrap_or(0);
    let input = input.unwrap_or(0);
    let output = output.unwrap_or(0);
    let total = first_number(usage, &["total_tokens", "totalTokenCount"]).unwrap_or(input + output);

    TokenUsage {
        input_tokens: input,
        output_tokens: output,
        reasoning_tokens: reasoning,
        cache_read_tokens: cache_read,
        cache_creation_tokens: cache_creation,
        total_tokens: total,
        raw: Some(compact_usage(usage)),
    }
}

/// Codex reports a per-tool-call `attribution` breakdown that dwarfs the usage
/// object itself (tens of KB versus a few hundred bytes) while contributing
/// nothing the archive does not already store in dedicated columns. Drop it so
/// one provider does not dominate the database.
const BULKY_USAGE_KEYS: [&str; 1] = ["attribution"];

fn compact_usage(usage: &Value) -> Value {
    let Value::Object(object) = usage else {
        return usage.clone();
    };
    if !BULKY_USAGE_KEYS.iter().any(|key| object.contains_key(*key)) {
        return usage.clone();
    }
    let mut compacted = object.clone();
    for key in BULKY_USAGE_KEYS {
        compacted.remove(key);
    }
    Value::Object(compacted)
}

fn first_number(value: &Value, keys: &[&str]) -> Option<i64> {
    for key in keys {
        if let Some(number) = value.get(*key).and_then(Value::as_i64) {
            return Some(number);
        }
    }
    None
}

fn nested_number(value: &Value, containers: &[&str], key: &str) -> Option<i64> {
    for container in containers {
        if let Some(number) = value
            .get(*container)
            .and_then(|nested| nested.get(key))
            .and_then(Value::as_i64)
        {
            return Some(number);
        }
    }
    None
}

/// Looks a header up case-insensitively; the core preserves whatever casing the
/// client sent.
pub(crate) fn header_value(headers: &BTreeMap<String, Vec<String>>, name: &str) -> String {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .and_then(|(_, values)| values.first())
        .cloned()
        .unwrap_or_default()
}

/// Extracts the request id the core appends to the log filename
/// (`<path>-<YYYY-MM-DDTHHMMSS>-<request-id>.log`).
pub(crate) fn request_id_from_filename(filename: &str) -> String {
    let stem = filename.strip_suffix(".log").unwrap_or(filename);
    stem.rsplit_once('-')
        .map(|(_, id)| id.to_string())
        .unwrap_or_default()
}

/// Extracts the endpoint path from the logged URL.
pub(crate) fn endpoint_from_url(url: &str) -> String {
    let without_query = url.split('?').next().unwrap_or(url);
    match without_query.find("://") {
        Some(index) => match without_query[index + 3..].find('/') {
            Some(path_index) => without_query[index + 3 + path_index..].to_string(),
            None => "/".to_string(),
        },
        None => without_query.to_string(),
    }
}
