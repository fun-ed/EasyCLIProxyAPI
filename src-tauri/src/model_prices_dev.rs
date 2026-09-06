//! models.dev as the primary model price source.
//!
//! The catalog bundled with the app (and its GitHub mirror) is a hand-maintained
//! list that lags behind new releases: at the time of writing it carried 57
//! models and knew nothing about `claude-opus-5`, so every request to that model
//! was left uncosted. models.dev tracks releases far more closely and publishes
//! per-million-token pricing for every provider it covers.
//!
//! This module only translates models.dev into the catalog schema that
//! `usage.rs` already understands, so the parsing, storage, manual-override and
//! fallback logic all stay exactly where they were.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{json, Value};

pub(crate) const MODELS_DEV_URL: &str = "https://models.dev/api.json";

/// models.dev is a provider map, and the same model id appears under many
/// providers with *different* prices: resellers list their own rates alongside
/// the vendor's. `gpt-5.6-terra`, for example, is 2/12 under `openai` and 2.5/15
/// under a reseller.
///
/// Iterating a `HashMap` would therefore pick a different price on each run,
/// because Rust randomises hash iteration order per process. Providers are
/// instead visited in a fixed order: first-party vendors first, then everyone
/// else alphabetically, and the first entry for a model id wins.
const FIRST_PARTY_PROVIDERS: [&str; 10] = [
    "anthropic",
    "openai",
    "google",
    "google-vertex",
    "xai",
    "deepseek",
    "mistral",
    "meta",
    "moonshotai",
    "amazon-bedrock",
];

fn provider_rank(id: &str) -> usize {
    FIRST_PARTY_PROVIDERS
        .iter()
        .position(|candidate| *candidate == id)
        .unwrap_or(FIRST_PARTY_PROVIDERS.len())
}

#[derive(Deserialize)]
struct Provider {
    #[serde(default)]
    models: HashMap<String, Model>,
}

#[derive(Deserialize)]
struct Model {
    #[serde(default)]
    cost: Option<Cost>,
}

/// Prices are per million tokens, matching the catalog schema.
///
/// `tiers` and `context_over_200k` describe long-context surcharges that the
/// usage database has no column for, so only the base tier is imported.
#[derive(Deserialize)]
struct Cost {
    #[serde(default)]
    input: Option<f64>,
    #[serde(default)]
    output: Option<f64>,
    #[serde(default)]
    cache_read: Option<f64>,
    #[serde(default)]
    cache_write: Option<f64>,
}

/// Converts a models.dev payload into the schema-1 catalog JSON consumed by
/// `parse_model_price_catalog`.
pub(crate) fn to_price_catalog_json(payload: &str, updated_at: &str) -> Result<String, String> {
    let providers = serde_json::from_str::<HashMap<String, Provider>>(payload)
        .map_err(|error| format!("解析 models.dev 价格数据失败: {error}"))?;

    let mut ordered = providers.into_iter().collect::<Vec<_>>();
    ordered.sort_by(|(left, _), (right, _)| {
        provider_rank(left)
            .cmp(&provider_rank(right))
            .then_with(|| left.cmp(right))
    });

    let mut models = serde_json::Map::new();
    for (_, provider) in ordered {
        let mut provider_models = provider.models.into_iter().collect::<Vec<_>>();
        provider_models.sort_by(|(left, _), (right, _)| left.cmp(right));
        for (model_id, model) in provider_models {
            let model_id = model_id.trim();
            if model_id.is_empty() || models.contains_key(model_id) {
                continue;
            }
            let Some(cost) = model.cost else { continue };
            // A model priced at zero on both axes carries no usable information
            // and would mask a later provider that does publish real numbers.
            let (Some(input), Some(output)) = (cost.input, cost.output) else {
                continue;
            };
            if !input.is_finite() || !output.is_finite() || input < 0.0 || output < 0.0 {
                continue;
            }

            let mut entry = serde_json::Map::new();
            entry.insert("inputPer1M".to_string(), json!(input));
            entry.insert("outputPer1M".to_string(), json!(output));
            if let Some(value) = cost.cache_read.filter(|v| v.is_finite() && *v >= 0.0) {
                entry.insert("cacheReadPer1M".to_string(), json!(value));
            }
            if let Some(value) = cost.cache_write.filter(|v| v.is_finite() && *v >= 0.0) {
                entry.insert("cacheCreationPer1M".to_string(), json!(value));
            }
            models.insert(model_id.to_string(), Value::Object(entry));
        }
    }

    if models.is_empty() {
        return Err("models.dev 价格数据中没有可用的模型".to_string());
    }

    let catalog = json!({
        "schemaVersion": 1,
        "updatedAt": updated_at,
        "models": Value::Object(models),
    });
    serde_json::to_string(&catalog)
        .map_err(|error| format!("生成模型价格目录失败: {error}"))
}

#[cfg(test)]
pub(crate) mod testing {
    pub(crate) use super::to_price_catalog_json;
}
