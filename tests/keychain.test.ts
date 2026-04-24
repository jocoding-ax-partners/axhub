// Tests for parseKeyringValue go-keyring-base64 envelope decoder.

import { describe, expect, test } from "bun:test";

import { parseKeyringValue } from "../src/axhub-helpers/keychain";

describe("parseKeyringValue (go-keyring-base64 decoder)", () => {
  test("strips 'go-keyring-base64:' prefix + decodes JSON + extracts access_token", () => {
    const json = JSON.stringify({
      schema_version: 2,
      access_token: "test_access_token_value_long_enough_to_pass",
      token_type: "bearer",
      expires_at: "2030-01-01T00:00:00Z",
      scopes: ["read", "write"],
    });
    const b64 = Buffer.from(json, "utf8").toString("base64");
    const raw = "go-keyring-base64:" + b64;
    const result = parseKeyringValue(raw);
    expect(result).toBe("test_access_token_value_long_enough_to_pass");
  });

  test("works without 'go-keyring-base64:' prefix (raw base64)", () => {
    const json = JSON.stringify({ access_token: "another_test_token_value_for_assertion" });
    const b64 = Buffer.from(json, "utf8").toString("base64");
    const result = parseKeyringValue(b64);
    expect(result).toBe("another_test_token_value_for_assertion");
  });

  test("returns null on empty input", () => {
    expect(parseKeyringValue("")).toBeNull();
  });

  test("returns null on invalid base64", () => {
    expect(parseKeyringValue("go-keyring-base64:!!!not-valid-base64@@@")).toBeNull();
  });

  test("returns null when decoded JSON has no access_token field", () => {
    const json = JSON.stringify({ token_type: "bearer", scopes: ["read"] });
    const b64 = Buffer.from(json, "utf8").toString("base64");
    const raw = "go-keyring-base64:" + b64;
    expect(parseKeyringValue(raw)).toBeNull();
  });

  test("returns null when access_token is too short (< 16 chars)", () => {
    const json = JSON.stringify({ access_token: "short" });
    const b64 = Buffer.from(json, "utf8").toString("base64");
    const raw = "go-keyring-base64:" + b64;
    expect(parseKeyringValue(raw)).toBeNull();
  });

  test("returns null when decoded payload is not valid JSON", () => {
    const b64 = Buffer.from("not json at all", "utf8").toString("base64");
    const raw = "go-keyring-base64:" + b64;
    expect(parseKeyringValue(raw)).toBeNull();
  });

  test("returns null when decoded JSON is array (not object)", () => {
    const b64 = Buffer.from(JSON.stringify(["a", "b"]), "utf8").toString("base64");
    const raw = "go-keyring-base64:" + b64;
    expect(parseKeyringValue(raw)).toBeNull();
  });
});
