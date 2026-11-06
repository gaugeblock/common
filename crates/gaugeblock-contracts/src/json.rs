//! The `--json` envelope every suite binary emits (`GB-274`).
//!
//! JSON output is a **contract, not a convenience**: its consumers are scripts that cannot see a shape
//! change. The suite had two shapes for one idea - `storypole list --json` returned a bare array while
//! `show --json` returned an object - and the asymmetry was only the symptom. The disease was that nothing
//! stated what `--json` emitted, so commands diverged and would diverge again after any one-off patch.
//!
//! Three rules, and the third is the one people skip:
//!
//! 1. **Always an object.** A bare array has nowhere to put anything added later - a count, echoed filters,
//!    warnings, provenance - so every future need re-opens the same break.
//! 2. **The payload key names the thing** (`tasks`, `task`, `receipt`), never `data`.
//! 3. **`schema_version` ships from day one**, even though it is `1` and will stay `1` through the beta.
//!    Adding it later is the breaking change it exists to prevent.
//!
//! No `--json=payload` escape hatch: a second mode recreates exactly the inconsistency this removes.

use serde::Serialize;
use serde_json::{Map, Value};

/// Version of the envelope and of the payload shapes carried inside it.
///
/// Deliberately `1` for the whole `0.1.x` line: consumers are the suite's own scripts, and bumping per
/// change would defeat the point of having a version at all. A bump means "a consumer must change".
pub const SCHEMA_VERSION: u32 = 1;

/// Wraps `payload` in the suite envelope: `{ "schema_version": 1, "command": "list", ... }`.
///
/// # Errors
///
/// Returns the serialisation error rather than emitting a partial envelope: a consumer receiving
/// `{"schema_version":1,"command":"list"}` with the data silently missing would have to guess whether the
/// command produced nothing or the envelope lost it.
pub fn envelope<T: Serialize>(command: &str, payload: &T) -> Result<Value, serde_json::Error> {
    let mut envelope = Map::new();
    envelope.insert("schema_version".to_string(), Value::from(SCHEMA_VERSION));
    envelope.insert("command".to_string(), Value::from(command));

    match serde_json::to_value(payload)? {
        Value::Object(fields) => {
            for (key, value) in fields {
                envelope.insert(key, value);
            }
        }
        // A non-object payload cannot be merged without losing its name, so it is nested under `payload`
        // rather than dropped - valid JSON with the data reachable beats an error at every call site.
        other => {
            envelope.insert("payload".to_string(), other);
        }
    }

    Ok(Value::Object(envelope))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn an_object_payload_is_merged_beside_the_envelope_keys() {
        let payload = serde_json::json!({ "tasks": [{ "key": "GB-1" }], "count": 1 });
        let value = envelope("list", &payload).unwrap();

        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["command"], "list");
        assert_eq!(value["count"], 1);
        assert_eq!(value["tasks"][0]["key"], "GB-1");
    }

    #[test]
    fn a_non_object_payload_is_nested_rather_than_lost() {
        let value = envelope("list", &vec!["a", "b"]).unwrap();

        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["payload"][1], "b", "data stays reachable");
    }

    #[test]
    fn the_version_is_present_so_a_future_change_has_somewhere_to_live() {
        let value = envelope("show", &serde_json::json!({ "task": {} })).unwrap();

        assert!(
            value.get("schema_version").is_some(),
            "adding this later would be the breaking change it prevents"
        );
    }
}
