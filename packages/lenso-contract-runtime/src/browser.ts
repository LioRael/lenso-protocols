import { validatePortableJson } from "./index.js";

export { validatePortableJson } from "./index.js";

/** JSON Schema subset accepted by generated browser request clients. */
export type PortableSchema = Readonly<Record<string, unknown>>;

/** Validates a payload against the portable contract schema subset. */
export function validateSchema(
  value: unknown,
  schema: PortableSchema,
  label: string,
): void {
  validatePortableJson(value);
  if (!matchesSchema(value, schema)) throw new Error(`invalid ${label} payload`);
}

/** Validates the result envelope and operation payload returned to a browser client. */
export function validateResult(
  result: unknown,
  responseSchema: PortableSchema,
  errorSchema: PortableSchema,
): void {
  if (!isRecord(result) || typeof result.ok !== "boolean") {
    throw new Error("invalid Capability result envelope");
  }
  if (result.ok) {
    if (!Object.hasOwn(result, "value")) {
      throw new Error("successful Capability result is missing value");
    }
    validateSchema(result.value, responseSchema, "response");
    return;
  }
  if (
    !isRecord(result.error) ||
    !["domain", "runtime"].includes(String(result.error.kind))
  ) {
    throw new Error("invalid invocation error envelope");
  }
  if (result.error.kind === "domain") {
    validateDomainError(result.error.error, errorSchema);
  } else if (
    !isRecord(result.error.error) ||
    typeof result.error.error.kind !== "string"
  ) {
    throw new Error("invalid Runtime Failure payload");
  }
}

function validateDomainError(value: unknown, schema: PortableSchema): void {
  if (matchesSchema(value, schema)) return;
  if (typeof value === "string") return;
  if (isRecord(value) && typeof value.code === "string") return;
  throw new Error("invalid Domain Error payload");
}

function matchesSchema(value: unknown, schema: PortableSchema): boolean {
  const oneOf = schemaList(schema.oneOf);
  if (
    oneOf &&
    oneOf.filter((candidate) => matchesSchema(value, candidate)).length !== 1
  ) {
    return false;
  }
  const anyOf = schemaList(schema.anyOf);
  if (anyOf && !anyOf.some((candidate) => matchesSchema(value, candidate))) {
    return false;
  }
  if (Object.hasOwn(schema, "const") && value !== schema.const) return false;
  if (Array.isArray(schema.enum) && !schema.enum.includes(value)) return false;
  if (Array.isArray(schema.type)) {
    return schema.type.some((type) => matchesSchema(value, { ...schema, type }));
  }
  if (schema.type === "null") return value === null;
  if (schema.type === "string") {
    if (typeof value !== "string" || !matchesFormat(value, schema.format)) {
      return false;
    }
    const length = Array.from(value).length;
    if (typeof schema.minLength === "number" && length < schema.minLength) {
      return false;
    }
    if (typeof schema.maxLength === "number" && length > schema.maxLength) {
      return false;
    }
  }
  if (schema.type === "boolean" && typeof value !== "boolean") return false;
  if (
    schema.type === "number" &&
    (typeof value !== "number" || !Number.isFinite(value))
  ) {
    return false;
  }
  if (schema.type === "integer" && !Number.isSafeInteger(value)) return false;
  if (schema.type === "number" || schema.type === "integer") {
    if (typeof value !== "number" || !matchesNumericBounds(value, schema)) {
      return false;
    }
  }
  if (schema.type === "array") {
    if (!Array.isArray(value)) return false;
    if (typeof schema.minItems === "number" && value.length < schema.minItems) {
      return false;
    }
    if (typeof schema.maxItems === "number" && value.length > schema.maxItems) {
      return false;
    }
    if (isRecord(schema.items)) {
      return value.every((item) => matchesSchema(item, schema.items as PortableSchema));
    }
  }
  if (schema.type === "object") {
    if (!isRecord(value) || Array.isArray(value)) return false;
    const required = Array.isArray(schema.required) ? schema.required : [];
    if (
      required.some(
        (key) => typeof key !== "string" || !Object.hasOwn(value, key),
      )
    ) {
      return false;
    }
    const properties = isRecord(schema.properties) ? schema.properties : {};
    for (const [key, item] of Object.entries(value)) {
      const property = properties[key];
      if (isRecord(property)) {
        if (!matchesSchema(item, property)) return false;
      } else if (schema.additionalProperties === false) {
        return false;
      } else if (
        isRecord(schema.additionalProperties) &&
        !matchesSchema(item, schema.additionalProperties)
      ) {
        return false;
      }
    }
  }
  return true;
}

function matchesNumericBounds(value: number, schema: PortableSchema): boolean {
  return !(
    (typeof schema.minimum === "number" && value < schema.minimum) ||
    (typeof schema.maximum === "number" && value > schema.maximum) ||
    (typeof schema.exclusiveMinimum === "number" &&
      value <= schema.exclusiveMinimum) ||
    (typeof schema.exclusiveMaximum === "number" &&
      value >= schema.exclusiveMaximum)
  );
}

function matchesFormat(value: string, format: unknown): boolean {
  if (!format) return true;
  if (format === "int64") return decimalInRange(value, true);
  if (format === "uint64") return decimalInRange(value, false);
  if (format === "byte") return isCanonicalBase64(value);
  if (format === "date-time") return isRfc3339(value);
  if (format === "duration") return isIso8601Duration(value);
  return false;
}

function decimalInRange(value: string, signed: boolean): boolean {
  const pattern = signed ? /^-?(0|[1-9][0-9]*)$/ : /^(0|[1-9][0-9]*)$/;
  if (!pattern.test(value)) return false;
  try {
    const number = BigInt(value);
    return signed
      ? number >= -9_223_372_036_854_775_808n &&
          number <= 9_223_372_036_854_775_807n
      : number >= 0n && number <= 18_446_744_073_709_551_615n;
  } catch {
    return false;
  }
}

function isCanonicalBase64(value: string): boolean {
  if (value === "") return true;
  if (value.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(value)) {
    return false;
  }
  try {
    return btoa(atob(value)) === value;
  } catch {
    return false;
  }
}

function isRfc3339(value: string): boolean {
  const match = /^(\d{4})-(\d{2})-(\d{2})[Tt](\d{2}):(\d{2}):(\d{2})(?:\.\d+)?(?:[Zz]|([+-])(\d{2}):(\d{2}))$/.exec(
    value,
  );
  if (!match) return false;
  const [, year, month, day, hour, minute, second, , offsetHour = "0", offsetMinute = "0"] = match;
  const numericYear = Number(year);
  const numericMonth = Number(month);
  const leap =
    numericYear % 4 === 0 &&
    (numericYear % 100 !== 0 || numericYear % 400 === 0);
  const days = [
    0,
    31,
    leap ? 29 : 28,
    31,
    30,
    31,
    30,
    31,
    31,
    30,
    31,
    30,
    31,
  ][numericMonth] ?? 0;
  return (
    numericMonth >= 1 &&
    numericMonth <= 12 &&
    Number(day) >= 1 &&
    Number(day) <= days &&
    Number(hour) <= 23 &&
    Number(minute) <= 59 &&
    Number(second) <= 60 &&
    Number(offsetHour) <= 23 &&
    Number(offsetMinute) <= 59
  );
}

function isIso8601Duration(value: string): boolean {
  let index = value.startsWith("-") ? 1 : 0;
  if (value[index] !== "P") return false;
  index += 1;
  let inTime = false;
  let sawComponent = false;
  let sawTime = false;
  while (index < value.length) {
    if (value[index] === "T") {
      if (inTime || index + 1 === value.length) return false;
      inTime = true;
      index += 1;
      continue;
    }
    const start = index;
    let separatorSeen = false;
    while (
      index < value.length &&
      (/[0-9]/.test(value[index] ?? "") ||
        (!separatorSeen && value[index] === "."))
    ) {
      separatorSeen ||= value[index] === ".";
      index += 1;
    }
    if (index === start || index >= value.length) return false;
    const unit = value[index];
    if (!(inTime ? ["H", "M", "S"] : ["Y", "M", "W", "D"]).includes(unit ?? "")) {
      return false;
    }
    if (inTime) sawTime = true;
    sawComponent = true;
    index += 1;
  }
  return sawComponent && (!inTime || sawTime);
}

function schemaList(value: unknown): PortableSchema[] | undefined {
  if (!Array.isArray(value)) return undefined;
  return value.filter(isRecord);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
