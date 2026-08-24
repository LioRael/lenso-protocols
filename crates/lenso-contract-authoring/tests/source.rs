#![allow(dead_code)]

use lenso_contract_authoring as lenso;

#[derive(lenso::JsonSchema)]
#[schemars(deny_unknown_fields)]
struct PingRequest {
    #[schemars(length(min = 1, max = 64))]
    value: String,
}

#[derive(lenso::JsonSchema)]
#[schemars(deny_unknown_fields)]
struct PingResponse {
    value: String,
}

#[derive(lenso::JsonSchema)]
#[schemars(deny_unknown_fields)]
struct RejectedPayload {
    reason: String,
}

#[derive(lenso::DomainError)]
enum PingError {
    Unavailable,
    Rejected { payload: RejectedPayload },
}

#[lenso::capability(
    id = "example.ping",
    major = 1,
    version = "1.0.0",
    portable = true,
    cross_lane_transfer = false
)]
trait Ping {
    async fn ping(
        &self,
        context: lenso::Ctx<'_>,
        request: PingRequest,
    ) -> Result<PingResponse, PingError>;
}

#[test]
fn annotated_trait_derives_identity_operations_values_and_errors() {
    let snapshot = __lenso_capability_snapshot();
    assert_eq!(snapshot.capability_id, "example.ping@1");
    assert_eq!(snapshot.version, "1.0.0");
    assert!(snapshot.portable);
    assert!(!snapshot.cross_lane_transfer);
    assert_eq!(snapshot.operations.len(), 1);
    let operation = &snapshot.operations[0];
    assert_eq!(operation.name, "ping");
    assert_eq!(operation.interaction, "request");
    assert_eq!(operation.request_schema["additionalProperties"], false);
    assert_eq!(
        operation.request_schema["properties"]["value"]["minLength"],
        1
    );
    assert_eq!(
        operation.request_schema["properties"]["value"]["maxLength"],
        64
    );
    assert_eq!(
        operation.domain_error_schema["oneOf"][0]["const"],
        "unavailable"
    );
    assert_eq!(
        operation.domain_error_schema["oneOf"][1]["properties"]["code"]["const"],
        "rejected"
    );
}
