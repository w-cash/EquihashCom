import { lookup } from "node:dns/promises";
import { isIP } from "node:net";

const normalizeHost = (value) => String(value || "").toLowerCase().replace(/^\[|\]$/g, "").replace(/\.$/, "");

export function isPublicAddress(value) {
  const address = normalizeHost(value).split("%")[0];
  const version = isIP(address);
  if (version === 4) {
    const octets = address.split(".").map(Number);
    const [a, b] = octets;
    return !(
      a === 0 || a === 10 || a === 127 || a >= 224 ||
      (a === 100 && b >= 64 && b <= 127) ||
      (a === 169 && b === 254) ||
      (a === 172 && b >= 16 && b <= 31) ||
      (a === 192 && b === 0) ||
      (a === 192 && b === 168) ||
      (a === 198 && (b === 18 || b === 19))
    );
  }
  if (version === 6) {
    if (address === "::" || address === "::1") return false;
    if (/^::ffff:/.test(address)) return isPublicAddress(address.slice(7));
    return !(/^(?:fc|fd)/.test(address) || /^fe[89ab]/.test(address) || /^ff/.test(address) || /^2001:db8(?::|$)/.test(address));
  }
  return false;
}

export function validateSourceUrl(raw, allowedHosts) {
  const url = new URL(raw);
  const host = normalizeHost(url.hostname);
  const allowed = new Set([...allowedHosts].map(normalizeHost));
  if (url.protocol !== "https:" || url.username || url.password) throw new Error("source must be a credential-free HTTPS URL");
  if (url.port && url.port !== "443") throw new Error("source port is not allowed");
  if (!allowed.has(host)) throw new Error(`source host is not allowlisted: ${host}`);
  if (host === "localhost" || host.endsWith(".localhost") || host.endsWith(".local") || (isIP(host) && !isPublicAddress(host))) {
    throw new Error("local, private, link-local or metadata source blocked");
  }
  return url;
}

async function assertPublicResolution(host, resolver) {
  const records = await resolver(host, { all: true, verbatim: true });
  if (!records.length || records.some((record) => !isPublicAddress(record.address))) {
    throw new Error(`source DNS did not resolve only to public addresses: ${host}`);
  }
}

export async function fetchBounded(raw, {
  allowedHosts,
  accept,
  contentTypes,
  maxBytes,
  timeoutMs = 20_000,
  maxRedirects = 3,
  userAgent = "equihash.com source collector (+https://equihash.com/sources)",
  headers = {},
  fetchImpl = fetch,
  resolver = lookup,
} = {}) {
  if (!allowedHosts?.length || !maxBytes || !contentTypes?.length) throw new Error("collector policy is incomplete");
  let url = validateSourceUrl(raw, allowedHosts);
  for (let redirects = 0; ; redirects++) {
    await assertPublicResolution(url.hostname, resolver);
    const response = await fetchImpl(url, {
      redirect: "manual",
      signal: AbortSignal.timeout(timeoutMs),
      headers: { ...headers, accept, "user-agent": userAgent },
    });
    if ([301, 302, 303, 307, 308].includes(response.status)) {
      if (redirects >= maxRedirects) throw new Error("source redirect limit exceeded");
      const location = response.headers.get("location");
      if (!location) throw new Error("source redirect has no location");
      url = validateSourceUrl(new URL(location, url).href, allowedHosts);
      continue;
    }
    if (!response.ok) throw new Error(`source HTTP ${response.status}`);
    const type = (response.headers.get("content-type") || "").split(";", 1)[0].trim().toLowerCase();
    if (!contentTypes.includes(type)) throw new Error(`source content type is not allowed: ${type || "missing"}`);
    const declared = Number(response.headers.get("content-length"));
    if (Number.isFinite(declared) && declared > maxBytes) throw new Error("source response exceeds size limit");
    const reader = response.body?.getReader();
    if (!reader) throw new Error("source response has no readable body");
    const chunks = [];
    let size = 0;
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > maxBytes) { await reader.cancel(); throw new Error("source response exceeds size limit"); }
      chunks.push(value);
    }
    const body = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) { body.set(chunk, offset); offset += chunk.byteLength; }
    return { body, type, url: url.href };
  }
}
