const assert = require("node:assert/strict");
const {
  PROCESS_PROFILE,
  encodeBase64Url,
  validateShutdownParams,
} = require("@lenso/process-protocol");

assert.equal(PROCESS_PROFILE, "lenso-process-jsonrpc-http-v1");
assert.doesNotThrow(() => validateShutdownParams({
  session: encodeBase64Url(new Uint8Array(32)),
}));
