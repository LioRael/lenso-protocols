# lenso-process-protocol

Runtime-neutral Rust types, strict JSON decoding, proof framing, and
conformance helpers for `lenso-process-jsonrpc-http-v1`.

This package owns no process spawning, HTTP client/server, cryptographic random
source, Capability payload Schema, Plugin behavior, or Kernel policy. Execution
Adapters supply those host facilities and use these types at their wire
boundary.

The sibling TypeScript package publishes the structural V1 JSON Schema. This
crate additionally enforces duplicate-key rejection, canonical ordering and
encodings, and expected identity/session equality at runtime.
