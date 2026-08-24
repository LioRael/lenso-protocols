import { expect, test } from "bun:test";
import {
  PROCESS_PROFILE,
  PROVIDE_REQUEST_PROFILE,
  VALUE_PROFILE,
  canonicalizeProofValue,
  decodeBase64Url32,
  encodeBase64Url,
  parseStrictJson,
  validateHandshakeIdentity,
  validateJsonRpcError,
  validateRequestResult,
  validateReadinessRecord,
  type HandshakeIdentity,
} from "@lenso/process-protocol";

const digest = (character: string) => `sha256:${character.repeat(64)}`;

function identity(): HandshakeIdentity {
  return {
    protocol_profile: PROCESS_PROFILE,
    value_profile: VALUE_PROFILE,
    module_instance: "tool-provider",
    module_generation: "7",
    generation_spec_digest: digest("a"),
    artifact_digest: digest("b"),
    effective_host_grant_set_digest: digest("c"),
    interaction_profiles: [PROVIDE_REQUEST_PROFILE],
    provided_capabilities: [{
      capability_id: "lenso.agent.tool-provider@1",
      descriptor_version: "1.0.0",
      descriptor_digest: digest("d"),
      operations: [{ operation: "catalog", interaction: "request" }],
    }],
    outbound_bindings: [],
    peer_limits: {
      max_http_body_bytes: 65_536,
      max_control_http_body_bytes: 16_384,
      max_concurrent_requests: 32,
      child_request_queue_capacity: 32,
      max_retired_correlation_ids: 65_536,
      control_queue_capacity: 32,
    },
  };
}

test("strict JSON parsing rejects duplicate keys recursively", () => {
  expect(() => parseStrictJson('{"payload":{"key":1,"key":2}}')).toThrow(
    "duplicate object key",
  );
});

test("readiness requires the exact profile and distinct ports", () => {
  expect(() => validateReadinessRecord({
    protocol: PROCESS_PROFILE,
    data_port: 31_001,
    control_port: 31_002,
  })).not.toThrow();
  expect(() => validateReadinessRecord({
    protocol: PROCESS_PROFILE,
    data_port: 31_001,
    control_port: 31_001,
  })).toThrow("distinct");
});

test("handshake identity requires canonical order", () => {
  expect(() => validateHandshakeIdentity(identity())).not.toThrow();
  expect(() => validateHandshakeIdentity({
    ...identity(),
    interaction_profiles: [PROVIDE_REQUEST_PROFILE, "z-profile"],
  })).toThrow("exactly");
  expect(() => validateHandshakeIdentity({
    ...identity(),
    outbound_bindings: [{
      binding_id: "outbound",
      capability_id: "example.outbound@1",
      descriptor_version: "1.0.0",
      descriptor_digest: digest("e"),
      provider_instance: "provider",
    }],
  })).toThrow("consume profile");
});

test("child cannot originate host-authoritative Runtime Failures", () => {
  const base = {
    session: encodeBase64Url(new Uint8Array(32)),
    correlation_id: "1",
  };
  expect(() => validateRequestResult({
    ...base,
    outcome: { kind: "runtime", failure: { kind: "deadline_exceeded" } },
  })).toThrow("host-authoritative");
  expect(() => validateRequestResult({
    ...base,
    outcome: { kind: "runtime", failure: { kind: "unavailable" } },
  })).toThrow("host-authoritative");
});

test("JSON-RPC errors use the closed standard envelope", () => {
  expect(() => validateJsonRpcError({
    jsonrpc: "2.0",
    id: "7",
    error: { code: -32602, message: "Invalid params" },
  })).not.toThrow();
  expect(() => validateJsonRpcError({
    jsonrpc: "2.0",
    id: "7",
    error: { code: -32000, message: "private extension" },
  })).toThrow("unsupported");
});

test("proof canonicalization uses UTF-16 key order and rejects floats", () => {
  expect(new TextDecoder().decode(canonicalizeProofValue({ "\ue000": 2, "😀": 1 })))
    .toBe('{"😀":1,"":2}');
  expect(() => canonicalizeProofValue(1.5)).toThrow("safe integers");
});

test("base64url encoding is canonical and byte exact", () => {
  const bytes = Uint8Array.from({ length: 32 }, (_, index) => index);
  const encoded = encodeBase64Url(bytes);
  expect(encoded).toHaveLength(43);
  expect(decodeBase64Url32(encoded)).toEqual(bytes);
  expect(() => decodeBase64Url32(`${encoded}=`)).toThrow("canonical base64url");
});
