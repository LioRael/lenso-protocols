use lenso_process_protocol::{authoring::*, decode_strict};
use serde::de::DeserializeOwned;
use serde_json::Value;

fn fixture() -> Value {
    decode_strict(include_bytes!(
        "../../../fixtures/process-protocol/authoring-v2-conformance.json"
    ))
    .expect("shared Authoring V2 fixture must be strict JSON")
}

fn field<T: DeserializeOwned>(fixture: &Value, name: &str) -> T {
    serde_json::from_value(fixture[name].clone()).expect("fixture value must decode")
}

#[test]
fn shared_authoring_v2_values_validate() {
    let values = fixture();
    let initialize: InitializeParams = field(&values, "initialize");
    initialize.validate().unwrap();
    initialize
        .validate_for_runtime_profile("lenso.process-stdio@2")
        .unwrap();
    initialize.validate_initialized(&initialize).unwrap();
    let identity = &initialize.identity;

    let construct: ConstructParams = field(&values, "construct");
    construct.validate_for(identity).unwrap();
    field::<ConstructedResult>(&values, "constructed")
        .validate_for(&construct)
        .unwrap();

    let invoke: InvokeParams = field(&values, "invoke");
    invoke.validate_for(identity).unwrap();
    invoke.validate_against(&initialize).unwrap();
    field::<InvocationResult>(&values, "result")
        .validate_for(&invoke)
        .unwrap();
    let outbound = field::<OutboundCallParams>(&values, "outbound_call");
    outbound
        .validate_against(&initialize, &invoke.scope, true)
        .unwrap();
    assert_eq!(
        field::<OutboundCallResult>(&values, "result")
            .validate_for_outbound(&outbound)
            .unwrap_err()
            .detail(),
        "response identity mismatch"
    );

    let cancel: CancelParams = field(&values, "cancel");
    cancel.validate_for(identity).unwrap();
    field::<CancelAck>(&values, "cancel_ack")
        .validate_for(&cancel)
        .unwrap();
    field::<Settlement>(&values, "settlement")
        .validate_for(identity)
        .unwrap();
    let stop: StopParams = field(&values, "stop");
    stop.validate_for(identity).unwrap();
    field::<StoppedResult>(&values, "stopped")
        .validate_for(&stop)
        .unwrap();
}

#[test]
fn named_requirements_reject_foreign_and_wrong_descriptor_routes() {
    let values = fixture();
    let mut initialize: InitializeParams = field(&values, "initialize");
    initialize.routes[0].requirement_id = "missing".to_owned();
    assert_eq!(
        initialize.validate().unwrap_err().detail(),
        "route references an unknown requirement_id"
    );

    let mut initialize: InitializeParams = field(&values, "initialize");
    initialize.routes[0].descriptor_version = "2.0.0".to_owned();
    assert_eq!(
        initialize.validate().unwrap_err().detail(),
        "route descriptor does not match its named requirement"
    );
}

#[test]
fn scopes_preserve_parent_authority_and_nonincreasing_budget() {
    let values = fixture();
    let parent = field::<InvokeParams>(&values, "invoke").scope;
    let mut child = field::<OutboundCallParams>(&values, "outbound_call").scope;
    child.remaining_budget_nanos = "900001".to_owned();
    assert_eq!(
        child.validate_child_of(&parent).unwrap_err().detail(),
        "outbound remaining budget may not increase"
    );
    child.remaining_budget_nanos = "800000".to_owned();
    child.permissions.clear();
    assert_eq!(
        child.validate_child_of(&parent).unwrap_err().detail(),
        "outbound scope must preserve permissions and extensions"
    );
}

#[test]
fn closed_parent_and_cross_requirement_routes_reject_before_dispatch() {
    let values = fixture();
    let initialize: InitializeParams = field(&values, "initialize");
    let invoke: InvokeParams = field(&values, "invoke");
    let mut call: OutboundCallParams = field(&values, "outbound_call");
    assert_eq!(
        call.validate_against(&initialize, &invoke.scope, false)
            .unwrap_err()
            .detail(),
        "closed parent scope cannot start an outbound call"
    );
    call.requirement_id = "secondary_store".to_owned();
    assert_eq!(
        call.validate_against(&initialize, &invoke.scope, true)
            .unwrap_err()
            .detail(),
        "outbound route belongs to another requirement"
    );
}

#[test]
fn strict_decoder_rejects_duplicate_and_unknown_fields() {
    assert!(decode_strict::<InitializeParams>(br#"{"api_version":2,"api_version":2}"#).is_err());
    let values = fixture();
    let mut initialize = values["initialize"].clone();
    initialize
        .as_object_mut()
        .unwrap()
        .insert("unknown".to_owned(), Value::Bool(true));
    let wire = serde_json::to_vec(&initialize).unwrap();
    assert!(decode_strict::<InitializeParams>(&wire).is_err());
    assert_eq!(
        decode_frame::<Value>(&vec![b' '; 1_048_577], DEFAULT_MAX_FRAME_BYTES)
            .unwrap_err()
            .detail(),
        "Authoring frame exceeds max_frame_bytes"
    );
}

#[test]
fn runtime_failures_are_structured_and_strict() {
    let values = fixture();
    let invoke: InvokeParams = field(&values, "invoke");
    let mut result = field::<InvocationResult>(&values, "result");
    result.outcome = InvocationOutcome::Runtime {
        failure: RuntimeFailure::ResourceExhausted {
            capability: "example.store@1".to_owned(),
            operation: "get".to_owned(),
        },
    };
    result.validate_for(&invoke).unwrap();

    let malformed = br#"{
        "session":"session-1",
        "correlation_id":"1",
        "outcome":{
            "kind":"runtime",
            "failure":{"kind":"cancelled","request_id":"1","detail":"extra"}
        }
    }"#;
    assert!(decode_strict::<InvocationResult>(malformed).is_err());
}

#[test]
fn exact_initialized_echo_rejects_session_and_profile_changes() {
    let values = fixture();
    let initialize: InitializeParams = field(&values, "initialize");
    let mut altered = initialize.clone();
    altered.identity.session = "session-2".to_owned();
    assert_eq!(
        initialize
            .validate_initialized(&altered)
            .unwrap_err()
            .detail(),
        "initialized result does not exactly echo initialization"
    );
    altered = initialize.clone();
    altered.identity.runtime_profile = "lenso.bun-authoring@2".to_owned();
    assert_eq!(
        altered
            .validate_for_runtime_profile("lenso.process-stdio@2")
            .unwrap_err()
            .detail(),
        "runtime profile does not match the selected Adapter profile"
    );
    assert_eq!(
        initialize
            .validate_initialized(&altered)
            .unwrap_err()
            .detail(),
        "initialized result does not exactly echo initialization"
    );
}
