# Agent instructions

This checkout retains pre-consolidation history. New Rust protocol work belongs
in `LioRael/lenso`, and TypeScript work in `LioRael/lenso-js`, under ADR 0077;
the rules below cover maintenance here.

This repository owns runtime-neutral protocol source, code generation, and
portable conformance vectors. Keep Kernel-specific bindings behind generator
backends and do not add product Capability or Plugin ownership here.

Use Conventional Commits and run the locked workspace checks before delivery.
