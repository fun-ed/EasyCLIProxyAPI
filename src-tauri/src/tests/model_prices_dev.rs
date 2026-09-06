//! Tests for the models.dev price source.

#[test]
fn models_dev_payload_becomes_a_schema_one_catalog() {
    use crate::model_prices_dev::testing::to_price_catalog_json;

    let payload = include_str!("fixtures/models-dev-sample.txt");
    let catalog = to_price_catalog_json(payload, "2026-09-06").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&catalog).unwrap();

    assert_eq!(parsed["schemaVersion"], 1);
    assert_eq!(parsed["updatedAt"], "2026-09-06");

    let opus = &parsed["models"]["claude-opus-5"];
    assert_eq!(opus["inputPer1M"], 5.0);
    assert_eq!(opus["outputPer1M"], 25.0);
    assert_eq!(opus["cacheReadPer1M"], 0.5);
    assert_eq!(opus["cacheCreationPer1M"], 6.25);

    // Only the base tier is imported; the long-context surcharge has no column.
    let terra = &parsed["models"]["gpt-5.6-terra"];
    assert_eq!(terra["inputPer1M"], 2.0);
    assert_eq!(terra["outputPer1M"], 12.0);

    // A model without a cost block is skipped rather than priced at zero.
    assert!(parsed["models"].get("no-cost-model").is_none());
}

#[test]
fn models_dev_rejects_unusable_payloads() {
    use crate::model_prices_dev::testing::to_price_catalog_json;

    assert!(to_price_catalog_json("not json", "2026-09-06").is_err());
    assert!(to_price_catalog_json(r#"{"p":{"models":{}}}"#, "2026-09-06").is_err());
    assert!(
        to_price_catalog_json(r#"{"p":{"models":{"m":{}}}}"#, "2026-09-06").is_err(),
        "a catalog with no priced model must fail so the caller falls back"
    );
}

#[test]
fn first_party_vendors_outrank_resellers_and_the_result_is_stable() {
    use crate::model_prices_dev::testing::to_price_catalog_json;

    // `aaa-reseller` sorts before `anthropic` and `openai` alphabetically and
    // quotes deliberately wrong prices, so a plain alphabetical or hash-ordered
    // walk would import them.
    let payload = include_str!("fixtures/models-dev-sample.txt");
    let first = to_price_catalog_json(payload, "2026-09-06").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&first).unwrap();

    assert_eq!(parsed["models"]["claude-opus-5"]["inputPer1M"], 5.0);
    assert_eq!(parsed["models"]["gpt-5.6-terra"]["inputPer1M"], 2.0);
    assert_eq!(parsed["models"]["gpt-5.6-terra"]["outputPer1M"], 12.0);

    // HashMap iteration order is randomised per process, so the conversion must
    // not depend on it: repeated runs have to agree byte for byte.
    for _ in 0..8 {
        assert_eq!(to_price_catalog_json(payload, "2026-09-06").unwrap(), first);
    }
}
