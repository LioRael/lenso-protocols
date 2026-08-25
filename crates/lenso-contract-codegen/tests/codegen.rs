use std::path::Path;

use lenso_contract_codegen::{
    CodegenError, CompatibilityError, ProjectionLanguage, check_generated, check_projection,
    generate, generate_browser_request_client, generate_projection, lint_compatibility,
    load_descriptor, round_trip_portable_json, validate_wire_value, write_generated,
    write_projection,
};
use serde_json::json;

const FIXTURE: &str = "tests/fixtures/profile/capability.json";
const STREAM_FIXTURE: &str = "tests/fixtures/stream/capability.json";
const EVENT_FIXTURE: &str = "tests/fixtures/event/capability.json";
const SENSITIVE_FIXTURE: &str = "tests/fixtures/sensitive/capability.json";
const TRANSFER_FIXTURE: &str = "tests/fixtures/transfer/capability.json";
const WIT_FIXTURE: &str = "tests/fixtures/wit/capability.json";

#[test]
fn request_descriptor_generates_exact_runtime_codec_projection() {
    let projection = generate_projection(Path::new(WIT_FIXTURE), ProjectionLanguage::RustRuntime)
        .expect("portable Request Descriptor should project to a runtime codec");

    assert!(
        projection
            .source
            .contains("impl lenso_runtime_codec::JsonCapabilityCodec for GreetingJsonCodec")
    );
    assert!(
        projection
            .source
            .contains("request.downcast_ref::<EchoRequest>()")
    );
    assert!(
        projection
            .source
            .contains("serde_json::from_value::<EchoResponse>")
    );
    assert!(
        projection
            .source
            .contains("serde_json::from_value::<EchoError>")
    );
    assert!(projection.source.contains("&[ECHO_OPERATION]"));
    assert!(projection.source.contains("fn invoke_host_request(&self"));
    assert!(
        projection
            .source
            .contains("dependency.typed::<Greeting>()?")
    );
    assert!(
        projection
            .source
            .contains("invoke_with_context(ECHO_OPERATION, context, request)")
    );
    assert!(projection.source.contains(
        "fn open_host_stream(&self, _dependency: lenso_kernel::ModuleStreamDependencyHandle"
    ));
    assert!(
        projection
            .source
            .contains("operation: &str, _request: &dyn std::any::Any")
    );
    assert!(
        projection
            .source
            .contains("operation: &str, _message: &dyn std::any::Any")
    );
    assert!(
        projection
            .source
            .contains("        Err(runtime_codec_unknown_operation(operation))")
    );
}

#[test]
fn multi_operation_runtime_codec_uses_generated_composite_markers() {
    let projection = generate_projection(Path::new(FIXTURE), ProjectionLanguage::RustRuntime)
        .expect("portable multi-operation Descriptor should project to a runtime codec");

    assert!(
        projection
            .source
            .contains("dependency.typed::<ProfileRoundTrip>()?")
    );
    assert!(
        projection
            .source
            .contains("dependency.typed::<ProfileCorpusRoundTrip>()?")
    );
}

#[test]
fn stream_descriptor_generates_exact_runtime_codec_projection() {
    let projection =
        generate_projection(Path::new(STREAM_FIXTURE), ProjectionLanguage::RustRuntime)
            .expect("portable Stream Descriptor should project to a runtime codec");

    assert!(projection.source.contains("stream_operations(&self)"));
    assert!(projection.source.contains("&[CHAT_OPERATION]"));
    assert!(projection.source.contains("fn open_host_stream(&self"));
    assert!(
        projection
            .source
            .contains("dependency.typed::<Conversation>()?")
    );
    assert!(
        projection
            .source
            .contains("lenso_runtime_codec::json_host_stream::<Conversation>")
    );
    assert!(
        projection
            .source
            .contains("operation: &str, _request: &dyn std::any::Any")
    );
    assert!(
        projection
            .source
            .contains("operation: &str, _value: serde_json::Value")
    );
    assert!(
        projection
            .source
            .contains("request.downcast_ref::<ChatRequest>()")
    );
    assert!(
        projection
            .source
            .contains("message.downcast_ref::<ChatResponse>()")
    );
    assert!(
        projection
            .source
            .contains("serde_json::from_value::<ChatResponse>")
    );
    assert!(
        projection
            .source
            .contains("serde_json::from_value::<ChatError>")
    );
    assert!(
        projection
            .source
            .contains("        Err(runtime_codec_unknown_operation(operation))")
    );
    assert!(!projection.source.contains(
        "match operation {\n            _ => Err(runtime_codec_unknown_operation(operation))"
    ));
}

#[test]
fn portable_request_descriptor_generates_deterministic_provider_and_consumer_wit() {
    let projection = generate_projection(Path::new(WIT_FIXTURE), ProjectionLanguage::Wit)
        .expect("portable request Descriptor should project to WIT");

    assert!(
        projection
            .source
            .starts_with("// @generated by lenso-contract-codegen")
    );
    assert!(
        projection
            .source
            .contains("package example:greeting@1.0.0;")
    );
    assert!(projection.source.contains("interface capability"));
    assert!(projection.source.contains("world provider"));
    assert!(projection.source.contains("world consumer"));
    assert!(projection.source.contains("echo: func("));
    wit_parser::Resolve::default()
        .push_str("generated.wit", &projection.source)
        .expect("generated WIT should parse as a Component Model package");
    check_projection(
        Path::new(WIT_FIXTURE),
        ProjectionLanguage::Wit,
        Path::new("tests/fixtures/wit/generated/capability.wit"),
    )
    .expect("checked-in WIT should match its Descriptor IR");
    assert_eq!(
        projection.source,
        generate_projection(Path::new(WIT_FIXTURE), ProjectionLanguage::Wit)
            .unwrap()
            .source
    );
}

#[test]
fn stream_descriptor_fails_wit_projection_closed() {
    assert!(matches!(
        generate_projection(Path::new(STREAM_FIXTURE), ProjectionLanguage::Wit),
        Err(CodegenError::UnsupportedInteraction { .. })
    ));
}

#[test]
fn descriptor_marks_generated_types_for_cross_lane_transfer() {
    let artifacts = generate(Path::new(TRANSFER_FIXTURE))
        .expect("cross-lane transfer Descriptor should generate");

    assert!(artifacts.metadata.cross_lane_transfer);
    assert!(
        artifacts
            .rust
            .contains("pub const CROSS_LANE_TRANSFER: bool = true;")
    );
    assert!(
        artifacts
            .typescript
            .contains("export const CROSS_LANE_TRANSFER = true;")
    );
}

#[test]
fn browser_request_clients_share_descriptor_identity_and_result_envelopes() {
    let client = generate_browser_request_client(Path::new(FIXTURE)).unwrap();

    assert!(client.contains("@generated by lenso-contract-codegen"));
    assert!(client.contains("from \"@lenso/contract-runtime/browser\""));
    assert!(client.contains("/api/capabilities/example.profile@1/round_trip"));
    assert!(client.contains("validateSchema(request"));
    assert!(client.contains("validateResult(result"));
    assert!(!client.contains("function validatePortableJson"));
    assert!(!client.contains("function matchesSchema"));
    assert!(!client.contains("response.ok"));
}

#[test]
fn sensitive_schema_fields_generate_redacted_rust_debug() {
    let artifacts =
        generate(Path::new(SENSITIVE_FIXTURE)).expect("sensitive Descriptor should generate");

    assert!(
        artifacts
            .rust
            .contains("impl fmt::Debug for InspectRequestCredential")
    );
    assert!(
        artifacts
            .rust
            .contains(".field(\"value\", &\"<redacted>\")")
    );
    assert!(
        artifacts
            .rust
            .contains("impl fmt::Debug for InspectResponseAssertion")
    );
    assert!(
        artifacts
            .rust
            .contains(".field(\"proof\", &\"<redacted>\")")
    );
    assert!(artifacts.typescript.contains("value: string"));
}

#[test]
fn stream_descriptors_generate_bidirectional_rust_and_typescript_bindings() {
    let artifacts = generate(Path::new(STREAM_FIXTURE)).expect("stream Descriptor should generate");

    assert!(artifacts.rust.contains("impl StreamCapability"));
    assert!(artifacts.rust.contains("NativeStreamEndpoint"));
    assert!(artifacts.rust.contains("NativeStreamHandle"));
    assert!(artifacts.rust.contains("StreamEvent"));
    assert!(
        artifacts
            .rust
            .contains("pub struct ConversationEndpoint<P: ConversationProvider>")
    );
    assert!(!artifacts.rust.contains("ConversationRequestEndpoint"));
    assert!(artifacts.rust.contains("fn chat("));
    assert!(
        artifacts
            .rust
            .contains("macro_rules! __lenso_provided_conversation")
    );
    assert!(
        artifacts
            .rust
            .contains("macro_rules! __lenso_native_provide_conversation")
    );
    assert!(
        artifacts
            .rust
            .contains("macro_rules! __lenso_native_endpoints_conversation")
    );
    assert!(
        artifacts
            .rust
            .contains("$crate::__lenso_native_endpoints_conversation!($provider, $support)")
    );
    assert!(
        artifacts
            .rust
            .contains("__LensoNativeSupport::NativeModuleInstance::with_all_endpoints")
    );
    assert!(!artifacts.rust.contains("pub use lenso_native_adapter"));
    assert!(!artifacts.rust.contains("dyn $support::"));
    assert!(
        !artifacts
            .rust
            .contains("::lenso_native_adapter::NativeModuleInstance::with_all_endpoints")
    );
    assert!(artifacts.typescript.contains("StreamEvent"));
    assert!(artifacts.typescript.contains(
        "export type StreamSession<Message, DomainError> = lensoContractRuntime.StreamSession<Message, DomainError>;"
    ));
    assert!(!artifacts.typescript.contains("send(message:"));
    assert!(
        artifacts.typescript.contains(
            "chat(context: InvocationContext, request: ChatRequest): Promise<ChatResult>;"
        )
    );
}

#[test]
fn event_descriptors_generate_ephemeral_fan_out_bindings() {
    let artifacts = generate(Path::new(EVENT_FIXTURE)).expect("Event Descriptor should generate");

    assert!(artifacts.rust.contains("impl EventCapability"));
    assert!(artifacts.rust.contains("NativeEventEndpoint"));
    assert!(artifacts.rust.contains("NativeEventHandle"));
    assert!(artifacts.rust.contains("EventPublishResult"));
    assert!(
        artifacts
            .rust
            .contains("macro_rules! __lenso_native_endpoints_notifications")
    );
    assert!(artifacts.rust.contains(
        "fn notify(&self, context: InvocationContext, event: NotifyRequest) -> LocalBoxFuture<'static, Result<(), RuntimeFailure>>;"
    ));
    assert!(
        artifacts.rust.contains(
            "impl __LensoIntoNotificationsNotifyEventResult for Result<(), RuntimeFailure>"
        )
    );
    assert!(
        artifacts
            .rust
            .contains("let result = <$module>::notify(&module, context, event).await;")
    );
    assert!(!artifacts.rust.contains("provider.notify(context, *event);"));
    assert!(
        artifacts
            .rust
            .contains("pub struct NotificationsEndpoint<P: NotificationsProvider>")
    );
    assert!(!artifacts.rust.contains("NotificationsRequestEndpoint"));
    assert!(artifacts.rust.contains("notify: NativeEventHandle<"));
    assert!(
        artifacts
            .rust
            .contains("pub fn new(handle: NativeEventHandle<")
    );
    assert!(
        !artifacts
            .rust
            .contains("pub fn new(handles: Vec<NativeEventHandle<")
    );
    assert!(!artifacts.rust.contains("futures::future::join_all"));
    assert!(artifacts.rust.contains("encode_notify_event"));
    assert!(artifacts.rust.ends_with('\n'));
    assert!(!artifacts.rust.ends_with("\n\n"));
    assert!(artifacts.typescript.contains("EventPublishResult"));
    assert!(
        artifacts
            .typescript
            .contains("export type EventPublishResult = lensoContractRuntime.EventPublishResult;")
    );
    assert!(artifacts.typescript.contains("encodeNotifyEvent"));
    assert!(
        artifacts
            .typescript
            .contains("export type NotifyResult = ReadonlyArray<EventPublishResult>;")
    );
    assert!(artifacts.typescript.contains(
        "notify(event: NotifyRequest, context?: InvocationContext): Promise<NotifyResult>;"
    ));
    assert!(
        artifacts
            .typescript
            .contains("notify(context: InvocationContext, event: NotifyRequest")
    );
}

#[allow(clippy::too_many_lines)]
#[test]
fn one_descriptor_generates_matching_rust_and_typescript_bindings() {
    let artifacts = generate(Path::new(FIXTURE)).expect("profile Descriptor should generate");

    assert_eq!(artifacts.metadata.capability_id, "example.profile@1");
    assert_eq!(artifacts.metadata.descriptor_version, "1.0.0");
    assert!(artifacts.metadata.portable);
    assert!(
        artifacts
            .rust
            .contains("pub const CAPABILITY_ID: &str = \"example.profile@1\";")
    );
    assert!(artifacts.rust.contains("pub use lenso_contract_runtime::{"));
    assert!(artifacts.rust.contains("Bytes, Duration, Int64"));
    assert!(artifacts.rust.contains("UnknownDomainError"));
    assert!(!artifacts.rust.contains("pub struct Bytes("));
    assert!(!artifacts.rust.contains("fn validate_portable_json_value"));
    assert!(artifacts.rust.contains("fn invoke_native("));
    assert!(artifacts.rust.contains(".typed_endpoint()"));
    assert!(
        artifacts
            .rust
            .contains("invoke_typed_or_erased_native_request::<Self>")
    );
    assert!(
        artifacts
            .rust
            .contains("-> NativeRequestFuture<ProfileRoundTrip>;")
    );
    assert!(
        artifacts
            .rust
            .contains("struct ProfileRequestEndpoint { provider: Rc<dyn ProfileProvider> }")
    );
    assert!(
        artifacts
            .rust
            .contains("pub struct ProfileEndpoint<P: ProfileProvider>")
    );
    assert!(artifacts.rust.contains("OptionalValue, Timestamp, Uint64"));
    assert!(artifacts.rust.contains("pub signed: Int64"));
    assert!(artifacts.rust.contains("pub enum RoundTripRequestKind"));
    assert!(artifacts.rust.contains("pub unsigned: Uint64"));
    assert!(artifacts.rust.contains("pub payload: Bytes"));
    assert!(artifacts.rust.contains("pub timestamp: Timestamp"));
    assert!(artifacts.rust.contains("pub duration: Duration"));
    assert!(artifacts.rust.contains("pub values: Vec<i64>"));
    assert!(
        artifacts
            .rust
            .contains("pub nullable_values: Option<Vec<Option<String>>>")
    );
    assert!(
        artifacts
            .rust
            .contains("pub nullable_map: Option<std::collections::BTreeMap<String, Option<i64>>>")
    );
    assert!(
        artifacts
            .rust
            .contains("pub optional_note: OptionalValue<String>")
    );
    assert!(artifacts.rust.contains("UnknownDomainError"));
    assert!(artifacts.rust.contains("serde::Serialize"));
    assert!(artifacts.rust.contains("encode_round_trip_request"));
    assert!(artifacts.rust.contains("decode_round_trip_error"));
    assert!(
        artifacts
            .rust
            .contains("macro_rules! __lenso_required_profile_client")
    );
    assert!(
        artifacts
            .rust
            .contains("RateLimited { payload: RoundTripErrorRateLimitedPayload }")
    );
    assert!(
        artifacts
            .typescript
            .contains("export const CAPABILITY_ID = \"example.profile@1\";")
    );
    assert!(
        artifacts
            .typescript
            .contains("export type Int64 = lensoContractRuntime.Int64;")
    );
    assert!(
        artifacts
            .typescript
            .contains("export type Bytes = lensoContractRuntime.Bytes;")
    );
    assert!(artifacts.typescript.contains("signed: Int64"));
    assert!(artifacts.typescript.contains("kind?: \"alpha\" | \"beta\""));
    assert!(artifacts.typescript.contains("unsigned: Uint64"));
    assert!(artifacts.typescript.contains("payload: Bytes"));
    assert!(artifacts.typescript.contains("timestamp: Timestamp"));
    assert!(artifacts.typescript.contains("duration: Duration"));
    assert!(artifacts.typescript.contains("values: Array<number>"));
    assert!(
        artifacts
            .typescript
            .contains("nullable_values?: Array<string | null>")
    );
    assert!(
        artifacts
            .typescript
            .contains("nullable_map?: Record<string, number | null>")
    );
    assert!(
        artifacts
            .typescript
            .contains("optional_note?: string | null")
    );
    assert!(
        artifacts
            .typescript
            .contains("export type UnknownDomainError = lensoContractRuntime.UnknownDomainError;")
    );
    assert!(
        artifacts
            .typescript
            .contains("export type RuntimeFailure = lensoContractRuntime.RuntimeFailure;")
    );
    assert!(
        artifacts
            .typescript
            .contains("export type InvocationContext = lensoContractRuntime.InvocationContext;")
    );
    assert!(artifacts.typescript.contains("encodeRoundTripRequest"));
    assert!(
        artifacts
            .typescript
            .contains("round_trip(context: InvocationContext")
    );
    assert!(artifacts.typescript.contains("Promise<RoundTripResult>"));
    assert!(
        artifacts
            .typescript
            .contains("export function bindProfileProvider(")
    );
    assert!(
        artifacts
            .typescript
            .contains("export type Provider = ProfileProvider;")
    );
    assert!(
        artifacts
            .typescript
            .contains("export const bindProvider = bindProfileProvider;")
    );
    assert!(
        artifacts
            .typescript
            .contains("async invokeRequest(operation, context, payload)")
    );
    assert!(
        artifacts
            .typescript
            .contains("operations: [\"corpus_round_trip\", \"round_trip\"]")
    );
    assert!(artifacts.typescript.contains("case \"round_trip\":"));
    assert!(
        artifacts
            .typescript
            .contains("readonly payload: RoundTripErrorRateLimitedPayload")
    );
    assert!(
        artifacts
            .typescript
            .contains("readonly ok: false; readonly error: RoundTripInvocationError")
    );
    assert!(
        artifacts
            .typescript
            .contains("readonly code: \"rate_limited\"")
    );
    assert!(
        artifacts
            .rust
            .contains("OptionalData { payload: OptionalValue<String> }")
    );
    assert!(
        artifacts
            .typescript
            .contains("readonly code: \"optional_data\"; readonly payload?: string | null")
    );
}

#[test]
fn contracts_without_bytes_reuse_the_contract_runtime_prelude() {
    let artifacts = generate(Path::new(EVENT_FIXTURE)).expect("event Descriptor should generate");

    assert!(artifacts.rust.contains("pub use lenso_contract_runtime::{"));
    assert!(artifacts.rust.contains("encode_portable_json"));
    assert!(!artifacts.rust.contains("fn decode_base64"));
    let runtime_import = artifacts
        .rust
        .lines()
        .find(|line| line.starts_with("pub use lenso_contract_runtime::"))
        .unwrap();
    assert!(!runtime_import.contains("Bytes"));
    assert!(
        artifacts
            .typescript
            .contains("import * as lensoContractRuntime from \"@lenso/contract-runtime\";")
    );
    assert!(
        artifacts
            .typescript
            .contains("lensoContractRuntime.encodePortableJson")
    );
    assert!(
        !artifacts
            .typescript
            .contains("function validatePortableJson")
    );
    assert!(!artifacts.typescript.contains("function isRecord"));
}

#[test]
fn generation_is_deterministic_and_metadata_does_not_depend_on_module_version() {
    let first = generate(Path::new(FIXTURE)).expect("first generation should succeed");
    let second = generate(Path::new(FIXTURE)).expect("second generation should succeed");

    assert_eq!(first.rust, second.rust);
    assert_eq!(first.typescript, second.typescript);
    assert!(!first.rust.contains("9.4.2"));
    assert!(!first.typescript.contains("9.4.2"));
    assert_eq!(first.metadata.capability_id, "example.profile@1");
    assert_eq!(first.metadata.descriptor_version, "1.0.0");
}

#[test]
fn additive_minor_evolution_is_accepted() {
    let result = lint_compatibility(
        Path::new("tests/fixtures/compatibility/base.json"),
        Path::new("tests/fixtures/compatibility/additive-minor.json"),
    );

    assert!(result.expect("documented additive minor changes should pass"));
}

#[test]
fn breaking_minor_evolution_requires_a_new_major_identity() {
    let error = lint_compatibility(
        Path::new("tests/fixtures/compatibility/base.json"),
        Path::new("tests/fixtures/compatibility/breaking-minor.json"),
    )
    .expect_err("breaking minor changes must be rejected");

    assert!(matches!(error, CompatibilityError::BreakingChanges { .. }));
}

#[test]
fn patch_evolution_cannot_change_the_observable_contract() {
    let error = lint_compatibility(
        Path::new("tests/fixtures/compatibility/base.json"),
        Path::new("tests/fixtures/compatibility/patch-with-change.json"),
    )
    .expect_err("patch changes must be rejected");

    assert!(matches!(error, CompatibilityError::BreakingChanges { .. }));
}

#[test]
fn descriptor_validation_keeps_capability_major_independent_from_semver() {
    let descriptor = load_descriptor(Path::new(FIXTURE)).expect("fixture should be valid");

    assert_eq!(descriptor.capability_id(), "example.profile@1");
    assert_eq!(descriptor.capability_major(), 1);
    assert_eq!(descriptor.version(), "1.0.0");
}

#[test]
fn shared_conformance_values_round_trip_without_precision_loss() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/portable-contract/conformance.json"
    ))
    .expect("the shared conformance corpus should be valid JSON");

    for value in corpus.as_array().expect("the corpus should be an array") {
        let round_tripped = round_trip_portable_json(&value["wire"])
            .expect("portable wire values should round-trip");
        assert_eq!(round_tripped, value["wire"]);
    }
}

#[test]
fn raw_wide_json_numbers_are_rejected_before_javascript_can_round_them() {
    let error = round_trip_portable_json(&json!(18_446_744_073_709_551_615_u64))
        .expect_err("uint64 must be encoded as a decimal string");

    assert!(error.to_string().contains("decimal-string"));
}

#[test]
fn wire_values_are_checked_against_portable_formats_and_schema_shape() {
    let schema = Path::new("tests/fixtures/profile/schemas/round-trip-request.schema.json");
    let valid = json!({
        "name": "Ada",
        "signed": "-9223372036854775808",
        "unsigned": "18446744073709551615",
        "payload": "AQI=",
        "timestamp": "2026-08-21T12:34:56.123Z",
        "duration": "PT1.5S",
        "values": [1, 2, 3],
        "nullable_values": ["one", null],
        "nullable_map": {"first": 1, "second": null},
        "optional_note": null
    });

    validate_wire_value(schema, &valid).expect("valid portable values should pass");

    let mut invalid = valid.clone();
    invalid["signed"] = json!("9223372036854775808");
    assert!(validate_wire_value(schema, &invalid).is_err());

    let mut invalid = valid.clone();
    invalid["payload"] = json!("not base64");
    assert!(validate_wire_value(schema, &invalid).is_err());

    let mut invalid = valid;
    invalid["optional_note"] = json!(42);
    assert!(validate_wire_value(schema, &invalid).is_err());

    let error_schema = Path::new("tests/fixtures/profile/schemas/round-trip-error.schema.json");
    let invalid_integer = json!({
        "code": "rate_limited",
        "payload": {"retry_after_ms": 9_007_199_254_740_992_i64}
    });
    assert!(validate_wire_value(error_schema, &invalid_integer).is_err());

    validate_wire_value(error_schema, &json!("future_variant"))
        .expect("unknown string Domain Errors should remain opaque");
    validate_wire_value(
        error_schema,
        &json!({"code": "future_variant", "payload": null}),
    )
    .expect("unknown object Domain Errors should remain opaque");
    assert!(
        validate_wire_value(
            error_schema,
            &json!({
                "code": "future_variant",
                "payload": {"unsafe_integer": 9_007_199_254_740_992_i64}
            })
        )
        .is_err()
    );
    assert!(
        validate_wire_value(
            error_schema,
            &json!({
                "code": "future_variant",
                "payload": 9_007_199_254_740_992.5_f64
            })
        )
        .is_err()
    );
}

#[test]
fn generated_artifact_check_detects_drift() {
    let root = std::env::temp_dir().join(format!("lenso-contract-codegen-{}", std::process::id()));
    std::fs::create_dir_all(&root).expect("the temporary artifact directory should exist");
    let rust_path = root.join("bindings.rs");
    let typescript_path = root.join("bindings.ts");

    write_generated(Path::new(FIXTURE), &rust_path, &typescript_path)
        .expect("the fixture should generate into the temporary directory");
    check_generated(Path::new(FIXTURE), &rust_path, &typescript_path)
        .expect("fresh generated artifacts should pass the drift check");

    std::fs::write(&rust_path, "stale\n")
        .expect("the test should be able to make the artifact stale");
    let error = check_generated(Path::new(FIXTURE), &rust_path, &typescript_path)
        .expect_err("drift must fail the check");
    assert!(matches!(error, CodegenError::GeneratedArtifactDrift { .. }));

    std::fs::remove_dir_all(root).expect("the temporary artifact directory should be removable");
}

#[test]
fn language_projections_write_and_check_independently() {
    let root = std::env::temp_dir().join(format!(
        "lenso-contract-codegen-projections-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("the temporary artifact directory should exist");
    let rust_path = root.join("bindings.rs");
    let typescript_path = root.join("bindings.ts");

    let rust = generate_projection(FIXTURE.as_ref(), ProjectionLanguage::Rust)
        .expect("Rust should generate independently");
    assert_eq!(rust.language, ProjectionLanguage::Rust);
    assert!(rust.source.contains("pub const CAPABILITY_ID"));

    write_projection(FIXTURE.as_ref(), ProjectionLanguage::Rust, &rust_path)
        .expect("Rust should generate without a TypeScript output");
    check_projection(FIXTURE.as_ref(), ProjectionLanguage::Rust, &rust_path)
        .expect("Rust should check without a TypeScript output");
    assert!(!typescript_path.exists());

    write_projection(
        FIXTURE.as_ref(),
        ProjectionLanguage::TypeScript,
        &typescript_path,
    )
    .expect("TypeScript should generate without a Rust output");
    check_projection(
        FIXTURE.as_ref(),
        ProjectionLanguage::TypeScript,
        &typescript_path,
    )
    .expect("TypeScript should check without a Rust output");

    std::fs::remove_dir_all(root).expect("the temporary artifact directory should be removable");
}

#[test]
fn checked_in_profile_artifacts_are_current() {
    check_generated(
        Path::new(FIXTURE),
        Path::new("../../fixtures/portable-contract/generated/profile.rs"),
        Path::new("../../fixtures/portable-contract/generated/profile.ts"),
    )
    .expect("checked-in profile bindings should be generated from the fixture");
}

#[test]
fn local_recursive_refs_fail_with_a_diagnostic() {
    let root = std::env::temp_dir().join(format!(
        "lenso-contract-codegen-cycle-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("the temporary contract directory should exist");
    std::fs::write(
        root.join("cycle.schema.json"),
        r##"{"type":"object","properties":{"self":{"$ref":"#"}},"additionalProperties":false}"##,
    )
    .expect("the cyclic Schema should be writable");
    std::fs::write(
        root.join("error.schema.json"),
        r#"{"oneOf":[{"const":"failed"}]}"#,
    )
    .expect("the Domain Error Schema should be writable");
    let descriptor_path = root.join("capability.json");
    std::fs::write(
        &descriptor_path,
        r#"{"id":"example.cycle@1","version":"1.0.0","portable":true,"operations":[{"name":"read","interaction":"request","request_schema":"cycle.schema.json","response_schema":"cycle.schema.json","domain_error_schema":"error.schema.json"}]}"#,
    )
    .expect("the Descriptor should be writable");

    let error = load_descriptor(&descriptor_path).expect_err("recursive refs must be rejected");
    assert!(error.to_string().contains("cyclic"));
    std::fs::remove_dir_all(root).expect("the temporary contract directory should be removable");
}

#[test]
fn generated_value_schemas_reject_all_of_instead_of_dropping_constraints() {
    let root = std::env::temp_dir().join(format!(
        "lenso-contract-codegen-all-of-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("the temporary contract directory should exist");
    std::fs::write(
        root.join("request.schema.json"),
        r#"{"type":"object","allOf":[{"type":"object","properties":{"ignored":{"type":"string"}}}],"additionalProperties":false}"#,
    )
    .expect("the allOf Schema should be writable");
    std::fs::write(
        root.join("response.schema.json"),
        r#"{"type":"object","additionalProperties":false}"#,
    )
    .expect("the response Schema should be writable");
    std::fs::write(
        root.join("error.schema.json"),
        r#"{"oneOf":[{"const":"failed"}]}"#,
    )
    .expect("the Domain Error Schema should be writable");
    let descriptor_path = root.join("capability.json");
    std::fs::write(
        &descriptor_path,
        r#"{"id":"example.all-of@1","version":"1.0.0","portable":true,"operations":[{"name":"read","interaction":"request","request_schema":"request.schema.json","response_schema":"response.schema.json","domain_error_schema":"error.schema.json"}]}"#,
    )
    .expect("the Descriptor should be writable");

    let error = load_descriptor(&descriptor_path).expect_err("allOf must not be dropped");
    assert!(error.to_string().contains("allOf"));
    std::fs::remove_dir_all(root).expect("the temporary contract directory should be removable");
}

#[test]
fn operation_generated_names_cannot_shadow_prelude_types() {
    let root = std::env::temp_dir().join(format!(
        "lenso-contract-codegen-operation-name-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("the temporary contract directory should exist");
    for (name, schema) in [
        (
            "request.schema.json",
            r#"{"type":"object","additionalProperties":false}"#,
        ),
        (
            "response.schema.json",
            r#"{"type":"object","additionalProperties":false}"#,
        ),
        ("error.schema.json", r#"{"oneOf":[{"const":"failed"}]}"#),
    ] {
        std::fs::write(root.join(name), schema).expect("the Schema should be writable");
    }
    let descriptor_path = root.join("capability.json");
    std::fs::write(
        &descriptor_path,
        r#"{"id":"example.shadow@1","version":"1.0.0","portable":true,"operations":[{"name":"unknown_domain","interaction":"request","request_schema":"request.schema.json","response_schema":"response.schema.json","domain_error_schema":"error.schema.json"}]}"#,
    )
    .expect("the Descriptor should be writable");

    let error =
        load_descriptor(&descriptor_path).expect_err("generated prelude names must be reserved");
    assert!(error.to_string().contains("UnknownDomainError"));
    std::fs::remove_dir_all(root).expect("the temporary contract directory should be removable");
}

#[test]
fn operation_generated_names_cannot_shadow_client_methods() {
    let root = std::env::temp_dir().join(format!(
        "lenso-contract-codegen-client-method-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("the temporary contract directory should exist");
    for (name, schema) in [
        (
            "request.schema.json",
            r#"{"type":"object","additionalProperties":false}"#,
        ),
        (
            "response.schema.json",
            r#"{"type":"object","additionalProperties":false}"#,
        ),
        ("error.schema.json", r#"{"oneOf":[{"const":"failed"}]}"#),
    ] {
        std::fs::write(root.join(name), schema).expect("the Schema should be writable");
    }
    let descriptor_path = root.join("capability.json");
    std::fs::write(
        &descriptor_path,
        r#"{"id":"example.client-method@1","version":"1.0.0","portable":true,"operations":[{"name":"new","interaction":"request","request_schema":"request.schema.json","response_schema":"response.schema.json","domain_error_schema":"error.schema.json"}]}"#,
    )
    .expect("the Descriptor should be writable");

    let error = load_descriptor(&descriptor_path).expect_err("Client methods must be reserved");
    assert!(error.to_string().contains("generated Client API"));
    std::fs::remove_dir_all(root).expect("the temporary contract directory should be removable");
}

#[test]
fn stream_operation_generated_names_cannot_shadow_client_methods() {
    let root = std::env::temp_dir().join(format!(
        "lenso-contract-codegen-stream-client-method-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("the temporary contract directory should exist");
    for (name, schema) in [
        (
            "request.schema.json",
            r#"{"type":"object","additionalProperties":false}"#,
        ),
        (
            "response.schema.json",
            r#"{"type":"object","additionalProperties":false}"#,
        ),
        ("error.schema.json", r#"{"oneOf":[{"const":"failed"}]}"#),
    ] {
        std::fs::write(root.join(name), schema).expect("the Schema should be writable");
    }
    let descriptor_path = root.join("capability.json");
    std::fs::write(
        &descriptor_path,
        r#"{"id":"example.stream-client-method@1","version":"1.0.0","portable":true,"operations":[{"name":"new","interaction":"stream","request_schema":"request.schema.json","response_schema":"response.schema.json","domain_error_schema":"error.schema.json"}]}"#,
    )
    .expect("the Descriptor should be writable");

    let error = load_descriptor(&descriptor_path).expect_err("Client methods must be reserved");
    assert!(error.to_string().contains("generated Client API"));
    std::fs::remove_dir_all(root).expect("the temporary contract directory should be removable");
}

#[test]
fn operation_generated_names_cannot_shadow_types_from_other_operations() {
    let root = std::env::temp_dir().join(format!(
        "lenso-contract-codegen-operation-type-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("the temporary contract directory should exist");
    for (name, schema) in [
        (
            "request.schema.json",
            r#"{"type":"object","additionalProperties":false}"#,
        ),
        (
            "response.schema.json",
            r#"{"type":"object","additionalProperties":false}"#,
        ),
        ("error.schema.json", r#"{"oneOf":[{"const":"failed"}]}"#),
    ] {
        std::fs::write(root.join(name), schema).expect("the Schema should be writable");
    }
    let descriptor_path = root.join("capability.json");
    std::fs::write(
        &descriptor_path,
        r#"{"id":"example.operation-type@1","version":"1.0.0","portable":true,"operations":[{"name":"foo","interaction":"request","request_schema":"request.schema.json","response_schema":"response.schema.json","domain_error_schema":"error.schema.json"},{"name":"foo_invocation","interaction":"request","request_schema":"request.schema.json","response_schema":"response.schema.json","domain_error_schema":"error.schema.json"}]}"#,
    )
    .expect("the Descriptor should be writable");

    let error = load_descriptor(&descriptor_path).expect_err("generated type names must be unique");
    assert!(error.to_string().contains("FooInvocationError"));
    std::fs::remove_dir_all(root).expect("the temporary contract directory should be removable");
}
