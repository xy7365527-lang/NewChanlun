/* #1789：受控部署的单次 bytes 尺寸证书，不参与内容准入。 */
(function (root, factory) {
  const common = typeof module === "object" && module.exports;
  const api = factory(common ? require("./s-content-v3.js") : root.SContentV3);
  if (common) module.exports = api;
  else root.Q3BytePlan = api;
})(globalThis, function (content) {
  "use strict";
  const MAX = (1n << 63n) - 1n, BLOCK = 4096n, ZERO = "0".repeat(64);
  const min = (a, b) => a < b ? a : b;
  const ceil = (a, b) => (a + b - 1n) / b;
  const height = n => n <= 1n ? 0n : BigInt((n - 1n).toString(2).length);
  const size = v => BigInt(new TextEncoder().encode(content.canonicalJSON(v)).length);
  const weight = bytes => 105n + 2n * bytes;
  function requestEnvelope(identity, payload, instance, serial, hash) {
    return {schema_revision: "s-session/3", ...identity,
      source_namespace: "s-observe/browser", source_epoch: instance,
      message_id: "query-" + serial, producer_id: "browser:" + instance,
      producer_epoch: "1", payload_hash: hash, causal_refs: [], payload};
  }
  function token(anchor, path, target, offset, limit) {
    return {token_schema: "s-content-token/1", anchor, path, target_ref: target,
      unit: "bytes", offset: String(offset), limit: String(limit),
      issued_capture_digest: ZERO, token_checksum: ZERO};
  }
  function keyBound(k) {
    const n = ceil(k, BLOCK);
    return weight(40n) + (n === 0n ? weight(0n) :
      (n - 1n) * weight(72n) + n * weight(32n) + n * weight(0n) + 2n * k);
  }
  function pathBound(parents) {
    let sum = weight(40n);
    for (const parent of parents) {
      sum += weight(40n);
      if (parent.kind === "json_text") continue;
      const n = BigInt(parent.n), h = height(n);
      if (n < 1n) return null;
      if (parent.kind === "array") sum += min(2n * n - 1n, 2n * h + 1n) * weight(72n);
      else if (parent.kind === "object") {
        sum += min(2n * n - 1n, (2n * h + 1n) ** 2n) * weight(72n);
        sum += min(n, 2n * h + 1n) * (weight(64n) + keyBound(BigInt(parent.K)));
      } else return null;
    }
    return sum;
  }
  function choose({anchor, path, target, parents, identity, instance, serial,
    pageSize, replyBytes, maxRequests, deployment}) {
    if (!deployment || !parents || parents.length !== path.length) return null;
    const E = BigInt(target.extent), base = BigInt(pageSize),
      cap = min(BigInt(replyBytes), BigInt(deployment.replyBytes)),
      maxSerial = serial + BigInt(maxRequests), P = pathBound(parents);
    if (P === null || cap < 1n) return null;
    const payload = {anchor, path, target_ref: target, unit: "bytes",
      offset: String(E), end: String(E), total: target.extent, items: "", nodes: [],
      done: false, next_token: token(anchor, path, target, E, MAX),
      validated_capture_digest: ZERO, ok: true};
    const F = size({schema_revision: "s-session/3", ...identity,
      source_namespace: "s-observe/replies", source_epoch: String(MAX),
      message_id: "reply-" + ZERO, producer_id: "Q:" + identity.session_id,
      producer_epoch: String(MAX), payload_hash: ZERO,
      causal_refs: [{source_namespace: "s-observe/browser", source_epoch: instance,
        message_id: "query-" + maxSerial, payload_hash: ZERO}], payload});
    const N = ceil(E, BLOCK), h = height(N);
    const upper = E === 0n ? 1n : min(E, MAX - E);
    const bound = L => {
      const m = N === 0n ? 0n : min(N, ceil(L + BLOCK - 1n, BLOCK));
      return F + P + (N === 0n ? weight(0n) :
        min(2n * N - 1n, 3n + m * (2n * h + 1n)) * weight(72n)) +
        m * weight(min(E, BLOCK)) + 2n * L;
    };
    if (upper <= base || bound(base + 1n) > cap) return null;
    let lo = base + 1n, hi = upper;
    while (lo < hi) {
      const mid = (lo + hi + 1n) / 2n;
      if (bound(mid) <= cap) lo = mid;
      else hi = mid - 1n;
    }
    const count = E === 0n ? 1n : ceil(E, lo);
    if (count > BigInt(maxRequests)) return null;
    const initial = {op: "content", anchor, path, unit: "bytes", offset: "0", limit: String(lo)};
    const frame = (p, s) => deployment.frameBytes(size(requestEnvelope(identity, p, instance, s, ZERO)));
    let frameMax = frame(initial, serial + 1n);
    if (count > 1n) {
      const last = frame({op: "content", content_token: token(anchor, path, target, (count - 1n) * lo, lo)}, serial + count);
      if (last > frameMax) frameMax = last;
    }
    if (typeof frameMax !== "bigint" || frameMax > BigInt(deployment.frameLimit)) return null;
    return Object.freeze({limit: String(lo), count, serialEnd: serial + count, replyBound: bound(lo), frameMax});
  }
  return Object.freeze({choose, requestEnvelope});
});
