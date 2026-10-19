//! AJ-E1102: the generated types equal `stats/schema.v1.json` exactly —
//! names, kinds, dimensions, enum values, buckets, gauge values and caps.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;

use juggler_stats::{METRICS, SCHEMA_VERSION, StatEvent, StatGauge};
use serde_json::Value;

fn schema() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../stats/schema.v1.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

fn values_of(schema: &Value, spec: &Value) -> Vec<String> {
    spec.as_str()
        .map_or_else(|| strings(spec), |name| strings(&schema["enums"][name]))
}

#[test]
fn the_metadata_table_equals_the_schema() {
    let schema = schema();
    assert_eq!(
        u64::from(SCHEMA_VERSION),
        schema["schema"].as_u64().unwrap()
    );
    let metrics = schema["metrics"].as_object().unwrap();
    assert_eq!(METRICS.len(), metrics.len());
    for (meta, (name, metric)) in METRICS.iter().zip(metrics) {
        assert_eq!(meta.name, name);
        assert_eq!(meta.kind, metric["type"].as_str().unwrap(), "{name}");
        let dims: Vec<(String, Vec<String>)> = metric
            .get("dims")
            .and_then(Value::as_object)
            .map(|d| {
                d.iter()
                    .map(|(k, v)| (k.clone(), values_of(&schema, v)))
                    .collect()
            })
            .unwrap_or_default();
        let got: Vec<(String, Vec<String>)> = meta
            .dims
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.iter().map(|s| (*s).to_owned()).collect()))
            .collect();
        assert_eq!(got, dims, "{name} dims");
        let buckets = metric.get("buckets").map(strings).unwrap_or_default();
        assert_eq!(meta.buckets, buckets.as_slice(), "{name} buckets");
        let values = metric
            .get("values")
            .map(|v| values_of(&schema, v))
            .unwrap_or_default();
        assert_eq!(meta.values, values.as_slice(), "{name} values");
        let default_max = if meta.kind == "histogram" {
            schema["limits"]["histogram_total_max"].as_u64()
        } else {
            schema["limits"]["counter_max"].as_u64()
        };
        assert_eq!(
            Some(meta.max),
            metric.get("max").and_then(Value::as_u64).or(default_max),
            "{name} max"
        );
    }
}

#[test]
fn every_metric_has_exactly_one_typed_variant() {
    let mut seen = BTreeSet::new();
    for event in StatEvent::examples() {
        let meta = METRICS.iter().find(|m| m.name == event.metric()).unwrap();
        assert_ne!(meta.kind, "gauge", "{} is a gauge", meta.name);
        assert_eq!(
            event.observation().is_some(),
            meta.kind == "histogram",
            "{}",
            meta.name
        );
        let dims: Vec<&str> = event.dims().iter().map(|(k, _)| *k).collect();
        let want: Vec<&str> = meta.dims.iter().map(|(k, _)| *k).collect();
        assert_eq!(dims, want, "{} dims", meta.name);
        for (k, v) in event.dims() {
            let allowed = meta.dims.iter().find(|(d, _)| *d == k).unwrap().1;
            assert!(allowed.contains(&v), "{} {k}={v}", meta.name);
        }
        assert!(seen.insert(meta.name), "{} twice", meta.name);
    }
    for gauge in StatGauge::examples() {
        let meta = METRICS.iter().find(|m| m.name == gauge.metric()).unwrap();
        assert_eq!(meta.kind, "gauge", "{}", meta.name);
        let dims: Vec<&str> = gauge.dims().iter().map(|(k, _)| *k).collect();
        let want: Vec<&str> = meta.dims.iter().map(|(k, _)| *k).collect();
        assert_eq!(dims, want, "{} dims", meta.name);
        let value = gauge.value();
        assert!(
            meta.values.is_empty() || meta.values.contains(&value.as_str()),
            "{} value {value}",
            meta.name
        );
        assert!(seen.insert(meta.name), "{} twice", meta.name);
    }
    assert_eq!(seen.len(), METRICS.len(), "a metric has no variant");
}

#[test]
fn the_generated_file_is_current() {
    let script = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../scripts/gen_stats_events.py"
    );
    let status = std::process::Command::new("python3")
        .arg(script)
        .arg("--check")
        .status()
        .unwrap();
    assert!(status.success(), "run scripts/gen_stats_events.py");
}
