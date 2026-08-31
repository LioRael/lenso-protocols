# Plan 004: Enforce one ISO 8601 duration grammar in Rust and browser validation

> Drift check: `git diff --stat 9d2774c..HEAD -- packages/lenso-contract-runtime/src/browser.ts packages/lenso-contract-runtime/test/browser.test.ts crates/lenso-contract-codegen/src/lib.rs`.

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: MED
- **Depends on**: `plans/001-browser-schema-subset.md`
- **Category**: bug
- **Planned at**: commit `9d2774c`, 2026-08-30

## Why this matters

Both validators label the format ISO 8601 but accept malformed forms such as empty
fractions, duplicate units, out-of-order units, and week values mixed with other date
components. Contract validity must not depend on which runtime checked it.

## Current state

- `packages/lenso-contract-runtime/src/browser.ts:226-259` scans components without
  enforcing unit order, uniqueness, or decimal digits.
- `crates/lenso-contract-codegen/src/lib.rs:1894-1939` contains the parallel Rust
  implementation.
- Browser tests currently reject only bare `P`.

## Scope

In scope: both implementations and their focused tests. Out of scope: duration
arithmetic, normalization, or accepting locale-specific syntax.

## Steps

1. Define a shared test vector table in each language from one documented grammar:
   ordered unique date/time units, fraction only on the smallest present unit, at
   least one digit on both sides policy chosen consistently, and weeks not mixed with
   other date units.
2. Replace the permissive scanners with deterministic parsers matching those vectors.
3. Cover valid negative durations and fractional seconds already used by contracts.

## Verification

- `bun test packages/lenso-contract-runtime/test/browser.test.ts` -> all pass.
- `cargo test -p lenso-contract-codegen` -> all pass.
- `bun run typecheck && git diff --check` -> exit 0/no output.

## STOP conditions

Stop if existing Descriptor fixtures intentionally rely on a form rejected by the
chosen grammar; list those fixtures before changing compatibility.
