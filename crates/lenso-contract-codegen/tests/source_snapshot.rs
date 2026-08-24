use std::path::Path;

use lenso_contract_authoring::{CapabilitySnapshot, OperationSnapshot};
use lenso_contract_codegen::{
    CodegenError, check_source_snapshot, generate, write_source_snapshot,
};
use serde_json::json;

fn snapshot() -> CapabilitySnapshot {
    CapabilitySnapshot {
        capability_id: "example.derived@1".to_owned(),
        version: "1.0.0".to_owned(),
        portable: true,
        cross_lane_transfer: false,
        operations: vec![OperationSnapshot {
            name: "run".to_owned(),
            interaction: "request".to_owned(),
            request_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            response_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            domain_error_schema: json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "oneOf": [{"const": "rejected"}]
            }),
        }],
    }
}

#[test]
fn source_snapshots_are_deterministic_and_drift_checked() {
    let root = tempfile::tempdir().unwrap();
    let descriptor = root.path().join("capability.json");
    write_source_snapshot(&snapshot(), &descriptor).unwrap();
    check_source_snapshot(&snapshot(), &descriptor).unwrap();
    let generated = generate(&descriptor).unwrap();
    assert_eq!(generated.metadata.capability_id, "example.derived@1");
    assert!(generated.rust.contains("pub trait DerivedProvider"));
    assert!(
        generated
            .typescript
            .contains("export interface DerivedProvider")
    );

    std::fs::write(root.path().join("schemas/run-request.schema.json"), "{}\n").unwrap();
    assert!(matches!(
        check_source_snapshot(&snapshot(), &descriptor),
        Err(CodegenError::GeneratedArtifactDrift { .. })
    ));
    assert!(Path::new(&descriptor).exists());
}
