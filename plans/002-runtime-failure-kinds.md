# Plan 002: Reject impossible Runtime Failure kinds in browser results

> Drift check: `git diff --stat 9d2774c..HEAD -- packages/lenso-contract-runtime/src/index.ts packages/lenso-contract-runtime/src/browser.ts packages/lenso-contract-runtime/test/browser.test.ts`.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: `plans/001-browser-schema-subset.md`
- **Category**: bug
- **Planned at**: commit `9d2774c`, 2026-08-30

## Why this matters

The TypeScript API exposes a closed `RuntimeFailureKind` union, but the browser
transport accepts any string. Remote JSON can therefore produce values that cannot
exist according to the generated client type.

## Current state

- `packages/lenso-contract-runtime/src/index.ts:23-43` defines the canonical union.
- `packages/lenso-contract-runtime/src/browser.ts:34-47` checks only that runtime
  failure `kind` is a string.

## Scope

In scope: the two files above and `packages/lenso-contract-runtime/test/browser.test.ts`.
Out of scope: changing the wire envelope or adding forward-compatible unknown kinds.

## Steps

1. Export or share one canonical readonly set derived alongside the union without
   duplicating spelling across validators.
2. Validate the runtime failure object against that set and retain optional details.
3. Add one positive case per category and negative cases for unknown/non-string kinds.

## Verification

- `bun test packages/lenso-contract-runtime/test/browser.test.ts` -> all pass.
- `bun run typecheck && bun run build` -> exit 0.
- `git diff --check` -> no output.

## STOP conditions

Stop if current design docs explicitly require unknown Runtime Failure kinds to be
forward-compatible; report the type-contract conflict instead.
