# Plan 001: Enforce every supported browser JSON Schema constraint

> Follow every step, run every verification command, and stop rather than widening
> scope silently. Drift check: `git diff --stat 9d2774c..HEAD -- packages/lenso-contract-runtime/src/browser.ts packages/lenso-contract-runtime/test/browser.test.ts`.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `9d2774c`, 2026-08-30

## Why this matters

Generated browser clients promise request and response validation from the same
Descriptor used by Rust and TypeScript bindings. `matchesSchema` currently ignores
validation keywords used by real Capability schemas, so invalid values cross the
browser boundary and fail only later at the server.

## Current state

- `packages/lenso-contract-runtime/src/browser.ts:57-137` handles type, enum,
  length, numeric, array-size, item, property, and additional-property checks but
  ignores `pattern` and `uniqueItems` and silently accepts unknown validation keywords.
- `packages/lenso-contract-runtime/test/browser.test.ts:4-50` is the existing
  table-driven validator test pattern.
- Preserve portable JSON validation and exact-one `oneOf` behavior.

## Scope

**In scope**: `packages/lenso-contract-runtime/src/browser.ts`,
`packages/lenso-contract-runtime/test/browser.test.ts`.

**Out of scope**: code generator output shape, server-side validation, adding a full
third-party JSON Schema engine.

## Steps

1. Add failing tests for string `pattern`, deep `uniqueItems` including objects with
   different key order, and an unsupported assertion keyword. Include the
   `^[A-Za-z0-9._:-]+$` pattern used by business-approval request IDs.
2. Implement Unicode-safe regular-expression matching and JSON deep equality for
   `uniqueItems`. Separate harmless annotations (`title`, `description`, `$schema`,
   `$id`, `default`, examples) from assertion keywords; reject unsupported assertion
   keywords instead of silently accepting them.
3. Ensure recursive schemas receive the same keyword checks and error labels remain
   stable.

## Verification

- `bun test packages/lenso-contract-runtime/test/browser.test.ts` -> all tests pass.
- `bun run typecheck` -> exit 0.
- `bun run build` -> exit 0.
- `git diff --check` -> no output.

## Done criteria

- Invalid pattern values and duplicate deep-equal items are rejected.
- Unknown assertion keywords fail closed; annotations remain accepted.
- Existing browser validator tests still pass.
- Only in-scope source/test files plus `plans/` are modified.

## STOP conditions

Stop if supporting the Descriptor subset requires remote references, dynamic code
execution, or changing public generated-client signatures.
