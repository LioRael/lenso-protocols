# Plan 003: Align strict JSON acceptance across Rust and TypeScript

> Drift check: `git diff --stat 9d2774c..HEAD -- packages/lenso-process-protocol/src/validate.ts packages/lenso-process-protocol/test fixtures/process-protocol crates/lenso-process-protocol/src/strict_json.rs crates/lenso-process-protocol/tests`.

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: MED
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `9d2774c`, 2026-08-30

## Why this matters

Rust and TypeScript both claim the same strict Process Protocol boundary, but the TS
scanner accepts isolated Unicode surrogates and deeper documents than `serde_json`.
A document can therefore be valid to one peer and invalid to the other.

## Current state

- `packages/lenso-process-protocol/src/validate.ts:497-523` validates each `\\uXXXX`
  escape independently and has no explicit nesting counter.
- `crates/lenso-process-protocol/src/strict_json.rs:7-12` delegates acceptance to
  `serde_json` before typed decoding.
- Existing shared conformance fixtures cover proof vectors, not negative JSON inputs.

## Scope

In scope: the TS/Rust strict decoders, their tests, and a shared negative fixture file
under `fixtures/process-protocol/`. Out of scope: changing the protocol size limits or
canonical encoding.

## Steps

1. Add a shared negative corpus for duplicate keys, lone high/low surrogates, valid
   surrogate pairs, depth 128, and depth 129. Consume it from both runtimes.
2. Teach the TS scanner to validate surrogate pairing and track nesting with the same
   accepted maximum as Rust. If Rust's default cannot be expressed stably, make the
   same explicit limit in both implementations.
3. Keep failure messages sanitized and avoid including document contents.

## Verification

- `bun test packages/lenso-process-protocol/test fixtures/process-protocol/conformance.test.ts` -> all pass.
- `cargo test -p lenso-process-protocol` -> all pass.
- `bun run typecheck && git diff --check` -> exit 0/no output.

## STOP conditions

Stop if either runtime cannot consume the same fixture corpus without generating
language-specific copies.
