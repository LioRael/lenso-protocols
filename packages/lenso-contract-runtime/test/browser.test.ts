import { expect, test } from "bun:test";
import { validateResult, validateSchema } from "@lenso/contract-runtime/browser";

const requestSchema = {
  type: "object",
  required: ["signed", "unsigned", "label", "payload", "timestamp", "duration", "values"],
  additionalProperties: false,
  properties: {
    signed: { type: "string", format: "int64" },
    unsigned: { type: "string", format: "uint64" },
    label: { type: "string", minLength: 2, maxLength: 4 },
    payload: { type: "string", format: "byte" },
    timestamp: { type: "string", format: "date-time" },
    duration: { type: "string", format: "duration" },
    values: {
      type: "array",
      minItems: 1,
      maxItems: 2,
      items: { type: "integer", minimum: 0, exclusiveMaximum: 10 },
    },
  },
} as const;

test("browser schema validation preserves portable formats and constraints", () => {
  const valid = {
    signed: "-9223372036854775808",
    unsigned: "18446744073709551615",
    label: "wire",
    payload: "AQI=",
    timestamp: "2026-08-23T12:34:56Z",
    duration: "PT1.5S",
    values: [0, 9],
  };
  expect(validateSchema(valid, requestSchema, "request")).toBeUndefined();

  for (const invalid of [
    { ...valid, signed: "9223372036854775808" },
    { ...valid, unsigned: "18446744073709551616" },
    { ...valid, label: "x" },
    { ...valid, payload: "AQJ=" },
    { ...valid, timestamp: "2026-02-30T12:34:56Z" },
    { ...valid, duration: "P" },
    { ...valid, values: [10] },
    { ...valid, extra: true },
  ]) {
    expect(() => validateSchema(invalid, requestSchema, "request")).toThrow(
      "invalid request payload",
    );
  }
});

test("browser result validation preserves success and native failure envelopes", () => {
  const responseSchema = {
    type: "object",
    required: ["accepted"],
    additionalProperties: false,
    properties: { accepted: { type: "boolean" } },
  } as const;
  const errorSchema = { oneOf: [{ const: "rejected" }] } as const;

  expect(
    validateResult({ ok: true, value: { accepted: true } }, responseSchema, errorSchema),
  ).toBeUndefined();
  expect(
    validateResult(
      { ok: false, error: { kind: "domain", error: { code: "future" } } },
      responseSchema,
      errorSchema,
    ),
  ).toBeUndefined();
  expect(
    validateResult(
      { ok: false, error: { kind: "runtime", error: { kind: "cancelled" } } },
      responseSchema,
      errorSchema,
    ),
  ).toBeUndefined();
  expect(() => validateResult({ ok: true }, responseSchema, errorSchema)).toThrow(
    "missing value",
  );
});
