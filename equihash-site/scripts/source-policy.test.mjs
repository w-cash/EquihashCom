import test from "node:test";
import assert from "node:assert/strict";
import { fetchBounded, isPublicAddress, validateSourceUrl } from "./source-policy.mjs";

test("public address policy rejects private, link-local and metadata ranges", () => {
  for (const address of ["127.0.0.1", "10.1.2.3", "172.16.0.1", "192.168.1.1", "169.254.169.254", "100.64.0.1", "::1", "fd00::1", "fe80::1", "::ffff:127.0.0.1"]) {
    assert.equal(isPublicAddress(address), false, address);
  }
  assert.equal(isPublicAddress("1.1.1.1"), true);
  assert.equal(isPublicAddress("2606:4700:4700::1111"), true);
});

test("source URL policy requires exact approved HTTPS hosts and no credentials", () => {
  const hosts = ["api.example.com"];
  assert.equal(validateSourceUrl("https://api.example.com/data", hosts).hostname, "api.example.com");
  for (const raw of ["http://api.example.com/data", "https://user:pass@api.example.com/", "https://evil.example/", "https://api.example.com.evil.test/", "https://api.example.com:8443/"]) {
    assert.throws(() => validateSourceUrl(raw, hosts), undefined, raw);
  }
});

test("bounded fetch rechecks redirects and DNS and enforces the decoded body limit", async () => {
  const publicDns = async () => [{ address: "1.1.1.1", family: 4 }];
  const redirect = async () => new Response(null, { status: 302, headers: { location: "https://169.254.169.254/latest/meta-data" } });
  await assert.rejects(fetchBounded("https://api.example.com/a", {
    allowedHosts: ["api.example.com"], contentTypes: ["application/json"], maxBytes: 20,
    fetchImpl: redirect, resolver: publicDns,
  }), /allowlisted|blocked/);

  const oversized = async () => new Response("123456", { status: 200, headers: { "content-type": "application/json" } });
  await assert.rejects(fetchBounded("https://api.example.com/a", {
    allowedHosts: ["api.example.com"], contentTypes: ["application/json"], maxBytes: 5,
    fetchImpl: oversized, resolver: publicDns,
  }), /size limit/);

  const rebinding = async () => [{ address: "169.254.169.254", family: 4 }];
  await assert.rejects(fetchBounded("https://api.example.com/a", {
    allowedHosts: ["api.example.com"], contentTypes: ["application/json"], maxBytes: 20,
    fetchImpl: oversized, resolver: rebinding,
  }), /public addresses/);
});
