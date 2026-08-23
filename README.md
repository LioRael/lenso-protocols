# Lenso Protocols

This repository owns runtime-neutral protocol tooling and portable conformance
artifacts for Lenso. It does not own the Kernel, host runtimes, product
Capabilities, or Module implementations.

The source was extracted from `LioRael/lenso` at monorepo commit
`67d21499548d07e92c2f6529d7c8345e58c067d9` under ADR 0064. Imported subtrees
retain their relevant Git history.

## Packages

- `lenso-contract-codegen`: generates Rust and TypeScript bindings from a
  runtime-neutral Capability descriptor.
- `lenso-contract-runtime`: provides the small, platform-neutral wire primitive,
  serde, and portable JSON surface shared by generated Rust bindings.
- `fixtures/portable-contract`: cross-language value-profile conformance data.

Generated bindings retain contract-specific values, Provider traits, Clients,
Endpoints, and operation dispatch. The runtime owns only reusable wire behavior;
patch and minor runtime releases must preserve that behavior, with conformance
tests and generated artifact drift checks guarding both sides of the boundary.

Rust request Providers return `NativeRequestFuture<Operation>` directly. The
generated Endpoint preserves the typed domain/runtime result without wrapping
the Provider future in a second allocation; erased dispatch remains available
only as the compatibility boundary. This Provider signature starts with
`lenso-contract-codegen` 0.4 and requires `lenso-kernel` 0.1.4 or newer.

## Validation

```sh
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets
cargo test --locked --workspace
bun test fixtures/portable-contract/conformance.test.ts
```
