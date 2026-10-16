/* #1394 generated browser dependencies begin */
if (typeof module !== 'object' || !module.exports) {
/* exact source s-content-v3.js SHA256 fb21d3467a189ac1d09cd585b07ca927083bee550fdd2c50419eac82b20ba926 */
/* s-content/1: portable, dependency-free content codec and proof reader.
 * Transport references are explicit edges. This module proves content only;
 * publication/cut authority, business semantics and resource budgets belong to callers.
 */
(function (root, factory) {
  "use strict";
  const api = factory(
    typeof module === "object" && module.exports
      ? require("node:crypto")
      : null,
  );
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.SContentV3 = api;
})(typeof globalThis === "object" ? globalThis : this, function (nodeCrypto) {
  "use strict";
  const MAX = (1n << 63n) - 1n;
  const encoder = new TextEncoder();
  const DOMAIN = encoder.encode("NewChanlun/SContent/1\0");
  const ROOT = /^sc1-[0-9a-f]{64}$/;
  const HEX = /^(?:[0-9a-f]{2})*$/;
  const NUMBER = /^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?$/;
  const kinds = [
    "null",
    "bool",
    "bool",
    "string",
    "number",
    "array",
    "object",
    "json_text",
    "template_batch",
  ];
  const EDGE = Symbol("s-content/1 explicit value edge");
  const GRAPH = Symbol("s-content/1 authenticated pool");
  function fail(message) {
    throw new Error("s-content/1: " + message);
  }
  function exact(value, fields) {
    if (
      !value ||
      typeof value !== "object" ||
      Array.isArray(value) ||
      Object.keys(value).length !== fields.length ||
      fields.some((k) => !Object.hasOwn(value, k))
    )
      fail("exact fields");
  }
  function decimal(value, positive = false) {
    if (typeof value !== "string" || !/^(?:0|[1-9][0-9]*)$/.test(value))
      fail("decimal");
    const n = BigInt(value);
    if (n > MAX || (positive && n === 0n)) fail("i64 extent");
    return n;
  }
  function utf8(value) {
    if (typeof value !== "string") fail("string");
    for (let i = 0; i < value.length; i++) {
      const c = value.charCodeAt(i);
      if (c >= 0xd800 && c <= 0xdbff) {
        const d = value.charCodeAt(++i);
        if (!(d >= 0xdc00 && d <= 0xdfff)) fail("isolated surrogate");
      } else if (c >= 0xdc00 && c <= 0xdfff) fail("isolated surrogate");
    }
    return encoder.encode(value);
  }
  function hex(bytes) {
    let s = "";
    for (const b of bytes) s += b.toString(16).padStart(2, "0");
    return s;
  }
  function unhex(s) {
    if (typeof s !== "string" || !HEX.test(s)) fail("hex");
    const b = new Uint8Array(s.length / 2);
    for (let i = 0; i < b.length; i++)
      b[i] = parseInt(s.slice(i * 2, i * 2 + 2), 16);
    return b;
  }
  function concat(...parts) {
    const b = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
    let i = 0;
    for (const p of parts) {
      b.set(p, i);
      i += p.length;
    }
    return b;
  }
  function u64(n) {
    n = BigInt(n);
    if (n < 0n || n > MAX) fail("U64 domain");
    const b = new Uint8Array(8);
    for (let i = 7; i >= 0; i--) {
      b[i] = Number(n & 255n);
      n >>= 8n;
    }
    return b;
  }
  function read64(b, at = 0) {
    if (b.length < at + 8) fail("short U64");
    let n = 0n;
    for (let i = at; i < at + 8; i++) n = (n << 8n) | BigInt(b[i]);
    if (n > MAX) fail("U64 domain");
    return n;
  }
  function hashBytes(root) {
    if (typeof root !== "string" || !ROOT.test(root)) fail("root");
    return unhex(root.slice(4));
  }
  function child(b, at = 0) {
    if (b.length < at + 32) fail("short child");
    return "sc1-" + hex(b.subarray(at, at + 32));
  }
  async function sha256(bytes) {
    if (nodeCrypto)
      return nodeCrypto.createHash("sha256").update(bytes).digest("hex");
    if (!globalThis.crypto || !globalThis.crypto.subtle)
      fail("WebCrypto unavailable");
    return hex(
      new Uint8Array(await globalThis.crypto.subtle.digest("SHA-256", bytes)),
    );
  }
  async function framedHash(tag, body) {
    return (
      "sc1-" +
      (await sha256(
        concat(DOMAIN, new Uint8Array([tag]), u64(body.length), body),
      ))
    );
  }
  function split(n) {
    let k = 1n;
    while (k << 1n < n) k <<= 1n;
    return k;
  }
  function byteCompare(a, b) {
    const n = Math.min(a.length, b.length);
    for (let i = 0; i < n; i++) if (a[i] !== b[i]) return a[i] - b[i];
    return a.length - b.length;
  }
  function compareText(a, b) {
    return byteCompare(utf8(a), utf8(b));
  }
  function quoteBody(s) {
    return s.replace(
      /["\\\u0000-\u001f]/g,
      (c) =>
        ({
          '"': '\\"',
          "\\": "\\\\",
          "\b": "\\b",
          "\t": "\\t",
          "\n": "\\n",
          "\f": "\\f",
          "\r": "\\r",
        })[c] || "\\u" + c.charCodeAt(0).toString(16).padStart(4, "0"),
    );
  }
  function validateRef(ref) {
    exact(ref, ["format", "root", "kind", "extent"]);
    ref = { ...ref };
    hashBytes(ref.root);
    if (ref.format !== "s-content/1" || !kinds.includes(ref.kind))
      fail("ContentRef kind");
    if (["string", "number", "array", "object"].includes(ref.kind))
      decimal(ref.extent);
    else if (ref.extent !== null) fail("ContentRef extent");
    return Object.freeze({ ...ref });
  }
  function refEdge(ref) {
    ref = validateRef(ref);
    return Object.freeze({ [EDGE]: true, ref });
  }
  // No automatic interpretation of ordinary four-field objects as references.
  function fromPlain(value) {
    const box = {},
      active = new Set();
    const stack = [
      {
        value,
        set: (x) => {
          box.value = x;
        },
      },
    ];
    while (stack.length) {
      const j = stack.pop();
      if (j.end) {
        active.delete(j.end);
        continue;
      }
      const { value: v, set } = j;
      if (v === null) set({ t: "null" });
      else if (typeof v === "boolean") set({ t: "bool", value: v });
      else if (typeof v === "string") {
        utf8(v);
        set({ t: "string", value: v });
      } else if (typeof v === "number")
        fail("plain Number loses token; use typed number");
      else if (v && v[EDGE]) set(v);
      else if (Array.isArray(v)) {
        if (active.has(v)) fail("plain JSON cycle");
        active.add(v);
        stack.push({ end: v });
        const a = { t: "array", items: new Array(v.length) };
        set(a);
        for (let i = v.length - 1; i >= 0; i--)
          stack.push({
            value: v[i],
            set: (x) => {
              a.items[i] = x;
            },
          });
      } else if (v && Object.getPrototypeOf(v) === Object.prototype) {
        if (active.has(v)) fail("plain JSON cycle");
        active.add(v);
        stack.push({ end: v });
        const a = { t: "object", fields: [] };
        set(a);
        const keys = Object.keys(v);
        for (let i = keys.length - 1; i >= 0; i--) {
          const j = i;
          a.fields[j] = [keys[j], null];
          stack.push({
            value: v[keys[j]],
            set: (x) => {
              a.fields[j][1] = x;
            },
          });
        }
      } else fail("plain JSON value");
    }
    return box.value;
  }
  // JSON tokenizer preserves every number token and rejects duplicate source keys.
  function parseJSON(text) {
    utf8(text);
    let at = 0;
    const frames = [];
    let result,
      hasResult = false;
    function ws() {
      while (at < text.length && /[\x20\t\r\n]/.test(text[at])) at++;
    }
    function str() {
      const start = at++;
      while (at < text.length) {
        const c = text[at++];
        if (c === '"') {
          const s = JSON.parse(text.slice(start, at));
          utf8(s);
          return s;
        }
        if (c === "\\") at++;
      }
      fail("unterminated string");
    }
    function attach(v) {
      if (!frames.length) {
        if (hasResult) fail("extra JSON");
        result = v;
        hasResult = true;
        return;
      }
      const f = frames[frames.length - 1];
      if (f.a.t === "array") f.a.items.push(v);
      else {
        f.a.fields.push([f.key, v]);
        f.key = null;
      }
      f.state = "comma";
    }
    function value() {
      ws();
      const c = text[at];
      if (c === "[" || c === "{") {
        at++;
        const a =
          c === "[" ? { t: "array", items: [] } : { t: "object", fields: [] };
        attach(a);
        frames.push({
          a,
          state: c === "[" ? "valueOrEnd" : "keyOrEnd",
          seen: new Set(),
        });
        return;
      }
      if (c === '"') {
        attach({ t: "string", value: str() });
        return;
      }
      for (const [literal, v] of [
        ["null", { t: "null" }],
        ["true", { t: "bool", value: true }],
        ["false", { t: "bool", value: false }],
      ])
        if (text.startsWith(literal, at)) {
          at += literal.length;
          attach(v);
          return;
        }
      const m = text
        .slice(at)
        .match(/^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/);
      if (!m) fail("JSON value");
      at += m[0].length;
      attach({ t: "number", token: m[0] });
    }
    value();
    while (frames.length) {
      ws();
      const f = frames[frames.length - 1],
        object = f.a.t === "object",
        end = object ? "}" : "]";
      if (f.state === "comma") {
        if (text[at] === end) {
          at++;
          frames.pop();
        } else if (text[at] === ",") {
          at++;
          f.state = object ? "key" : "value";
        } else fail("JSON comma");
      } else if (f.state === "key" || f.state === "keyOrEnd") {
        if (f.state === "keyOrEnd" && text[at] === end) {
          at++;
          frames.pop();
          continue;
        }
        if (text[at] !== '"') fail("JSON key");
        f.key = str();
        if (f.seen.has(f.key)) fail("duplicate key");
        f.seen.add(f.key);
        ws();
        if (text[at++] !== ":") fail("JSON colon");
        f.state = "value";
      } else {
        if (f.state === "valueOrEnd" && text[at] === end) {
          at++;
          frames.pop();
        } else value();
      }
    }
    ws();
    if (at !== text.length) fail("trailing JSON");
    return result;
  }
  // Own immutable snapshots preserve DAG sharing. Copying does not freeze or
  // mutate caller objects, and occurs synchronously before any crypto await.
  function freezeJSON(value) {
    const copy = JSON.parse(canonicalJSON(value)),
      stack = [copy];
    while (stack.length) {
      const v = stack.pop();
      if (v && typeof v === "object") {
        for (const x of Object.values(v)) stack.push(x);
        Object.freeze(v);
      }
    }
    return copy;
  }
  function snapshotAST(root) {
    const copies = new Map(),
      pending = [root];
    function copy(a) {
      if (!a || typeof a !== "object") fail("typed AST");
      if (!copies.has(a)) {
        copies.set(a, {});
        pending.push(a);
      }
      return copies.get(a);
    }
    pending.length = 0;
    copy(root);
    while (pending.length) {
      const a = pending.pop(),
        out = copies.get(a);
      if (a[EDGE]) {
        out[EDGE] = true;
        out.ref = Object.freeze({ ...validateRef(a.ref) });
        Object.freeze(out);
        continue;
      }
      out.t = a.t;
      switch (a.t) {
        case "null":
        case "batch_self":
          exact(a, ["t"]);
          break;
        case "bool":
        case "string":
          exact(a, ["t", "value"]);
          out.value = a.value;
          break;
        case "number":
          exact(a, ["t", "token"]);
          out.token = a.token;
          break;
        case "array":
          exact(a, ["t", "items"]);
          if (!Array.isArray(a.items)) fail("array");
          out.items = Object.freeze(a.items.map(copy));
          break;
        case "object":
          exact(a, ["t", "fields"]);
          if (!Array.isArray(a.fields)) fail("object");
          out.fields = Object.freeze(
            a.fields.map((f) => {
              if (!Array.isArray(f) || f.length !== 2) fail("field");
              return Object.freeze([f[0], copy(f[1])]);
            }),
          );
          break;
        case "json_text":
          exact(a, ["t", "value"]);
          out.value = copy(a.value);
          break;
        default:
          fail("typed AST kind");
      }
      Object.freeze(out);
    }
    return copies.get(root);
  }
  async function encode(ast, options = {}) {
    ast = snapshotAST(ast);
    const allowTemplate = options.allowTemplate === true;
    const contexts = new Map(),
      frames = new Map();
    const nodes = new Map(),
      results = new Map(),
      active = new Set();
    let provisional = false,
      template = false;
    async function add(tag, body) {
      const bodyHex = hex(body),
        key = tag + ":" + bodyHex;
      if (frames.has(key)) return frames.get(key);
      const root = await framedHash(tag, body);
      frames.set(key, root);
      nodes.set(
        root,
        Object.freeze({
          root,
          tag: tag.toString(16).padStart(2, "0"),
          body_hex: hex(body),
        }),
      );
      return root;
    }
    async function merkle(roots) {
      const jobs = [{ start: 0, n: roots.length, phase: 0 }],
        out = new Map();
      while (jobs.length) {
        const j = jobs.pop(),
          key = j.start + ":" + j.n;
        if (j.n === 0) out.set(key, await add(0x11, new Uint8Array()));
        else if (j.n === 1)
          out.set(key, await add(0x12, hashBytes(roots[j.start])));
        else {
          const k = Number(split(BigInt(j.n)));
          if (!j.phase) {
            jobs.push(
              { ...j, phase: 1 },
              { start: j.start + k, n: j.n - k, phase: 0 },
              { start: j.start, n: k, phase: 0 },
            );
          } else
            out.set(
              key,
              await add(
                0x13,
                concat(
                  u64(j.n),
                  hashBytes(out.get(j.start + ":" + k)),
                  hashBytes(out.get(j.start + k + ":" + (j.n - k))),
                ),
              ),
            );
        }
      }
      return out.get("0:" + roots.length);
    }
    async function bytesRoot(b) {
      const r = [];
      for (let i = 0; i < b.length; i += 4096)
        r.push(await add(0x10, b.subarray(i, i + 4096)));
      return merkle(r);
    }
    const jobs = [{ a: ast, phase: 0, ordinary: false }];
    while (jobs.length) {
      const j = jobs.pop(),
        a = j.a;
      if (!a || typeof a !== "object") fail("typed AST");
      if (a[EDGE]) {
        if (j.ordinary && ["json_text", "template_batch"].includes(a.ref.kind))
          fail("json_text descendant");
        results.set(a, a.ref.root);
        provisional = true;
        continue;
      }
      if (!j.phase) {
        if (contexts.get(a)?.has(j.ordinary)) continue;
        if (active.has(a)) fail("AST cycle");
        active.add(a);
        let children = [];
        switch (a.t) {
          case "null":
            exact(a, ["t"]);
            break;
          case "bool":
            exact(a, ["t", "value"]);
            if (typeof a.value !== "boolean") fail("bool");
            break;
          case "string":
            exact(a, ["t", "value"]);
            utf8(a.value);
            break;
          case "number":
            exact(a, ["t", "token"]);
            if (typeof a.token !== "string" || !NUMBER.test(a.token))
              fail("number token");
            break;
          case "array":
            exact(a, ["t", "items"]);
            if (!Array.isArray(a.items)) fail("array");
            children = a.items;
            break;
          case "object":
            exact(a, ["t", "fields"]);
            if (!Array.isArray(a.fields)) fail("object");
            {
              const seen = new Set();
              for (const f of a.fields) {
                if (!Array.isArray(f) || f.length !== 2) fail("field");
                utf8(f[0]);
                if (seen.has(f[0])) fail("duplicate key");
                seen.add(f[0]);
              }
              children = a.fields.map((f) => f[1]);
            }
            break;
          case "json_text":
            exact(a, ["t", "value"]);
            if (j.ordinary) fail("json_text descendant");
            children = [a.value];
            break;
          case "batch_self":
            exact(a, ["t"]);
            if (j.ordinary || !allowTemplate || a !== ast)
              fail("template standalone encoding only");
            template = true;
            break;
          default:
            fail("typed AST kind");
        }
        if (j.ordinary && (a.t === "json_text" || a.t === "batch_self"))
          fail("json_text descendant");
        jobs.push({ ...j, phase: 1 });
        for (let i = children.length - 1; i >= 0; i--)
          jobs.push({
            a: children[i],
            phase: 0,
            ordinary: j.ordinary || a.t === "json_text",
          });
      } else {
        if (results.has(a)) {
          active.delete(a);
          let c = contexts.get(a);
          if (!c) contexts.set(a, (c = new Set()));
          c.add(j.ordinary);
          continue;
        }
        let tag,
          body = new Uint8Array();
        switch (a.t) {
          case "null":
            tag = 0;
            break;
          case "bool":
            tag = a.value ? 2 : 1;
            break;
          case "batch_self":
            tag = 8;
            break;
          case "string":
          case "number": {
            tag = a.t === "string" ? 3 : 4;
            const b = utf8(a.t === "string" ? a.value : a.token);
            body = concat(u64(b.length), hashBytes(await bytesRoot(b)));
            break;
          }
          case "array":
            tag = 5;
            body = concat(
              u64(a.items.length),
              hashBytes(await merkle(a.items.map((c) => results.get(c)))),
            );
            break;
          case "object": {
            tag = 6;
            const fields = [...a.fields].sort((x, y) =>
              compareText(x[0], y[0]),
            );
            const r = [];
            for (const [key, value] of fields) {
              const b = utf8(key),
                keyRoot = await add(
                  3,
                  concat(u64(b.length), hashBytes(await bytesRoot(b))),
                );
              r.push(
                await add(
                  0x14,
                  concat(hashBytes(keyRoot), hashBytes(results.get(value))),
                ),
              );
            }
            body = concat(u64(fields.length), hashBytes(await merkle(r)));
            break;
          }
          case "json_text":
            tag = 7;
            body = hashBytes(results.get(a.value));
            break;
        }
        results.set(a, await add(tag, body));
        let c = contexts.get(a);
        if (!c) contexts.set(a, (c = new Set()));
        c.add(j.ordinary);
        active.delete(a);
      }
    }
    const root = results.get(ast),
      list = [...nodes.values()].sort((a, b) => (a.root < b.root ? -1 : 1));
    let ref;
    if (ast[EDGE]) ref = Object.freeze({ ...ast.ref });
    else {
      const n = list.find((n) => n.root === root);
      ref = headerFromRaw(n).ref;
    }
    const g = await GraphView.create(list);
    if (!provisional && !template) await g.verifyComplete(ref);
    return Object.freeze({
      ref,
      nodes: Object.freeze(list),
      semanticValidated: !provisional && !template,
      metrics: g.metrics,
    });
  }
  function headerFromRaw(raw) {
    const b = unhex(raw.body_hex),
      tag = parseInt(raw.tag, 16);
    let n = null;
    if (tag <= 2 || tag === 8 || tag === 0x11) {
      if (b.length !== 0) fail("empty body");
    } else if (tag >= 3 && tag <= 6) {
      if (b.length !== 40) fail("value header length");
      n = read64(b);
    } else if (tag === 7 || tag === 0x12) {
      if (b.length !== 32) fail("single child length");
    } else if (tag === 0x10) {
      if (!b.length || b.length > 4096) fail("block size");
    } else if (tag === 0x13) {
      if (b.length !== 72) fail("branch length");
      n = read64(b);
      if (n <= 1n) fail("branch count");
    } else if (tag === 0x14) {
      if (b.length !== 64) fail("field length");
    } else fail("unknown tag");
    const ref =
      tag <= 8
        ? Object.freeze({
            format: "s-content/1",
            root: raw.root,
            kind: kinds[tag],
            extent: n === null ? null : n.toString(),
          })
        : null;
    return { tag, b, n, ref };
  }
  // Transition monoids validate compressed byte ropes without replaying a
  // shared block at every logical offset. Each map includes every legal entry
  // state, so a memoized block cannot hide a UTF-8 or number boundary violation.
  function utf8Step(state, b) {
    if (state === 0) {
      if (b <= 0x7f) return 0;
      if (b >= 0xc2 && b <= 0xdf) return 1;
      if (b === 0xe0) return 4;
      if (b === 0xed) return 5;
      if ((b >= 0xe1 && b <= 0xec) || (b >= 0xee && b <= 0xef)) return 2;
      if (b === 0xf0) return 6;
      if (b === 0xf4) return 7;
      if (b >= 0xf1 && b <= 0xf3) return 3;
      return -1;
    }
    if (state <= 3) return b >= 0x80 && b <= 0xbf ? state - 1 : -1;
    if (state === 4) return b >= 0xa0 && b <= 0xbf ? 1 : -1;
    if (state === 5) return b >= 0x80 && b <= 0x9f ? 1 : -1;
    if (state === 6) return b >= 0x90 && b <= 0xbf ? 2 : -1;
    return b >= 0x80 && b <= 0x8f ? 2 : -1;
  }
  function numberStep(state, b) {
    const digit = b >= 48 && b <= 57;
    if (state === 0 || state === 1) {
      if (state === 0 && b === 45) return 1;
      if (b === 48) return 2;
      return b >= 49 && b <= 57 ? 3 : -1;
    }
    if (state === 2 || state === 3) {
      if (state === 3 && digit) return 3;
      if (b === 46) return 4;
      if (b === 101 || b === 69) return 6;
      return -1;
    }
    if (state === 4) return digit ? 5 : -1;
    if (state === 5) {
      if (digit) return 5;
      return b === 101 || b === 69 ? 6 : -1;
    }
    if (state === 6) {
      if (b === 43 || b === 45) return 7;
      return digit ? 8 : -1;
    }
    return digit ? 8 : -1;
  }
  function composeTransitions(left, right) {
    return left.map((state) => (state < 0 ? -1 : right[state]));
  }
  // create() authenticates supplied raw nodes and headers only. A partial pool
  // is deliberately usable for verifyRange; verifyComplete() is the separate
  // closure/semantic gate. text() and decodedValue() are explicit materializing
  // accessors, while renderChunks() does not materialize the entire value.
  class GraphView {
    #summaries = new Map();
    #keyOrders = new Map();
    #work = { contextsComputed: 0, transitionByteReads: 0n };
    #nodes;
    #headers;
    constructor(nodes, headers, metrics, guard) {
      if (guard !== GRAPH) fail("use GraphView.create for authenticated nodes");
      this.#nodes = nodes;
      this.#headers = headers;
      Object.defineProperty(this, "metrics", {
        value: Object.freeze(metrics),
        enumerable: true,
      });
    }
    static async create(nodes) {
      if (!Array.isArray(nodes)) fail("nodes array");
      const map = new Map(),
        headers = new Map();
      let framedBytes = 0n;
      const snapshot = nodes.map((input) => {
        exact(input, ["root", "tag", "body_hex"]);
        return Object.freeze({ ...input });
      });
      for (const raw of snapshot) {
        hashBytes(raw.root);
        if (typeof raw.tag !== "string" || !/^[0-9a-f]{2}$/.test(raw.tag))
          fail("tag");
        const b = unhex(raw.body_hex);
        if (map.has(raw.root)) fail("duplicate node/root collision");
        if ((await framedHash(parseInt(raw.tag, 16), b)) !== raw.root)
          fail("node hash");
        map.set(raw.root, raw);
        headers.set(raw.root, headerFromRaw(raw));
        framedBytes += BigInt(DOMAIN.length + 1 + 8 + b.length);
      }
      return new GraphView(
        map,
        headers,
        {
          uniqueNodes: map.size,
          framedBytes: framedBytes.toString(),
        },
        GRAPH,
      );
    }
    nodes() {
      return [...this.#nodes.values()].sort((a, b) =>
        a.root < b.root ? -1 : 1,
      );
    }
    has(root) {
      return this.#nodes.has(root);
    }
    header(root) {
      const h = this.#headers.get(root);
      if (!h) fail("missing node " + root);
      if (!h.ref) fail("internal node as value");
      return h.ref;
    }
    checkedRef(ref) {
      ref = validateRef(ref);
      if (!equal(this.header(ref.root), ref))
        fail("ContentRef header mismatch");
      return this.header(ref.root);
    }
    #get(root) {
      const h = this.#headers.get(root);
      if (!h) fail("missing node " + root);
      return h;
    }
    // Only intersecting branches are required by a range proof. Loaded branch
    // headers are checked against the position/count supplied by their parent.
    *#leaves(root, count, start = 0n, end = count, sparse = false) {
      const stack = [{ root, n: count, base: 0n, ancestors: new Set() }];
      while (stack.length) {
        const j = stack.pop();
        if (j.base >= end || j.base + j.n <= start) {
          if (j.n !== 0n) continue;
        }
        if (sparse && !this.has(j.root)) continue;
        if (j.ancestors.has(j.root)) fail("sequence cycle");
        const h = this.#get(j.root);
        if (j.n === 0n) {
          if (h.tag !== 0x11) fail("empty sequence shape");
          continue;
        }
        if (j.n === 1n) {
          if (h.tag !== 0x12) fail("single sequence shape");
          yield { root: child(h.b), index: j.base };
          continue;
        }
        if (h.tag !== 0x13 || h.n !== j.n) fail("sequence split/count");
        const k = split(j.n),
          ancestors = new Set(j.ancestors);
        ancestors.add(j.root);
        stack.push(
          { root: child(h.b, 40), n: j.n - k, base: j.base + k, ancestors },
          { root: child(h.b, 8), n: k, base: j.base, ancestors },
        );
      }
    }
    *byteChunks(ref, start = 0n, end = null) {
      ref = this.checkedRef(ref);
      if (ref.kind !== "string" && ref.kind !== "number") fail("byte value");
      const h = this.#get(ref.root),
        total = h.n;
      if (end === null) end = total;
      if (start < 0n || start > end || end > total) fail("byte range");
      const count = (total + 4095n) / 4096n;
      if (total === 0n) {
        [...this.#leaves(child(h.b, 8), 0n)];
        return;
      }
      if (start === end) return;
      for (const leaf of this.#leaves(
        child(h.b, 8),
        count,
        start / 4096n,
        (end + 4095n) / 4096n,
      )) {
        const chunk = this.#get(leaf.root),
          expected =
            leaf.index === count - 1n
              ? Number(total - leaf.index * 4096n)
              : 4096;
        if (chunk.tag !== 0x10 || chunk.b.length !== expected)
          fail("byte block boundary");
        const base = leaf.index * 4096n,
          lo = Number(start > base ? start - base : 0n),
          hi = Number(
            end < base + BigInt(expected) ? end - base : BigInt(expected),
          );
        yield chunk.b.slice(lo, hi);
      }
    }
    text(ref) {
      const decoder = new TextDecoder("utf-8", {
        fatal: true,
        ignoreBOM: true,
      });
      let text = "";
      for (const b of this.byteChunks(ref))
        text += decoder.decode(b, { stream: true });
      text += decoder.decode();
      if (ref.kind === "number" && !NUMBER.test(text)) fail("number token");
      return text;
    }
    *entries(ref, start = 0n, end = null, sparse = false) {
      ref = this.checkedRef(ref);
      if (!["array", "object"].includes(ref.kind)) fail("container");
      const h = this.#get(ref.root);
      if (end === null) end = h.n;
      if (start < 0n || start > end || end > h.n) fail("item range");
      let previous = null;
      for (const leaf of this.#leaves(child(h.b, 8), h.n, start, end, sparse)) {
        if (ref.kind === "array") {
          const value_ref = this.header(leaf.root);
          if (value_ref.kind === "template_batch") fail("template public");
          yield { index: leaf.index, value_ref };
        } else {
          if (sparse && !this.has(leaf.root)) continue;
          const f = this.#get(leaf.root);
          if (f.tag !== 0x14) fail("object field role");
          if (sparse && !this.has(child(f.b))) continue;
          const keyRef = this.header(child(f.b));
          if (keyRef.kind !== "string") fail("field key role");
          const key = this.text(keyRef);
          if (previous !== null && compareText(previous, key) >= 0)
            fail("field key order");
          previous = key;
          if (sparse && !this.has(child(f.b, 32))) continue;
          const value_ref = this.header(child(f.b, 32));
          if (value_ref.kind === "template_batch") fail("template public");
          yield { index: leaf.index, key, value_ref };
        }
      }
    }
    at(ref, index) {
      index = typeof index === "bigint" ? index : decimal(index);
      ref = this.checkedRef(ref);
      if (ref.kind !== "array" || index >= decimal(ref.extent))
        fail("array index");
      return this.entries(ref, index, index + 1n).next().value.value_ref;
    }
    field(ref, key, { partial = false } = {}) {
      utf8(key);
      ref = this.checkedRef(ref);
      if (ref.kind !== "object") fail("field path role");
      let found = null;
      for (const e of this.entries(ref, 0n, decimal(ref.extent), partial))
        if (e.key === key) {
          if (found) fail("duplicate field membership");
          found = e.value_ref;
        }
      if (!found) fail("field membership missing");
      return found;
    }
    resolve(ref, path, { partial = false } = {}) {
      path = validatePath(path);
      ref = this.checkedRef(ref);
      let ordinary = false;
      for (const step of path) {
        if (ordinary && ["json_text", "template_batch"].includes(ref.kind))
          fail("json_text descendant");
        if (Object.hasOwn(step, "field"))
          ref = this.field(ref, step.field, { partial });
        else if (Object.hasOwn(step, "index")) ref = this.at(ref, step.index);
        else {
          if (ref.kind !== "json_text") fail("json_text path role");
          ordinary = true;
          ref = this.header(child(this.#get(ref.root).b));
          if (["json_text", "template_batch"].includes(ref.kind))
            fail("json_text descendant");
        }
      }
      if (
        ref.kind === "template_batch" ||
        (ordinary && ref.kind === "json_text")
      )
        fail("template/json_text descendant");
      return ref;
    }
    #compareKeys(a, b) {
      if (a.root === b.root) return 0;
      const pair = a.root + ":" + b.root;
      if (this.#keyOrders.has(pair)) return this.#keyOrders.get(pair);
      const left = [child(this.#get(a.root).b, 8)],
        right = [child(this.#get(b.root).b, 8)];
      const expand = (stack, root, h) => {
        if (h.tag === 0x13) stack.push(child(h.b, 40), child(h.b, 8));
        else if (h.tag === 0x12) stack.push(child(h.b));
        else if (h.tag !== 0x11) fail("key byte role");
      };
      let result = 0;
      while (left.length && right.length) {
        const ar = left.pop(),
          br = right.pop();
        if (ar === br) continue;
        const ah = this.#get(ar),
          bh = this.#get(br);
        if (ah.tag === 0x10 && bh.tag === 0x10) {
          result = byteCompare(ah.b, bh.b);
          if (result) break;
          continue;
        }
        if (ah.tag === 0x10) {
          left.push(ar);
          expand(right, br, bh);
        } else if (bh.tag === 0x10) {
          right.push(br);
          expand(left, ar, ah);
        } else {
          expand(left, ar, ah);
          expand(right, br, bh);
        }
      }
      if (!result) result = left.length ? 1 : right.length ? -1 : 0;
      this.#keyOrders.set(pair, result);
      this.#keyOrders.set(b.root + ":" + a.root, -result);
      return result;
    }
    #blockSummary(h, mode) {
      if (h.tag !== 0x10) fail("byte block role");
      const states = mode === "number" ? 9 : 8,
        step = mode === "number" ? numberStep : utf8Step,
        transitions = [];
      for (let entry = 0; entry < states; entry++) {
        let state = entry;
        for (const b of h.b) {
          this.#work.transitionByteReads++;
          state = step(state, b);
          if (state < 0) break;
        }
        transitions.push(state);
      }
      return {
        length: BigInt(h.b.length),
        full: h.b.length === 4096,
        nonLastFull: true,
        lastLength: h.b.length,
        transitions,
      };
    }
    async verifyComplete(ref) {
      ref = this.checkedRef(ref);
      const make = (root, role = "value", n = null, ordinary = false) => ({
        root,
        role,
        n,
        ordinary,
        key: root + ":" + role + ":" + n + ":" + ordinary,
      });
      const first = make(ref.root),
        stack = [first],
        active = new Set();
      while (stack.length) {
        const j = stack[stack.length - 1];
        if (this.#summaries.has(j.key)) {
          stack.pop();
          continue;
        }
        if (!j.prepared) {
          if (active.has(j.key)) fail("graph cycle");
          active.add(j.key);
          j.prepared = true;
          j.h = this.#get(j.root);
          j.deps = [];
          const h = j.h;
          if (j.role === "value" || j.role === "key") {
            if (!h.ref || h.tag === 8 || (j.ordinary && h.tag === 7))
              fail("nonordinary/template value");
            if (j.role === "key" && h.tag !== 3) fail("field key role");
            if (h.tag === 3 || h.tag === 4)
              j.deps = [
                make(
                  child(h.b, 8),
                  "seq:bytes:" + (h.tag === 4 ? "number" : "string"),
                  (h.n + 4095n) / 4096n,
                  j.ordinary,
                ),
              ];
            else if (h.tag === 5 || h.tag === 6)
              j.deps = [
                make(
                  child(h.b, 8),
                  h.tag === 5 ? "seq:value" : "seq:field",
                  h.n,
                  j.ordinary,
                ),
              ];
            else if (h.tag === 7)
              j.deps = [make(child(h.b), "value", null, true)];
          } else if (j.role === "field") {
            if (h.tag !== 0x14) fail("object field role");
            j.deps = [
              make(child(h.b), "key"),
              make(child(h.b, 32), "value", null, j.ordinary),
            ];
          } else if (j.role.startsWith("block:")) {
            j.direct = this.#blockSummary(h, j.role.slice(6));
          } else if (j.role.startsWith("seq:")) {
            if (j.n === 0n) {
              if (h.tag !== 0x11) fail("empty sequence shape");
            } else if (j.n === 1n) {
              if (h.tag !== 0x12) fail("single sequence shape");
              const leaf =
                j.role === "seq:value"
                  ? "value"
                  : j.role === "seq:field"
                    ? "field"
                    : "block:" + j.role.slice(10);
              j.deps = [make(child(h.b), leaf, null, j.ordinary)];
            } else {
              if (h.tag !== 0x13 || h.n !== j.n) fail("sequence split/count");
              const k = split(j.n);
              j.deps = [
                make(child(h.b, 8), j.role, k, j.ordinary),
                make(child(h.b, 40), j.role, j.n - k, j.ordinary),
              ];
            }
          } else fail("validation role");
          j.next = 0;
        }
        if (j.next < j.deps.length) {
          const dep = j.deps[j.next++];
          if (!this.#summaries.has(dep.key)) stack.push(dep);
          continue;
        }
        const values = j.deps.map((d) => this.#summaries.get(d.key)),
          h = j.h;
        let summary = j.direct || {};
        if (j.role === "field")
          summary = {
            firstKey: this.header(child(h.b)),
            lastKey: this.header(child(h.b)),
          };
        else if (j.role.startsWith("seq:bytes:")) {
          if (j.n === 0n)
            summary = {
              length: 0n,
              full: true,
              nonLastFull: true,
              lastLength: 0,
              transitions: Array.from(
                { length: j.role.endsWith("number") ? 9 : 8 },
                (_, i) => i,
              ),
            };
          else if (j.n === 1n) summary = values[0];
          else {
            const [l, r] = values;
            summary = {
              length: l.length + r.length,
              full: l.full && r.full,
              nonLastFull: l.full && r.nonLastFull,
              lastLength: r.lastLength,
              transitions: composeTransitions(l.transitions, r.transitions),
            };
          }
        } else if (j.role === "seq:field") {
          if (j.n === 0n) summary = { firstKey: null, lastKey: null };
          else if (j.n === 1n) summary = values[0];
          else {
            const [l, r] = values;
            if (this.#compareKeys(l.lastKey, r.firstKey) >= 0)
              fail("field key order");
            summary = { firstKey: l.firstKey, lastKey: r.lastKey };
          }
        } else if (
          (j.role === "value" || j.role === "key") &&
          (h.tag === 3 || h.tag === 4)
        ) {
          const bytes = values[0],
            count = (h.n + 4095n) / 4096n;
          if (
            bytes.length !== h.n ||
            !bytes.nonLastFull ||
            (h.n !== 0n &&
              bytes.lastLength !== Number(h.n - (count - 1n) * 4096n))
          )
            fail("byte block boundary/extent");
          if (h.tag === 3) {
            if (bytes.transitions[0] !== 0) fail("UTF-8");
          } else if (![2, 3, 5, 8].includes(bytes.transitions[0]))
            fail("number token");
        }
        this.#summaries.set(j.key, summary);
        this.#work.contextsComputed++;
        active.delete(j.key);
        stack.pop();
      }
      return Object.freeze({
        ref,
        semanticValidated: true,
        metrics: this.metrics,
        validationWork: Object.freeze({
          contextsComputed: this.#work.contextsComputed,
          transitionByteReads: this.#work.transitionByteReads.toString(),
        }),
      });
    }
    async decodedValue(ref) {
      ref = this.checkedRef(ref);
      await this.verifyComplete(ref);
      const box = {},
        stack = [
          {
            ref,
            set: (v) => {
              box.value = v;
            },
          },
        ];
      while (stack.length) {
        const j = stack.pop(),
          r = j.ref;
        let a;
        if (r.kind === "null") a = { t: "null" };
        else if (r.kind === "bool")
          a = { t: "bool", value: this.#get(r.root).tag === 2 };
        else if (r.kind === "string") a = { t: "string", value: this.text(r) };
        else if (r.kind === "number") a = { t: "number", token: this.text(r) };
        else if (r.kind === "json_text") {
          a = { t: "json_text", value: null };
          stack.push({
            ref: this.header(child(this.#get(r.root).b)),
            set: (v) => {
              a.value = v;
            },
          });
        } else {
          const entries = [...this.entries(r)];
          a =
            r.kind === "array"
              ? { t: "array", items: new Array(entries.length) }
              : { t: "object", fields: new Array(entries.length) };
          for (let i = entries.length - 1; i >= 0; i--) {
            const e = entries[i],
              idx = i;
            if (r.kind === "object") a.fields[idx] = [e.key, null];
            stack.push({
              ref: e.value_ref,
              set: (v) => {
                if (a.t === "array") a.items[idx] = v;
                else a.fields[idx][1] = v;
              },
            });
          }
        }
        j.set(a);
      }
      return box.value;
    }
    *#renderEntries(ref) {
      const h = this.#get(ref.root);
      for (const leaf of this.#leaves(child(h.b, 8), h.n)) {
        if (ref.kind === "array") yield { value_ref: this.header(leaf.root) };
        else {
          const f = this.#get(leaf.root);
          yield {
            key_ref: this.header(child(f.b)),
            value_ref: this.header(child(f.b, 32)),
          };
        }
      }
    }
    async *renderChunks(ref) {
      ref = this.checkedRef(ref);
      await this.verifyComplete(ref);
      const stack = [{ ref, quoteLevel: 0 }];
      function escaped(s, n) {
        for (let i = 0; i < n; i++) s = quoteBody(s);
        return s;
      }
      while (stack.length) {
        const j = stack.pop(),
          q = j.quoteLevel;
        // Every task kind is dispatched here, including container continuations
        // after a json_text closing quote. No frame depends on its predecessor.
        if (Object.hasOwn(j, "text")) {
          yield escaped(j.text, q);
          continue;
        }
        if (j.iterator) {
          const n = j.iterator.next();
          if (n.done) {
            yield escaped(j.object ? "}" : "]", q);
            continue;
          }
          if (!j.first) yield escaped(",", q);
          j.first = false;
          stack.push(j, { ref: n.value.value_ref, quoteLevel: q });
          if (j.object)
            stack.push(
              { text: ":", quoteLevel: q },
              { ref: n.value.key_ref, quoteLevel: q },
            );
          continue;
        }
        const r = j.ref;
        if (r.kind === "null") yield escaped("null", q);
        else if (r.kind === "bool")
          yield escaped(this.#get(r.root).tag === 2 ? "true" : "false", q);
        else if (r.kind === "string" || r.kind === "number") {
          const decoder = new TextDecoder("utf-8", {
            fatal: true,
            ignoreBOM: true,
          });
          if (r.kind === "string") yield escaped('"', q);
          for (const b of this.byteChunks(r)) {
            const text = decoder.decode(b, { stream: true });
            yield escaped(r.kind === "string" ? quoteBody(text) : text, q);
          }
          const tail = decoder.decode();
          if (tail)
            yield escaped(r.kind === "string" ? quoteBody(tail) : tail, q);
          if (r.kind === "string") yield escaped('"', q);
        } else if (r.kind === "json_text") {
          yield escaped('"', q);
          stack.push(
            { text: '"', quoteLevel: q },
            { ref: this.header(child(this.#get(r.root).b)), quoteLevel: q + 1 },
          );
        } else {
          const object = r.kind === "object";
          yield escaped(object ? "{" : "[", q);
          stack.push({
            iterator: this.#renderEntries(r),
            object,
            first: true,
            quoteLevel: q,
          });
        }
      }
    }
  }
  // Control JSON only. Numeric business content must use the typed codec above.
  function canonicalJSON(value) {
    const parts = [],
      active = new Set(),
      stack = [{ v: value }];
    while (stack.length) {
      const j = stack.pop();
      if (j.end) {
        active.delete(j.end);
        continue;
      }
      if (Object.hasOwn(j, "s")) {
        parts.push(j.s);
        continue;
      }
      const v = j.v;
      if (v === null) parts.push("null");
      else if (typeof v === "boolean") parts.push(v ? "true" : "false");
      else if (typeof v === "string") {
        utf8(v);
        parts.push('"' + quoteBody(v) + '"');
      } else if (typeof v === "number") {
        if (!Number.isSafeInteger(v) || Object.is(v, -0))
          fail("control JSON Number");
        parts.push(String(v));
      } else if (v && typeof v === "object") {
        if (active.has(v)) fail("control JSON cycle");
        active.add(v);
        stack.push({ end: v });
        if (Array.isArray(v)) {
          stack.push({ s: "]" });
          for (let i = v.length - 1; i >= 0; i--) {
            stack.push({ v: v[i] });
            if (i) stack.push({ s: "," });
          }
          parts.push("[");
        } else {
          if (Object.getPrototypeOf(v) !== Object.prototype)
            fail("control JSON object");
          const keys = Object.keys(v).sort(compareText);
          stack.push({ s: "}" });
          for (let i = keys.length - 1; i >= 0; i--) {
            stack.push(
              { v: v[keys[i]] },
              { s: '"' + quoteBody(keys[i]) + '":' },
            );
            if (i) stack.push({ s: "," });
          }
          parts.push("{");
        }
      } else fail("control JSON type");
    }
    return parts.join("");
  }
  function equal(a, b) {
    return canonicalJSON(a) === canonicalJSON(b);
  }
  function validatePath(path) {
    path = freezeJSON(path);
    if (!Array.isArray(path)) fail("path");
    for (const s of path) {
      if (s && Object.hasOwn(s, "field")) {
        exact(s, ["field"]);
        utf8(s.field);
      } else if (s && Object.hasOwn(s, "index")) {
        exact(s, ["index"]);
        decimal(s.index);
      } else {
        exact(s, ["json_text"]);
        if (s.json_text !== true) fail("path json_text");
      }
    }
    return path;
  }
  function validateAnchor(a) {
    a = freezeJSON(a);
    if (!a || typeof a !== "object") fail("anchor");
    if (a.kind === "projection") {
      exact(a, [
        "kind",
        "fixed_cut",
        "cut_projection_digest",
        "projection_ref",
      ]);
      if (
        !a.fixed_cut ||
        typeof a.fixed_cut !== "object" ||
        Array.isArray(a.fixed_cut)
      )
        fail("fixed_cut");
      if (
        typeof a.cut_projection_digest !== "string" ||
        !/^qcut3-[0-9a-f]{64}$/.test(a.cut_projection_digest)
      )
        fail("qcut3");
      validateRef(a.projection_ref);
      if (a.projection_ref.kind !== "object") fail("projection kind");
      return a.projection_ref;
    }
    if (a.kind === "delta") {
      exact(a, [
        "kind",
        "session_id",
        "session_generation",
        "generation",
        "base_cut",
        "next_cut",
        "index_frontier",
        "delta_ref",
      ]);
      utf8(a.session_id);
      decimal(a.session_generation, true);
      decimal(a.generation, true);
      utf8(a.base_cut);
      utf8(a.next_cut);
      if (
        typeof a.index_frontier !== "string" ||
        !/^batch3-[0-9a-f]{64}$/.test(a.index_frontier)
      )
        fail("batch3");
      validateRef(a.delta_ref);
      if (a.delta_ref.kind !== "object") fail("delta kind");
      return a.delta_ref;
    }
    fail("anchor kind");
  }
  function capture(s) {
    if (typeof s !== "string" || !/^[0-9a-f]{64}$/.test(s))
      fail("capture digest");
  }
  const TOKEN = [
    "token_schema",
    "anchor",
    "path",
    "target_ref",
    "unit",
    "offset",
    "limit",
    "issued_capture_digest",
    "token_checksum",
  ];
  async function tokenChecksum(token) {
    const copy = { ...token };
    delete copy.token_checksum;
    return sha256(utf8(canonicalJSON(copy)));
  }
  async function validateToken(token) {
    token = freezeJSON(token);
    exact(token, TOKEN);
    if (token.token_schema !== "s-content-token/1") fail("token schema");
    validateAnchor(token.anchor);
    validatePath(token.path);
    validateRef(token.target_ref);
    const offset = decimal(token.offset),
      limit = decimal(token.limit, true);
    if (offset + limit > MAX) fail("token offset plus limit overflow");
    if (!["items", "bytes", "value"].includes(token.unit)) fail("token unit");
    capture(token.issued_capture_digest);
    capture(token.token_checksum);
    if ((await tokenChecksum(token)) !== token.token_checksum)
      fail("token checksum");
    return token;
  }
  // A successful interval is not evidence of an assembled prefix. Stream owners
  // pass their previously accepted next_token as previousToken on continuation;
  // initial nonzero requests remain independent intervals. Anchor publication
  // authority and staged/committed installation remain the caller's gates.
  async function verifyRange(request, response, options = {}) {
    // Snapshot bounded control inputs before async hashing. Cache is never used
    // to satisfy proof dependencies: each response builds its own GraphView.
    request = freezeJSON(request);
    response = freezeJSON(response);
    const prior = options.previousToken;
    const previousToken = prior === undefined ? undefined : freezeJSON(prior);
    const cache = options.cache;
    if (cache !== undefined && !(cache instanceof Map)) fail("cache Map");
    if (request.op !== "content") fail("content op");
    let c;
    if (Object.hasOwn(request, "content_token")) {
      exact(request, ["op", "content_token"]);
      await validateToken(request.content_token);
      c = request.content_token;
    } else {
      exact(request, ["op", "anchor", "path", "unit", "offset", "limit"]);
      c = request;
      validateAnchor(c.anchor);
      validatePath(c.path);
    }
    exact(response, [
      "anchor",
      "path",
      "target_ref",
      "unit",
      "offset",
      "end",
      "total",
      "items",
      "nodes",
      "done",
      "next_token",
      "validated_capture_digest",
      "ok",
    ]);
    if (response.ok !== true) fail("content not ok");
    capture(response.validated_capture_digest);
    for (const k of ["anchor", "path", "unit", "offset"])
      if (!equal(response[k], c[k])) fail("request echo " + k);
    const offset = decimal(c.offset),
      limit = decimal(c.limit, true);
    if (offset + limit > MAX) fail("offset plus limit overflow");
    const total = decimal(response.total),
      end = decimal(response.end);
    if (
      offset > total ||
      end !== (offset + limit < total ? offset + limit : total)
    )
      fail("range end");
    if (typeof response.done !== "boolean" || response.done !== (end === total))
      fail("done");
    if (!Array.isArray(response.nodes)) fail("nodes");
    for (let i = 1; i < response.nodes.length; i++)
      if (response.nodes[i - 1].root >= response.nodes[i].root)
        fail("node root order");
    const graph = await GraphView.create(response.nodes),
      root = validateAnchor(c.anchor),
      target = graph.resolve(root, c.path, { partial: true });
    graph.checkedRef(response.target_ref);
    if (!equal(target, response.target_ref)) fail("target identity");
    if (c.target_ref && !equal(c.target_ref, target))
      fail("token target changed");
    if (previousToken) {
      const p = previousToken;
      await validateToken(p);
      if (
        !Object.hasOwn(request, "content_token") ||
        !equal(p, request.content_token)
      )
        fail("continuity token");
    }
    const kind = target.kind,
      unit =
        kind === "array" || kind === "object"
          ? "items"
          : kind === "string" || kind === "number"
            ? "bytes"
            : "value";
    if (c.unit !== unit) fail("unit");
    const actualTotal = unit === "value" ? 1n : decimal(target.extent);
    if (total !== actualTotal) fail("total header");
    let expected;
    if (unit === "items")
      expected = [...graph.entries(target, offset, end)].map((e) => {
        if (
          c.path.some((s) => Object.hasOwn(s, "json_text")) &&
          e.value_ref.kind === "json_text"
        )
          fail("json_text descendant");
        return kind === "object"
          ? { key: e.key, value_ref: e.value_ref }
          : e.value_ref;
      });
    else if (unit === "bytes") {
      expected = "";
      for (const b of graph.byteChunks(target, offset, end)) expected += hex(b);
    } else {
      if (limit !== 1n) fail("value limit");
      // The 07 value item stays unchanged. Its independent proof must include
      // the direct ordinary-JSON header needed to choose the next path unit.
      if (kind === "json_text") graph.resolve(target, [{ json_text: true }]);
      expected = offset === 0n ? [target] : [];
    }
    if (!equal(response.items, expected)) fail("range contents/position");
    if (end === total) {
      if (response.next_token !== null) fail("terminal token");
    } else {
      await validateToken(response.next_token);
      const t = response.next_token;
      for (const k of ["anchor", "path", "target_ref", "unit"])
        if (!equal(t[k], k === "target_ref" ? target : c[k]))
          fail("next token identity");
      if (t.offset !== end.toString() || t.limit !== c.limit)
        fail("next token extent");
      if (
        t.issued_capture_digest !==
        (c.issued_capture_digest || response.validated_capture_digest)
      )
        fail("issued capture changed");
    }
    // Conflict preflight precedes all cache writes, including unrelated extras.
    if (cache) {
      for (const n of graph.nodes()) {
        const old = Map.prototype.get.call(cache, n.root);
        if (old && !equal(old, n)) fail("cache root conflict");
      }
      for (const n of graph.nodes()) Map.prototype.set.call(cache, n.root, n);
    }
    return Object.freeze({
      target_ref: target,
      offset: offset.toString(),
      end: end.toString(),
      total: total.toString(),
      done: response.done,
      next_token: response.next_token,
      graph,
      rangeValidated: true,
      continuityValidated: !!previousToken,
      semanticValidated: false,
      metrics: graph.metrics,
    });
  }
  async function encodeJSONText(text) {
    const ast = parseJSON(text),
      encoded = await encode(ast),
      graph = await GraphView.create(encoded.nodes);
    let at = 0;
    for await (const c of graph.renderChunks(encoded.ref)) {
      if (text.slice(at, at + c.length) !== c)
        fail("noncanonical original JSON text");
      at += c.length;
    }
    if (at !== text.length) fail("noncanonical original JSON text");
    return encode({ t: "json_text", value: ast });
  }
  return Object.freeze({
    encode,
    encodeJSONText,
    fromPlain,
    parseJSON,
    refEdge,
    validateRef,
    validateAnchor,
    validatePath,
    canonicalJSON,
    tokenChecksum,
    validateToken,
    verifyRange,
    GraphView,
  });
});

/* exact source s-content-job-v3.js SHA256 df8c2a3c9864fd2290e1c929dfb5d9cdad799539cca585e424440d29776b402c */
/* #1394 C §5.6.1：纯累计账；实际 I/O、实际分配测量及业务验证属于调用方。 */
(function (root, factory) {
  "use strict";
  const common = typeof module === "object" && module.exports;
  const api = factory(common ? require("./s-content-v3.js") : root.SContentV3);
  if (common) module.exports = api;
  else root.SContentJobV3 = api;
})(typeof globalThis === "object" ? globalThis : this, function (content) {
  "use strict";
  const MAX = (1n << 63n) - 1n;
  const LIMIT_FIELDS = Object.freeze(["max_total_wire_bytes", "max_total_node_bytes",
    "max_total_nodes", "max_total_records", "max_total_requests", "max_elapsed_ms",
    "max_staged_bytes", "max_store_bytes"]);
  function fail(code, detail) {
    const error = new Error(detail);
    error.code = code;
    throw error;
  }
  function exact(value, fields, code) {
    if (!value || typeof value !== "object" || Array.isArray(value) ||
      Reflect.ownKeys(value).length !== fields.length || fields.some(k => !Object.hasOwn(value, k)))
      fail(code, "字段集合必须 exact；没有缺省值");
  }
  function captureFields(value, fields, code) {
    exact(value, fields, code);
    // 不在验证后重读调用方属性；getter 产生的值也只捕获一次。
    return Object.freeze(Object.fromEntries(fields.map(k => [k, value[k]])));
  }
  function parseLimits(value) {
    value = captureFields(value, LIMIT_FIELDS, "job_limit_field");
    const copy = {};
    for (const key of LIMIT_FIELDS) {
      const text = value[key];
      if (typeof text !== "string") fail("job_limit_field", key + " 必须是十进制文本");
      if (!/^[1-9][0-9]*$/.test(text) || BigInt(text) > MAX)
        fail("job_limit_domain", key + " 不在规范 1..2^63-1 域");
      copy[key] = text;
    }
    return Object.freeze(copy);
  }
  function validateConfig(value) {
    const controls = ["pageSize", "watchBatches", "pollMs", "timeoutMs", "replyBytes"];
    value = captureFields(value, ["schema_revision", ...controls, "job_limits"], "job_config_field");
    if (value.schema_revision !== "s-query-client-resources/3")
      fail("job_config_schema", "浏览器配置必须为 s-query-client-resources/3");
    const copy = {schema_revision: value.schema_revision};
    for (const key of controls) {
      const n = value[key];
      if (!Number.isSafeInteger(n) || n <= 0) fail("job_config_domain", key + " 必须是显式安全正整数");
      copy[key] = n;
    }
    copy.job_limits = parseLimits(value.job_limits);
    return Object.freeze(copy);
  }
  function amount(value, code = "job_amount") {
    if (typeof value === "bigint" && value >= 0n) return value;
    if (typeof value === "string" && /^(?:0|[1-9][0-9]*)$/.test(value)) return BigInt(value);
    fail(code, "实际量必须是非负 BigInt 或规范非负十进制文本");
  }
  const space = baseline => ({baseline, committed: 0n, pending: 0n, peak: baseline});
  class Job {
    #limits; #caps; #clock; #readingClock = false; #last = null; #started = null; #stopped = null;
    #counts = {wire_bytes: 0n, node_bytes: 0n, nodes: 0n, records: 0n, requests: 0n};
    #staged; #store; #accepted = new Map(); #open = new Map();
    constructor(limits, existing, clock) {
      this.#limits = parseLimits(limits);
      this.#caps = Object.fromEntries(LIMIT_FIELDS.map(k => [k, BigInt(this.#limits[k])]));
      existing = captureFields(existing, ["store_bytes", "staged_bytes"], "job_existing");
      const store = amount(existing.store_bytes, "job_limit_domain");
      const staged = amount(existing.staged_bytes, "job_limit_domain");
      if (staged > store) fail("job_limit_domain", "既存暂存必须是总存储的子集");
      if (staged > this.#caps.max_staged_bytes || store > this.#caps.max_store_bytes)
        fail("job_limit_exceeded", "既存空间已超上限，即使纯读取也拒绝");
      if (typeof clock !== "function") fail("job_clock", "必须显式注入单调毫秒时钟");
      this.#clock = clock;
      this.#staged = space(staged); this.#store = space(store);
    }
    #stop(code, detail) {
      if (this.#stopped === null) this.#stopped = Object.freeze({code, detail});
      fail(this.#stopped.code, this.#stopped.detail);
    }
    #assertLive() {
      this.#assertOutsideClock();
      if (this.#stopped) fail(this.#stopped.code, this.#stopped.detail);
    }
    #assertOutsideClock() {
      if (this.#readingClock) this.#stop("job_clock", "时钟回调不得重入 Job 操作");
    }
    #now() {
      this.#assertOutsideClock();
      let now;
      this.#readingClock = true;
      try {
        let raw = this.#clock();
        if (typeof raw === "number" && Number.isSafeInteger(raw) && raw >= 0) raw = BigInt(raw);
        now = amount(raw, "job_clock");
      } catch { this.#stop("job_clock", "时钟回调失败或未返回规范非负毫秒值"); }
      finally { this.#readingClock = false; }
      if (this.#last !== null && now < this.#last) this.#stop("job_clock", "时钟不得倒退");
      this.#last = now;
      return now;
    }
    #checkTime(now) {
      if (this.#started !== null && now - this.#started >= this.#caps.max_elapsed_ms)
        this.#stop("job_deadline", "首次读取的绝对期限已过");
    }
    #gate() {
      this.#assertLive();
      const now = this.#now();
      // 外部回调可能 cancel 或吞掉重入拒绝；返回后必须再核永久停止状态。
      this.#assertLive();
      this.#checkTime(now);
    }
    beginFirstRead() {
      this.#gate();
      if (this.#started === null) this.#started = this.#last;
    }
    firstReadDeadline() {
      this.beginFirstRead();
      return (this.#started + this.#caps.max_elapsed_ms).toString();
    }
    checkDeadline() { this.#gate(); }
    preIoCheck() { this.#gate(); }
    // 本口只收已经发生的实际量：先永久入账，再判 cap/期限。
    // 写前预检被拒、没有访问或发起请求时，adapter 不得调用 actual charge。
    #charge(field, cap, actual) {
      this.#assertLive();
      actual = amount(actual);
      // 已发生且合法的 actual 不可逆；时钟失败也不能丢掉尾量。
      this.#counts[field] += actual;
      const now = this.#now();
      this.#assertLive();
      if (this.#started === null) this.#started = now;
      if (this.#counts[field] > this.#caps[cap]) this.#stop("job_limit_exceeded", cap + " 实际累计已超上限");
      this.#checkTime(now);
    }
    chargeWireBytes(actual) { this.#charge("wire_bytes", "max_total_wire_bytes", actual); }
    chargeRecords(actual) { this.#charge("records", "max_total_records", actual); }
    chargeRequests(actual) { this.#charge("requests", "max_total_requests", actual); }
    async acceptNode(node) {
      this.beginFirstRead();
      // 输入均为字符串，await 前复制；不保存调用方可变对象或借用其字节缓冲区。
      const raw = captureFields(node, ["root", "tag", "body_hex"], "node_digest");
      this.#assertLive();
      if ([raw.root, raw.tag, raw.body_hex].some(v => typeof v !== "string"))
        fail("node_digest", "节点原件字段必须为字符串");
      if (!content?.GraphView) fail("job_content_core", "必须加载固定 s-content-v3 核心");
      let graph;
      try { graph = await content.GraphView.create([raw]); }
      catch (error) {
        this.#gate();
        fail("node_digest", "原件 F 或局部 header 不合法：" + error.message);
      }
      // hash 的异步边界后复查取消/绝对期限；晚到结果不得重新开启 job。
      this.#gate();
      const old = this.#accepted.get(raw.root);
      if (old) {
        if (old.tag !== raw.tag || old.body_hex !== raw.body_hex)
          fail("node_digest", "相同根不同字节");
        return "already_accepted";
      }
      const bytes = this.#counts.node_bytes + BigInt(graph.metrics.framedBytes);
      const nodes = this.#counts.nodes + 1n;
      // “接纳”的写前准入：候选 F 核过不等于本 job 已接纳；真实 wire 另走 actual charge。
      if (bytes > this.#caps.max_total_node_bytes || nodes > this.#caps.max_total_nodes)
        this.#stop("job_limit_exceeded", "候选节点超出节点接纳上限");
      this.#counts.node_bytes = bytes; this.#counts.nodes = nodes;
      this.#accepted.set(raw.root, raw);
      return "first_accept";
    }
    cancel() {
      if (this.#stopped === null) this.#stopped = Object.freeze({code: "job_cancelled", detail: "调用方取消 job"});
    }
    #refreshPeaks() {
      const staged = this.#stagedExposure(), store = this.#storeExposure();
      if (staged > this.#staged.peak) this.#staged.peak = staged;
      if (store > this.#store.peak) this.#store.peak = store;
    }
    #reserve(kind, requested) {
      this.#gate(); requested = amount(requested);
      // 两本 cap 先一起核完，再动 pending/open/peak，拒绝时不残留部分预留。
      if ((kind === "staged" && this.#stagedExposure() + requested > this.#caps.max_staged_bytes) ||
        this.#storeExposure() + requested > this.#caps.max_store_bytes)
        this.#stop("job_limit_exceeded", "空间预留超出暂存或总存储上限");
      const grant = Object.freeze({kind, requested: requested.toString()});
      const entry = Object.freeze({kind, requested});
      // 对象 identity 与本 job 的私有表绑定，spread/继承/跨 job 都不是句柄。
      this.#open.set(grant, entry);
      (kind === "staged" ? this.#staged : this.#store).pending += requested;
      this.#refreshPeaks();
      return grant;
    }
    reserveStaged(requested) { return this.#reserve("staged", requested); }
    reserveStore(requested) { return this.#reserve("store", requested); }
    #entry(grant) {
      this.#assertOutsideClock();
      const entry = this.#open.get(grant);
      if (!entry) fail("job_reservation", "伪造、跨 job 或已关闭的预留句柄");
      return entry;
    }
    reconcile(grant, actual) {
      const entry = this.#entry(grant); actual = amount(actual);
      const ledger = entry.kind === "staged" ? this.#staged : this.#store;
      this.#open.delete(grant); ledger.pending -= entry.requested; ledger.committed += actual;
      this.#refreshPeaks();
      // stopped 仍能清理旧预留；实际超预留必须照实进账，首次停止原因不会被替换。
      if (actual > entry.requested) this.#stop("job_reconcile", "实际分配超过预留，已照实入账");
    }
    release(grant) {
      const entry = this.#entry(grant);
      this.#open.delete(grant);
      (entry.kind === "staged" ? this.#staged : this.#store).pending -= entry.requested;
    }
    reclaimStaged(actualDeleted) {
      this.#assertOutsideClock();
      actualDeleted = amount(actualDeleted);
      if (actualDeleted > this.#staged.committed) fail("job_reclaim", "只能回收本 job 已对账的暂存");
      this.#staged.committed -= actualDeleted;
    }
    finishCheck() {
      this.#gate();
      if (this.#open.size) fail("job_reservation", "安装前必须完成全部预留对账或释放");
    }
    #stagedExposure() { const s = this.#staged; return s.baseline + s.committed + s.pending; }
    #storeExposure() {
      const s = this.#store;
      return s.baseline + s.committed + s.pending + this.#staged.committed + this.#staged.pending;
    }
    #spaceSnapshot(s, exposure) {
      return Object.freeze({baseline: s.baseline.toString(), committed: s.committed.toString(),
        pending: s.pending.toString(), exposure: exposure.toString(), peak: s.peak.toString()});
    }
    snapshot() {
      const now = this.#now();
      // 取快照不能把过期的 job 描述为仍可使用；旧停止原因永久保留。
      if (!this.#stopped && this.#started !== null && now - this.#started >= this.#caps.max_elapsed_ms)
        this.#stopped = Object.freeze({code: "job_deadline", detail: "首次读取的绝对期限已过"});
      return Object.freeze({limits: this.#limits, started: this.#started !== null,
        elapsed_ms: this.#started === null ? null : (now - this.#started).toString(),
        ...Object.fromEntries(Object.entries(this.#counts).map(([k, v]) => [k, v.toString()])),
        staged: this.#spaceSnapshot(this.#staged, this.#stagedExposure()),
        store: this.#spaceSnapshot(this.#store, this.#storeExposure()),
        open_reservations: this.#open.size.toString(), stopped: this.#stopped?.code ?? null});
    }
  }
  return Object.freeze({LIMIT_FIELDS, parseLimits, validateConfig, Job});
});

/* exact source q3-values.js SHA256 2d881fd06f538218ac73291fde4ba6ea0bb5679ba8480c6c6f047e2e3f84fe4e */
/* #1394：现有 Q3 消费者的私有值访问器。普通值与图值共用字段/位置遍历。 */
(function (root, factory) {
  const common = typeof module === "object" && module.exports;
  const api = factory(common ? require("./s-content-v3.js") : root.SContentV3);
  if (common) module.exports = api;
  else root.Q3Values = api;
})(globalThis, function (content) {
  "use strict";
  const encoder = new TextEncoder(),
    NUMBER = Symbol("exact JSON number");
  function need(ok, message) {
    if (!ok) throw new Error(message);
  }
  function number(token) {
    need(
      typeof token === "string" &&
        /^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?$/.test(token),
      "精确 JSON number token 非法",
    );
    return Object.freeze({ [NUMBER]: true, token });
  }
  function isNumber(v) {
    return !!v?.[NUMBER];
  }
  // 文本合法性（含孤立代理项）的唯一判据；utf8() 在其后编码，field() 快路径只判不编码。
  function wellFormed(s) {
    need(typeof s === "string", "要求 UTF-8 文本");
    for (let i = 0; i < s.length; i++) {
      let c = s.charCodeAt(i);
      if (c >= 0xd800 && c <= 0xdbff) {
        c = s.charCodeAt(++i);
        need(c >= 0xdc00 && c <= 0xdfff, "孤立 Unicode 代理项");
      } else need(c < 0xdc00 || c > 0xdfff, "孤立 Unicode 代理项");
    }
  }
  function utf8(s) {
    wellFormed(s);
    return encoder.encode(s);
  }
  function cmp(a, b) {
    const x = utf8(a),
      y = utf8(b);
    for (let i = 0; i < Math.min(x.length, y.length); i++)
      if (x[i] !== y[i]) return x[i] < y[i] ? -1 : 1;
    return Math.sign(x.length - y.length);
  }
  function quote(s) {
    utf8(s);
    return JSON.stringify(s);
  }
  class Value {
    constructor(plain, reader = null, ref = null, path = []) {
      this.plain = plain;
      this.reader = reader;
      this.ref = ref;
      this.path = path;
      Object.freeze(this);
    }
    static of(v) {
      return v instanceof Value ? v : new Value(v);
    }
    async check() {
      if (this.reader) this.reader.check();
    }
    get kind() {
      return this.reader
        ? this.ref.kind
        : this.plain === null
          ? "null"
          : isNumber(this.plain)
            ? "number"
            : Array.isArray(this.plain)
              ? "array"
              : typeof this.plain === "object"
                ? "object"
                : typeof this.plain === "boolean"
                  ? "bool"
                  : typeof this.plain === "string"
                    ? "string"
                    : "invalid";
    }
    async length() {
      await this.check();
      need(["array", "object"].includes(this.kind), "要求完整容器");
      return this.reader
        ? BigInt(this.ref.extent)
        : BigInt(
            Array.isArray(this.plain)
              ? this.plain.length
              : Object.keys(this.plain).length,
          );
    }
    async scalar() {
      await this.check();
      if (this.kind === "json_text") {
        let result = "";
        for await (const c of canonicalChunks(await this.reader.inner(this)))
          result += c;
        return result;
      }
      need(
        ["string", "number", "bool", "null"].includes(this.kind),
        "要求标量",
      );
      if (!this.reader) return this.plain;
      return this.reader.scalar(this);
    }
    async *entries() {
      await this.check();
      need(this.kind === "object", "要求对象");
      if (this.reader) {
        yield* this.reader.entries(this);
        return;
      }
      let previous = null;
      for (const key of Object.keys(this.plain).sort(cmp)) {
        utf8(key);
        need(
          previous === null || cmp(previous, key) < 0,
          "对象键重复/顺序错误",
        );
        previous = key;
        yield { key, value: Value.of(this.plain[key]) };
      }
    }
    async *items() {
      await this.check();
      need(this.kind === "array", "要求数组");
      if (this.reader) {
        for await (const e of this.reader.entries(this)) yield e.value;
        return;
      }
      for (let i = 0; i < this.plain.length; i++) {
        need(Object.hasOwn(this.plain, i), "稀疏数组");
        yield Value.of(this.plain[i]);
      }
    }
    async field(key) {
      utf8(key);
      if (!this.reader) {
        // #1394 整合（根裁定 A）：普通值按自有属性直接取字段。旧路径每取一个字段都经 entries() 把全部键
        // 按 UTF-8 重排、每次比较都编码（剖析：TextEncoder.encode 53%、cmp 12%）。JS 对象键不会重复，
        // entries() 的顺序断言对普通值恒真；其余检查与先后次序不变：参数键 → 对象类型 → 全部键的孤立
        // 代理项 → 缺字段。图值路径不变。
        await this.check();
        need(this.kind === "object", "要求对象");
        for (const k of Object.keys(this.plain)) wellFormed(k);
        if (Object.hasOwn(this.plain, key)) return Value.of(this.plain[key]);
        throw new Error("缺完整字段：" + key);
      }
      for await (const e of this.entries()) if (e.key === key) return e.value;
      throw new Error("缺完整字段：" + key);
    }
    async has(key) {
      for await (const e of this.entries()) if (e.key === key) return true;
      return false;
    }
    async at(index) {
      index = BigInt(index);
      need(index >= 0n && index < (await this.length()), "数组索引越界");
      let i = 0n;
      for await (const v of this.items()) {
        if (i++ === index) return v;
      }
      throw new Error("数组截断");
    }
  }
  const of = Value.of,
    field = async (v, k) => of(v).field(k);
  async function keys(v, expected, name) {
    v = of(v);
    need(v.kind === "object", name + " 完整对象不符");
    const actual = [];
    for await (const e of v.entries()) actual.push(e.key);
    need(
      actual.length === expected.length &&
        expected.every((k) => actual.includes(k)),
      name + " 完整字段不符",
    );
    return v;
  }
  async function S(v) {
    const s = await of(v).scalar();
    utf8(s);
    return s;
  }
  async function T(v) {
    const s = await S(v);
    need(s.length > 0, "身份文本为空");
    return s;
  }
  async function I(v, positive = false, frontier = false) {
    const s = await S(v);
    need(
      s.length <= 19 && /^(?:0|[1-9][0-9]*|-1)$/.test(s),
      "精确整数必须是规范文本",
    );
    const n = BigInt(s);
    need(
      n >= (frontier ? -1n : positive ? 1n : 0n) && n <= 9223372036854775807n,
      "精确整数超出 i64 域",
    );
    return n;
  }
  async function N(v) {
    const s = await S(v);
    need(
      s.length <= 20 && /^(?:0|[1-9][0-9]*|-[1-9][0-9]*)$/.test(s),
      "价格须为规范整数文本",
    );
    const n = BigInt(s);
    need(
      n >= -9223372036854775808n && n <= 9223372036854775807n,
      "价格超出 i64",
    );
    return n;
  }
  async function list(v) {
    const a = [];
    for await (const item of of(v).items()) a.push(await item.scalar());
    return a;
  }
  async function equal(a, b) {
    a = of(a);
    b = of(b);
    await a.check();
    await b.check();
    if (a.kind !== b.kind) return false;
    if (["null", "bool", "string", "number"].includes(a.kind)) {
      const x = await a.scalar(),
        y = await b.scalar();
      return a.kind === "number" ? x.token === y.token : x === y;
    }
    if (a.kind === "json_text") {
      return equal(await a.reader.inner(a), await b.reader.inner(b));
    }
    if ((await a.length()) !== (await b.length())) return false;
    const ai = a.kind === "object" ? a.entries() : a.items(),
      bi = b.kind === "object" ? b.entries() : b.items();
    while (true) {
      const x = await ai.next(),
        y = await bi.next();
      if (x.done || y.done) return x.done === y.done;
      if (a.kind === "object") {
        if (
          x.value.key !== y.value.key ||
          !(await equal(x.value.value, y.value.value))
        )
          return false;
      } else if (!(await equal(x.value, y.value))) return false;
    }
  }
  async function* canonicalChunks(v) {
    v = of(v);
    await v.check();
    if (v.kind === "json_text") {
      yield '"';
      for await (const c of canonicalChunks(await v.reader.inner(v)))
        yield quote(c).slice(1, -1);
      yield '"';
      return;
    }
    if (v.reader && ["string", "number"].includes(v.kind)) {
      const loaded = await v.reader.load(v),
        decoder = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true });
      if (v.kind === "string") yield '"';
      for (const b of loaded.items) {
        v.reader.check();
        const c = decoder.decode(b, { stream: true });
        yield v.kind === "string" ? quote(c).slice(1, -1) : c;
      }
      const tail = decoder.decode();
      if (tail) yield v.kind === "string" ? quote(tail).slice(1, -1) : tail;
      if (v.kind === "string") yield '"';
      return;
    }
    if (["null", "bool", "string", "number"].includes(v.kind)) {
      const x = await v.scalar();
      yield v.kind === "number" ? x.token : quoteScalar(x);
      return;
    }
    need(["array", "object"].includes(v.kind), "非法值类型");
    yield v.kind === "array" ? "[" : "{";
    let first = true;
    if (v.kind === "array") {
      for await (const x of v.items()) {
        if (!first) yield ",";
        first = false;
        yield* canonicalChunks(x);
      }
    } else {
      for await (const e of v.entries()) {
        if (!first) yield ",";
        first = false;
        yield quote(e.key) + ":";
        yield* canonicalChunks(e.value);
      }
    }
    yield v.kind === "array" ? "]" : "}";
  }
  function quoteScalar(x) {
    return typeof x === "string"
      ? quote(x)
      : x === null
        ? "null"
        : x
          ? "true"
          : "false";
  }
  // 增量 SHA256 只保留64字节尾块；Browser不要求把全文复制给subtle.digest。
  class SHA256 {
    constructor() {
      this.h = new Uint32Array([
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c,
        0x1f83d9ab, 0x5be0cd19,
      ]);
      this.tail = new Uint8Array(64);
      this.used = 0;
      this.bytes = 0n;
    }
    update(data) {
      if (typeof data === "string") data = utf8(data);
      this.bytes += BigInt(data.length);
      for (const b of data) {
        this.tail[this.used++] = b;
        if (this.used === 64) {
          this.block(this.tail);
          this.used = 0;
        }
      }
      return this;
    }
    block(b) {
      const w = new Uint32Array(64),
        r = (v, n) => (v >>> n) | (v << (32 - n));
      for (let i = 0; i < 16; i++)
        w[i] =
          ((b[i * 4] << 24) |
            (b[i * 4 + 1] << 16) |
            (b[i * 4 + 2] << 8) |
            b[i * 4 + 3]) >>>
          0;
      for (let i = 16; i < 64; i++) {
        let x = w[i - 15],
          y = w[i - 2];
        w[i] =
          (w[i - 16] +
            (r(x, 7) ^ r(x, 18) ^ (x >>> 3)) +
            w[i - 7] +
            (r(y, 17) ^ r(y, 19) ^ (y >>> 10))) >>>
          0;
      }
      let [a, c, d, e, f, g, h, j] = this.h;
      for (let i = 0; i < 64; i++) {
        const t1 =
            (j +
              (r(f, 6) ^ r(f, 11) ^ r(f, 25)) +
              ((f & g) ^ (~f & h)) +
              K[i] +
              w[i]) >>>
            0,
          t2 =
            ((r(a, 2) ^ r(a, 13) ^ r(a, 22)) +
              ((a & c) ^ (a & d) ^ (c & d))) >>>
            0;
        j = h;
        h = g;
        g = f;
        f = (e + t1) >>> 0;
        e = d;
        d = c;
        c = a;
        a = (t1 + t2) >>> 0;
      }
      const q = [a, c, d, e, f, g, h, j];
      for (let i = 0; i < 8; i++) this.h[i] = (this.h[i] + q[i]) >>> 0;
    }
    digest() {
      const bit = this.bytes * 8n;
      this.tail[this.used++] = 128;
      if (this.used > 56) {
        this.tail.fill(0, this.used);
        this.block(this.tail);
        this.used = 0;
      }
      this.tail.fill(0, this.used, 56);
      for (let i = 0; i < 8; i++)
        this.tail[63 - i] = Number((bit >> BigInt(i * 8)) & 255n);
      this.block(this.tail);
      let s = "";
      for (const h of this.h) s += h.toString(16).padStart(8, "0");
      return s;
    }
  }
  const K = new Uint32Array([
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
    0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
    0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
  ]);
  async function sha(v) {
    const h = new SHA256();
    for await (const c of canonicalChunks(v)) h.update(c);
    return h.digest();
  }
  function rawSha(s) {
    return new SHA256().update(s).digest();
  }
  function unhex(s) {
    const b = new Uint8Array(s.length / 2);
    for (let i = 0; i < b.length; i++)
      b[i] = parseInt(s.slice(i * 2, i * 2 + 2), 16);
    return b;
  }
  function u64(n) {
    n = BigInt(n);
    const b = new Uint8Array(8);
    for (let i = 7; i >= 0; i--) {
      b[i] = Number(n & 255n);
      n >>= 8n;
    }
    return b;
  }
  function concat(...parts) {
    let n = 0;
    for (const p of parts) n += p.length;
    const b = new Uint8Array(n);
    let i = 0;
    for (const p of parts) {
      b.set(p, i);
      i += p.length;
    }
    return b;
  }
  function F(tag, body) {
    return new SHA256()
      .update(utf8("NewChanlun/SContent/1\0"))
      .update(new Uint8Array([tag]))
      .update(u64(body.length))
      .update(body)
      .digest();
  }
  function M(values) {
    if (values.length === 0) return F(0x11, new Uint8Array());
    if (values.length === 1) return F(0x12, unhex(values[0]));
    let k = 1;
    while (k * 2 < values.length) k *= 2;
    return F(
      0x13,
      concat(
        u64(values.length),
        unhex(M(values.slice(0, k))),
        unhex(M(values.slice(k))),
      ),
    );
  }
  async function valueHash(v) {
    v = of(v);
    await v.check();
    if (v.reader) return v.ref.root.slice(4);
    const kind = v.kind;
    if (kind === "null") return F(0, new Uint8Array());
    if (kind === "bool") return F(v.plain ? 2 : 1, new Uint8Array());
    if (kind === "string" || kind === "number") {
      const b = utf8(kind === "number" ? v.plain.token : v.plain),
        hashes = [];
      for (let i = 0; i < b.length; i += 4096)
        hashes.push(F(0x10, b.slice(i, i + 4096)));
      return F(
        kind === "string" ? 3 : 4,
        concat(u64(b.length), unhex(M(hashes))),
      );
    }
    const hashes = [];
    if (kind === "array") {
      for await (const x of v.items()) hashes.push(await valueHash(x));
      return F(5, concat(u64(hashes.length), unhex(M(hashes))));
    }
    need(kind === "object", "非法内容值");
    for await (const e of v.entries())
      hashes.push(
        F(
          0x14,
          concat(
            unhex(await valueHash(e.key)),
            unhex(await valueHash(e.value)),
          ),
        ),
      );
    return F(6, concat(u64(hashes.length), unhex(M(hashes))));
  }
  async function identity(tag, v, prefix) {
    return prefix + F(tag, unhex(await valueHash(v)));
  }
  class Reader {
    constructor({ anchor, request, check, accept, store, limit }) {
      this.anchor = anchor;
      this.request = request;
      this.check = check;
      this.accept = accept;
      this.store = store;
      this.limit = limit;
      this.cache = new Map();
      this.values = new Map();
    }
    root() {
      return this.anchor.kind === "projection"
        ? this.anchor.projection_ref
        : this.anchor.delta_ref;
    }
    value(ref, path = []) {
      return new Value(undefined, this, content.validateRef(ref), path);
    }
    async load(v) {
      this.check();
      const id = v.ref.root;
      let prior = this.values.get(id);
      if (prior) return prior;
      const task = this.read(v);
      this.values.set(id, task);
      try {
        return await task;
      } catch (e) {
        this.values.delete(id);
        throw e;
      }
    }
    async read(v) {
      need(!this.committed, "已提交视图缺范围，不能伪造已加载");
      const ref = v.ref,
        unit = ["array", "object"].includes(ref.kind)
          ? "items"
          : ["string", "number"].includes(ref.kind)
            ? "bytes"
            : "value";
      const size = unit === "value" ? "1" : String(this.limit);
      let request = {
          op: "content",
          anchor: this.anchor,
          path: v.path,
          unit,
          offset: "0",
          limit: size,
        },
        last = null,
        items = [],
        graphs = [];
      do {
        this.check();
        const reply = await this.request(request);
        this.check();
        const proof = await content.verifyRange(request, reply, {
          previousToken: last === null ? undefined : last,
        });
        this.check();
        need(
          await equal(proof.target_ref, ref),
          "片段目标与已取得字段header不同",
        );
        for (const node of reply.nodes) {
          this.check();
          await this.accept(node);
          this.check();
          const old = this.cache.get(node.root);
          need(!old || (await equal(old, node)), "缓存相同根不同原件");
          if (!old) {
            await this.store(node);
            this.check();
            this.cache.set(node.root, Object.freeze({ ...node }));
          }
        }
        if (ref.kind === "json_text") {
          const inner = proof.graph.resolve(ref, [{ json_text: true }], {
            partial: true,
          });
          items.push(inner);
        } else if (unit === "bytes") items.push(unhex(reply.items));
        else if (unit === "items")
          for (const item of reply.items) items.push(item);
        else items.push(...reply.items);
        graphs.push(proof.graph);
        if (proof.done) break;
        last = proof.next_token;
        request = { op: "content", content_token: last };
      } while (true);
      this.check();
      return { items, graphs };
    }
    async *entries(v) {
      const loaded = await this.load(v);
      this.check();
      let i = 0n,
        previous = null;
      for (const item of loaded.items) {
        this.check();
        if (v.ref.kind === "object") {
          need(
            previous === null || cmp(previous, item.key) < 0,
            "跨片对象键未严格UTF8递增",
          );
          previous = item.key;
          yield {
            key: item.key,
            value: this.value(item.value_ref, [...v.path, { field: item.key }]),
          };
        } else
          yield {
            value: this.value(item, [...v.path, { index: (i++).toString() }]),
          };
      }
      need(
        BigInt(loaded.items.length) === BigInt(v.ref.extent),
        "容器完整范围截断",
      );
    }
    async scalar(v) {
      const loaded = await this.load(v);
      this.check();
      if (v.kind === "null") return null;
      if (v.kind === "bool") {
        const graph = loaded.graphs[0],
          raw = graph.nodes().find((x) => x.root === v.ref.root);
        return raw.tag === "02";
      }
      const decoder = new TextDecoder("utf-8", {
        fatal: true,
        ignoreBOM: true,
      });
      let text = "";
      for (const bytes of loaded.items)
        text += decoder.decode(bytes, { stream: true });
      text += decoder.decode();
      return v.kind === "number" ? number(text) : text;
    }
    async inner(v) {
      need(v.kind === "json_text", "json_text角色错误");
      const loaded = await this.load(v);
      return this.value(loaded.items[0], [...v.path, { json_text: true }]);
    }
    async complete(v) {
      v = of(v);
      if (v.kind === "json_text") {
        await this.complete(await this.inner(v));
        return;
      }
      if (v.kind === "array") {
        for await (const x of v.items()) await this.complete(x);
      } else if (v.kind === "object") {
        for await (const e of v.entries()) await this.complete(e.value);
      } else await this.load(v);
    }
    freeze() {
      this.committed = true;
      this.check = () => {};
    }
    async finish() {
      this.check();
      const nodes = [];
      for (const n of this.cache.values()) nodes.push(n);
      const graph = await content.GraphView.create(nodes);
      this.check();
      await graph.verifyComplete(this.root());
      this.check();
      return graph;
    }
  }
  return Object.freeze({
    Value,
    of,
    field,
    need,
    keys,
    S,
    T,
    I,
    N,
    list,
    equal,
    cmp,
    number,
    isNumber,
    canonicalChunks,
    sha,
    rawSha,
    valueHash,
    identity,
    F,
    M,
    unhex,
    u64,
    concat,
    Reader,
  });
});

/* exact source q3-typed.js SHA256 10ad88858cfaa85e5324617cb584deb06427b0d4565d909892a18c0f26552ffa */
/* #1394：typed 与形态来源的同一判据；只把访问迁为完整异步 Value 遍历。 */
(function (root, factory) {
  const api = factory(typeof module === "object" && module.exports ? require("./q3-values.js") : root.Q3Values);
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.Q3Typed = api;
})(globalThis, function (V) {
  "use strict";
  // #1405：旧合同 AsKnown 保留，代际由会话规则绑定，不能混用。
  const KNOWLEDGE_DOMAINS = ["established_direction_without_extreme_identity_competition", "established_direction_with_extreme_root_sets"];
  const BI_VERSIONS = {
    "s-new-bi/1": ["new-bi-dual-coordinate/1", "standard_raw_gap_3"],
    "s-new-bi/2": ["new-bi-dual-coordinate/2", "standard_raw_gap_3;same-price/keep-earlier+extreme-region/1"],
  };
  const RULE_CONTRACTS = {
    "s2-axis-quantifiers": [null, null, KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/1": ["s-new-bi/1", null, KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/1+segment-first/1": ["s-new-bi/1", "s-segment/1", KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/1+segment-second/1": ["s-new-bi/1", "s-segment/2", KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/2": ["s-new-bi/2", null, KNOWLEDGE_DOMAINS[1]],
    "s2-axis-quantifiers+new-bi/2+segment-second/1": ["s-new-bi/2", "s-segment/2", KNOWLEDGE_DOMAINS[1]],
    "s2-axis-quantifiers+new-bi/1+segment-second/2": ["s-new-bi/1", "s-segment/3", KNOWLEDGE_DOMAINS[0]],
    "s2-axis-quantifiers+new-bi/2+segment-second/2": ["s-new-bi/2", "s-segment/3", KNOWLEDGE_DOMAINS[1]],
  };
  const SAME_PRICE_KEEP_EVIDENCE = "same_kind_equal_price_keep_earlier";
  const RETIRED_SAME_PRICE_WAITING = ["raw_extreme_identity_tie", "actual_extreme_mapping_missing", "same_kind_endpoint_identity_tie", "earlier_endpoint_identity_unsettled"];
  const TB02_KINDS = ["CC-004.inclusion_step", "CC-005.inclusion_group", "CC-007.fractal_description", "CC-054.knowledge_state"];
  const BI_KINDS = ["CC-008.endpoint", "CC-008.new_bi_pair", "CC-009.same_kind", "CC-010.bi", "CC-055.relation", "CC-055.change_event", "CC-056.version"];
  TB02_KINDS.push(...BI_KINDS);
  const SEGMENT_KINDS = ["CC-011.segment_seed", "CC-012.feature_sequence", "CC-013.segment", "CC-055.segment_relation", "CC-055.segment_change", "CC-056.segment_version", "CC-011.seed_candidate"];
  TB02_KINDS.push(...SEGMENT_KINDS);
  const REF_KEYS = ["identity_key", "payload_hash", "receipt_id", "event_id", "input_revision", "revision", "seq", "source_coord"];
  const {need, keys, T, S, I, N, equal} = V;
  const field = (v, k) => V.field(v, k);
  const isNull = v => V.of(v).kind === "null";
  const eqField = async (v, k, expected) => equal(await field(v, k), expected);
  async function enumValue(v, values, message) {
    need((V.of(v).kind) === "string", message);
    const value = await S(v); need(values.includes(value), message); return value;
  }
  async function array(v, message) { v = V.of(v); need((v.kind) === "array", message); return v; }
  async function texts(v) {
    v = await array(v, "要求文本数组"); for await (const item of v.items()) await T(item); return v;
  }
  async function coordinates(v) {
    v = await array(v, "要求完整坐标数组"); let last = -1n;
    for await (const item of v.items()) { const n = await I(item); need(n > last, "来源坐标未严格递增/重复"); last = n; }
    return v;
  }
  async function coordinateSet(v) {
    const result = new Set(); for await (const item of (await coordinates(v)).items()) result.add(await S(item)); return result;
  }
  async function digest(v) { const value = await S(v); need(/^[a-f0-9]{64}$/.test(value), "SHA-256 文本不规范"); }
  async function barRef(v) {
    await keys(v, ["source_coord", "open", "high", "low", "close"], "BarRef"); await I(await field(v, "source_coord"));
    for (const k of ["open", "high", "low", "close"]) await N(await field(v, k));
  }
  async function direction(v) { if (!(isNull(v))) await enumValue(v, ["UP", "DOWN"], "未知方向枚举"); }
  async function directionEvidence(v) {
    if (isNull(v)) return;
    await keys(v, ["direction", "established_at", "previous_acc", "incoming", "source_coords"], "方向见证");
    await enumValue(await field(v, "direction"), ["UP", "DOWN"], "方向见证缺方向"); await I(await field(v, "established_at"));
    await barRef(await field(v, "previous_acc")); const incoming = await field(v, "incoming"); await barRef(incoming);
    await coordinates(await field(v, "source_coords"));
    need(await equal(await field(v, "established_at"), await field(incoming, "source_coord")), "方向建立坐标与原 incoming 不同");
  }
  async function reference(v, name, revisionMessage) {
    await keys(v, REF_KEYS, name); for await (const entry of V.of(v).entries()) await T(entry.value);
    await digest(await field(v, "payload_hash")); await I(await field(v, "input_revision"), true); await I(await field(v, "revision"), true);
    await I(await field(v, "seq")); await I(await field(v, "source_coord"));
    need(await equal(await field(v, "input_revision"), await field(v, "revision")), revisionMessage);
  }
  // 比较规范文本槽时逐片消费，避免把无限长窗口或 payload 物化。
  async function canonicalMatches(v, expected) {
    let offset = 0;
    for await (const chunk of V.canonicalChunks(V.of(v))) {
      if (expected.slice(offset, offset + chunk.length) !== chunk) return false;
      offset += chunk.length;
    }
    return offset === expected.length;
  }
  async function validateTyped(record) {
    record = V.of(record);
    await keys(record, ["object_id", "object_revision", "kind", "batch_id", "fact_key", "payload", "input_refs", "source_coords", "first_known_generation", "first_known_cut", "published_generation", "withdrawn_generation", "withdrawal_reason", "superseded_by", "lifecycle"], "typed 对象");
    for (const k of ["object_id", "kind", "batch_id", "fact_key", "first_known_cut"]) await T(await field(record, k));
    const kind = await S(await field(record, "kind"));
    need(TB02_KINDS.includes(kind) && await eqField(record, "object_revision", "1"), "typed kind/revision 不符");
    const first = await I(await field(record, "first_known_generation"), true); await I(await field(record, "published_generation"), true);
    need(await eqField(record, "first_known_cut", "cut-" + await S(await field(record, "first_known_generation"))) && await equal(await field(record, "published_generation"), await field(record, "first_known_generation")), "typed 首次获知不闭合");
    const withdrawn = await field(record, "withdrawn_generation"), reason = await field(record, "withdrawal_reason"), superseded = await field(record, "superseded_by");
    if (isNull(withdrawn)) need(await eqField(record, "lifecycle", "active") && isNull(reason) && isNull(superseded), "活动对象携带撤回字段");
    else {
      need(await I(withdrawn, true) > first && await eqField(record, "lifecycle", "withdrawn"), "撤回代际不符");
      const why = await enumValue(reason, ["fact_removed", "superseded_by_revision"], "撤回原因/目标不符");
      need((why === "fact_removed") === isNull(superseded), "撤回原因/目标不符"); if (!(isNull(superseded))) await T(superseded);
    }
    const refs = await array(await field(record, "input_refs"), "typed input_refs 不是数组"), coords = await coordinates(await field(record, "source_coords"));
    const coordIterator = coords.items()[Symbol.asyncIterator]();
    for await (const ref of refs.items()) {
      await reference(ref, "typed input_ref", "输入修订引用不同"); const next = await coordIterator.next();
      need(!next.done && await equal(next.value, await field(ref, "source_coord")), "typed 源引用/坐标不一一对应");
    }
    need((await coordIterator.next()).done, "typed 源引用/坐标不一一对应");
    const p = await field(record, "payload");
    if (SEGMENT_KINDS.includes(kind)) validateSegmentPayload(kind, await plain(p));
    else if (BI_KINDS.includes(kind)) await validateBiPayload(kind, p);
    else if (kind === TB02_KINDS[0]) {
      await keys(p, ["action", "incoming", "acc_before", "acc_after", "direction", "direction_evidence", "comparisons", "endpoint_order", "contains", "high_sources", "low_sources", "waiting_reasons"], "包含步骤");
      await enumValue(await field(p, "action"), ["seed", "establish_direction", "merge", "new_group", "waiting"], "步骤动作不符"); await barRef(await field(p, "incoming"));
      for (const k of ["acc_before", "acc_after"]) { const value = await field(p, k); if (!(isNull(value))) await barRef(value); }
      await direction(await field(p, "direction")); await directionEvidence(await field(p, "direction_evidence")); await texts(await field(p, "waiting_reasons"));
      await coordinates(await field(p, "high_sources")); await coordinates(await field(p, "low_sources"));
      const comparisons = await array(await field(p, "comparisons"), "包含比较两轴不完整");
      need([0n, 2n].includes(await comparisons.length()), "包含比较两轴不完整"); let index = 0;
      for await (const c of comparisons.items()) {
        await keys(c, ["axis", "acc", "incoming", "order"], "比较");
        need(await eqField(c, "axis", ["high", "low"][index++]), "比较轴/枚举不符"); await enumValue(await field(c, "order"), ["LT", "EQ", "GT"], "比较轴/枚举不符");
        await N(await field(c, "acc")); await N(await field(c, "incoming"));
      }
      const endpoints = await array(await field(p, "endpoint_order"), "端点等价类不是数组"), labels = new Set(); let count = 0n, valid = true;
      for await (const cls of endpoints.items()) {
        await texts(cls); need(await cls.length() > 0n, "端点空类");
        for await (const label of cls.items()) { const value = await S(label); count++; labels.add(value); valid = valid && ["acc.high", "acc.low", "incoming.high", "incoming.low"].includes(value); }
      }
      need(count === 0n || (count === 4n && labels.size === 4 && valid), "端点标签缺失/重复");
      const contains = await field(p, "contains"); need(["null", "bool"].includes(contains.kind), "contains 不是 bool/null");
      if (isNull(await field(p, "acc_before"))) need(count === 0n && await comparisons.length() === 0n && isNull(contains), "seed 虚构比较");
    } else if (kind === TB02_KINDS[1]) {
      await keys(p, ["group_index", "group_anchor", "members", "open", "high", "low", "close", "high_sources", "low_sources", "confirmed", "direction", "direction_evidence", "confirmation_evidence", "waiting_reasons"], "包含组");
      await I(await field(p, "group_index")); await I(await field(p, "group_anchor"));
      const members = await coordinates(await field(p, "members")), memberSet = await coordinateSet(members), firstMember = await members.items()[Symbol.asyncIterator]().next();
      need(!firstMember.done && await equal(firstMember.value, await field(p, "group_anchor")), "组首未绑定首成员");
      for (const k of ["open", "high", "low", "close"]) await N(await field(p, k));
      for (const k of ["high_sources", "low_sources"]) {
        const sources = await coordinates(await field(p, k)); let present = true;
        for await (const source of sources.items()) present = present && memberSet.has(await S(source));
        need(await sources.length() > 0n && present, "极值来源缺失/不属于成员");
      }
      const confirmed = await field(p, "confirmed"), evidence = await field(p, "confirmation_evidence");
      need(confirmed.kind === "bool" && await equal(confirmed, !(isNull(evidence))), "组确认字段/见证不符");
      await directionEvidence(evidence); await direction(await field(p, "direction")); await directionEvidence(await field(p, "direction_evidence")); await texts(await field(p, "waiting_reasons"));
    } else if (kind === TB02_KINDS[2]) {
      await keys(p, ["shape_object_id", "branch", "window", "description", "reference_prices", "descriptive_labels", "subsequent_development"], "分型描述");
      await T(await field(p, "shape_object_id")); await T(await field(p, "description"));
      await enumValue(await field(p, "branch"), ["TOP", "BOTTOM"], "CC007 不是已成立顶/底");
      const window = await coordinates(await field(p, "window")); need(await window.length() === 3n, "CC007 不是已成立顶/底"); const windows = await V.list(window);
      const names = ["left.high", "left.low", "mid.high", "mid.low", "right.high", "right.low"], pricesValue = await array(await field(p, "reference_prices"), "六个参考价不完整");
      need(await pricesValue.length() === 6n, "六个参考价不完整"); const prices = []; for await (const price of pricesValue.items()) prices.push(price);
      for (let i = 0; i < 6; i++) {
        const r = prices[i]; await keys(r, ["name", "source_coord", "axis", "value"], "参考价");
        need(await eqField(r, "name", names[i]) && await eqField(r, "axis", names[i].split(".")[1]) && await equal(await field(r, "source_coord"), windows[Math.floor(i / 2)]), "参考价来源不符"); await N(await field(r, "value"));
      }
      const labels = await field(p, "descriptive_labels"); await keys(labels, ["status", "reason", "labels", "source", "raw_ohlc"], "形容词未定声明"); const bars = await field(labels, "raw_ohlc");
      need(await eqField(labels, "status", "not_determined") && await eqField(labels, "reason", "no_settled_numeric_rule") && await eqField(labels, "labels", []) && await eqField(labels, "source", "fenxing.md:64-74") && bars.kind === "array", "形容词未定边界改变");
      for await (const bar of bars.items()) await barRef(bar);
      const sub = await field(p, "subsequent_development"); await keys(sub, ["status", "reason", "bars"], "后续已知事实"); const subsequent = await array(await field(sub, "bars"), "后续 bars 不完整");
      need(await subsequent.length() > 0n ? await eqField(sub, "status", "observed") && await eqField(sub, "reason", null) : await eqField(sub, "status", "insufficient_knowledge") && await eqField(sub, "reason", "no_subsequent_bar"), "后续状态与原件不一致");
      for await (const entry of subsequent.items()) {
        await keys(entry, ["bar", "relations"], "后续 bar"); const bar = await field(entry, "bar"); await barRef(bar);
        const relations = await array(await field(entry, "relations"), "后续四价×六参考关系不完整"); need(await relations.length() === 24n, "后续四价×六参考关系不完整"); let i = 0;
        for await (const r of relations.items()) {
          const axis = ["open", "high", "low", "close"][Math.floor(i / 6)], ref = prices[i++ % 6];
          await keys(r, ["axis", "value", "reference", "reference_value", "order"], "后续关系");
          need(await eqField(r, "axis", axis) && await equal(await field(r, "value"), await field(bar, axis)) && await equal(await field(r, "reference"), await field(ref, "name")) && await equal(await field(r, "reference_value"), await field(ref, "value")), "后续关系来源不同");
          await enumValue(await field(r, "order"), ["LT", "EQ", "GT"], "后续关系来源不同");
        }
      }
    } else {
      await keys(p, ["scope", "input_frontier", "known_facts", "waiting_reasons", "domain", "requests"], "知识状态");
      need(await eqField(p, "scope", "TB-02-A") && KNOWLEDGE_DOMAINS.includes(await S(await field(p, "domain"))) && await eqField(p, "known_facts", ["raw_ohlc", "inclusion_trace", "group_provenance"]), "知识域/已知事实不符");
      await I(await field(p, "input_frontier"), false, true); await texts(await field(p, "waiting_reasons")); const requests = await array(await field(p, "requests"), "逐请求集合不完整"), ids = new Set();
      const enums = {input_quality:["SUFFICIENT", "INSUFFICIENT", "INCONSISTENT"], applicability:["IN_DOMAIN", "OUTSIDE_DOMAIN", "DOMAIN_PROOF_MISSING"], computation:["COMPLETE", "INCOMPLETE"], validity:["CURRENT", "SUPERSEDED", "WITHDRAWN"]};
      for await (const r of requests.items()) {
        await keys(r, ["request_id", "subject_id", "axes", "reasons", "source_coords"], "请求状态"); const id = await T(await field(r, "request_id")); need(!ids.has(id), "请求身份重复"); ids.add(id);
        const subject = await field(r, "subject_id"); if (!(isNull(subject))) await T(subject); const axes = await field(r, "axes"); await keys(axes, Object.keys(enums), "请求四轴");
        for (const k of Object.keys(enums)) await enumValue(await field(axes, k), enums[k], "请求四轴枚举不符"); await texts(await field(r, "reasons")); await coordinates(await field(r, "source_coords"));
      }
    }
    const slot = [...BI_KINDS, ...SEGMENT_KINDS].includes(kind) ? await field(p, "slot") : kind === TB02_KINDS[0] ? await field(await field(p, "incoming"), "source_coord") : kind === TB02_KINDS[1] ? await field(p, "group_anchor") : kind === TB02_KINDS[2] ? await field(p, "window") : "TB-02-A";
    need(await canonicalMatches([kind, slot], await S(await field(record, "fact_key"))), "事实语义槽未绑定 payload");
  }
  // #1404/#1414/#1420：s-segment/1、/2、/3 段合同沿用已审同步判据（单一实现，含 K1 游标）。
  // 调用面两处：导出给既有观察器/合同测试的同步入口；validateTyped 在该 payload 的完整原件上调用（见 plain）。
  const validateSegmentPayload = (function () {
    const need = V.need;
    const object = v => v !== null && typeof v === "object" && !Array.isArray(v);
    function keys(value, expected, name) {
      need(object(value) && Object.keys(value).length === expected.length && expected.every(k => Object.hasOwn(value, k)), name + " 完整字段不符");
    }
    function scalar(value) {
      need(typeof value === "string", "要求 UTF-8 文本");
      for (let i = 0; i < value.length; i++) {
        const code = value.charCodeAt(i);
        if (code >= 0xd800 && code <= 0xdbff) {
          const next = value.charCodeAt(++i);
          need(next >= 0xdc00 && next <= 0xdfff, "孤立 Unicode 高代理项");
        } else need(code < 0xdc00 || code > 0xdfff, "孤立 Unicode 低代理项");
      }
      return value;
    }
    function text(value) { scalar(value); need(value.length > 0, "身份文本为空"); return value; }
    function integer(value, positive = false, frontier = false) {
      need(typeof value === "string" && value.length <= 19 && /^(?:0|[1-9][0-9]*|-1)$/.test(value), "精确整数必须是规范文本");
      const n = BigInt(value);
      need(n >= (frontier ? -1n : positive ? 1n : 0n) && n <= 9223372036854775807n, "精确整数超出 i64 域");
      return n;
    }
    function signed(value) {
      need(typeof value === "string" && value.length <= 20 && /^(?:0|[1-9][0-9]*|-[1-9][0-9]*)$/.test(value), "价格须为规范整数文本");
      const n = BigInt(value); need(n >= -9223372036854775808n && n <= 9223372036854775807n, "价格超出 i64"); return n;
    }
    function texts(value) { need(Array.isArray(value), "要求文本数组"); value.forEach(text); return value; }
    function coordinates(value) {
      need(Array.isArray(value), "要求完整坐标数组");
      let last = -1n; for (const v of value) { const n = integer(v); need(n > last, "来源坐标未严格递增/重复"); last = n; } return value;
    }
    function direction(value) { need(value === null || ["UP", "DOWN"].includes(value), "未知方向枚举"); }
    function utf8Compare(a, b) {
      // UTF-8字典序与合法Unicode标量序相同；先完整验证两参数，保留首错顺序。
      scalar(a); scalar(b);
      let ai = 0, bi = 0;
      while (ai < a.length && bi < b.length) {
        const aa = a.codePointAt(ai), bb = b.codePointAt(bi);
        if (aa !== bb) return aa < bb ? -1 : 1;
        ai += aa > 0xffff ? 2 : 1;
        bi += bb > 0xffff ? 2 : 1;
      }
      return ai < a.length ? 1 : bi < b.length ? -1 : 0;
    }
    function canonical(value) {
      if (value === null || typeof value === "boolean") return JSON.stringify(value);
      if (typeof value === "string") return JSON.stringify(scalar(value));
      if (Array.isArray(value)) return "[" + value.map(canonical).join(",") + "]";
      need(object(value), "v2 JSON 数值必须在源头投影为精确文本");
      return "{" + Object.keys(value).sort(utf8Compare).map(k => JSON.stringify(scalar(k)) + ":" + canonical(value[k])).join(",") + "}";
    }
    const equal = (a, b) => canonical(a) === canonical(b);
    const SECOND_FOLD_CURSOR = ["schema", "rule_revision", "budget_policy", "budget_granted", "candidate", "input_stroke_count", "from_stroke", "through_stroke", "consumed_len", "consumed_digest", "dir_state", "standard", "breach_watermark", "semantic_state", "killed_by", "matched"];
    // #1420 K1 游标线形自身：形状/枚举/整数/摘要、候选起点（b+1）与封存见证非空性；同对象相等绑定由调用方另做（与 s_tb02_contract._fold_cursor 对称）。
    function foldCursor(c, element, kill, check) {
      keys(c, SECOND_FOLD_CURSOR, "K1游标");
      need(c.schema === "second-fold-cursor/1" && c.rule_revision === "second-candidate-terminal/1" && c.budget_policy === "second-seq-scan-budget/1", "K1游标版本不符");
      const candidate = c.candidate; keys(candidate, ["seg_dir", "b_stroke", "candidate_extreme"], "K1候选身份");
      need(["UP", "DOWN"].includes(candidate.seg_dir), "K1候选方向未声明"); signed(candidate.candidate_extreme);
      const start = integer(c.from_stroke);
      need(integer(candidate.b_stroke) + 1n === start, "K1游标起点不是b笔之后");
      for (const name of ["input_stroke_count", "through_stroke", "consumed_len"]) integer(c[name]);
      if (c.budget_granted !== null) integer(c.budget_granted, true);
      need(typeof c.consumed_digest === "string" && /^[0-9a-f]{16}$/.test(c.consumed_digest), "K1游标指纹不规范");
      direction(c.dir_state); need(Array.isArray(c.standard), "K1游标标准元素错型"); c.standard.forEach(e => element(e));
      if (c.breach_watermark !== null) signed(c.breach_watermark);
      const sealed = c.semantic_state;
      // 游标只封存已决（CONFIRMED/KILLED）或未决（null）；OPEN 与覆盖只在出具结果时按输入末端判定。
      need([null, "CONFIRMED", "KILLED_BY_NEW_EXTREME"].includes(sealed), "K1游标封存了未决终态");
      need((c.killed_by !== null) === (sealed === "KILLED_BY_NEW_EXTREME") && (c.matched !== null) === (sealed === "CONFIRMED"), "K1游标见证与封存终态不符");
      if (c.killed_by !== null) kill(c.killed_by);
      if (c.matched !== null) check(c.matched);
      return start;
    }
    // CC-056 续扫检查点没有 stroke_refs：元素只验形状，笔号只验为非负整数串。
    function checkpointElement(e) {
      keys(e, ["high", "low", "stroke_idx", "stroke_indices"], "FIRST元素"); signed(e.high); signed(e.low);
      need(Array.isArray(e.stroke_indices), "要求完整坐标数组"); e.stroke_indices.forEach(i => integer(i));
      need(e.stroke_indices.includes(e.stroke_idx), "元素代表笔不属于来源");
    }
    function checkpointKill(k) {
      keys(k, ["stroke_idx", "price", "candidate_extreme", "comparison"], "失效见证"); integer(k.stroke_idx); signed(k.price); signed(k.candidate_extreme);
      need([">", "<"].includes(k.comparison), "失效见证比较未声明");
    }
    function checkpointCheck(c) {
      keys(c, ["elements", "top_fractal", "bottom_fractal", "through_stroke"], "SECOND分型见证");
      need(Array.isArray(c.elements) && c.elements.length === 3, "SECOND分型窗口缺元素"); c.elements.forEach(e => checkpointElement(e));
      need(typeof c.top_fractal === "boolean" && typeof c.bottom_fractal === "boolean", "SECOND分型错型"); integer(c.through_stroke);
    }
    // #1420 r2 CC-056.scan_cursors：本 cut 续扫检查点只含调度相关游标（累计预算大于每轮或本轮耗尽）；无同对象 facts 可比。
    function scanCheckpoints(cursors, roundStrokes) {
      need(Array.isArray(cursors), "续扫检查点不是数组");
      need(roundStrokes !== 0n || cursors.length === 0, "预算0却登记续扫检查点");
      const seen = new Set();
      for (const c of cursors) {
        foldCursor(c, checkpointElement, checkpointKill, checkpointCheck);
        need(c.budget_granted !== null && BigInt(c.consumed_len) <= BigInt(c.budget_granted), "续扫检查点预算不符");
        const identity = canonical([c.candidate.seg_dir, c.candidate.b_stroke, c.candidate.candidate_extreme]);
        need(!seen.has(identity), "续扫检查点候选身份重复"); seen.add(identity);
      }
    }
    // #1420 s-segment/3：SECOND 语义终点、扫描覆盖、预算与 K1 游标的类型和同对象绑定；不重放折叠（与 s_tb02_contract._second_terminal 对称）。
    function secondTerminal(d, element, indices, secondCheck) {
      const scan = d.scan, f = d.facts, semantic = scan.semantic_state;
      need([null, "CONFIRMED", "KILLED_BY_NEW_EXTREME", "OPEN_AT_PREFIX_END"].includes(semantic) && ["COMPLETE", "INCOMPLETE_RESOURCE_BOUND"].includes(scan.coverage), "SECOND终态/覆盖未声明");
      const incomplete = scan.coverage === "INCOMPLETE_RESOURCE_BOUND";
      need(incomplete === scan.resource_truncated && (semantic === null) === incomplete, "SECOND覆盖与语义/截断不一致");
      need(d.second_sequence_status === (semantic === "CONFIRMED" ? "CONFIRMED" : semantic === "KILLED_BY_NEW_EXTREME" ? "KILLED" : "PENDING"), "SECOND状态未绑定语义终点");
      need(f.semantic_state === semantic && f.coverage === scan.coverage, "SECOND事实终态与扫描不一致");
      keys(scan.budget, ["policy", "granted"], "扫描预算");
      need(scan.budget.policy === "second-seq-scan-budget/1", "扫描预算政策版本不符");
      if (scan.budget.granted !== null) integer(scan.budget.granted, true);
      need(!incomplete || scan.budget.granted !== null, "资源截断缺有限扫描预算");
      const extreme = signed(f.candidate_extreme);
      if (f.breach_watermark !== null) signed(f.breach_watermark);
      const comparison = d.direction === "UP" ? ">" : d.direction === "DOWN" ? "<" : null;
      function kill(k) {
        keys(k, ["stroke_idx", "price", "candidate_extreme", "comparison"], "失效见证"); indices([k.stroke_idx]);
        const price = signed(k.price);
        need(comparison !== null && k.comparison === comparison && k.candidate_extreme === f.candidate_extreme && (comparison === ">" ? price > extreme : price < extreme), "失效见证未严格越过候选极值");
      }
      need((f.killed_by !== null) === (semantic === "KILLED_BY_NEW_EXTREME"), "失效见证与语义终点不符");
      if (f.killed_by !== null) { kill(f.killed_by); need(f.breach_watermark === f.killed_by.price, "越界水位未绑定失效见证"); }
      const c = scan.cursor, start = foldCursor(c, element, kill, secondCheck);
      need(c.budget_granted === scan.budget.granted, "K1游标预算未绑定扫描预算");
      need(c.budget_granted === null || BigInt(c.consumed_len) <= BigInt(c.budget_granted), "SECOND游标消费超过扫描预算");
      need(c.candidate.seg_dir === d.direction && start === BigInt(scan.from_stroke) && c.candidate.candidate_extreme === f.candidate_extreme, "K1候选身份未绑定SECOND扫描");
      need(c.input_stroke_count === scan.input_stroke_count && BigInt(c.consumed_len) <= (BigInt(c.input_stroke_count) > start ? BigInt(c.input_stroke_count) - start : 0n), "K1游标消费范围越界");
      need(c.semantic_state === null || c.semantic_state === semantic, "K1游标封存终态与扫描不同");
      need(c.killed_by === null || equal(c.killed_by, f.killed_by), "K1游标失效见证与事实不同");
      need(c.matched === null || equal(c.matched, f.matched), "K1游标命中与事实不同");
    }
    return function validateSegmentPayload(kind, p) {
      keys(p, ["schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"], "段合同");
      // #1420：v2＝「含第二种」（s-segment/2 与 /3 共用）；v3 只另管版本、SECOND 三态终点/K1 游标、KILLED_CASE_TWO、CC-056 范围与槽。
      const v3 = p.schema_revision === "s-segment/3", v2 = v3 || p.schema_revision === "s-segment/2";
      need(p.schema_revision === (v3 ? "s-segment/3" : v2 ? "s-segment/2" : "s-segment/1") && p.semantic_version === (v3 ? "segment-second/2" : v2 ? "segment-second/1" : "segment-seed-first/1") && p.policy_version === (v2 ? "seed_closed_strict_second" : "seed_closed_first_no_gap") && p.view_role === "main", "段规则/范围版本不符");
      integer(p.object_generation, true);
      const d = p.data, refs = new Map();
      let segmentStart = null;
      function known(k) {
        keys(k, ["generation", "input_frontier", "receipt_id", "received_at", "semantic_commit_ns"], "段获知时点");
        integer(k.generation, true); integer(k.input_frontier, false, true);
        for (const name of ["receipt_id", "received_at"]) if (k[name] !== null) text(k[name]);
        if (k.semantic_commit_ns !== null) integer(k.semantic_commit_ns);
      }
      for (const r of d.stroke_refs || []) {
        keys(r, ["stroke_index", "entity_id", "start_anchor", "end_anchor", "start_price", "end_price", "start_roots", "end_roots"], "段笔引用");
        for (const name of ["stroke_index", "start_anchor", "end_anchor"]) integer(r[name]);
        signed(r.start_price); signed(r.end_price);
        need(coordinates(r.start_roots).length > 0 && coordinates(r.end_roots).length > 0, "段笔缺实际根");
        need(r.entity_id === `bi:${p.object_generation}:${r.start_anchor}` && !refs.has(r.stroke_index), "段笔身份不符/重复");
        refs.set(r.stroke_index, r);
      }
      function indices(xs) { coordinates(xs); need(xs.every(i => refs.has(i)), "段事实引用未绑定笔"); }
      function construction(v) {
        keys(v, ["start_stroke", "stroke_indices", "length", "vector", "overlap", "overlap_relation", "direction", "failed_conditions"], "段完整条件");
        integer(v.start_stroke); indices(v.stroke_indices);
        const n = v.stroke_indices.length, length = n < 3 ? `n=${n}` : n % 2 === 0 ? "even_ge3" : "odd_ge3";
        need(v.length === length, "段长度分域不符");
        need(v.stroke_indices.every((x,i) => BigInt(x) === BigInt(v.start_stroke) + BigInt(i)), "候选笔片段不连续");
        need(length === "odd_ge3" ? Array.isArray(v.vector) && v.vector.length === 4 && v.vector.every(x => typeof x === "boolean") : v.vector === null, "段四条件缺位/不适用虚填");
        if (n >= 3) { need(Array.isArray(v.overlap) && v.overlap.length === 2 && ["OVERLAP", "TOUCH", "DISJOINT"].includes(v.overlap_relation), "缺闭区间见证"); v.overlap.forEach(signed); }
        else need(v.overlap === null && v.overlap_relation === null, "不足三笔虚填公共区间");
        texts(v.failed_conditions);
        need(equal(v.vector, [true,true,true,true]) ? ["UP", "DOWN"].includes(v.direction) && !v.failed_conditions.length : v.direction === null, "段方向与适用性不符");
      }
      function element(e) {
        keys(e, ["high", "low", "stroke_idx", "stroke_indices"], "FIRST元素"); signed(e.high); signed(e.low); indices(e.stroke_indices);
        need(e.stroke_indices.includes(e.stroke_idx), "元素代表笔不属于来源");
      }
      if ([SEGMENT_KINDS[0], SEGMENT_KINDS[6]].includes(kind)) {
        keys(d, ["entity_id", "construction", "stroke_refs"], kind); construction(d.construction);
        need((kind === SEGMENT_KINDS[0]) === equal(d.construction.vector,[true,true,true,true]), "未满足完整条件的候选冒充seed");
      } else if (kind === SEGMENT_KINDS[1]) {
        keys(d, ["entity_id", "segment_id", "role", "direction", "member_direction", "facts", "stroke_refs", "scan", "second_sequence_status"], kind);
        const second = v2 && d.role === "SECOND_SEQUENCE";
        need((v2 ? ["FIRST_SEQUENCE", "SECOND_SEQUENCE"] : ["FIRST_SEQUENCE"]).includes(d.role) && ["UP", "DOWN"].includes(d.member_direction) && (second ? ["PENDING", "CONFIRMED", ...(v3 ? ["KILLED"] : [])] : v2 ? ["NOT_APPLICABLE", "PENDING", "CONFIRMED", ...(v3 ? ["KILLED"] : [])] : ["NOT_APPLICABLE", "WAITING_FOLLOWUP"]).includes(d.second_sequence_status), "序列角色/适用性不符"); direction(d.direction);
        keys(d.scan, ["from_stroke", "through_stroke", "input_stroke_count", "resource_truncated", "tail_cache_window", ...(second && v3 ? ["semantic_state", "coverage", "budget", "cursor"] : [])], "扫描范围");
        for (const [k,v] of Object.entries(d.scan)) if (["from_stroke", "through_stroke", "input_stroke_count", "tail_cache_window"].includes(k)) integer(v);
        need(typeof d.scan.resource_truncated === "boolean", "截断状态错型");
        need(refs.size > 0, "特征序列缺段笔引用");
        // 身份锚＝stroke_refs 中数值最小的笔号，即段起笔；wire 整数是十进制文本，须按数值取最小。
        segmentStart = [...refs.keys()].reduce((a, b) => BigInt(b) < BigInt(a) ? b : a);
        need(second || d.scan.from_stroke === segmentStart, "FIRST扫描起点不是段起笔");
        const f = d.facts; keys(f, ["raw_elements", "standard", "steps", "checks", ...(second ? ["from_stroke", "through_stroke", "input_stroke_count", "resource_truncated", "matched"] : []), ...(second && v3 ? ["candidate_extreme", "breach_watermark", "semantic_state", "killed_by", "coverage"] : [])], "序列事实");
        for (const k of ["raw_elements", "standard", "steps", "checks"]) need(Array.isArray(f[k]), "序列事实集合错型");
        if (second) {
          integer(f.from_stroke); integer(f.through_stroke, false, true); integer(f.input_stroke_count);
          need(typeof f.resource_truncated === "boolean" && d.scan.tail_cache_window === "0" && ["from_stroke", "through_stroke", "input_stroke_count", "resource_truncated"].every(k => f[k] === d.scan[k]) && BigInt(d.scan.from_stroke) > BigInt(segmentStart), "SECOND扫描边界不符");
        }
        [...f.raw_elements, ...f.standard].forEach(element);
        for (const s of f.steps) {
          keys(s, ["incoming", "before", "after", "action", "merge_direction", "scope"], "包含步骤"); element(s.incoming); element(s.after); if (s.before !== null) element(s.before);
          need((second ? ["append", "merge"] : ["append", "merge", "turn_boundary"]).includes(s.action) && s.scope === (second ? "SECOND_SEQUENCE" : s.action === "turn_boundary" ? "TURN_HYPOTHESIS" : "SAME_SEQUENCE"), "包含跨作用域/动作未声明");
          need(s.action === "merge" ? ["UP", "DOWN"].includes(s.merge_direction) : s.merge_direction === null, "方向性包含缺见证");
        }
        function secondCheck(c) {
          keys(c, ["elements", "top_fractal", "bottom_fractal", "through_stroke"], "SECOND分型见证");
          need(Array.isArray(c.elements) && c.elements.length === 3, "SECOND分型窗口缺元素"); c.elements.forEach(element);
          need(typeof c.top_fractal === "boolean" && typeof c.bottom_fractal === "boolean", "SECOND分型错型"); indices([c.through_stroke]);
        }
        if (second) {
          // v3：未决（PENDING）与失效（KILLED）都无命中；v2 只有 PENDING 无命中。
          need(v3 ? (f.matched !== null) === (d.second_sequence_status === "CONFIRMED") : (f.matched === null) === (d.second_sequence_status === "PENDING"), "SECOND命中与状态不符");
          if (f.matched !== null) { secondCheck(f.matched); need(f.checks.some(c => equal(c, f.matched)) && (f.matched.top_fractal || f.matched.bottom_fractal), "SECOND命中未绑定检查"); }
        }
        for (const c of f.checks) {
          if (second) { secondCheck(c); continue; }
          keys(c, ["elements", "first_fractal", "gap", "gap_closed_by_c", "provisional", "turn_hypothesis_pair"], "分型见证");
          need(Array.isArray(c.elements) && c.elements.length === 3, "分型窗口缺元素"); c.elements.forEach(element);
          need(equal(c.turn_hypothesis_pair, c.first_fractal ? c.elements.slice(0,2).map(e => e.stroke_idx) : null), "假设转折点两侧未绑定a/b元素");
          need(typeof c.first_fractal === "boolean" && typeof c.provisional === "boolean", "分型事实错型");
          for (const k of ["gap", "gap_closed_by_c"]) need(c[k] === null || typeof c[k] === "boolean", "gap见证错型");
          need((c.first_fractal || c.gap === null) && (c.gap === true || c.gap_closed_by_c === null), "不适用条件虚填布尔");
        }
        if (second && v3) secondTerminal(d, element, indices, secondCheck);
      } else if (kind === SEGMENT_KINDS[2]) {
        keys(d, ["entity_id", "entity_revision", "seed_id", "sequence_id", "sequence_object_id", "construction", "result", "waiting_reasons", "termination_attempts", "geometric_end", "trigger_stroke", "formed_known_at", "terminated_known_at", "stroke_refs", ...(v2 ? ["second_sequence_id", "second_sequence_object_id", "strict", "fact_vector"] : [])], kind);
        integer(d.entity_revision, true); construction(d.construction); texts(d.waiting_reasons); known(d.formed_known_at);
        need(Array.isArray(d.termination_attempts), "终结准入见证错型");
        for (const a of d.termination_attempts) {
          keys(a, ["end_stroke", "trigger_stroke", "minimum_count_met", "construction"], "终结准入见证");
          indices([a.end_stroke]); indices([a.trigger_stroke]); need(typeof a.minimum_count_met === "boolean", "最小笔数条件错型"); construction(a.construction);
        }
        need(["NO_FIRST", "CASE_ONE", "PENDING_CASE_TWO", "FAILED_GEOMETRY", ...(v2 ? ["CASE_TWO"] : []), ...(v3 ? ["KILLED_CASE_TWO"] : [])].includes(d.result), "冒称交付第二种终结");
        if (["CASE_ONE", "CASE_TWO"].includes(d.result)) {
          known(d.terminated_known_at); const e = d.geometric_end;
          keys(e, ["stroke_index", "group_anchor", "extreme_roots", "price"], "几何段端"); indices([e.stroke_index]); indices([d.trigger_stroke]); coordinates(e.extreme_roots); integer(e.group_anchor); signed(e.price);
        } else need(d.geometric_end === null && d.trigger_stroke === null && d.terminated_known_at === null, "发展态虚填终结");
        if (v2) {
          const strict = d.strict;
          if (strict !== null) {
            keys(strict, ["elements", "raw_gap", "gap_closed_by_c", "effective_case", "closure_operator", "closure_left", "closure_right"], "strict事实");
            need(Array.isArray(strict.elements) && strict.elements.length === 3, "strict缺FIRST窗口"); strict.elements.forEach(element);
            need(typeof strict.raw_gap === "boolean" && (strict.raw_gap ? typeof strict.gap_closed_by_c === "boolean" : strict.gap_closed_by_c === null), "strict适用性不符");
            need(["CASE_ONE", "CASE_TWO"].includes(strict.effective_case) && ["<=", ">="].includes(strict.closure_operator), "strict规则未声明"); signed(strict.closure_left); signed(strict.closure_right);
            const a = strict.elements[0], c = strict.elements[2], up = strict.closure_operator === "<=";
            need(equal([strict.closure_operator, strict.closure_left, strict.closure_right], up ? ["<=", c.low, a.high] : [">=", c.high, a.low]), "strict比较未绑定FIRST元素");
          }
          const terminated = ["CASE_ONE", "CASE_TWO"].includes(d.result);
          need(!terminated || strict !== null && strict.effective_case === d.result, "终结缺适用strict对象");
          need((d.second_sequence_id === null) === (d.second_sequence_object_id === null), "SECOND身份/对象缺位");
          if (d.second_sequence_id !== null) { need(d.second_sequence_id === "second:" + d.entity_id, "SECOND身份未绑定候选"); text(d.second_sequence_object_id); }
          need(!["CASE_TWO", "PENDING_CASE_TWO", "KILLED_CASE_TWO"].includes(d.result) || d.second_sequence_id !== null, "第二种缺SECOND引用");
          const b = d.fact_vector;
          keys(b, ["stroke_broken", "new_segment_formed", "segment_broken", "stroke_break_witness", "scan_from_stroke", "scan_through_stroke", "scan_complete", "termination_witness"], "独立破坏事实");
          need(["stroke_broken", "scan_complete"].every(k => typeof b[k] === "boolean") && ["new_segment_formed", "segment_broken"].every(k => b[k] === null || typeof b[k] === "boolean"), "破坏事实错型");
          integer(b.scan_from_stroke); integer(b.scan_through_stroke, false, true);
          need((b.new_segment_formed === true) === terminated && b.segment_broken === b.new_segment_formed && (b.new_segment_formed === null) === !b.scan_complete, "段破坏适用性不符");
          const certificate = b.termination_witness;
          need((certificate !== null) === (b.new_segment_formed === true), "段破坏缺同次终结证书");
          if (certificate !== null) {
            keys(certificate, ["kind", "strict", "second_match", "construction", "minimum_count_met", "through_stroke"], "终结证书");
            need(certificate.kind === d.result && equal(certificate.strict, strict) && equal(certificate.construction, d.construction), "终结证书未绑定同次strict/几何");
            construction(certificate.construction);
            need(certificate.minimum_count_met === true && equal(certificate.construction.vector, [true,true,true,true]) && certificate.through_stroke === d.trigger_stroke, "终结证书缺有效准入/实际前沿");
            indices([certificate.through_stroke]);
            const match = certificate.second_match;
            need((match !== null) === (certificate.kind === "CASE_TWO"), "终结证书SECOND适用性不符");
            if (match !== null) {
              keys(match, ["elements", "top_fractal", "bottom_fractal", "through_stroke"], "SECOND终结证书");
              need(Array.isArray(match.elements) && match.elements.length === 3 && typeof match.top_fractal === "boolean" && typeof match.bottom_fractal === "boolean" && (match.top_fractal || match.bottom_fractal), "终结证书缺任意分型见证");
              match.elements.forEach(element); indices([match.through_stroke]);
            }
          }
          const w = b.stroke_break_witness;
          need((w !== null) === b.stroke_broken, "笔破坏缺独立见证");
          if (w !== null) {
            keys(w, ["earlier_stroke", "later_stroke", "earlier_price", "later_price", "comparison"], "笔破坏见证");
            indices([w.earlier_stroke, w.later_stroke]); signed(w.earlier_price); signed(w.later_price); need(["<=", ">="].includes(w.comparison), "笔破坏比较未声明");
          }
        }
      } else if (kind === SEGMENT_KINDS[3]) {
        keys(d, ["source_id", "relation_kind", "target_id", "version", "witness_object_id"], kind); Object.values(d).forEach(text); need(["member_of", "derived_from"].includes(d.relation_kind), "段关系未声明");
      } else if (kind === SEGMENT_KINDS[4]) {
        keys(d, ["entity_id", "change", "before", "after", "known_at", "object_id"], kind); known(d.known_at);
        need(["formed", "updated", "terminated", "withdrawn"].includes(d.change), "段变化未声明");
        for (const k of ["before", "after"]) if (d[k] !== null) validateSegmentPayload(SEGMENT_KINDS[2], {...p, slot:d[k].entity_id, object_generation:d[k].entity_id.split(":")[1], data:d[k]});
      } else {
        keys(d, ["input_frontier", "waiting_reasons", "known_at", "delivered_scope", "deferred_scope", ...(v3 ? ["scan_budget", "scan_cursors"] : [])], kind); integer(d.input_frontier, false, true); known(d.known_at); texts(d.waiting_reasons);
        need(d.delivered_scope === (v3 ? "seed_strict_first_and_second_terminal" : v2 ? "seed_strict_first_and_finite_second" : "seed_and_no_gap_first_kind") && equal(d.deferred_scope, v3 ? ["general_equal_identity", "global_G006_bridge"] : v2 ? ["general_equal_identity", "general_scan_continuation", "global_G006_bridge"] : ["second_kind", "general_equal_identity"]), "段交付范围不符");
        if (v3) { keys(d.scan_budget, ["policy", "round_strokes"], "段扫描预算"); need(d.scan_budget.policy === "second-seq-scan-budget/1", "扫描预算政策版本不符"); scanCheckpoints(d.scan_cursors, integer(d.scan_budget.round_strokes)); }
      }
      if ([...SEGMENT_KINDS.slice(0,3), SEGMENT_KINDS[6]].includes(kind)) {
        if (kind === SEGMENT_KINDS[1]) {
          const expected = `segment:${p.object_generation}:${refs.get(segmentStart).start_anchor}`;
          need(d.segment_id === expected && d.entity_id === (d.role === "SECOND_SEQUENCE" ? "second:" : "first:") + expected && ((d.direction === d.member_direction) === (d.role === "SECOND_SEQUENCE")), "FIRST身份/反向成员不符");
        } else {
          const anchor = refs.size ? refs.get(d.construction.start_stroke).start_anchor : "empty";
          const expected = `${[SEGMENT_KINDS[0], SEGMENT_KINDS[6]].includes(kind) ? "seed" : "segment"}:${p.object_generation}:${anchor}`;
          need(d.entity_id === expected, "段身份未绑定代际/起笔");
          if (kind === SEGMENT_KINDS[2]) {
            need(d.seed_id === `seed:${p.object_generation}:${anchor}` && d.sequence_id === "first:" + expected, "段缺同源seed/FIRST身份"); text(d.sequence_object_id);
            if (d.geometric_end !== null) {
              const e = d.geometric_end, r = refs.get(e.stroke_index);
              need(e.group_anchor === r.end_anchor && e.price === r.end_price && equal(e.extreme_roots,r.end_roots), "几何段端与实际笔端不符");
            }
          }
        }
      }
      const slot = kind === SEGMENT_KINDS[3] ? [d.source_id, d.relation_kind, d.target_id] : kind === SEGMENT_KINDS[4] ? [d.known_at.generation, d.entity_id, d.change] : kind === SEGMENT_KINDS[5] ? (v3 ? "TB-02-W" : v2 ? "TB-02-D" : "TB-02-C") : d.entity_id;
      need(equal(p.slot, slot), "段语义槽身份不符");
    };
  })();
  // 段合同按上面的原判据在完整 payload 原件上执行：普通值即原件；图值经同一 Reader（含其期限/累计账检查）逐字段取回。
  // 生产 v2 wire 不含 JSON number（parse 已拒绝），图值的 number 同样拒绝，与原判据 canonical 的拒绝一致。
  async function plain(v) {
    v = V.of(v);
    if (!v.reader) return v.plain;
    if (v.kind === "array") {
      const out = [];
      for await (const item of v.items()) out.push(await plain(item));
      return out;
    }
    if (v.kind === "object") {
      const out = {};
      for await (const entry of v.entries())
        Object.defineProperty(out, entry.key, {value: await plain(entry.value), enumerable: true, writable: true, configurable: true});
      return out;
    }
    need(v.kind !== "number", "v2 JSON 数值必须在源头投影为精确文本");
    return v.scalar();
  }
  async function validateBiPayload(kind, p) {
    p = V.of(p);
    await keys(p, ["schema_revision", "semantic_version", "policy_version", "view_role", "object_generation", "slot", "data"], "新笔合同");
    const schema = await S(await field(p, "schema_revision"));
    const version = Object.hasOwn(BI_VERSIONS, schema) ? BI_VERSIONS[schema] : null;
    need(version !== null && await eqField(p, "semantic_version", version[0]) && await eqField(p, "policy_version", version[1]) && await eqField(p, "view_role", "main"), "新笔规则/政策版本不符");
    const v2 = schema === "s-new-bi/2";
    async function waiting(value) {
      const reasons = await texts(value);
      if (v2) for await (const reason of reasons.items())
        need(!RETIRED_SAME_PRICE_WAITING.includes(await S(reason)), "同价域退役的等待态在 v2 对象复现");
    }
    await I(await field(p, "object_generation"), true); const d = await field(p, "data");
    async function endpoint(e) {
      await keys(e, ["kind", "merged_index", "group_anchor", "price", "extreme_roots", ...(v2 ? ["raw_first", "raw_last"] : ["raw_position"]), "source_coords", "sealed_at", "waiting_reasons"], "新笔端点");
      await enumValue(await field(e, "kind"), ["TOP", "BOTTOM"], "端点类型不符"); await I(await field(e, "merged_index")); await I(await field(e, "group_anchor")); await N(await field(e, "price"));
      const roots = await coordinates(await field(e, "extreme_roots")), sources = await coordinateSet(await field(e, "source_coords")); await waiting(await field(e, "waiting_reasons"));
      for await (const root of roots.items()) need(sources.has(await S(root)), "实际根缺原始来源");
      if (v2) {
        const first = await I(await field(e, "raw_first")), last = await I(await field(e, "raw_last")), count = await roots.length();
        need(count > 0n && last >= first && last - first >= count - 1n, "极值根集与首末 raw 序位不闭合");
      } else { const position = await field(e, "raw_position"); if (!isNull(position)) await I(position); }
      const sealed = await field(e, "sealed_at"); if (!isNull(sealed)) await I(sealed);
    }
    async function conditions(c) {
      await keys(c, ["merged_gap", "raw_between_actual_extrema", "top_price", "bottom_price", "vector", "failed_conditions", "waiting_reasons"], "新笔完整条件");
      await I(await field(c, "merged_gap")); const raw = await field(c, "raw_between_actual_extrema"); if (v2 || !isNull(raw)) await I(raw); await N(await field(c, "top_price")); await N(await field(c, "bottom_price"));
      const vector = await array(await field(c, "vector"), "条件向量缺位/错型"); need(await vector.length() === 3n, "条件向量缺位/错型");
      for await (const value of vector.items()) need(value.kind === "bool" || (!v2 && value.kind === "null"), "条件向量缺位/错型");
      await texts(await field(c, "failed_conditions")); await waiting(await field(c, "waiting_reasons"));
    }
    async function known(k) {
      await keys(k, ["generation", "input_frontier", "receipt_id", "received_at", "semantic_commit_ns"], "新笔获知时点");
      await I(await field(k, "generation"), true); await I(await field(k, "input_frontier")); await T(await field(k, "receipt_id")); await T(await field(k, "received_at")); const ns = await field(k, "semantic_commit_ns"); if (!(isNull(ns))) await I(ns);
    }
    if (kind === BI_KINDS[0]) await endpoint(d);
    else if ([BI_KINDS[1], BI_KINDS[2]].includes(kind)) {
      await keys(d, ["old", "new", "endpoint_kind_pair", "conditions", "comparison", "selection", "retained_anchors", ...(v2 ? ["evidence"] : []), "waiting_reasons"], "端点对");
      const old = await field(d, "old"), fresh = await field(d, "new"); await endpoint(old); await endpoint(fresh);
      const oldKind = await S(await field(old, "kind")), newKind = await S(await field(fresh, "kind")); need(await eqField(d, "endpoint_kind_pair", oldKind + "/" + newKind), "端点分域标签不符");
      if (kind === BI_KINDS[1]) { need(oldKind !== newKind, "异型请求实际同型"); await conditions(await field(d, "conditions")); need(await eqField(d, "comparison", null) && await eqField(d, "selection", null) && (!v2 || await eqField(d, "evidence", [])), "异型域混入同型选择"); }
      else {
        need(oldKind === newKind && await eqField(d, "conditions", null), "同型域混入假条件或未声明取舍");
        const comparison = await enumValue(await field(d, "comparison"), ["LT", "EQ", "GT"], "同型域混入假条件或未声明取舍");
        await enumValue(await field(d, "selection"), v2 ? ["KEEP", "REPLACE"] : ["KEEP", "REPLACE", "UNDETERMINED"], "同型域混入假条件或未声明取舍");
        if (v2) {
          const stronger = comparison === (oldKind === "TOP" ? "GT" : "LT");
          need(await eqField(d, "selection", stronger ? "REPLACE" : "KEEP"), "同型取舍与价格比较不闭合");
          need(await eqField(d, "retained_anchors", [await field(stronger ? fresh : old, "group_anchor")]), "同型保留锚不符取舍");
          need(await eqField(d, "evidence", comparison === "EQ" ? [SAME_PRICE_KEEP_EVIDENCE] : []), "同价保留在先者的证据缺失或混入");
        }
      }
      await coordinates(await field(d, "retained_anchors")); if (v2) await texts(await field(d, "evidence")); await waiting(await field(d, "waiting_reasons"));
    } else if (kind === BI_KINDS[3]) {
      await keys(d, ["identity_anchor", "start", "end", "formation", "confirmation", "entity_id", "entity_revision", "state", "formed_known_at", "confirmed_known_at", "formed_evidence", "version_causes"], "新笔实体");
      await I(await field(d, "identity_anchor")); await I(await field(d, "entity_revision"), true); await T(await field(d, "entity_id")); await endpoint(await field(d, "start")); await endpoint(await field(d, "end"));
      await conditions(await field(d, "formation")); await conditions(await field(d, "formed_evidence")); await known(await field(d, "formed_known_at")); await texts(await field(d, "version_causes"));
      const confirmation = await field(d, "confirmation"), absent = isNull(confirmation);
      need(await eqField(d, "state", absent ? "FORMED_UNCONFIRMED" : "CONFIRMED"), "生命周期/确认见证不符");
      need(await eqField(d, "entity_id", "bi:" + await S(await field(p, "object_generation")) + ":" + await S(await field(d, "identity_anchor"))), "笔身份未绑定代际与形成起点");
      if (!absent) {
        await keys(confirmation, ["successor_start", "successor_end", "successor_conditions", "right_group_sealed_at", "source_coords"], "确认见证");
        await I(await field(confirmation, "successor_start")); await I(await field(confirmation, "successor_end")); await I(await field(confirmation, "right_group_sealed_at")); await coordinates(await field(confirmation, "source_coords"));
        await conditions(await field(confirmation, "successor_conditions")); await known(await field(d, "confirmed_known_at"));
      } else need(await eqField(d, "confirmed_known_at", null), "无确认见证却有确认时点");
    } else if (kind === BI_KINDS[4]) {
      await keys(d, ["source_id", "relation_kind", "target_id", "version", "witness_object_id"], "新笔关系"); for await (const entry of d.entries()) await T(entry.value);
      await enumValue(await field(d, "relation_kind"), ["member_of", "derived_from", "successor_of", "leaves", "retests", "returns_to", "extends", "newborn_after", "expands_with", "caused_turn_at", "confirms", "selected_from"], "未声明关系种类");
    } else if (kind === BI_KINDS[5]) {
      await keys(d, ["entity_id", "change", "before", "after", "known_at", "event_at", "causes", "object_id"], "新笔变化");
      await T(await field(d, "entity_id")); await enumValue(await field(d, "change"), ["formed", "extended", "endpoint_replaced", "confirmed", "source_or_boundary_updated", "withdrawn"], "未声明变化"); await known(await field(d, "known_at")); await N(await field(d, "event_at")); await texts(await field(d, "causes"));
      for (const k of ["before", "after"]) {
        const entity = await field(d, k); if (isNull(entity)) continue;
        const entityId = await S(await field(entity, "entity_id"));
        // before 按其原件形状保留历史代际；after 必须与本事件同代。
        const entitySchema = k === "after" ? schema : await (await field(entity, "start")).has("raw_first") ? "s-new-bi/2" : "s-new-bi/1";
        const synthetic = {schema_revision:entitySchema, semantic_version:BI_VERSIONS[entitySchema][0], policy_version:BI_VERSIONS[entitySchema][1], view_role:await field(p, "view_role")};
        synthetic.slot = await field(entity, "entity_id"); synthetic.object_generation = entityId.split(":")[1]; synthetic.data = entity;
        await validateBiPayload(BI_KINDS[3], synthetic);
      }
    } else {
      await keys(d, ["input_frontier", "version_causes", "identity_basis", "classification_obligation", "waiting_reasons", "known_at"], "新笔版本");
      await I(await field(d, "input_frontier"), false, true); await texts(await field(d, "version_causes")); await waiting(await field(d, "waiting_reasons")); await known(await field(d, "known_at"));
    }
    const generation = await field(p, "object_generation");
    const slot = kind === BI_KINDS[0] ? "fx:" + await S(generation) + ":" + await S(await field(d, "group_anchor")) :
      [BI_KINDS[1], BI_KINDS[2]].includes(kind) ? [generation, await field(await field(d, "old"), "group_anchor"), await field(await field(d, "new"), "group_anchor")] :
      kind === BI_KINDS[3] ? await field(d, "entity_id") :
      kind === BI_KINDS[4] ? [await field(d, "source_id"), await field(d, "relation_kind"), await field(d, "target_id")] :
      kind === BI_KINDS[5] ? [await field(await field(d, "known_at"), "generation"), await field(d, "entity_id"), await field(d, "change")] : "TB-02-B";
    need(await equal(await field(p, "slot"), slot), "新笔 slot 未绑定其具名对象身份");
  }
  async function validateTB02ShapeRefs(shape) {
    shape = V.of(shape);
    const refs = await field(shape, "input_refs"); need(await eqField(shape, "kind", "CC-006.local_shape") && refs.kind === "array" && await refs.length() === 3n, "OHLC形态缺完整三组");
    const anchors = [await field(shape, "window_start"), await field(shape, "window_mid"), await field(shape, "window_end")], all = new Set(); let previousIndex = null, index = 0;
    for await (const g of refs.items()) {
      await keys(g, ["merged_index", "raw_refs", "dependency_refs"], "OHLC形态来源组"); const mergedIndex = await I(await field(g, "merged_index"));
      need(previousIndex === null || mergedIndex === previousIndex + 1n, "形态来源组序号不连续"); previousIndex = mergedIndex;
      const sets = {}; let firstRaw = null;
      for (const role of ["raw_refs", "dependency_refs"]) {
        const groupRefs = await array(await field(g, role), "形态来源角色不是数组"), set = new Set(); let last = -1n;
        for await (const r of groupRefs.items()) {
          await reference(r, "形态来源引用", "形态来源修订不符"); const coordValue = await field(r, "source_coord"), coord = await S(coordValue), n = await I(coordValue);
          need(n > last, "来源坐标未严格递增/重复"); last = n; all.add(coord); set.add(coord);
          if (role === "raw_refs" && firstRaw === null) firstRaw = coordValue;
        }
        sets[role] = set;
      }
      let disjoint = true; for (const coord of sets.dependency_refs) disjoint = disjoint && !sets.raw_refs.has(coord);
      need(sets.raw_refs.size > 0 && disjoint, "形态将构造/边界依据伪作真实成员");
      need(await equal(firstRaw, anchors[index++]), "形态实际成员首坐标与窗口锚不同");
    }
    const coords = await coordinates(await field(shape, "source_coords"));
    // source_coords 已严格递增，等长加每项属于完整并集等价于原排序后全数组比较。
    need(await coords.length() === BigInt(all.size), "形态来源不是成员与构造/边界依据并集");
    for await (const coord of coords.items()) need(all.has(await S(coord)), "形态来源不是成员与构造/边界依据并集");
  }
  return Object.freeze({RULE_CONTRACTS, validateTyped, validateBiPayload, validateSegmentPayload, validateTB02ShapeRefs, plain,
    TB02_KINDS: Object.freeze(TB02_KINDS), BI_KINDS, SEGMENT_KINDS: Object.freeze(SEGMENT_KINDS)});
});

/* exact source q3-consumer.js SHA256 85b6cc1edb1fa519ea004c41d31fe24a7ff103e1b60c5b896a2e6401defd32a0 */
/* #1394：Q3完整消费者；只供既有tb01c页面调用，没有产品路由或默认资源。 */
(function (root, factory) {
  const common = typeof module === "object" && module.exports;
  const build = factory(
    common ? require("./q3-values.js") : root.Q3Values,
    common ? require("./q3-typed.js") : root.Q3Typed,
    common ? require("./s-content-job-v3.js") : root.SContentJobV3,
    common ? require("./s-content-v3.js") : root.SContentV3,
  );
  if (common) module.exports = build;
  else root.Q3Consumer = build;
})(globalThis, function (V, typed, jobs, content) {
  "use strict";
  const {
    of,
    field,
    need,
    keys,
    T,
    S,
    I,
    N,
    equal,
    sha,
    identity,
    number,
    cmp,
  } = V;
  const f = field,
    t = async (v, k) => T(await f(v, k)),
    s = async (v, k) => S(await f(v, k)),
    i = async (v, k, p = false, fr = false) => I(await f(v, k), p, fr),
    n = async (v, k) => N(await f(v, k));
  const FAMILIES = [
    "objects",
    "withdrawn_objects",
    "witnesses",
    "relations",
    "observations",
    "raw_history",
  ];
  const FIXED = [
    "session_id",
    "session_generation",
    "structure_cut",
    "cut_generation",
    "catalog_revision",
    "index_frontier",
    "profile_id",
    "profile_hash",
    "input_frontier",
    "seq_range",
    "catalog_run_status",
    "catalog_evidence",
    "scope",
    "rule_revision",
    "history_mode",
    "as_of_generation",
  ];
  const HEADER = [
    "schema_revision",
    "session_id",
    "session_generation",
    "source_namespace",
    "source_epoch",
    "message_id",
    "producer_id",
    "producer_epoch",
    "payload_hash",
    "causal_refs",
    "payload",
  ];
  const PAGE = [
    "fixed_cut",
    "catalog",
    "rows",
    "counts",
    "scope",
    "omissions",
    "order_version",
    "offset",
    "done",
    "next_token",
    "cut_projection_digest",
    "validated_capture_digest",
    "ok",
  ];
  const TOKEN = [
    "token_schema",
    "session_id",
    "session_generation",
    "cut_generation",
    "scope",
    "history_mode",
    "structure_cut",
    "catalog_revision",
    "index_frontier",
    "profile_hash",
    "rule_revision",
    "order_version",
    "page_size",
    "offset",
    "cut_projection_digest",
    "issued_capture_digest",
    "token_checksum",
  ];
  const CURSOR = [
    "session_id",
    "session_generation",
    "catalog_revision",
    "scope",
    "after_generation",
    "base_cut",
    "index_frontier",
    "last_seq",
    "order_version",
  ];
  const D = [
    "generation",
    "session_id",
    "catalog_revision",
    "base_cut",
    "next_cut",
    "seq_range",
    "index_frontier",
    "input_frontier",
    "catalog_run_status",
    "catalog_evidence",
  ];
  const CHANGES = [
    "upserts",
    "withdrawals",
    "replaces",
    "witnesses",
    "relations",
    "observations",
    "raw_history_added",
  ];
  const SCOPE = { structure: "CompleteCut", economic: "not_started" };
  const RAWREF = [
    "identity_key",
    "payload_hash",
    "receipt_id",
    "event_id",
    "input_revision",
    "revision",
    "seq",
    "source_coord",
  ];
  const RAW = [
    "identity_key",
    "revision",
    "input_revision",
    "payload_hash",
    "receipt_id",
    "seq",
    "source_namespace",
    "source_epoch",
    "instrument",
    "event_id",
    "received_at",
    "raw_text",
    "ts",
    "volume",
    "source_coord",
    "supersedes_revision",
    "schema_revision",
    "open",
    "high",
    "low",
    "close",
  ];
  const digest = (x) =>
    need(typeof x === "string" && /^[0-9a-f]{64}$/.test(x), "SHA-256格式错误");
  const objectId = (x) =>
    need(/^obj3-[0-9a-f]{64}$/.test(x), "新域对象身份格式错误");
  const batch = (x) =>
    need(/^batch3-[0-9a-f]{64}$/.test(x), "新域batch身份格式错误");
  function smallIdentity(v) {
    return {
      session_id: v.session_id,
      session_generation: v.session_generation,
    };
  }
  function sameIdentity(a, b) {
    return (
      a.session_id === b.session_id &&
      a.session_generation === b.session_generation
    );
  }
  async function select(v, names) {
    const p = {};
    for (const k of names) p[k] = await f(v, k);
    return of(p);
  }
  async function scalars(v, names) {
    const p = {};
    for (const k of names) p[k] = await (await f(v, k)).scalar();
    return p;
  }
  async function texts(v) {
    for await (const x of of(v).items()) await T(x);
  }
  async function coords(v) {
    let prior = -1n;
    for await (const x of of(v).items()) {
      const c = await I(x);
      need(c > prior, "来源坐标重复/未严格递增");
      prior = c;
    }
  }
  async function rawReference(raw) {
    return select(raw, RAWREF);
  }
  async function rawOriginal(raw) {
    await rawValidate(raw); // 只逆投影已证明恰为生产wire的完整原件，不能吞掉额外price/字段。
    const out = {};
    for await (const e of of(raw).entries()) {
      if (["revision", "seq", "supersedes_revision"].includes(e.key))
        out[e.key] =
          (await e.value.scalar()) === null
            ? null
            : number((await I(e.value, e.key === "revision")).toString());
      else out[e.key] = e.value;
    }
    out.price = await f(raw, "close");
    return of(out);
  }
  async function rawValidate(raw) {
    await keys(raw, RAW, "OHLC原始历史");
    need((await s(raw, "schema_revision")) === "s-ohlc/1", "OHLC schema错误");
    const low = await n(raw, "low"),
      high = await n(raw, "high");
    for (const k of ["open", "close"]) {
      const p = await n(raw, k);
      need(low <= p && p <= high, "原始OHLC几何不符");
    }
    await n(raw, "ts");
    need((await n(raw, "volume")) >= 0n, "volume为负");
    for (const k of [
      "source_namespace",
      "source_epoch",
      "instrument",
      "event_id",
      "received_at",
    ])
      await t(raw, k);
    await completeValue(await f(raw, "raw_text"));
  }
  async function auditRaw(snapshot, charge = () => {}) {
    const byId = new Map(),
      effective = new Map(),
      owners = new Map(),
      positions = new Map(),
      revisions = new Map();
    let seq = 0n;
    for await (const raw of (await f(snapshot, "raw_history")).items()) {
      charge();
      await rawValidate(raw);
      const identityKey = await t(raw, "identity_key"),
        rev = await i(raw, "revision", true),
        coord = await s(raw, "source_coord");
      await I(coord);
      need(
        (await s(raw, "input_revision")) === rev.toString() &&
          (await i(raw, "seq")) === seq++,
        "raw接纳序/input修订错误",
      );
      const expected =
        (await t(raw, "source_namespace")) +
        "|" +
        (await t(raw, "source_epoch")) +
        "|" +
        (await t(raw, "instrument")) +
        "|" +
        (await t(raw, "event_id"));
      need(identityKey === expected, "raw namespace/业务身份不符");
      need(
        !owners.has(coord) || owners.get(coord) === identityKey,
        "同源坐标多owner",
      );
      need(
        !positions.has(identityKey) || positions.get(identityKey) === coord,
        "同业务身份多坐标",
      );
      owners.set(coord, identityKey);
      positions.set(identityKey, coord);
      const old = revisions.get(identityKey),
        sup = await (await f(raw, "supersedes_revision")).scalar();
      need(
        (old === undefined ? sup === null : sup === old.toString()) &&
          (old === undefined || rev > old),
        "raw修订链断裂",
      );
      revisions.set(identityKey, rev);
      const payload = await select(raw, [
        "event_id",
        "received_at",
        "raw_text",
        "open",
        "high",
        "low",
        "close",
        "volume",
      ]);
      const p = {};
      for await (const e of payload.entries()) p[e.key] = e.value;
      p.revision = await f(raw, "input_revision");
      p.seq = await f(raw, "source_coord");
      p.timestamp = await f(raw, "ts");
      const hash = await sha(p);
      need((await s(raw, "payload_hash")) === hash, "原始payload_hash不符");
      need(
        (await s(raw, "receipt_id")) ===
          "rcpt-" + V.rawSha(identityKey + "|" + hash).slice(0, 16),
        "原始receipt不符",
      );
      const key = JSON.stringify([identityKey, rev.toString()]);
      need(!byId.has(key), "raw历史身份重复");
      byId.set(key, raw);
      effective.set(coord, raw);
    }
    return { byId, effective };
  }
  async function shapeRecord(record) {
    await keys(
      record,
      [
        "object_id",
        "object_revision",
        "kind",
        "batch_id",
        "branch",
        "dir_ab",
        "dir_bc",
        "window_start",
        "window_mid",
        "window_end",
        "comparisons",
        "input_refs",
        "source_coords",
        "first_known_generation",
        "first_known_cut",
        "published_generation",
        "withdrawn_generation",
        "withdrawal_reason",
        "superseded_by",
        "lifecycle",
      ],
      "CC006完整记录",
    );
    await typed.validateTB02ShapeRefs(record);
    need((await s(record, "object_revision")) === "1", "CC006修订错误");
    const first = await i(record, "first_known_generation", true);
    need(
      (await s(record, "first_known_cut")) === "cut-" + first &&
        (await i(record, "published_generation", true)) === first,
      "CC006首次获知不闭合",
    );
    const withdrawn = await (await f(record, "withdrawn_generation")).scalar(),
      reason = await (await f(record, "withdrawal_reason")).scalar(),
      sup = await (await f(record, "superseded_by")).scalar();
    if (withdrawn === null)
      need(
        (await s(record, "lifecycle")) === "active" &&
          reason === null &&
          sup === null,
        "活动CC006携撤回字段",
      );
    else
      need(
        BigInt(withdrawn) > first &&
          (await s(record, "lifecycle")) === "withdrawn" &&
          ["fact_removed", "superseded_by_revision"].includes(reason) &&
          (reason === "fact_removed") === (sup === null),
        "CC006撤回不闭合",
      );
    need(
      ["TOP", "BOTTOM", "RISING", "FALLING"].includes(
        await s(record, "branch"),
      ) &&
        ["UP", "DOWN"].includes(await s(record, "dir_ab")) &&
        ["UP", "DOWN"].includes(await s(record, "dir_bc")),
      "CC006分类枚举不符",
    );
    const comparisons = await f(record, "comparisons");
    need((await comparisons.length()) === 4n, "CC006比较四条件缺失");
    let index = 0;
    for await (const c of comparisons.items()) {
      await keys(
        c,
        ["axis", "cur", "held", "pair", "prev", "strict_up"],
        "CC006比较",
      );
      const axis = ["high", "low"][index % 2],
        pair = ["ab", "bc"][Math.floor(index / 2)];
      need(
        (await s(c, "axis")) === axis && (await s(c, "pair")) === pair,
        "CC006比较轴/顺序不符",
      );
      await n(c, "cur");
      await n(c, "prev");
      need(
        (await f(c, "held")).kind === "bool" &&
          (await f(c, "strict_up")).kind === "bool",
        "CC006比较bool不符",
      );
      index++;
    }
  }
  async function bindSources(snapshot, charge = () => {}) {
    const raw = await auditRaw(snapshot, charge),
      groups = new Map(),
      shapes = new Map(),
      records = [];
    for (const family of ["objects", "withdrawn_objects"])
      for await (const record of (await f(snapshot, family)).items()) {
        charge();
        records.push(record);
        const kind = await t(record, "kind");
        if (typed.TB02_KINDS.includes(kind)) await typed.validateTyped(record);
        else await shapeRecord(record);
        const refs = await f(record, "input_refs"),
          active = (await s(record, "lifecycle")) === "active";
        async function checkRef(ref) {
          await keys(ref, RAWREF, "来源八键");
          const r = raw.byId.get(
            JSON.stringify([
              await t(ref, "identity_key"),
              await s(ref, "revision"),
            ]),
          );
          need(
            r && (await equal(ref, await rawReference(r))),
            "对象八键来源未绑定独立完整原始历史",
          );
          if (active)
            need(
              await equal(
                ref,
                await rawReference(
                  raw.effective.get(await s(ref, "source_coord")) || {},
                ),
              ),
              "活动对象使用旧修订",
            );
        }
        if (typed.TB02_KINDS.includes(kind)) {
          for await (const ref of refs.items()) await checkRef(ref);
        } else
          for await (const group of refs.items())
            for (const name of ["raw_refs", "dependency_refs"])
              for await (const ref of (await f(group, name)).items())
                await checkRef(ref);
        if (kind === "CC-005.inclusion_group") {
          const anchor = await s(await f(record, "payload"), "group_anchor");
          let versions = groups.get(anchor);
          if (!versions) groups.set(anchor, (versions = []));
          versions.push(record);
        }
        if (kind === "CC-006.local_shape")
          shapes.set(await t(record, "object_id"), record);
      }
    const partition = new Set(),
      currentGroups = [];
    for (const versions of groups.values())
      for (const record of versions) {
        if ((await s(record, "lifecycle")) === "active")
          currentGroups.push(await f(record, "payload"));
      }
    const orderedGroups = [];
    for (const payload of currentGroups)
      orderedGroups.push({ index: await i(payload, "group_index"), payload });
    orderedGroups.sort((a, b) =>
      a.index < b.index ? -1 : a.index > b.index ? 1 : 0,
    );
    let expectedIndex = 0n;
    for (const group of orderedGroups) {
      need(group.index === expectedIndex++, "同cut组合并序号不连续");
      for await (const c of (await f(group.payload, "members")).items()) {
        const coord = await S(c);
        need(!partition.has(coord), "同cut组成员重叠");
        partition.add(coord);
      }
    }
    need(
      partition.size === raw.effective.size &&
        [...partition].every((c) => raw.effective.has(c)),
      "同cut组未完整覆盖生效原始历史",
    );
    const bound = new Map();
    for (const [id, shape] of shapes) {
      const cut =
        (await s(shape, "lifecycle")) === "active"
          ? await i(snapshot, "generation")
          : await i(shape, "first_known_generation");
      const anchors = [
          await s(shape, "window_start"),
          await s(shape, "window_mid"),
          await s(shape, "window_end"),
        ],
        selected = [];
      let index = 0;
      for await (const ref of (await f(shape, "input_refs")).items()) {
        let chosen = null;
        for (const g of groups.get(anchors[index]) || []) {
          const first = await i(g, "first_known_generation"),
            withdraw = await (await f(g, "withdrawn_generation")).scalar();
          if (first <= cut && (withdraw === null || BigInt(withdraw) > cut)) {
            need(chosen === null, "同cut组版本不唯一");
            chosen = g;
          }
        }
        need(chosen, "对应cut缺唯一完整组");
        const payload = await f(chosen, "payload");
        need(
          await equal(
            await f(ref, "merged_index"),
            await f(payload, "group_index"),
          ),
          "形态/组序号不同",
        );
        const members = [];
        for await (const r of (await f(ref, "raw_refs")).items())
          members.push(await s(r, "source_coord"));
        need(
          await equal(members, await f(payload, "members")),
          "形态实际成员与组不同",
        );
        const deps = new Set();
        for (const role of ["direction_evidence", "confirmation_evidence"]) {
          const evidence = await f(payload, role);
          if (evidence.kind !== "null")
            for await (const c of (await f(evidence, "source_coords")).items())
              deps.add(await S(c));
        }
        for (const c of members) deps.delete(c);
        const depList = [];
        for await (const r of (await f(ref, "dependency_refs")).items())
          depList.push(await s(r, "source_coord"));
        need(
          await equal(
            depList,
            [...deps].sort((a, b) => (BigInt(a) < BigInt(b) ? -1 : 1)),
          ),
          "形态依赖与组事实不同",
        );
        const groupRefs = new Map();
        for await (const r of (await f(chosen, "input_refs")).items())
          groupRefs.set(await s(r, "source_coord"), r);
        for (const role of ["raw_refs", "dependency_refs"])
          for await (const r of (await f(ref, role)).items())
            need(
              await equal(r, groupRefs.get(await s(r, "source_coord")) || null),
              "形态与组修订不同",
            );
        selected.push(payload);
        index++;
      }
      need(selected.length === 3, "形态组缺失");
      bound.set(id, selected);
    }
    return { raw, records, shapes, bound };
  }
  async function shapeIdentity(
    record,
    groupFacts,
    profile,
    rule,
    numeric = true,
  ) {
    const p = {};
    for (const k of [
      "branch",
      "dir_ab",
      "dir_bc",
      "window_start",
      "window_mid",
      "window_end",
      "comparisons",
    ])
      p[k] = await f(record, k);
    p.merged_highs = [];
    p.merged_lows = [];
    for (const g of groupFacts) {
      p.merged_highs.push(await f(g, "high"));
      p.merged_lows.push(await f(g, "low"));
    }
    p.profile_id = profile;
    p.rule_revision = rule;
    p.input_refs = [];
    for await (const g of (await f(record, "input_refs")).items()) {
      const out = {
        merged_index: number((await i(g, "merged_index")).toString()),
        raw_refs: [],
        dependency_refs: [],
      };
      for (const k of ["raw_refs", "dependency_refs"])
        for await (const r of (await f(g, k)).items()) {
          const refs = {};
          for await (const e of r.entries())
            refs[e.key] = ["seq", "revision"].includes(e.key)
              ? number((await I(e.value, e.key === "revision")).toString())
              : e.value;
          out[k].push(of(refs));
        }
      p.input_refs.push(of(out));
    }
    return of(p);
  }
  async function auditWitnesses(snapshot, binding, charge = () => {}) {
    const seen = new Set();
    for await (const w of (await f(snapshot, "witnesses")).items()) {
      charge();
      await keys(
        w,
        [
          "witness_id",
          "object_id",
          "slot",
          "merged_source_index",
          "merged_high",
          "merged_low",
          "merged_open",
          "merged_close",
          "raw_bars",
        ],
        "witness",
      );
      const id = await t(w, "object_id"),
        slot = await i(w, "slot");
      need(slot < 3n, "witness槽越界");
      const key = JSON.stringify([id, slot.toString()]);
      need(!seen.has(key), "witness槽重复");
      seen.add(key);
      const facts = binding.bound.get(id),
        shape = binding.shapes.get(id);
      need(facts && shape, "witness缺同cut形态");
      const group = facts[Number(slot)],
        mi = await i(group, "group_index");
      need(
        await equal(
          await f(w, "merged_source_index"),
          await f(group, "group_anchor"),
        ),
        "witness锚与组不同",
      );
      for (const axis of ["open", "high", "low", "close"])
        need(
          await equal(await f(w, "merged_" + axis), await f(group, axis)),
          "witness merged_" + axis + "不是同cut组事实",
        );
      const members = await f(group, "members"),
        bars = await f(w, "raw_bars");
      need(
        (await bars.length()) === (await members.length()),
        "witness raw成员数不同",
      );
      const originalBars = [];
      let index = 0n;
      for await (const bar of bars.items()) {
        const coord = await S(await members.at(index++)),
          sourceRef = await (
            await f(await (await f(shape, "input_refs")).at(slot), "raw_refs")
          ).at(index - 1n),
          original = binding.raw.byId.get(
            JSON.stringify([
              await t(sourceRef, "identity_key"),
              await s(sourceRef, "revision"),
            ]),
          );
        need(original, "witness raw缺独立历史原件");
        charge(); // 独立raw_history原件的完整复核，不计普通raw_bars数组元素。
        const expected = await rawOriginal(original),
          actual = await rawOriginal(bar);
        need(await equal(actual, expected), "witness raw原件/次序不同");
        originalBars.push(actual);
      }
      const witnessId =
        "wit-" +
        (
          await sha({
            mi: number(mi.toString()),
            object_id: id,
            raw_bars: originalBars,
            slot: number(slot.toString()),
          })
        ).slice(0, 16);
      need((await t(w, "witness_id")) === witnessId, "witness身份不符");
    }
    need(seen.size === binding.shapes.size * 3, "形态缺完整三槽见证");
  }
  // #1404/#1414：逐cut轴集合随规则修订扩展（同 main validateTB02Axes）；规则→轴表见 RULE_AXES。
  const LEGACY_TB02_AXES = [
      "CC-001",
      "CC-002",
      "CC-003",
      "CC-004",
      "CC-005",
      "CC-006",
      "CC-007",
      "CC-054",
      "CC-056",
    ],
    BI_AXES = [...LEGACY_TB02_AXES, "CC-008", "CC-009", "CC-010", "CC-055"],
    FIRST_SEGMENT_AXES = [...BI_AXES, "CC-011", "CC-012", "CC-013"],
    TB02_AXES = [...FIRST_SEGMENT_AXES, "CC-014"];
  const RULE_AXES = new Map([
    ["s2-axis-quantifiers", LEGACY_TB02_AXES],
    ["s2-axis-quantifiers+new-bi/1", BI_AXES],
    ["s2-axis-quantifiers+new-bi/2", BI_AXES],
    ["s2-axis-quantifiers+new-bi/1+segment-first/1", FIRST_SEGMENT_AXES],
    ["s2-axis-quantifiers+new-bi/1+segment-second/1", TB02_AXES],
    ["s2-axis-quantifiers+new-bi/2+segment-second/1", TB02_AXES],
    ["s2-axis-quantifiers+new-bi/1+segment-second/2", TB02_AXES],
    ["s2-axis-quantifiers+new-bi/2+segment-second/2", TB02_AXES],
  ]);
  async function axesValidate(axes, active = null, ruleRevision = null) {
    axes = of(axes);
    const second = await axes.has("CC-014");
    let secondScope = second ? await s(await f(await f(axes, "CC-011"), "evidence"), "scope") : null;
    if (ruleRevision !== null) {
      need(RULE_AXES.has(ruleRevision), "未声明的目录规则修订");
      await keys(axes, RULE_AXES.get(ruleRevision), "cut规则目录轴");
      secondScope = typed.RULE_CONTRACTS[ruleRevision][1] === "s-segment/3" ? "TB-02-W" : "TB-02-D";
    }
    need(!second || ["TB-02-D", "TB-02-W"].includes(secondScope), "轴范围错误");
    await keys(
      axes,
      second
        ? TB02_AXES
        : (await axes.has("CC-011"))
          ? FIRST_SEGMENT_AXES
          : (await axes.has("CC-008"))
            ? BI_AXES
            : LEGACY_TB02_AXES,
      "逐cut目录轴",
    );
    let ids = null;
    const byId = new Map();
    if (active) {
      ids = new Set();
      for await (const r of of(active).items()) {
        const id = await t(r, "object_id");
        ids.add(id);
        byId.set(id, r);
      }
    }
    for await (const { key: cid, value: axis } of axes.entries()) {
      await keys(
        axis,
        ["impl_status", "proof_status", "run_status", "evidence"],
        "目录轴",
      );
      need(
        ["implemented", "not_implemented"].includes(
          await s(axis, "impl_status"),
        ) &&
          (await s(axis, "proof_status")) === "not_proved" &&
          ["run", "waiting", "not_run"].includes(await s(axis, "run_status")),
        "目录轴状态错误",
      );
      const e = await f(axis, "evidence");
      const scoped =
        e.kind === "object" &&
        (await e.has("scope")) &&
        ["TB-02-D", "TB-02-W"].includes(await s(e, "scope"));
      await keys(
        e,
        [
          ...(cid === "CC-056"
            ? [
                "scope",
                "object_ids",
                "waiting_reasons",
                "raw_revisions",
                "withdrawals",
                "replaces",
              ]
            : ["scope", "object_ids", "waiting_reasons"]),
          ...(scoped
            ? ["delivered_subobligations", "deferred_subobligations"]
            : []),
        ],
        "目录轴证据",
      );
      if (scoped) {
        const continued = await s(e, "scope") === "TB-02-W";
        need(
          (await equal(await f(e, "delivered_subobligations"), [
            "finite_complete_prefix_strict_second",
            "independent_stroke_and_segment_break_facts",
            ...(continued ? ["general_scan_continuation"] : []),
          ])) &&
            (await equal(await f(e, "deferred_subobligations"), [
              "general_equal_identity",
              ...(continued ? [] : ["general_scan_continuation"]),
              "global_G006_bridge",
            ])),
          "第二种子义务范围不符",
        );
        need(
          !["CC-012", "CC-013", "CC-014"].includes(cid) ||
            (await s(axis, "impl_status")) === "not_implemented",
          "有限交付冒称全轴完成",
        );
      }
      need(
        (await s(e, "scope")) ===
          (second && ["CC-011", "CC-012", "CC-013", "CC-014"].includes(cid)
            ? secondScope
            : ["CC-011", "CC-012", "CC-013"].includes(cid)
              ? "TB-02-C"
              : "TB-02-A"),
        "轴范围错误",
      );
      await texts(await f(e, "waiting_reasons"));
      const seen = new Set();
      for await (const idV of (await f(e, "object_ids")).items()) {
        const id = await T(idV);
        need(!seen.has(id) && (!ids || ids.has(id)), "目录引用缺失/重复");
        seen.add(id);
      }
      if (cid === "CC-056") {
        let any = false;
        for (const name of ["raw_revisions", "withdrawals", "replaces"]) {
          const a = await f(e, name);
          need(a.kind === "array", "修订轴集合不完整");
          if ((await a.length()) > 0n) any = true;
          for await (const x of a.items()) await completeValue(x);
        }
        need(
          (await s(axis, "run_status")) === (any ? "run" : "not_run"),
          "修订轴状态不同",
        );
      }
    }
    if (second) {
      const objectIds = async (cid) =>
        f(await f(await f(axes, cid), "evidence"), "object_ids");
      need(
        await equal(await objectIds("CC-014"), await objectIds("CC-013")),
        "CC014未绑定CC013对象",
      );
      if (active)
        for await (const idV of (await objectIds("CC-014")).items()) {
          const r = byId.get(await T(idV));
          need(
            r && (await s(r, "kind")) === "CC-013.segment",
            "CC014引用非段对象",
          );
        }
    }
  }
  async function completeValue(v) {
    v = of(v);
    if (v.reader) {
      await v.reader.complete(v);
      return;
    }
    if (v.kind === "array")
      for await (const x of v.items()) await completeValue(x);
    else if (v.kind === "object")
      for await (const e of v.entries()) await completeValue(e.value);
    else if (v.reader) await v.reader.load(v);
    else await v.scalar();
  }
  async function integerWalk(v) {
    v = of(v);
    const names = new Set([
      "revision",
      "seq",
      "object_revision",
      "first_known_generation",
      "published_generation",
      "withdrawn_generation",
      "supersedes_revision",
      "slot",
      "merged_source_index",
      "merged_index",
      "window_start",
      "window_mid",
      "window_end",
    ]);
    if (v.kind === "array") {
      for await (const x of v.items()) await integerWalk(x);
    } else if (v.kind === "object") {
      // #1392/#1404：新笔/段 payload.slot 是逻辑身份；见证 slot 仍按整数核对。
      const logicalSlot =
        (await v.has("schema_revision")) &&
        ["s-new-bi/1", "s-new-bi/2", "s-segment/1", "s-segment/2", "s-segment/3"].includes(
          await s(v, "schema_revision"),
        );
      for await (const e of v.entries()) {
        need(
          e.value.kind !== "json_text" ||
            ["payload_json", "input_refs_json", "source_coords_json"].includes(
              e.key,
            ),
          "未声明的json_text位置",
        );
        if (
          names.has(e.key) &&
          e.value.kind !== "null" &&
          !(e.key === "slot" && logicalSlot)
        )
          await I(e.value);
        await integerWalk(e.value);
      }
    } else if (v.kind === "json_text")
      await integerWalk(await v.reader.inner(v));
    else if (v.reader) await v.reader.load(v);
    else await v.scalar();
  }
  async function rowKey(family, r) {
    const names =
      family === "objects" || family === "withdrawn_objects"
        ? ["object_id"]
        : family === "relations"
          ? ["subject", "relation_type", "object"]
          : family === "raw_history"
            ? ["identity_key", "revision"]
            : [family === "witnesses" ? "witness_id" : "observation_id"];
    const out = [];
    for (const k of names) out.push(await t(r, k));
    return out;
  }
  async function tuple(family, r) {
    if (family === "objects")
      return (await r.has("fact_key"))
        ? [
            1n,
            await t(r, "kind"),
            await t(r, "fact_key"),
            await t(r, "object_id"),
            await i(r, "object_revision"),
          ]
        : [
            0n,
            await i(r, "window_start"),
            await i(r, "window_mid"),
            await i(r, "window_end"),
            await t(r, "object_id"),
            await i(r, "object_revision"),
          ];
    if (family === "withdrawn_objects")
      return [
        await i(r, "withdrawn_generation"),
        await t(r, "object_id"),
        await i(r, "object_revision"),
      ];
    if (family === "witnesses")
      return [
        await t(r, "object_id"),
        await i(r, "slot"),
        await t(r, "witness_id"),
      ];
    if (family === "relations")
      return [
        await t(r, "subject"),
        await t(r, "relation_type"),
        await t(r, "object"),
      ];
    if (family === "observations")
      return [
        await t(r, "kind"),
        (await f(r, "window_start")).kind === "null"
          ? -1n
          : await i(r, "window_start"),
        await t(r, "observation_id"),
      ];
    return [
      await i(r, "seq"),
      await t(r, "identity_key"),
      await i(r, "revision"),
    ];
  }
  function compareTuples(a, b) {
    for (let j = 0; j < a.length; j++) {
      const c =
        typeof a[j] === "bigint"
          ? a[j] < b[j]
            ? -1
            : a[j] > b[j]
              ? 1
              : 0
          : cmp(a[j], b[j]);
      if (c) return c;
    }
    return 0;
  }
  function chargeActualIo(job, requests, wireBytes) {
    let firstError = null;
    try {
      if (requests > 0n) job.chargeRequests(requests);
    } catch (error) {
      firstError = error;
    }
    try {
      if (wireBytes > 0n) job.chargeWireBytes(wireBytes);
    } catch (error) {
      firstError ||= error;
    }
    if (firstError) throw firstError;
  }
  function operationLedger(core, operation) {
    const actual = { wire_bytes: 0n, records: 0n, requests: 0n };
    let firstStop = null;
    const ledger = {};
    const remember = (e) => {
      firstStop ||= { code: e.code || "query_validation", reason: String(e) };
    };
    for (const method of [
      "beginFirstRead",
      "firstReadDeadline",
      "checkDeadline",
      "preIoCheck",
      "reserveStaged",
      "reserveStore",
      "reconcile",
      "release",
      "reclaimStaged",
      "finishCheck",
      "cancel",
    ]) {
      ledger[method] = (...args) => {
        try {
          return core[method](...args);
        } catch (e) {
          remember(e);
          throw e;
        }
      };
    }
    ledger.acceptNode = async (node) => {
      try {
        return await core.acceptNode(node);
      } catch (e) {
        remember(e);
        throw e;
      }
    };
    for (const [method, key] of [
      ["chargeWireBytes", "wire_bytes"],
      ["chargeRecords", "records"],
      ["chargeRequests", "requests"],
    ])
      ledger[method] = (amount) => {
        need(
          typeof amount === "bigint" && amount >= 0n,
          "实际量必须为规范非负BigInt",
        );
        // actual已经发生；pure Job闭账也不能把网络尾量或已经访问的记录从本operation账里删掉。
        actual[key] += amount;
        try {
          return core[method](amount);
        } catch (e) {
          remember(e);
          throw e;
        }
      };
    ledger.snapshot = () => {
      let snapshot;
      try {
        snapshot = core.snapshot();
      } catch (e) {
        remember(e);
        return Object.freeze({
          ...Object.fromEntries(
            Object.entries(actual).map(([k, v]) => [k, v.toString()]),
          ),
          stopped: e.code || firstStop.code,
          first_failure: firstStop,
          core_snapshot_error: String(e),
          core_metrics_verified: false,
          accounting_operation: String(operation),
          actual_total_fields: ["wire_bytes", "records", "requests"],
          pure_metrics:
            "unknown: monotonic clock failed; no physical amount substituted",
        });
      }
      return Object.freeze({
        ...snapshot,
        accounting_operation: String(operation),
        actual_total_fields: ["wire_bytes", "records", "requests"],
        pure_snapshot_fields: [
          "node_bytes",
          "nodes",
          "staged",
          "store",
          "open_reservations",
          "elapsed_ms",
          "stopped",
        ],
        actual_tail: Object.freeze(
          Object.fromEntries(
            Object.entries(actual).map(([key, value]) => [
              key,
              (value - BigInt(snapshot[key])).toString(),
            ]),
          ),
        ),
        ...Object.fromEntries(
          Object.entries(actual).map(([k, v]) => [k, v.toString()]),
        ),
      });
    };
    return Object.freeze(ledger);
  }
  return function build(api) {
    // snapshot=null：只核目录自身结构并返回静态列（main validateTB02Catalog 的面）；轴绑定与静态SHA要完整同cut态。
    async function catalogValidate(catalog, snapshot = null) {
      const expected = api.TB02_CATALOG;
      need(
        (await s(catalog, "catalog_revision")) === expected.catalog_revision,
        "未绑定冻结目录版本",
      );
      const items = await f(catalog, "items");
      need(
        (await items.length()) === BigInt(expected.ids.length),
        "目录完整数量不符",
      );
      const staticFields = {},
        seen = new Set(),
        counts = { implemented: 0, not_implemented: 0, run: 0, not_run: 0 };
      const generation =
          snapshot === null ? null : await s(snapshot, "generation"),
        axes =
          snapshot === null || generation === "0"
            ? null
            : await f(await f(snapshot, "catalog_evidence"), "axes");
      if (axes) await axesValidate(axes, await f(snapshot, "objects"), await s(snapshot, "rule_revision"));
      for await (const item of items.items()) {
        await keys(
          item,
          [
            "id",
            "kind",
            "title",
            "domain",
            "branches",
            "implementation_status",
            "proof_status",
            "run_status",
            "evidence",
          ],
          "目录项",
        );
        const id = await t(item, "id");
        need(
          expected.ids.includes(id) && !seen.has(id),
          "目录id缺失/重复/额外",
        );
        seen.add(id);
        staticFields[id] = await select(item, [
          "kind",
          "title",
          "domain",
          "branches",
        ]);
        const impl = await s(item, "implementation_status"),
          run = await s(item, "run_status"),
          proof = await s(item, "proof_status");
        need(
          ["implemented", "not_implemented"].includes(impl) &&
            proof === "not_proved" &&
            ["run", "waiting", "not_run"].includes(run),
          "目录状态不符",
        );
        counts[impl]++;
        if (Object.hasOwn(counts, run)) counts[run]++;
        const axis = axes && (await axes.has(id)) ? await f(axes, id) : null;
        if (snapshot === null) continue;
        if (axis) {
          need(
            impl === (await s(axis, "impl_status")) &&
              proof === (await s(axis, "proof_status")) &&
              run === (await s(axis, "run_status")) &&
              (await equal(
                await f(item, "evidence"),
                await f(axis, "evidence"),
              )),
            "目录未绑定同cut完整轴",
          );
        } else
          need(
            impl === "not_implemented" &&
              proof === "not_proved" &&
              run === "not_run" &&
              (await equal(await f(item, "evidence"), {})),
            "目录回填未来事实",
          );
      }
      if (snapshot !== null)
        need(
          (await sha(staticFields)) === expected.public_static_sha256,
          "冻结目录全部静态列SHA不符",
        );
      const actual = await f(catalog, "counts");
      for (const k of Object.keys(counts))
        need((await s(actual, k)) === String(counts[k]), "目录状态计数不符");
      return staticFields;
    }
    async function validateState(
      state,
      protocol = "s-session/3",
      check = () => {},
      charge = () => {},
    ) {
      state = of(state);
      const snapshot = await f(state, "snapshot"),
        catalog = await f(state, "catalog"),
        cut = await f(state, "cut");
      const pubs = [
        "session_id",
        "generation",
        "structure_cut",
        "catalog_revision",
        "index_frontier",
        "scope",
        "input_frontier",
        "seq_range",
        "catalog_run_status",
        "catalog_evidence",
      ];
      for (const k of pubs)
        need(
          await equal(await f(catalog, k), await f(snapshot, k)),
          "Catalog/Snapshot同cut字段不同：" + k,
        );
      for (const k of [
        "session_id",
        "generation",
        "structure_cut",
        "catalog_revision",
        "index_frontier",
      ])
        need(
          await equal(await f(cut, k), await f(snapshot, k)),
          "State.cut字段不同",
        );
      const generation = await s(snapshot, "generation");
      await I(generation);
      need(
        (await s(snapshot, "structure_cut")) === "cut-" + generation,
        "cut代际错误",
      );
      await i(snapshot, "input_frontier", false, true);
      const seq = await f(snapshot, "seq_range");
      await keys(seq, ["from", "to"], "seq_range");
      await i(seq, "from");
      await i(seq, "to", false, true);
      need(
        (await equal(
          await f(seq, "to"),
          await f(snapshot, "input_frontier"),
        )) && (await equal(await f(snapshot, "scope"), SCOPE)),
        "输入前沿/scope错误",
      );
      const mode = await s(snapshot, "history_mode"),
        asof = await (await f(snapshot, "as_of_generation")).scalar();
      need(
        mode === "AsKnown"
          ? asof === generation
          : mode === "RecomputedWithRevision" && asof === null,
        "历史模式/cut错误",
      );
      const profile = await s(snapshot, "profile_id"),
        ph = await s(snapshot, "profile_hash"),
        rule = await s(snapshot, "rule_revision");
      // 空会话也必须先声明已知规则，不能等首次发布才选择合同。
      need(RULE_AXES.has(rule), "cut规则修订未声明");
      if (generation === "0") {
        need(profile && /^[0-9a-f]{64}$/.test(ph), "cut0无冻结profile");
      } else {
        const e = await f(snapshot, "catalog_evidence");
        for (const k of [
          "classified_objects",
          "windows_total",
          "merged_bars",
          "effective_source_positions",
          "withdrawals",
          "replaces",
          "insufficient_knowledge",
          "domain_not_satisfied",
        ])
          await i(e, k);
        for (const k of [
          "profile_id",
          "profile_hash",
          "rule_revision",
          "structure_cut",
          "scope",
        ])
          need(
            await equal(await f(e, k), await f(snapshot, k)),
            "目录证据同cut来源绑定错误",
          );
        need(
          await equal(
            await f(e, "batch_id"),
            await f(snapshot, "index_frontier"),
          ),
          "目录证据batch不同",
        );
      }
      for (const family of FAMILIES) {
        const records = await f(snapshot, family);
        need(records.kind === "array", "六族不是数组");
        for await (const record of records.items()) {
          charge();
          need(record.kind === "object", "记录非对象");
          if (family === "objects" || family === "withdrawn_objects")
            need(
              (await s(record, "lifecycle")) ===
                (family === "objects" ? "active" : "withdrawn"),
              "六族/生命周期错位",
            );
          if (family === "relations") {
            await keys(
              record,
              ["subject", "relation_type", "object"],
              "完整关系",
            );
            for (const name of ["subject", "relation_type", "object"])
              await t(record, name);
          }
          if (family === "observations") {
            await keys(
              record,
              [
                "observation_id",
                "batch_id",
                "kind",
                "reason",
                "window_start",
                "window_mid",
                "window_end",
                "detail",
              ],
              "完整观察",
            );
            for (const name of ["observation_id", "batch_id", "kind", "reason"])
              await t(record, name);
          }

          await integerWalk(record);
          check();
        }
      }
      need(
        profile === "ohlc_integer_tb02a_v1",
        "Q3完整消费者只接受显式OHLC已批准profile",
      );
      // #1404/#1414：按 cut 规则修订选轴表（同 main validateTB02Candidate，先于逐轴校验）。
      if (generation !== "0") {
        await keys(
          await f(await f(snapshot, "catalog_evidence"), "axes"),
          RULE_AXES.get(rule),
          "cut规则目录轴",
        );
      }
      await catalogValidate(catalog, snapshot);
      const binding = await bindSources(snapshot, charge);
      await bindSegmentSequences(snapshot);
      for (const record of binding.records) {
        charge();
        let want;
        need(
          (await i(record, "first_known_generation", true)) <=
            BigInt(generation),
          "对象首次获知来自未来cut",
        );
        const withdrawn = await (
          await f(record, "withdrawn_generation")
        ).scalar();
        if (withdrawn !== null)
          need(BigInt(withdrawn) <= BigInt(generation), "撤回来自未来cut");
        if (protocol === "s-session/3") {
          objectId(await t(record, "object_id"));
          batch(await t(record, "batch_id"));
        }
        const kind = await t(record, "kind");
        if (typed.TB02_KINDS.includes(kind)) {
          const [biSchema, segmentSchema, domain] = Object.hasOwn(typed.RULE_CONTRACTS, rule) ? typed.RULE_CONTRACTS[rule] : [null, null, null];
          const payload = await f(record, "payload");
          if (typed.SEGMENT_KINDS.includes(kind))
            need(await equal(await f(payload, "schema_revision"), segmentSchema), "段合同与cut规则版本不符");
          if (typed.BI_KINDS.includes(kind))
            need(await equal(await f(payload, "schema_revision"), biSchema), "新笔合同与cut规则版本不符");
          if (kind === "CC-054.knowledge_state" && domain !== null)
            need(await equal(await f(payload, "domain"), domain), "TB-02-A 知识域与cut规则版本不符");
          const p = {};
          for (const k of ["kind", "fact_key", "payload", "input_refs"])
            p[k] = await f(record, k);
          p.profile_id = profile;
          p.rule_revision = rule;
          want =
            protocol === "s-session/3"
              ? await identity(0x20, p, "obj3-")
              : "obj-" + (await sha(p));
        } else {
          const p = await shapeIdentity(
            record,
            binding.bound.get(await t(record, "object_id")),
            profile,
            rule,
          );
          want =
            protocol === "s-session/3"
              ? await identity(0x20, p, "obj3-")
              : "obj-" + (await sha(p));
        }
        need((await t(record, "object_id")) === want, "对象内容身份不符");
        check();
      }
      await auditWitnesses(snapshot, binding, charge);
      check();
      return binding;
    }
    // #1414/#1420：同 cut 活动集内 s-segment/2、/3 段与其 FIRST/SECOND 序列对象互绑；判据同已审
    // validateTB02Candidate（payload 已由 typed 完整校验，此处在同一原件上做跨对象绑定）。
    async function bindSegmentSequences(snapshot) {
      const equalPlain = (a, b) => api.canonical(a) === api.canonical(b);
      const activeValues = new Map();
      for await (const record of (await f(snapshot, "objects")).items())
        activeValues.set(await t(record, "object_id"), record);
      for (const value of activeValues.values()) {
        if ((await s(value, "kind")) !== "CC-013.segment") continue;
        const r = await typed.plain(value);
        if (!["s-segment/2", "s-segment/3"].includes(r.payload.schema_revision)) continue;
        const d = r.payload.data;
        need(d.second_sequence_id !== null || d.fact_vector.scan_complete, "未完成扫描缺SECOND资源证据");
        for (const [prefix, role] of [["sequence", "FIRST_SEQUENCE"], ["second_sequence", "SECOND_SEQUENCE"]]) {
          const oid = d[prefix + "_object_id"]; if (oid === null) continue;
          const qValue = activeValues.get(oid), q = qValue ? await typed.plain(qValue) : undefined;
          need(q && q.kind === "CC-012.feature_sequence" && q.payload.schema_revision === r.payload.schema_revision, "段引用缺同cut序列对象");
          const v = q.payload.data;
          need(v.entity_id === d[prefix + "_id"] && v.segment_id === d.entity_id && v.role === role, "段序列引用身份/角色不符");
          need(q.source_coords.every(c => r.source_coords.includes(c)), "段缺序列实际来源");
          if (role === "FIRST_SEQUENCE" && d.strict !== null) need(v.facts.checks.some(c => equalPlain(c.elements, d.strict.elements)), "strict未绑定FIRST检查");
          if (role === "SECOND_SEQUENCE") need(d.fact_vector.scan_complete !== (v.facts.resource_truncated && v.facts.matched === null), "段事实可知性未绑定SECOND扫描证据");
          if (role === "SECOND_SEQUENCE" && d.result === "CASE_TWO") need(v.second_sequence_status === "CONFIRMED" && equalPlain(d.fact_vector.termination_witness.second_match, v.facts.matched), "第二种终结证书未绑定同次SECOND命中");
          if (role === "SECOND_SEQUENCE" && r.payload.schema_revision === "s-segment/3" && ["KILLED_CASE_TWO", "PENDING_CASE_TWO"].includes(d.result)) need(v.second_sequence_status === (d.result === "KILLED_CASE_TWO" ? "KILLED" : "PENDING"), "段结果未绑定SECOND终态");
        }
      }
    }
    // #1394 整合（根裁定 (1)，(4) 扩到 v3；与 #1414 validate_once 同一原则）：同一 State 只完整验证一次。
    // s-session/2：LegacyClient.collect 先经页面钩子、再自己各调一次，两次入参只差 cut 的表示与
    // rule_revision 的取值来源（TB-02-D cut40 上两遍合计约 32–35 s，超过 30 s 读取期限）；
    // s-session/3：Q3.Client 采集先自己调一次、再经页面钩子调一次，是同一 state。
    // 缓存键 = 协议 + validateState 实际读取的全部输入（catalog、snapshot、cut 的五个同 cut 键）的内容
    // 编码的 SHA-256：普通值逐值编码；Reader 支持的图值取它已核的内容引用（kind、root、extent），root 是
    // Merkle 内容摘要，任一字节变化即换根。任一内容变化即换键重验。只记通过，失败不记；命中时按首验的
    // charge 次数原样重放（v3 的记录计费与预算判定不被缓存吃掉），再调一次 check() 守读取期限；判据一字不改。
    const ONCE_CUT_KEYS = [
      "session_id",
      "generation",
      "structure_cut",
      "catalog_revision",
      "index_frontier",
    ];
    const ONCE_CAPACITY = 2;
    const validatedOnce = new Map();
    // 普通对象（任一 realm 的 Object.prototype 或 null 原型）；Value 包装与数组不算。
    function plainRecord(value) {
      if (value === null || typeof value !== "object" || Array.isArray(value))
        return false;
      const proto = Object.getPrototypeOf(value);
      return proto === null || Object.getPrototypeOf(proto) === null;
    }
    const own = (value, key) => Object.prototype.hasOwnProperty.call(value, key);
    // 无 Reader 的 Value 只是普通值的包装；Reader 支持的图值返回 null（不可当普通容器展开）。
    const plainOf = (value) =>
      value instanceof V.Value ? (value.reader ? null : value.plain) : value;
    // 带类型标签的内容编码：不同类型、不同结构不会编成同一串；不可编码的值抛出，交给完整验证。
    function onceShape(value) {
      if (value instanceof V.Value) {
        if (value.reader) {
          const ref = value.ref;
          return [
            "ref",
            ref.kind,
            ref.root,
            ref.extent === null ? "" : String(ref.extent),
          ];
        }
        return onceShape(value.plain);
      }
      if (value === null) return ["null"];
      if (typeof value === "boolean") return ["bool", value ? "1" : "0"];
      if (typeof value === "string") return ["text", value];
      if (V.isNumber(value)) return ["number", value.token];
      if (Array.isArray(value)) {
        const items = ["array"];
        for (let i = 0; i < value.length; i++) {
          need(own(value, i), "稀疏数组不进缓存");
          items.push(onceShape(value[i]));
        }
        return items;
      }
      need(plainRecord(value), "不可编码值不进缓存");
      const fields = ["object"];
      for (const key of Object.keys(value).sort())
        fields.push([key, onceShape(value[key])]);
      return fields;
    }
    async function onceKey(state, protocol) {
      if (protocol !== "s-session/2" && protocol !== "s-session/3") return null;
      const root = plainOf(state);
      if (
        !plainRecord(root) ||
        !["catalog", "snapshot", "cut"].every((k) => own(root, k))
      )
        return null;
      const cut = plainOf(root.cut);
      if (!plainRecord(cut) || !ONCE_CUT_KEYS.every((k) => own(cut, k)))
        return null;
      try {
        return await api.sha256([
          "q3-validate-once/1",
          protocol,
          onceShape(root.catalog),
          onceShape(root.snapshot),
          onceShape(Object.fromEntries(ONCE_CUT_KEYS.map((k) => [k, cut[k]]))),
        ]);
      } catch {
        return null; // 无规范内容编码的入参不进缓存，交给完整验证去拒绝。
      }
    }
    async function validateStateOnce(
      state,
      protocol = "s-session/3",
      check = () => {},
      charge = () => {},
    ) {
      const key = await onceKey(state, protocol);
      if (key !== null && validatedOnce.has(key)) {
        const entry = validatedOnce.get(key);
        validatedOnce.delete(key);
        validatedOnce.set(key, entry);
        for (let n = 0; n < entry.charges; n++) charge();
        check();
        return entry.binding;
      }
      let charges = 0;
      const binding = await validateState(state, protocol, check, () => {
        charges++;
        charge();
      });
      if (key !== null) {
        validatedOnce.set(key, { binding, charges });
        while (validatedOnce.size > ONCE_CAPACITY)
          validatedOnce.delete(validatedOnce.keys().next().value);
      }
      return binding;
    }
    async function validateDelta(outer, charge = () => {}) {
      outer = of(outer);
      await keys(outer, [...D, "delta"], "Delta外层");
      const inner = await f(outer, "delta");
      await keys(inner, [...D, ...CHANGES], "Delta完整内层");
      for (const k of D)
        need(
          await equal(await f(outer, k), await f(inner, k)),
          "Delta内外字段不同：" + k,
        );
      const generation = await s(inner, "generation");
      await I(generation, true);
      need(
        (await s(inner, "next_cut")) === "cut-" + generation,
        "Delta cut不符",
      );
      batch(await s(inner, "index_frontier"));
      const range = await f(inner, "seq_range");
      await keys(range, ["from", "to"], "Delta.seq_range");
      await i(range, "from");
      await i(range, "to", false, true);
      need(
        await equal(await f(range, "to"), await f(inner, "input_frontier")),
        "Delta前沿不符",
      );
      const replacements = new Set();
      for (const name of CHANGES) {
        const arr = await f(inner, name);
        need(arr.kind === "array", "Delta变化集合缺失");
        for await (const record of arr.items()) {
          charge();
          await integerWalk(record);
        }
      }
      for await (const w of (await f(inner, "withdrawals")).items()) {
        charge();
        const hasFact = await w.has("fact_key");
        await keys(
          w,
          hasFact
            ? ["object_id", "fact_key", "reason", "superseded_by"]
            : [
                "object_id",
                "reason",
                "superseded_by",
                "window_start",
                "window_mid",
                "window_end",
              ],
          "撤回",
        );
        const reason = await t(w, "reason"),
          sup = await (await f(w, "superseded_by")).scalar();
        need(
          ["fact_removed", "superseded_by_revision"].includes(reason) &&
            (reason === "fact_removed") === (sup === null),
          "撤回原因/替代不符",
        );
        if (sup !== null)
          replacements.add(
            JSON.stringify([await t(w, "object_id"), await T(sup)]),
          );
      }
      const actual = new Set();
      for await (const r of (await f(inner, "replaces")).items()) {
        charge();
        await keys(r, ["old_object_id", "new_object_id"], "替代");
        const key = JSON.stringify([
          await t(r, "old_object_id"),
          await t(r, "new_object_id"),
        ]);
        need(!actual.has(key), "替代端点重复");
        actual.add(key);
      }
      need(
        actual.size === replacements.size &&
          [...actual].every((k) => replacements.has(k)),
        "Delta遗漏/额外replaces",
      );
      const e = await f(inner, "catalog_evidence");
      await axesValidate(await f(e, "axes"), null, await s(e, "rule_revision"));
      return inner;
    }
    async function comparePublication(delta, snapshot) {
      const dd = await f(delta, "delta");
      for (const [dk, sk] of [
        ["session_id", "session_id"],
        ["generation", "generation"],
        ["catalog_revision", "catalog_revision"],
        ["next_cut", "structure_cut"],
        ["index_frontier", "index_frontier"],
        ["input_frontier", "input_frontier"],
        ["seq_range", "seq_range"],
        ["catalog_run_status", "catalog_run_status"],
        ["catalog_evidence", "catalog_evidence"],
      ])
        need(
          await equal(await f(dd, dk), await f(snapshot, sk)),
          "Delta/Snapshot字段不同：" + sk,
        );
      for (const k of ["profile_id", "profile_hash", "scope"])
        need(
          await equal(
            await f(await f(dd, "catalog_evidence"), k),
            await f(snapshot, k),
          ),
          "Delta来源不同",
        );
    }
    class Client {
      #committed = null;
      get committed() {
        return this.#committed;
      }
      constructor({
        fetch: fetcher,
        hooks = {},
        onCommit = () => {},
        limits,
        allocation,
        clock = () => Math.floor(performance.now()),
      }) {
        need(
          typeof (fetcher || globalThis.fetch) === "function",
          "缺网络读取能力",
        );
        this.fetch = fetcher || globalThis.fetch.bind(globalThis);
        this.hooks = hooks;
        this.onCommit = onCommit;
        this.limits = jobs.validateConfig(limits);
        const configIdentity = {
          schema_revision: this.limits.schema_revision,
          job_limits: this.limits.job_limits,
        };
        for (const key of [
          "pageSize",
          "watchBatches",
          "pollMs",
          "timeoutMs",
          "replyBytes",
        ])
          configIdentity[key] = number(String(this.limits[key]));
        this.configDigest = sha(configIdentity);
        need(
          allocation &&
            allocation.contract === "s-query-private-allocation/1" &&
            typeof allocation.measure === "function" &&
            typeof allocation.reserveBound === "function" &&
            typeof allocation.write === "function" &&
            typeof allocation.release === "function",
          "未验证：没有实际分配测量/暂存适配器，不能宣称Browser物理限额通过",
        );
        need(
          typeof allocation.measurementSource === "string" &&
            allocation.measurementSource.length > 0,
          "测量适配器缺具名实际来源",
        );
        api.canonical(allocation.measurementSource);
        this.allocation = allocation;
        this.clock = clock;
        this.operation = 0;
        this.serial = 0n;
        this.instance =
          globalThis.crypto?.randomUUID?.() ||
          (typeof require === "function"
            ? require("node:crypto").randomUUID()
            : null);
        need(this.instance, "缺查询身份");
        this.active = null;
        this.lastJob = null;
      }
      cancel() {
        // 立即禁止旧operation安装并停止纯Job，Hash晚到不准入；
        // 已发起I/O的真实尾量仍由同operation诊断账保留，不能复活纯Job。
        this.operation++;
        this.active?.job.cancel();
      }
      async begin() {
        this.cancel();
        const operation = this.operation,
          started = this.clock(),
          abort = new AbortController();
        const elapsedCap = BigInt(this.limits.job_limits.max_elapsed_ms);
        const waitLimit =
          elapsedCap < BigInt(this.limits.timeoutMs)
            ? Number(elapsedCap)
            : this.limits.timeoutMs;
        let measureTimer, measured;
        try {
          measured = await Promise.race([
            this.allocation.measure({ operation, signal: abort.signal }),
            new Promise((_, reject) => {
              measureTimer = setTimeout(() => {
                abort.abort();
                reject(new Error("实际分配基线测量超过本operation绝对期限"));
              }, waitLimit);
            }),
          ]);
        } finally {
          clearTimeout(measureTimer);
        }
        need(operation === this.operation, "旧分配测量晚到");
        need(
          measured &&
            measured.exact === true &&
            measured.source === this.allocation.measurementSource,
          "缺exact measured基线",
        );
        const coreJob = new jobs.Job(
          this.limits.job_limits,
          {
            store_bytes: measured.store_bytes,
            staged_bytes: measured.staged_bytes,
          },
          (() => {
            let first = true;
            return () => {
              if (first) {
                first = false;
                return started;
              }
              return this.clock();
            };
          })(),
        );
        const job = operationLedger(coreJob, operation);
        job.beginFirstRead();
        const budget = { operation, job, readers: [], identity: null };
        this.active = budget;
        this.lastJob = job;
        this.check(budget);
        return budget;
      }
      check(b) {
        need(
          b.operation === this.operation && this.active === b,
          "旧操作/取消结果不得安装",
        );
        b.job.checkDeadline();
      }
      async store(node, b) {
        this.check(b);
        const requested = await this.allocation.reserveBound(node);
        this.check(b);
        const grant = b.job.reserveStaged(requested);
        let actual;
        try {
          actual = await this.allocation.write(node, {
            operation: b.operation,
          });
        } catch (e) {
          if (e.allocated_bytes !== undefined)
            b.job.reconcile(grant, e.allocated_bytes);
          else b.job.release(grant);
          throw e;
        }
        need(
          actual &&
            actual.exact === true &&
            actual.source === this.allocation.measurementSource,
          "写后实际分配没有exact measured对账",
        );
        b.job.reconcile(grant, actual.allocated_bytes);
        this.check(b);
      }
      async request(identity, payload, b) {
        this.check(b);
        identity = Object.freeze(smallIdentity(identity));
        payload = api.parse(api.canonical(payload));
        need(
          typeof identity.session_id === "string" && identity.session_id,
          "会话身份为空",
        );
        await I(identity.session_generation, true);
        b.job.beginFirstRead();
        const request = {
          schema_revision: "s-session/3",
          ...smallIdentity(identity),
          source_namespace: "s-observe/browser",
          source_epoch: this.instance,
          message_id: "query-" + (++this.serial).toString(),
          producer_id: "browser:" + this.instance,
          producer_epoch: "1",
          payload_hash: await api.sha256(payload),
          causal_refs: [],
          payload,
        };
        this.check(b);
        const body = api.canonical(request),
          abort = new AbortController(),
          started = this.clock(),
          timer = setTimeout(() => abort.abort(), this.limits.timeoutMs);
        let reader;
        try {
          b.job.preIoCheck();
          let pendingResponse, launchError;
          try {
            pendingResponse = Promise.resolve(
              this.fetch("/api/v3/" + payload.op, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body,
                signal: abort.signal,
                cache: "no-store",
              }),
            );
          } catch (error) {
            launchError = error;
          }
          if (pendingResponse) pendingResponse.catch(() => {});
          chargeActualIo(
            b.job,
            1n,
            BigInt(new TextEncoder().encode(body).length),
          );
          if (launchError) throw launchError;
          const response = await pendingResponse;
          this.check(b);
          need(
            response.body && typeof response.body.getReader === "function",
            "缺有限响应流",
          );
          reader = response.body.getReader();
          const chunks = [];
          let length = 0;
          while (true) {
            this.check(b);
            need(
              this.clock() - started < this.limits.timeoutMs,
              "单请求期限已过",
            );
            const { done, value } = await reader.read();
            if (value) {
              length += value.byteLength;
              b.job.chargeWireBytes(BigInt(value.byteLength));
            }
            this.check(b);
            need(length <= this.limits.replyBytes, "响应超过显式replyBytes");
            if (done) break;
            chunks.push(value);
          }
          const bytes = new Uint8Array(length);
          let at = 0;
          for (const chunk of chunks) {
            bytes.set(chunk, at);
            at += chunk.byteLength;
          }
          const reply = api.parse(
            new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(
              bytes,
            ),
          );
          await keys(reply, HEADER, "响应11字段");
          need(reply.schema_revision === "s-session/3", "响应协议不符");
          for (const k of HEADER.filter(
            (k) => !["causal_refs", "payload"].includes(k),
          ))
            await T(reply[k]);
          await I(reply.session_generation, true);
          await I(reply.producer_epoch, true);
          digest(reply.payload_hash);
          need(
            reply.source_namespace === "s-observe/replies" &&
              reply.source_epoch === reply.producer_epoch &&
              reply.producer_id === "Q:" + reply.session_id,
            "响应来源/生产者身份错误",
          );
          const causal = {};
          for (const k of [
            "source_namespace",
            "source_epoch",
            "message_id",
            "payload_hash",
          ])
            causal[k] = request[k];
          need(await equal(reply.causal_refs, [causal]), "因果身份不同");
          need(
            reply.payload_hash === (await api.sha256(reply.payload)),
            "payload_hash错误",
          );
          need(
            reply.message_id ===
              "reply-" +
                (await api.sha256([
                  request.source_namespace,
                  request.source_epoch,
                  request.message_id,
                  reply.payload_hash,
                ])),
            "reply消息身份错误",
          );
          this.check(b);
          need(
            response.ok && reply.payload.ok === true,
            "查询未完成：" + (reply.payload.error || response.status),
          );
          return reply;
        } finally {
          clearTimeout(timer);
          abort.abort();
          if (reader) {
            await reader.cancel().catch(() => {});
            reader.releaseLock();
          }
        }
      }
      async fixed(fixed) {
        await keys(fixed, FIXED, "fixed_cut");
        await T(fixed.session_id);
        await I(fixed.session_generation, true);
        await I(fixed.cut_generation);
        await I(fixed.input_frontier, false, true);
        await T(fixed.catalog_revision);
        await T(fixed.rule_revision);
        await S(fixed.profile_id);
        await S(fixed.profile_hash);
        need(
          fixed.structure_cut === "cut-" + fixed.cut_generation &&
            (await equal(fixed.scope, SCOPE)),
          "fixed cut/scope错误",
        );
        await keys(fixed.seq_range, ["from", "to"], "seq_range");
        await I(fixed.seq_range.from);
        await I(fixed.seq_range.to, false, true);
        need(fixed.seq_range.to === fixed.input_frontier, "fixed前沿错误");
        need(
          fixed.history_mode === "AsKnown"
            ? fixed.as_of_generation === fixed.cut_generation
            : fixed.history_mode === "RecomputedWithRevision" &&
                fixed.as_of_generation === null,
          "历史cut错误",
        );
        content.validateRef(fixed.catalog_evidence);
        if (fixed.cut_generation !== "0") batch(fixed.index_frontier);
      }
      async token(token, fixed, projectionDigest, offset, b) {
        await keys(token, TOKEN, "Snapshot token");
        need(
          token.token_schema === "s-snapshot-token/2" &&
            token.order_version === "s-record-order/2" &&
            token.page_size === String(this.limits.pageSize) &&
            token.offset === String(offset),
          "token版本/顺序/范围错误",
        );
        digest(token.issued_capture_digest);
        digest(token.token_checksum);
        for (const key of [
          "session_id",
          "session_generation",
          "cut_generation",
          "scope",
          "history_mode",
          "structure_cut",
          "catalog_revision",
          "index_frontier",
          "profile_hash",
          "rule_revision",
        ])
          need(
            await equal(token[key], fixed[key]),
            "token fixed身份错误：" + key,
          );
        need(
          token.cut_projection_digest === projectionDigest,
          "token投影根错误",
        );
        const unsigned = { ...token };
        delete unsigned.token_checksum;
        need(
          (await api.sha256(unsigned)) === token.token_checksum,
          "token checksum错误",
        );
        this.check(b);
      }
      reader(anchor, identity, b) {
        content.validateAnchor(anchor);
        const r = new V.Reader({
          anchor,
          request: async (p) => {
            const e = await this.request(identity, p, b);
            need(sameIdentity(e, identity), "片段会话化身不同");
            return e.payload;
          },
          check: () => this.check(b),
          accept: async (n) => {
            this.check(b);
            await b.job.acceptNode(n);
            this.check(b);
          },
          store: (n) => this.store(n, b),
          limit: this.limits.pageSize,
        });
        b.readers.push(r);
        return r;
      }
      async collect(
        {
          identity,
          mode,
          generation = null,
          token = null,
          expectedFixed = null,
        },
        b,
      ) {
        this.check(b);
        need(
          mode === "AsKnown" || mode === "RecomputedWithRevision",
          "历史模式无效",
        );
        if (generation !== null) await I(generation);
        if (token) {
          await this.fixed(expectedFixed);
          await this.token(
            token,
            expectedFixed,
            token.cut_projection_digest,
            0,
            b,
          );
        }
        let next = token
            ? { op: "snapshot", snapshot_token: token }
            : {
                op: "snapshot",
                ...smallIdentity(identity),
                scope: SCOPE,
                history_mode: mode,
                as_of_generation: generation,
                page_size: String(this.limits.pageSize),
                order_version: "s-record-order/2",
              },
          base = null,
          issued = token?.issued_capture_digest,
          rows = [],
          seen = new Set(),
          objectIds = new Set(),
          pages = 0;
        for (;;) {
          const envelope = await this.request(identity, next, b),
            page = envelope.payload;
          pages++;
          await keys(page, PAGE, "Snapshot page");
          await this.fixed(page.fixed_cut);
          need(
            sameIdentity(envelope, identity) &&
              sameIdentity(page.fixed_cut, identity),
            "Snapshot会话/化身不符",
          );
          need(
            page.fixed_cut.history_mode === mode &&
              (generation === null ||
                page.fixed_cut.cut_generation === generation),
            "Snapshot模式/cut不符",
          );
          if (expectedFixed)
            need(await equal(page.fixed_cut, expectedFixed), "Gap指定切面不同");
          need(
            (await equal(page.scope, SCOPE)) &&
              (await equal(page.omissions, [])) &&
              page.order_version === "s-record-order/2",
            "Snapshot scope/order/omissions错误",
          );
          need(
            /^qcut3-[0-9a-f]{64}$/.test(page.cut_projection_digest),
            "qcut3格式错误",
          );
          digest(page.validated_capture_digest);
          content.validateRef(page.catalog);
          await keys(page.counts, [...FAMILIES, "total"], "counts");
          let total = 0n;
          for (const family of FAMILIES) total += await I(page.counts[family]);
          need(total === (await I(page.counts.total)), "六族总计数不同");
          need(
            Array.isArray(page.rows) &&
              typeof page.done === "boolean" &&
              page.offset === String(rows.length),
            "page offset/done/rows错误",
          );
          const metadata = {};
          for (const key of [
            "fixed_cut",
            "catalog",
            "counts",
            "scope",
            "omissions",
            "order_version",
            "cut_projection_digest",
          ])
            metadata[key] = page[key];
          if (base) need(await equal(base, metadata), "跨页固定头变化");
          else {
            base = metadata;
            issued ||= page.validated_capture_digest;
          }
          const rest = total - BigInt(rows.length),
            length =
              rest < BigInt(this.limits.pageSize)
                ? rest
                : BigInt(this.limits.pageSize);
          need(
            rest >= 0n &&
              BigInt(page.rows.length) === length &&
              page.done === (rest === length),
            "page缺漏/截断/空转",
          );
          for (const row of page.rows) {
            await keys(row, ["family", "key", "record"], "row");
            need(
              FAMILIES.includes(row.family) && Array.isArray(row.key),
              "未知族/key",
            );
            content.validateRef(row.record);
            need(row.record.kind === "object", "record引用非对象");
            for (const key of row.key) await T(key);
            const id = api.canonical([row.family, row.key]);
            need(!seen.has(id), "公开key重复");
            seen.add(id);
            if (["objects", "withdrawn_objects"].includes(row.family)) {
              need(
                row.key.length === 1 && !objectIds.has(row.key[0]),
                "活动/撤回身份冲突",
              );
              objectIds.add(row.key[0]);
            }
            rows.push(row);
          }
          this.check(b);
          if (page.done) {
            need(page.next_token === null, "末页仍带token");
            break;
          }
          await this.token(
            page.next_token,
            base.fixed_cut,
            base.cut_projection_digest,
            rows.length,
            b,
          );
          need(
            page.next_token.issued_capture_digest === issued,
            "token签发capture变化",
          );
          next = { op: "snapshot", snapshot_token: page.next_token };
        }
        const counts = {};
        for (const family of FAMILIES) counts[family] = 0n;
        for (const row of rows) counts[row.family]++;
        for (const family of FAMILIES)
          need(counts[family].toString() === base.counts[family], "族计数不符");
        const fixedWire = base.fixed_cut,
          logicFixed = {
            ...fixedWire,
            seq_range: { ...fixedWire.seq_range },
            scope: { ...fixedWire.scope },
            catalog_evidence: content.refEdge(fixedWire.catalog_evidence),
          },
          logical = {
            fixed_cut: logicFixed,
            catalog: content.refEdge(base.catalog),
            rows: rows.map((row) => ({
              ...row,
              key: [...row.key],
              record: content.refEdge(row.record),
            })),
            counts: { ...base.counts },
            scope: { ...base.scope },
            omissions: [],
            order_version: base.order_version,
          };
        const built = await content.encode(content.fromPlain(logical));
        this.check(b);
        need(
          "qcut3-" + V.F(0x22, V.unhex(built.ref.root.slice(4))) ===
            base.cut_projection_digest,
          "完整七字段projection qcut3不符",
        );
        const anchor = {
            kind: "projection",
            fixed_cut: fixedWire,
            cut_projection_digest: base.cut_projection_digest,
            projection_ref: built.ref,
          },
          reader = this.reader(anchor, identity, b),
          root = reader.value(built.ref);
        await reader.complete(root);
        this.check(b);
        await reader.finish();
        this.check(b);
        const fixed = await root.field("fixed_cut"),
          catalog = await root.field("catalog"),
          families = {};
        for (const family of FAMILIES) families[family] = [];
        let previous = null;
        for await (const row of (await root.field("rows")).items()) {
          b.job.chargeRecords(1n);
          this.check(b);
          const family = await t(row, "family"),
            record = await row.field("record"),
            key = await rowKey(family, record);
          need(
            await equal(await row.field("key"), key),
            "row.key与真实记录派生不同",
          );
          const tupleValue = await tuple(family, record),
            order = { family, tuple: tupleValue };
          if (previous) {
            const fc =
              FAMILIES.indexOf(family) - FAMILIES.indexOf(previous.family);
            need(
              fc > 0 ||
                (fc === 0 && compareTuples(previous.tuple, tupleValue) < 0),
              "六族原顺序/元组没有严格递增",
            );
          }
          previous = order;
          families[family].push(record);
        }
        const snapshotFields = {};
        for await (const e of fixed.entries())
          if (e.key !== "cut_generation") snapshotFields[e.key] = e.value;
        snapshotFields.generation = fixedWire.cut_generation;
        for (const family of FAMILIES)
          snapshotFields[family] = families[family];
        const snapshot = of(snapshotFields),
          cut = {
            ...smallIdentity(identity),
            generation: fixedWire.cut_generation,
            structure_cut: fixedWire.structure_cut,
            catalog_revision: fixedWire.catalog_revision,
            index_frontier: fixedWire.index_frontier,
          },
          state = of({ ok: true, catalog, snapshot, cut });
        // 与页面钩子共用同一 validate-once 入口：同一 state 只完整验证一次（根裁定 (4)）。
        await validateStateOnce(
          state,
          "s-session/3",
          () => this.check(b),
          () => b.job.chargeRecords(1n),
        );
        this.check(b);
        if (this.hooks.validateState)
          await this.hooks.validateState(state, "s-session/3");
        this.check(b);
        const cursor = {
          ...smallIdentity(identity),
          catalog_revision: fixedWire.catalog_revision,
          scope: fixedWire.scope,
          after_generation: fixedWire.cut_generation,
          base_cut: fixedWire.structure_cut,
          index_frontier: fixedWire.index_frontier,
          last_seq: fixedWire.input_frontier,
          order_version: "s-record-order/2",
        };
        return {
          state,
          projection: root,
          fixed: fixedWire,
          cursor,
          pages,
          families,
          reader,
          fullyValidated: true,
          resources_sha256: await this.configDigest,
          allocation_measurement_source: this.allocation.measurementSource,
          allocation_test_only: this.allocation.testOnly === true,
        };
      }
      async commit(candidate, b, event) {
        this.check(b);
        need(candidate.fullyValidated, "未完成候选不得安装");
        for (const reader of b.readers) await reader.finish();
        this.check(b);
        const finalMeasure = await this.allocation.measure();
        this.check(b);
        need(
          finalMeasure.exact === true &&
            finalMeasure.source === this.allocation.measurementSource,
          "安装前缺实际测量",
        );
        const ledger = b.job.snapshot();
        need(
          BigInt(finalMeasure.store_bytes) === BigInt(ledger.store.exposure) &&
            BigInt(finalMeasure.staged_bytes) ===
              BigInt(ledger.staged.exposure),
          "安装前实际分配/累计Job账不符",
        );
        b.job.finishCheck();
        this.check(b);
        for (const reader of b.readers) reader.freeze();
        for (const family of FAMILIES)
          Object.freeze(candidate.families[family]);
        Object.freeze(candidate.families);
        Object.freeze(candidate.cursor.scope);
        Object.freeze(candidate.cursor);
        this.#committed = candidate;
        try {
          this.onCommit(candidate, event);
        } catch (error) {
          candidate.presentation_error = String(error);
        }
        return candidate;
      }
      async run(fn) {
        const b = await this.begin();
        try {
          return await fn(b);
        } catch (error) {
          this.lastFailure = Object.freeze({
            operation: b.operation,
            code: error.code || "query_validation",
            reason: String(error),
          });
          b.job.cancel();
          try {
            const deleted = await this.allocation.release({
              operation: b.operation,
            });
            if (
              deleted?.exact === true &&
              deleted.source === this.allocation.measurementSource
            )
              b.job.reclaimStaged(deleted.deleted_bytes);
          } catch {}
          throw error;
        }
      }
      async ready(b) {
        this.check(b);
        b.job.beginFirstRead();
        const abort = new AbortController(),
          timer = setTimeout(() => abort.abort(), this.limits.timeoutMs);
        let reader;
        try {
          b.job.preIoCheck();
          let pendingResponse, launchError;
          try {
            pendingResponse = Promise.resolve(
              this.fetch("/api/ready", {
                method: "GET",
                cache: "no-store",
                signal: abort.signal,
              }),
            );
          } catch (error) {
            launchError = error;
          }
          if (pendingResponse) pendingResponse.catch(() => {});
          b.job.chargeRequests(1n);
          if (launchError) throw launchError;
          const response = await pendingResponse;
          this.check(b);
          need(response.body?.getReader, "Ready缺有限字节流");
          reader = response.body.getReader();
          const chunks = [];
          let total = 0;
          while (true) {
            this.check(b);
            const item = await reader.read();
            if (item.value) {
              total += item.value.byteLength;
              b.job.chargeWireBytes(BigInt(item.value.byteLength));
            }
            this.check(b);
            need(total <= this.limits.replyBytes, "Ready超过replyBytes");
            if (item.done) break;
            chunks.push(item.value);
          }
          const bytes = new Uint8Array(total);
          let offset = 0;
          for (const c of chunks) {
            bytes.set(c, offset);
            offset += c.length;
          }
          const ready = api.parse(
            new TextDecoder("utf-8", { fatal: true }).decode(bytes),
          );
          await keys(
            ready,
            ["ok", "cut", "producer_epoch", "control_instance_id"],
            "Ready",
          );
          need(response.ok && ready.ok === true, "Ready未完成");
          await I(ready.producer_epoch, true);
          await S(ready.control_instance_id);
          await keys(
            ready.cut,
            [
              "session_id",
              "session_generation",
              "generation",
              "structure_cut",
              "catalog_revision",
              "index_frontier",
            ],
            "Ready cut",
          );
          await T(ready.cut.session_id);
          await I(ready.cut.session_generation, true);
          await I(ready.cut.generation);
          need(
            ready.cut.structure_cut === "cut-" + ready.cut.generation,
            "Ready cut不同",
          );
          await T(ready.cut.catalog_revision);
          if (ready.cut.generation !== "0") batch(ready.cut.index_frontier);
          this.check(b);
          return ready;
        } finally {
          clearTimeout(timer);
          abort.abort();
          if (reader) {
            await reader.cancel().catch(() => {});
            reader.releaseLock();
          }
        }
      }
      async loadReady(mode = "RecomputedWithRevision", generation = null) {
        return this.run(async (b) => {
          if (mode === "AsKnown") await I(generation);
          const ready = await this.ready(b);
          const candidate = await this.collect(
            { identity: smallIdentity(ready.cut), mode, generation },
            b,
          );
          candidate.ready = ready;
          return this.commit(candidate, b, { kind: "snapshot" });
        });
      }
      async load(
        discovery,
        mode = "RecomputedWithRevision",
        generation = null,
      ) {
        const discoveredCut = discovery?.cut;
        discovery = {
          schema_revision: discovery?.schema_revision,
          cut: discoveredCut
            ? Object.freeze(smallIdentity(discoveredCut))
            : null,
        };
        return this.run(async (b) => {
          if (mode === "AsKnown") await I(generation);
          need(
            discovery?.schema_revision === "s-session/3" && discovery?.cut,
            "需要显式Q3 Ready身份",
          );
          const identity = smallIdentity(discovery.cut);
          await T(identity.session_id);
          await I(identity.session_generation, true);
          return this.commit(
            await this.collect({ identity, mode, generation }, b),
            b,
            { kind: "snapshot" },
          );
        });
      }
      async applyBatch(candidate, delta, b) {
        const inner = await validateDelta(delta, () => b.job.chargeRecords(1n));
        if (this.hooks.validateDelta) await this.hooks.validateDelta(delta);
        this.check(b);
        const maps = {};
        for (const family of FAMILIES) {
          const map = new Map();
          for (const r of candidate.families[family]) {
            b.job.chargeRecords(1n);
            map.set(api.canonical(await rowKey(family, r)), r);
          }
          maps[family] = map;
        }
        for (const name of CHANGES) {
          const unique = new Set();
          for await (const r of (await f(inner, name)).items()) {
            b.job.chargeRecords(1n);
            this.check(b);
            const family = {
                upserts: "objects",
                withdrawals: "objects",
                witnesses: "witnesses",
                relations: "relations",
                observations: "observations",
                raw_history_added: "raw_history",
              }[name],
              key =
                name === "replaces"
                  ? api.canonical([
                      await t(r, "old_object_id"),
                      await t(r, "new_object_id"),
                    ])
                  : api.canonical(await rowKey(family, r));
            need(!unique.has(key), "Delta变化集合重复身份：" + name);
            unique.add(key);
          }
        }
        for await (const w of (await f(inner, "withdrawals")).items()) {
          const key = api.canonical([await t(w, "object_id")]),
            prior = maps.objects.get(key);
          need(prior, "撤回不存在于已提交活动对象");
          for (const name of (await w.has("fact_key"))
            ? ["fact_key"]
            : ["window_start", "window_mid", "window_end"])
            need(
              await equal(await f(w, name), await f(prior, name)),
              "撤回语义槽/窗口与原活动对象不同",
            );
          const overlay = {};
          for await (const e of prior.entries()) overlay[e.key] = e.value;
          overlay.lifecycle = "withdrawn";
          overlay.withdrawal_reason = await f(w, "reason");
          overlay.superseded_by = await f(w, "superseded_by");
          overlay.withdrawn_generation = await f(inner, "generation");
          maps.withdrawn_objects.set(key, of(overlay));
          maps.objects.delete(key);
        }
        for await (const r of (await f(inner, "upserts")).items()) {
          need(
            (await s(r, "lifecycle")) === "active",
            "Delta upsert生命周期不同",
          );
          const key = api.canonical(await rowKey("objects", r));
          maps.objects.set(key, r);
          maps.withdrawn_objects.delete(key);
        }
        for (const [name, family] of [
          ["witnesses", "witnesses"],
          ["relations", "relations"],
          ["observations", "observations"],
          ["raw_history_added", "raw_history"],
        ])
          for await (const r of (await f(inner, name)).items())
            maps[family].set(api.canonical(await rowKey(family, r)), r);
        const families = {};
        for (const family of FAMILIES) {
          const ordered = [];
          for (const r of maps[family].values())
            ordered.push({ r, tuple: await tuple(family, r) });
          ordered.sort((a, b) => compareTuples(a.tuple, b.tuple));
          families[family] = ordered.map((x) => x.r);
        }
        await axesValidate(
          await f(await f(inner, "catalog_evidence"), "axes"),
          families.objects,
          await s(await f(inner, "catalog_evidence"), "rule_revision"),
        );
        return families;
      }
      async watch() {
        return this.run(async (b) => {
          need(this.committed, "尚未安装完整Snapshot");
          const before = this.committed;
          for (const node of before.reader.cache.values()) {
            this.check(b);
            await b.job.acceptNode(node);
            this.check(b);
          }
          const reply = await this.request(
              before.cursor,
              {
                op: "watch",
                cursor: before.cursor,
                max_batches: String(this.limits.watchBatches),
                client_id: this.instance,
              },
              b,
            ),
            page = reply.payload;
          await keys(
            page,
            [
              "deltas",
              "next_cursor",
              "observed_head",
              "delivery_frontier",
              "has_more",
              "gap",
              "validated_capture_digest",
              "ok",
            ],
            "Watch",
          );
          await this.fixed(page.observed_head);
          digest(page.validated_capture_digest);
          need(
            sameIdentity(reply, page.observed_head) &&
              page.observed_head.history_mode === "RecomputedWithRevision",
            "Watch观测头身份/模式不同",
          );
          await keys(
            page.delivery_frontier,
            [
              "policy_revision",
              "retain_generations",
              "first_available_generation",
              "head_generation",
            ],
            "投递前沿",
          );
          await T(page.delivery_frontier.policy_revision);
          await I(page.delivery_frontier.retain_generations, true);
          await I(page.delivery_frontier.first_available_generation, true);
          await I(page.delivery_frontier.head_generation);
          need(
            page.delivery_frontier.head_generation ===
              page.observed_head.cut_generation &&
              BigInt(page.delivery_frontier.first_available_generation) <=
                BigInt(page.delivery_frontier.head_generation) + 1n,
            "投递前沿/head不同",
          );
          need(
            Array.isArray(page.deltas) &&
              page.deltas.length <= this.limits.watchBatches &&
              typeof page.has_more === "boolean",
            "Watch批次数错误",
          );
          await keys(page.next_cursor, CURSOR, "cursor");
          if (page.gap !== null) {
            const gap = page.gap;
            await keys(
              gap,
              [
                "requested_identity",
                "reason",
                "missing_range",
                "rebuild",
                "snapshot_token",
              ],
              "Gap",
            );
            need(
              (await equal(gap.requested_identity, before.cursor)) &&
                (await equal(page.next_cursor, before.cursor)) &&
                page.deltas.length === 0 &&
                !page.has_more,
              "Gap提前推进cursor",
            );
            need(
              await equal(gap.rebuild, page.observed_head),
              "Gap rebuild/observed不同",
            );
            const changed = !sameIdentity(before.cursor, gap.rebuild);
            if (gap.reason === "session_identity_changed")
              need(changed && gap.missing_range === null, "会话Gap范围不同");
            else {
              need(
                !changed &&
                  [
                    "delivery_retention_gap",
                    "client_backlog_overflow",
                  ].includes(gap.reason),
                "Gap reason错误",
              );
              await keys(
                gap.missing_range,
                ["from_generation", "to_generation"],
                "missing_range",
              );
              const to =
                gap.reason === "delivery_retention_gap"
                  ? BigInt(page.delivery_frontier.first_available_generation) -
                    1n
                  : BigInt(gap.rebuild.cut_generation);
              need(
                gap.missing_range.from_generation ===
                  (BigInt(before.cursor.after_generation) + 1n).toString() &&
                  gap.missing_range.to_generation === to.toString() &&
                  BigInt(gap.missing_range.from_generation) <= to,
                "Gap缺口不同",
              );
            }
            return this.commit(
              await this.collect(
                {
                  identity: smallIdentity(gap.rebuild),
                  mode: gap.rebuild.history_mode,
                  generation: gap.rebuild.cut_generation,
                  token: gap.snapshot_token,
                  expectedFixed: gap.rebuild,
                },
                b,
              ),
              b,
              { kind: "gap", reason: gap.reason },
            );
          }
          need(sameIdentity(reply, before.cursor), "无Gap不准换会话");
          let candidate = before;
          for (const wire of page.deltas) {
            await keys(wire, [...D, "delta"], "Delta wire");
            content.validateRef(wire.delta);
            content.validateRef(wire.catalog_evidence);
            need(
              wire.generation ===
                (BigInt(candidate.cursor.after_generation) + 1n).toString() &&
                wire.base_cut === candidate.cursor.base_cut &&
                wire.session_id === before.cursor.session_id &&
                wire.catalog_revision === before.cursor.catalog_revision &&
                wire.seq_range.from ===
                  (BigInt(candidate.cursor.last_seq) + 1n).toString(),
              "Delta断代/会话/cut/前沿",
            );
            const anchor = {
                kind: "delta",
                ...smallIdentity(before.cursor),
                generation: wire.generation,
                base_cut: wire.base_cut,
                next_cut: wire.next_cut,
                index_frontier: wire.index_frontier,
                delta_ref: wire.delta,
              },
              reader = this.reader(anchor, before.cursor, b),
              inner = reader.value(wire.delta);
            await reader.complete(inner);
            await reader.finish();
            const outer = {};
            for (const key of D)
              outer[key] =
                key === "catalog_evidence"
                  ? reader.value(wire.catalog_evidence, [
                      { field: "catalog_evidence" },
                    ])
                  : wire[key];
            outer.delta = inner;
            const delta = of(outer),
              applied = await this.applyBatch(candidate, delta, b);
            candidate = await this.collect(
              {
                identity: before.cursor,
                mode: "RecomputedWithRevision",
                generation: wire.generation,
              },
              b,
            );
            await comparePublication(
              delta,
              await candidate.state.field("snapshot"),
            );
            if (this.hooks.comparePublication)
              await this.hooks.comparePublication(
                delta,
                await candidate.state.field("snapshot"),
              );
            for (const family of FAMILIES) {
              need(
                applied[family].length === candidate.families[family].length,
                "Delta/Snapshot六族数量不同",
              );
              for (let index = 0; index < applied[family].length; index++) {
                b.job.chargeRecords(2n);
                this.check(b);
                need(
                  await equal(
                    applied[family][index],
                    candidate.families[family][index],
                  ),
                  "Delta/Snapshot全部字段/原顺序不同：" + family,
                );
              }
            }
          }
          if (page.deltas.length === 0) {
            candidate = await this.collect(
              {
                identity: before.cursor,
                mode: before.fixed.history_mode,
                generation: before.cursor.after_generation,
              },
              b,
            );
            for (const family of FAMILIES)
              b.job.chargeRecords(
                BigInt(
                  before.families[family].length +
                    candidate.families[family].length,
                ),
              );
            need(
              await equal(candidate.projection, before.projection),
              "空Delta完整projection改变",
            );
          }
          need(
            await equal(page.next_cursor, candidate.cursor),
            "next_cursor不是末批完整身份",
          );
          const head = BigInt(page.observed_head.cut_generation),
            end = BigInt(candidate.cursor.after_generation);
          need(
            head >= end &&
              page.has_more === head > end &&
              (page.deltas.length > 0 || head === end),
            "Watch head/has_more/空批不同",
          );
          if (head === end)
            need(
              await equal(page.observed_head, {
                ...candidate.fixed,
                history_mode: "RecomputedWithRevision",
                as_of_generation: null,
              }),
              "Watch head不是同cut原完整身份",
            );
          return this.commit(candidate, b, {
            kind: "watch",
            batches: page.deltas.length,
            hasMore: page.has_more,
          });
        });
      }
    }
    return Object.freeze({
      Client,
      validateState: validateStateOnce,
      validateTB02Catalog: (catalog) => catalogValidate(catalog, null),
      validateDelta,
      comparePublication,
      rowKey,
      tuple,
      axesValidate,
      rawValidate,
      bindSources,
      auditWitnesses,
      completeValue,
      typed,
      V,
    });
  };
});

}
/* #1394 generated browser dependencies end */
/* #1372：Q v2 严格只读消费者。暂存完整分页/批次，全部通过才安装 cut 与 cursor。 */
(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.TB01C = api;
})(globalThis, function () {
  "use strict";
  const FAMILIES = [
    "objects",
    "withdrawn_objects",
    "witnesses",
    "relations",
    "observations",
    "raw_history",
  ];
  const ORDER = "s-record-order/1",
    TOKEN_SCHEMA = "s-snapshot-token/1";
  const SCOPE = { structure: "CompleteCut", economic: "not_started" };
  const HEADER = [
    "schema_revision",
    "session_id",
    "session_generation",
    "source_namespace",
    "source_epoch",
    "message_id",
    "producer_id",
    "producer_epoch",
    "payload_hash",
    "causal_refs",
    "payload",
  ];
  const FIXED = [
    "session_id",
    "session_generation",
    "structure_cut",
    "cut_generation",
    "catalog_revision",
    "index_frontier",
    "profile_id",
    "profile_hash",
    "input_frontier",
    "seq_range",
    "catalog_run_status",
    "catalog_evidence",
    "scope",
    "rule_revision",
    "history_mode",
    "as_of_generation",
  ];
  const TOKEN = [
    "token_schema",
    "session_id",
    "session_generation",
    "cut_generation",
    "scope",
    "history_mode",
    "structure_cut",
    "catalog_revision",
    "index_frontier",
    "profile_hash",
    "rule_revision",
    "order_version",
    "page_size",
    "offset",
    "cut_projection_digest",
    "issued_capture_digest",
    "token_checksum",
  ];
  const CURSOR = [
    "session_id",
    "session_generation",
    "catalog_revision",
    "scope",
    "after_generation",
    "base_cut",
    "index_frontier",
    "last_seq",
    "order_version",
  ];
  const PAGE = [
    "fixed_cut",
    "catalog",
    "rows",
    "counts",
    "scope",
    "omissions",
    "order_version",
    "offset",
    "done",
    "next_token",
    "cut_projection_digest",
    "validated_capture_digest",
    "ok",
  ];
  const WATCH = [
    "deltas",
    "next_cursor",
    "observed_head",
    "delivery_frontier",
    "has_more",
    "gap",
    "validated_capture_digest",
    "ok",
  ];
  const DEFAULTS = Object.freeze({
    pageSize: 31,
    watchBatches: 4,
    pollMs: 500,
    timeoutMs: 30000,
    replyBytes: 8 * 1024 * 1024,
    totalBytes: 64 * 1024 * 1024,
    maxRows: 100000,
    maxPages: 4096,
  });
  const encoder = new TextEncoder();
  const object = (v) =>
    v !== null && typeof v === "object" && !Array.isArray(v);
  function need(condition, message) {
    if (!condition) throw new Error(message);
  }
  function keys(value, expected, name) {
    need(
      object(value) &&
        Object.keys(value).length === expected.length &&
        expected.every((k) => Object.hasOwn(value, k)),
      name + " 完整字段不符",
    );
  }
  function scalar(value) {
    need(typeof value === "string", "要求 UTF-8 文本");
    for (let i = 0; i < value.length; i++) {
      const code = value.charCodeAt(i);
      if (code >= 0xd800 && code <= 0xdbff) {
        const next = value.charCodeAt(++i);
        need(next >= 0xdc00 && next <= 0xdfff, "孤立 Unicode 高代理项");
      } else need(code < 0xdc00 || code > 0xdfff, "孤立 Unicode 低代理项");
    }
    return value;
  }
  function text(value) {
    scalar(value);
    need(value.length > 0, "身份文本为空");
    return value;
  }
  function integer(value, positive = false, frontier = false) {
    need(
      typeof value === "string" &&
        value.length <= 19 &&
        /^(?:0|[1-9][0-9]*|-1)$/.test(value),
      "精确整数必须是规范文本",
    );
    const n = BigInt(value);
    need(
      n >= (frontier ? -1n : positive ? 1n : 0n) && n <= 9223372036854775807n,
      "精确整数超出 i64 域",
    );
    return n;
  }
  function digest(value) {
    need(
      typeof value === "string" && /^[a-f0-9]{64}$/.test(value),
      "SHA-256 文本不规范",
    );
  }
  function utf8Compare(a, b) {
    // UTF-8字典序与合法Unicode标量序相同；先完整验证两参数，保留首错顺序。
    scalar(a);
    scalar(b);
    let ai = 0,
      bi = 0;
    while (ai < a.length && bi < b.length) {
      const aa = a.codePointAt(ai),
        bb = b.codePointAt(bi);
      if (aa !== bb) return aa < bb ? -1 : 1;
      ai += aa > 0xffff ? 2 : 1;
      bi += bb > 0xffff ? 2 : 1;
    }
    return ai < a.length ? 1 : bi < b.length ? -1 : 0;
  }
  function canonical(value) {
    if (value === null || typeof value === "boolean")
      return JSON.stringify(value);
    if (typeof value === "string") return JSON.stringify(scalar(value));
    if (Array.isArray(value)) return "[" + value.map(canonical).join(",") + "]";
    need(object(value), "v2 JSON 数值必须在源头投影为精确文本");
    return (
      "{" +
      Object.keys(value)
        .sort(utf8Compare)
        .map((k) => JSON.stringify(scalar(k)) + ":" + canonical(value[k]))
        .join(",") +
      "}"
    );
  }
  const equal = (a, b) => canonical(a) === canonical(b);
  // #1414：保留 canonical→parse 的严格值域与新树语义，避免中间规范字符串。
  // 对象形状同本文件 parse（普通对象、键按 UTF-8 序逐个 defineProperty），不是 null 原型。
  function clone(value, depth = 0) {
    need(depth <= 128, "JSON 嵌套超过浏览器界限");
    if (value === null || typeof value === "boolean") return value;
    if (typeof value === "string") return scalar(value);
    if (Array.isArray(value)) {
      const out = [],
        length = value.length;
      // 原 map/join 将唯一空槽编码为 []；其余含空槽数组由 parse 拒绝。
      if (length === 1 && !(0 in value)) return out;
      for (let i = 0; i < length; i++) {
        need(i in value, "JSON 非法值；v2 整数必须使用精确文本");
        out.push(clone(value[i], depth + 1));
      }
      return out;
    }
    need(object(value), "v2 JSON 数值必须在源头投影为精确文本");
    const out = {};
    for (const key of Object.keys(value).sort(utf8Compare))
      Object.defineProperty(out, scalar(key), {
        value: clone(value[key], depth + 1),
        enumerable: true,
        writable: true,
        configurable: true,
      });
    return out;
  }
  // JSON.parse 会丢失重复键和整数精度；只有单个字符串 token 借用其转义解码。
  function parse(source) {
    let p = 0;
    function whitespace() {
      while (/[ \t\r\n]/.test(source[p] || "x")) p++;
    }
    function string() {
      const start = p++;
      while (p < source.length) {
        const ch = source[p++];
        if (ch === "\\") {
          p++;
          continue;
        }
        if (ch === '"') return scalar(JSON.parse(source.slice(start, p)));
      }
      throw new Error("JSON 字符串截断");
    }
    function value(depth) {
      need(depth <= 128, "JSON 嵌套超过浏览器界限");
      whitespace();
      const ch = source[p];
      if (ch === '"') return string();
      if (ch === "{") {
        p++;
        const out = {};
        whitespace();
        if (source[p] === "}") {
          p++;
          return out;
        }
        while (true) {
          whitespace();
          need(source[p] === '"', "JSON 对象键无效");
          const k = string();
          need(!Object.hasOwn(out, k), "重复 JSON 键：" + k);
          whitespace();
          need(source[p++] === ":", "JSON 缺少冒号");
          Object.defineProperty(out, k, {
            value: value(depth + 1),
            enumerable: true,
            writable: true,
            configurable: true,
          });
          whitespace();
          const end = source[p++];
          if (end === "}") return out;
          need(end === ",", "JSON 对象截断或分隔错误");
        }
      }
      if (ch === "[") {
        p++;
        const out = [];
        whitespace();
        if (source[p] === "]") {
          p++;
          return out;
        }
        while (true) {
          out.push(value(depth + 1));
          whitespace();
          const end = source[p++];
          if (end === "]") return out;
          need(end === ",", "JSON 数组截断或分隔错误");
        }
      }
      for (const [word, result] of [
        ["true", true],
        ["false", false],
        ["null", null],
      ]) {
        if (source.slice(p, p + word.length) === word) {
          p += word.length;
          return result;
        }
      }
      throw new Error("JSON 非法值；v2 整数必须使用精确文本");
    }
    const result = value(0);
    whitespace();
    need(p === source.length, "JSON 尾部额外内容");
    return result;
  }
  async function sha256(value) {
    return sha256Text(canonical(value));
  }
  async function sha256Text(source) {
    const crypto =
      globalThis.crypto ||
      (typeof require === "function" ? require("node:crypto").webcrypto : null);
    need(crypto && crypto.subtle, "当前环境缺少 SHA-256，无法验证");
    const bytes = await crypto.subtle.digest("SHA-256", encoder.encode(source));
    return Array.from(new Uint8Array(bytes), (b) =>
      b.toString(16).padStart(2, "0"),
    ).join("");
  }
  // 由 signed-catalog.json 的全部公共静态列生成；对应漂移锁逐值重算，不是新分类语义。
  const TB02_CATALOG = Object.freeze({
    catalog_revision:
      "s2-axis-quantifiers (SPEC-COVERAGE-INPUT.json sha256=76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c)",
    source_sha256:
      "76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c",
    ids: [
      "CC-001",
      "CC-002",
      "CC-003",
      "CC-004",
      "CC-005",
      "CC-006",
      "CC-007",
      "CC-008",
      "CC-009",
      "CC-010",
      "CC-011",
      "CC-012",
      "CC-013",
      "CC-014",
      "CC-015",
      "CC-016",
      "CC-017",
      "CC-018",
      "CC-019",
      "CC-020",
      "CC-021",
      "CC-022",
      "CC-023",
      "CC-024",
      "CC-025",
      "CC-026",
      "CC-027",
      "CC-028",
      "CC-029",
      "CC-030",
      "CC-031",
      "CC-032",
      "CC-033",
      "CC-034",
      "CC-035",
      "CC-036",
      "CC-037",
      "CC-038",
      "CC-039",
      "CC-040",
      "CC-041",
      "CC-042",
      "CC-043",
      "CC-044",
      "CC-045",
      "CC-046",
      "CC-047",
      "CC-048",
      "CC-049",
      "CC-050",
      "CC-051",
      "CC-052",
      "CC-053",
      "CC-054",
      "CC-055",
      "CC-056",
      "CC-057",
      "CC-058",
      "CC-059",
      "CC-060",
      "CC-061",
      "CC-062",
      "LC-01",
      "LC-02",
      "LC-03",
      "LC-04",
      "LC-05",
      "LC-06",
      "LC-07",
      "LC-08",
      "LC-09",
      "LC-10",
      "ST-001",
      "ST-002",
      "ST-003",
      "ST-004",
      "ST-005",
      "ST-006",
      "ST-007",
      "ST-008",
      "ST-009",
      "ST-010",
      "ST-011",
      "ST-012",
      "ST-013",
      "ST-014",
      "ST-015",
      "ST-016",
      "ST-017",
      "ST-018",
      "ST-019",
      "ST-020",
      "ST-021",
      "ST-022",
      "ST-023",
      "ST-024",
      "ST-025",
      "ST-026",
      "ST-027",
      "ST-028",
      "ST-029",
      "ST-030",
      "ST-031",
      "ST-032",
      "ST-033",
      "ST-034",
      "ST-035",
      "ST-036",
      "ST-037",
      "ST-038",
      "ST-043",
      "ST-044",
      "ST-045",
      "ST-046",
      "ST-047",
      "ST-048",
    ],
    public_static_sha256:
      "974299745db38c9b83de3ccbd1e5fc8aab93d58037fd6ef5c1667509d6da8de2",
  });
  function validateFixed(fixed) {
    keys(fixed, FIXED, "fixed_cut");
    text(fixed.session_id);
    integer(fixed.session_generation, true);
    const gen = integer(fixed.cut_generation);
    integer(fixed.input_frontier, false, true);
    text(fixed.catalog_revision);
    text(fixed.rule_revision);
    scalar(fixed.index_frontier);
    scalar(fixed.profile_id);
    scalar(fixed.profile_hash);
    need(
      fixed.structure_cut === "cut-" + fixed.cut_generation &&
        equal(fixed.scope, SCOPE),
      "固定 cut/scope 不符",
    );
    keys(fixed.seq_range, ["from", "to"], "seq_range");
    integer(fixed.seq_range.from);
    integer(fixed.seq_range.to, false, true);
    need(fixed.seq_range.to === fixed.input_frontier, "固定 cut 输入前沿不符");
    need(
      fixed.history_mode === "AsKnown"
        ? fixed.as_of_generation === fixed.cut_generation
        : fixed.history_mode === "RecomputedWithRevision" &&
            fixed.as_of_generation === null,
      "历史模式与指定 cut 不符",
    );
    if (gen > 0n)
      need(
        object(fixed.catalog_evidence) &&
          fixed.catalog_evidence.rule_revision === fixed.rule_revision,
        "固定 cut 规则修订与目录证据不符",
      );
  }
  function parseDiscovery(value) {
    need(object(value) && value.ok === true, "发现回执未成功");
    if (value.protocol_revision === null) {
      keys(value, ["ok", "protocol_revision", "producer_epoch"], "legacy发现");
      integer(value.producer_epoch, true);
      return clone(value);
    }
    keys(
      value,
      ["ok", "protocol_revision", "cut", "snapshot", "producer_epoch"],
      "v2发现",
    );
    need(value.protocol_revision === "s-session/2", "发现协议不支持");
    keys(value.cut, ["session_id", "session_generation"], "发现会话");
    keys(value.snapshot, ["profile_id"], "发现profile");
    text(value.cut.session_id);
    integer(value.cut.session_generation, true);
    integer(value.producer_epoch, true);
    need(
      ["testonly_tick_1_1_ohlc", "ohlc_integer_tb02a_v1"].includes(
        value.snapshot.profile_id,
      ),
      "发现profile不支持",
    );
    return clone(value);
  }
  async function readDiscovery(fetcher) {
    // 与后续load共享绝对期限及已耗字节；不为身份发现另开可叠加预算。
    const budget = { deadline: performance.now() + DEFAULTS.timeoutMs, bytes: 0 };
    const abort = new AbortController(),
      timer = setTimeout(() => abort.abort(), DEFAULTS.timeoutMs);
    let reader;
    try {
      const response = await fetcher("/api/discover", {
        signal: abort.signal,
        cache: "no-store",
      });
      need(response.ok, "发现请求不是成功 HTTP 响应");
      need(
        response.body && typeof response.body.getReader === "function",
        "缺少发现响应字节流",
      );
      reader = response.body.getReader();
      const chunks = [];
      while (true) {
        need(performance.now() < budget.deadline, "发现超过本次读取总期限");
        const { done, value } = await reader.read();
        need(performance.now() < budget.deadline, "发现超过本次读取总期限");
        if (done) break;
        budget.bytes += value.byteLength;
        need(
          budget.bytes <= DEFAULTS.replyBytes &&
            budget.bytes <= DEFAULTS.totalBytes,
          "发现响应达到字节界限",
        );
        chunks.push(value);
      }
      const bytes = new Uint8Array(budget.bytes);
      let offset = 0;
      for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.byteLength;
      }
      return {
        discovery: parseDiscovery(
          parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
        ),
        budget,
      };
    } catch (error) {
      if (reader) await reader.cancel().catch(() => {});
      throw error;
    } finally {
      if (reader) reader.releaseLock();
      clearTimeout(timer);
    }
  }
  function orderFor(profileId) {
    return profileId === "ohlc_integer_tb02a_v1" ? "s-record-order/2" : ORDER;
  }
  function cursorOf(fixed) {
    return {
      session_id: fixed.session_id,
      session_generation: fixed.session_generation,
      catalog_revision: fixed.catalog_revision,
      scope: clone(fixed.scope),
      after_generation: fixed.cut_generation,
      base_cut: fixed.structure_cut,
      index_frontier: fixed.index_frontier,
      last_seq: fixed.input_frontier,
      order_version: orderFor(fixed.profile_id),
    };
  }
  function identityOf(fixed) {
    return {
      session_id: fixed.session_id,
      session_generation: fixed.session_generation,
    };
  }
  function sameIdentity(a, b) {
    return (
      a.session_id === b.session_id &&
      a.session_generation === b.session_generation
    );
  }
  function rowKey(family, record) {
    const fields =
      family === "objects" || family === "withdrawn_objects"
        ? ["object_id"]
        : family === "relations"
          ? ["subject", "relation_type", "object"]
          : family === "raw_history"
            ? ["identity_key", "revision"]
            : [family === "witnesses" ? "witness_id" : "observation_id"];
    return fields.map((k) => text(record[k]));
  }
  function orderTuple(family, r) {
    const n = (k) => integer(r[k]),
      s = (k) => text(r[k]);
    switch (family) {
      case "objects":
        return Object.hasOwn(r, "fact_key")
          ? [1n, s("kind"), s("fact_key"), s("object_id"), n("object_revision")]
          : [
              0n,
              n("window_start"),
              n("window_mid"),
              n("window_end"),
              s("object_id"),
              n("object_revision"),
            ];
      case "withdrawn_objects":
        return [
          n("withdrawn_generation"),
          s("object_id"),
          n("object_revision"),
        ];
      case "witnesses":
        return [s("object_id"), n("slot"), s("witness_id")];
      case "relations":
        return [s("subject"), s("relation_type"), s("object")];
      case "observations":
        return [
          s("kind"),
          r.window_start === null ? -1n : n("window_start"),
          s("observation_id"),
        ];
      case "raw_history":
        return [n("seq"), s("identity_key"), n("revision")];
      default:
        throw new Error("未知记录族");
    }
  }
  function compareRows(a, b) {
    const family = FAMILIES.indexOf(a.family) - FAMILIES.indexOf(b.family);
    if (family) return family;
    const aa = orderTuple(a.family, a.record),
      bb = orderTuple(b.family, b.record);
    for (let i = 0; i < aa.length; i++) {
      const cmp =
        typeof aa[i] === "bigint"
          ? aa[i] < bb[i]
            ? -1
            : aa[i] > bb[i]
              ? 1
              : 0
          : utf8Compare(aa[i], bb[i]);
      if (cmp) return cmp;
    }
    return 0;
  }
  function rowsOf(snapshot) {
    return FAMILIES.flatMap((family) =>
      snapshot[family]
        .map((record) => ({ family, key: rowKey(family, record), record }))
        .sort(compareRows),
    );
  }
  function stateOf(projection) {
    const fixed = projection.fixed_cut,
      snapshot = { ...clone(fixed), generation: fixed.cut_generation };
    delete snapshot.cut_generation;
    for (const family of FAMILIES)
      snapshot[family] = projection.rows
        .filter((row) => row.family === family)
        .map((row) => clone(row.record));
    return {
      ok: true,
      catalog: clone(projection.catalog),
      snapshot,
      cut: {
        session_id: fixed.session_id,
        session_generation: fixed.session_generation,
        generation: fixed.cut_generation,
        structure_cut: fixed.structure_cut,
        catalog_revision: fixed.catalog_revision,
        index_frontier: fixed.index_frontier,
      },
    };
  }
  class Client {
    #committed = null;
    get committed() {
      return this.#committed;
    }
    constructor({ fetch: fetcher, hooks, onCommit = () => {}, limits = {} }) {
      const useDefaultFetch = fetcher === undefined;
      if (useDefaultFetch) fetcher = globalThis.fetch;
      need(typeof fetcher === "function", "浏览器缺少网络读取能力");
      for (const name of [
        "validateState",
        "validateDelta",
        "comparePublication",
      ])
        need(typeof hooks?.[name] === "function", "缺少严格校验 " + name);
      this.fetch = useDefaultFetch ? fetcher.bind(globalThis) : fetcher;
      this.hooks = hooks;
      this.onCommit = onCommit;
      this.limits = { ...DEFAULTS, ...limits };
      for (const value of Object.values(this.limits))
        need(Number.isSafeInteger(value) && value > 0, "浏览器资源界限无效");
      this.instance = globalThis.crypto?.randomUUID
        ? globalThis.crypto.randomUUID()
        : typeof require === "function"
          ? require("node:crypto").randomUUID()
          : null;
      need(this.instance, "无法建立独立查询身份");
      this.serial = 0n;
      this.operation = 0;
    }
    begin() {
      return {
        operation: ++this.operation,
        deadline: performance.now() + this.limits.timeoutMs,
        bytes: 0,
      };
    }
    check(budget) {
      need(
        budget.operation === this.operation,
        "已被较新读取取代，旧响应不提交",
      );
      need(
        performance.now() < budget.deadline,
        "本次读取超过声明总期限，未验证",
      );
    }
    cancel() {
      this.operation++;
    }
    async request(identity, payload, budget) {
      this.check(budget);
      text(identity.session_id);
      integer(identity.session_generation, true);
      const request = {
        schema_revision: "s-session/2",
        ...identityOf(identity),
        source_namespace: "s-observe/browser",
        source_epoch: this.instance,
        message_id: "query-" + (++this.serial).toString(),
        producer_id: "browser:" + this.instance,
        producer_epoch: "1",
        payload_hash: await sha256(payload),
        causal_refs: [],
        payload,
      };
      this.check(budget);
      const abort = new AbortController(),
        timer = setTimeout(
          () => abort.abort(),
          Math.max(1, budget.deadline - performance.now()),
        );
      try {
        const response = await this.fetch("/api/v2/" + payload.op, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: canonical(request),
          signal: abort.signal,
          cache: "no-store",
        });
        need(
          response.body && typeof response.body.getReader === "function",
          "缺少有限响应字节流",
        );
        const reader = response.body.getReader(),
          chunks = [];
        let length = 0;
        try {
          while (true) {
            this.check(budget);
            const { done, value } = await reader.read();
            this.check(budget);
            if (done) break;
            length += value.byteLength;
            budget.bytes += value.byteLength;
            need(
              length <= this.limits.replyBytes &&
                budget.bytes <= this.limits.totalBytes,
              "读取达到浏览器字节界限，未验证",
            );
            chunks.push(value);
          }
        } catch (error) {
          await reader.cancel().catch(() => {});
          throw error;
        } finally {
          reader.releaseLock();
        }
        const bytes = new Uint8Array(length);
        let offset = 0;
        for (const chunk of chunks) {
          bytes.set(chunk, offset);
          offset += chunk.byteLength;
        }
        const reply = parse(
          new TextDecoder("utf-8", { fatal: true }).decode(bytes),
        );
        keys(reply, HEADER, "响应公共头");
        need(reply.schema_revision === "s-session/2", "响应协议版本错误");
        for (const k of HEADER.filter(
          (k) => !["causal_refs", "payload"].includes(k),
        ))
          text(reply[k]);
        integer(reply.session_generation, true);
        integer(reply.producer_epoch, true);
        digest(reply.payload_hash);
        need(
          reply.source_namespace === "s-observe/replies" &&
            reply.source_epoch === reply.producer_epoch &&
            reply.producer_id === "Q:" + reply.session_id &&
            object(reply.payload),
          "响应生产者/来源身份错误",
        );
        const causal = Object.fromEntries(
          [
            "source_namespace",
            "source_epoch",
            "message_id",
            "payload_hash",
          ].map((k) => [k, request[k]]),
        );
        need(equal(reply.causal_refs, [causal]), "响应因果身份不属于本次请求");
        need(
          (await sha256(reply.payload)) === reply.payload_hash,
          "响应 payload SHA-256 不符",
        );
        need(
          reply.message_id ===
            "reply-" +
              (await sha256([
                request.source_namespace,
                request.source_epoch,
                request.message_id,
                reply.payload_hash,
              ])),
          "响应消息身份不符",
        );
        this.check(budget);
        need(
          response.ok && reply.payload.ok === true,
          "查询未完成：" + (reply.payload.error || response.status),
        );
        return reply;
      } finally {
        clearTimeout(timer);
      }
    }
    async validateToken(token, fixed, projectionDigest, offset) {
      keys(token, TOKEN, "分页 token");
      need(
        token.token_schema === TOKEN_SCHEMA &&
          token.order_version === orderFor(fixed.profile_id) &&
          token.page_size === String(this.limits.pageSize) &&
          token.offset === String(offset),
        "分页 token 页序/资源/schema 不符",
      );
      integer(token.offset);
      integer(token.page_size, true);
      digest(token.issued_capture_digest);
      digest(token.token_checksum);
      for (const key of [
        "session_id",
        "session_generation",
        "cut_generation",
        "scope",
        "history_mode",
        "structure_cut",
        "catalog_revision",
        "index_frontier",
        "profile_hash",
        "rule_revision",
      ])
        need(equal(token[key], fixed[key]), "分页 token 固定身份不符：" + key);
      need(
        token.cut_projection_digest === projectionDigest,
        "分页 token 投影摘要变化",
      );
      const unsigned = { ...token };
      delete unsigned.token_checksum;
      need(
        (await sha256(unsigned)) === token.token_checksum,
        "分页 token checksum 不符",
      );
    }
    async collect(
      {
        identity,
        mode,
        generation = null,
        token = null,
        expectedFixed = null,
        expectedProfile = null,
      },
      budget,
    ) {
      const expectedOrder = expectedFixed
        ? orderFor(expectedFixed.profile_id)
        : identity.order_version || this.requestOrder || ORDER;
      const payload = token
        ? { op: "snapshot", snapshot_token: token }
        : {
            op: "snapshot",
            ...identityOf(identity),
            scope: SCOPE,
            history_mode: mode,
            as_of_generation: generation,
            page_size: String(this.limits.pageSize),
            order_version: expectedOrder,
          };
      if (generation !== null) integer(generation);
      if (token) {
        validateFixed(expectedFixed);
        await this.validateToken(
          token,
          expectedFixed,
          token.cut_projection_digest,
          0,
        );
      }
      const rows = [],
        seen = new Set(),
        objectIds = new Set();
      let base = null,
        baseCanonical = null,
        next = payload,
        issued = token?.issued_capture_digest;
      for (let pageNumber = 0; ; pageNumber++) {
        need(pageNumber < this.limits.maxPages, "分页数达到浏览器界限，未验证");
        const envelope = await this.request(identity, next, budget),
          page = envelope.payload;
        keys(page, PAGE, "Snapshot 页");
        validateFixed(page.fixed_cut);
        need(
          sameIdentity(envelope, identity) &&
            sameIdentity(page.fixed_cut, identity),
          "Snapshot 响应会话/化身不符",
        );
        need(
          page.fixed_cut.history_mode === mode &&
            (generation === null ||
              page.fixed_cut.cut_generation === generation),
          "Snapshot 不属于请求的模式/cut",
        );
        if (expectedProfile !== null)
          need(
            page.fixed_cut.profile_id === expectedProfile,
            "Snapshot profile与发现不符",
          );
        if (expectedFixed)
          need(
            equal(page.fixed_cut, expectedFixed),
            "重建页不属于 Gap 声明的固定切面",
          );
        need(
          equal(page.scope, SCOPE) &&
            equal(page.omissions, []) &&
            page.order_version === expectedOrder &&
            page.order_version === orderFor(page.fixed_cut.profile_id),
          "Snapshot scope/omissions/order 不符",
        );
        digest(page.cut_projection_digest);
        digest(page.validated_capture_digest);
        keys(page.counts, [...FAMILIES, "total"], "六族完整计数");
        for (const count of Object.values(page.counts)) integer(count);
        need(
          FAMILIES.reduce((n, k) => n + BigInt(page.counts[k]), 0n) ===
            BigInt(page.counts.total) &&
            BigInt(page.counts.total) <= BigInt(this.limits.maxRows),
          "完整计数不符或达到浏览器行数界限",
        );
        need(
          page.offset === String(rows.length) &&
            typeof page.done === "boolean" &&
            Array.isArray(page.rows),
          "分页 offset/done/rows 无效",
        );
        const metadata = Object.fromEntries(
          [
            "fixed_cut",
            "catalog",
            "counts",
            "scope",
            "omissions",
            "order_version",
            "cut_projection_digest",
          ].map((k) => [k, page[k]]),
        );
        if (base) {
          // #1414：base是本次collect私有clone，分页内不写；仍逐页完整规范化当前metadata。
          baseCanonical ??= canonical(base);
          need(
            baseCanonical === canonical(metadata),
            "跨页固定切面/目录/计数/摘要变化",
          );
        } else {
          base = clone(metadata);
          issued ||= page.validated_capture_digest;
        }
        const rest = BigInt(page.counts.total) - BigInt(rows.length);
        const expectedLength =
          rest < BigInt(this.limits.pageSize)
            ? Number(rest)
            : this.limits.pageSize;
        need(
          rest >= 0n &&
            page.rows.length === expectedLength &&
            page.done === (rest === BigInt(page.rows.length)),
          "分页截断/提前结束/空转",
        );
        for (const row of page.rows) {
          keys(row, ["family", "key", "record"], "记录行");
          need(
            FAMILIES.includes(row.family) && object(row.record),
            "未知记录族/记录形状",
          );
          need(
            equal(row.key, rowKey(row.family, row.record)),
            "记录公开主键不符",
          );
          orderTuple(row.family, row.record);
          const id = canonical([row.family, row.key]);
          need(!seen.has(id), "分页重复公开身份");
          seen.add(id);
          if (["objects", "withdrawn_objects"].includes(row.family)) {
            need(!objectIds.has(row.key[0]), "活动/撤回身份冲突");
            objectIds.add(row.key[0]);
          }
          need(
            rows.length === 0 || compareRows(rows[rows.length - 1], row) < 0,
            "分页记录未按声明全序严格前进",
          );
          rows.push(clone(row));
        }
        if (page.done) {
          need(page.next_token === null, "末页仍返回 token");
          break;
        }
        await this.validateToken(
          page.next_token,
          base.fixed_cut,
          base.cut_projection_digest,
          rows.length,
        );
        need(
          page.next_token.issued_capture_digest === issued,
          "续页 token 签发来源改变",
        );
        next = { op: "snapshot", snapshot_token: clone(page.next_token) };
      }
      for (const family of FAMILIES)
        need(
          String(rows.filter((r) => r.family === family).length) ===
            base.counts[family],
          "六族完整计数不符：" + family,
        );
      const projection = { ...base, rows };
      delete projection.cut_projection_digest;
      need(
        (await sha256(projection)) === base.cut_projection_digest,
        "完整分页投影 SHA-256 不符",
      );
      const state = stateOf(projection);
      await this.hooks.validateState(state, "s-session/2");
      this.check(budget);
      if (base.fixed_cut.profile_id === "ohlc_integer_tb02a_v1") {
        await Q3.validateState(
          {
            ...state,
            snapshot: {
              ...state.snapshot,
              rule_revision: base.fixed_cut.rule_revision,
            },
          },
          "s-session/2",
          () => this.check(budget),
        );
      }
      return {
        state,
        projection,
        cursor: cursorOf(base.fixed_cut),
        pages: Math.max(1, Math.ceil(rows.length / this.limits.pageSize)),
      };
    }
    #commit(candidate, budget, event) {
      this.check(budget);
      this.#committed = candidate;
      this.onCommit(candidate, event);
      return candidate;
    }
    async load(
      discovery,
      mode = "RecomputedWithRevision",
      generation = null,
      discoveryBudget = null,
    ) {
      const budget = this.begin(); // 无效的新请求也使旧响应失效，不能事后覆盖错误状态。
      if (discoveryBudget !== null) {
        need(
          object(discoveryBudget) &&
            Number.isFinite(discoveryBudget.deadline) &&
            Number.isSafeInteger(discoveryBudget.bytes) &&
            discoveryBudget.bytes >= 0,
          "发现预算无效",
        );
        budget.deadline = Math.min(budget.deadline, discoveryBudget.deadline);
        budget.bytes = discoveryBudget.bytes;
        need(budget.bytes <= this.limits.totalBytes, "发现已达到总字节界限");
        this.check(budget);
      }
      discovery = parseDiscovery(discovery);
      need(
        discovery.protocol_revision === "s-session/2",
        "v2消费者拒绝legacy发现",
      );
      const identity = identityOf(discovery.cut);
      text(identity.session_id);
      integer(identity.session_generation, true);
      this.requestOrder = orderFor(discovery.snapshot.profile_id);
      need(
        mode === "AsKnown" || mode === "RecomputedWithRevision",
        "历史模式无效",
      );
      if (mode === "AsKnown") integer(generation);
      const candidate = await this.collect(
        {
          identity,
          mode,
          generation,
          expectedProfile: discovery.snapshot.profile_id,
        },
        budget,
      );
      return this.#commit(candidate, budget, { kind: "snapshot" });
    }
    async applyBatch(snapshot, delta) {
      keys(
        delta,
        [
          "generation",
          "session_id",
          "catalog_revision",
          "base_cut",
          "next_cut",
          "seq_range",
          "index_frontier",
          "input_frontier",
          "catalog_run_status",
          "catalog_evidence",
          "delta",
        ],
        "完整原子批次",
      );
      keys(
        delta.delta,
        [
          "generation",
          "session_id",
          "catalog_revision",
          "base_cut",
          "next_cut",
          "seq_range",
          "index_frontier",
          "input_frontier",
          "catalog_run_status",
          "catalog_evidence",
          "upserts",
          "withdrawals",
          "replaces",
          "witnesses",
          "relations",
          "observations",
          "raw_history_added",
        ],
        "完整原子批次载荷",
      );
      await this.hooks.validateDelta(delta);
      const result = clone(snapshot),
        dd = delta.delta;
      const maps = Object.fromEntries(
        FAMILIES.map((family) => [
          family,
          new Map(result[family].map((r) => [canonical(rowKey(family, r)), r])),
        ]),
      );
      for (const [field, family] of [
        ["upserts", "objects"],
        ["withdrawals", "objects"],
        ["witnesses", "witnesses"],
        ["relations", "relations"],
        ["observations", "observations"],
        ["raw_history_added", "raw_history"],
      ]) {
        const unique = new Set();
        for (const record of dd[field]) {
          const key = canonical(rowKey(family, record));
          need(!unique.has(key), "批次含重复身份：" + field);
          unique.add(key);
        }
      }
      for (const withdrawal of dd.withdrawals) {
        const key = canonical([withdrawal.object_id]),
          previous = maps.objects.get(key);
        need(previous, "撤回引用不属于已提交活动对象");
        maps.withdrawn_objects.set(key, {
          ...previous,
          lifecycle: "withdrawn",
          withdrawal_reason: withdrawal.reason,
          superseded_by: withdrawal.superseded_by,
          withdrawn_generation: delta.generation,
        });
        maps.objects.delete(key);
      }
      for (const record of dd.upserts) {
        const key = canonical(rowKey("objects", record));
        maps.objects.set(key, { ...clone(record), lifecycle: "active" });
        maps.withdrawn_objects.delete(key);
      }
      for (const [field, family] of [
        ["witnesses", "witnesses"],
        ["relations", "relations"],
        ["observations", "observations"],
        ["raw_history_added", "raw_history"],
      ])
        for (const record of dd[field])
          maps[family].set(canonical(rowKey(family, record)), clone(record));
      for (const family of FAMILIES)
        result[family] = [...maps[family].values()];
      // #1394：changes-only/1 的 Delta 只带本代变化，轴引用本代完整活动集；按应用后的活动对象核引用（与整份态同一份判据）。
      if (dd.catalog_evidence.profile_id === "ohlc_integer_tb02a_v1")
        await Q3.axesValidate(dd.catalog_evidence.axes, result.objects);
      return result;
    }
    async watch() {
      const budget = this.begin();
      need(this.committed, "尚未安装完整分页，不能 Watch");
      const before = this.committed;
      const reply = await this.request(
        before.cursor,
        {
          op: "watch",
          cursor: before.cursor,
          max_batches: String(this.limits.watchBatches),
          client_id: this.instance,
        },
        budget,
      );
      const page = reply.payload;
      keys(page, WATCH, "Watch");
      validateFixed(page.observed_head);
      digest(page.validated_capture_digest);
      need(
        page.observed_head.history_mode === "RecomputedWithRevision",
        "Watch observed_head 不是当前重算切面",
      );
      need(
        sameIdentity(reply, page.observed_head),
        "Watch 头与观测切面身份不符",
      );
      keys(
        page.delivery_frontier,
        [
          "policy_revision",
          "retain_generations",
          "first_available_generation",
          "head_generation",
        ],
        "投递前沿",
      );
      text(page.delivery_frontier.policy_revision);
      integer(page.delivery_frontier.retain_generations, true);
      integer(page.delivery_frontier.first_available_generation, true);
      integer(page.delivery_frontier.head_generation);
      need(
        page.delivery_frontier.head_generation ===
          page.observed_head.cut_generation &&
          BigInt(page.delivery_frontier.first_available_generation) <=
            BigInt(page.delivery_frontier.head_generation) + 1n,
        "投递前沿与观测 cut 不符",
      );
      need(
        Array.isArray(page.deltas) &&
          page.deltas.length <= this.limits.watchBatches &&
          typeof page.has_more === "boolean",
        "Watch 批次数/has_more 无效",
      );
      keys(page.next_cursor, CURSOR, "完整 Watch cursor");
      if (page.gap !== null) {
        const gap = page.gap;
        keys(
          gap,
          [
            "requested_identity",
            "reason",
            "missing_range",
            "rebuild",
            "snapshot_token",
          ],
          "Gap",
        );
        need(
          equal(gap.requested_identity, before.cursor) &&
            equal(page.next_cursor, before.cursor) &&
            page.deltas.length === 0 &&
            !page.has_more,
          "Gap 与请求身份/批次不符",
        );
        need(
          equal(gap.rebuild, page.observed_head),
          "Gap 重建切面与当前观测身份不符",
        );
        const changed = !sameIdentity(before.cursor, gap.rebuild);
        if (gap.reason === "session_identity_changed")
          need(
            changed && gap.missing_range === null,
            "会话 Gap 的身份/缺口范围不符",
          );
        else {
          need(
            !changed &&
              ["delivery_retention_gap", "client_backlog_overflow"].includes(
                gap.reason,
              ),
            "未知或错身份 Gap",
          );
          keys(
            gap.missing_range,
            ["from_generation", "to_generation"],
            "Gap 缺口范围",
          );
          const to =
            gap.reason === "delivery_retention_gap"
              ? BigInt(page.delivery_frontier.first_available_generation) - 1n
              : BigInt(gap.rebuild.cut_generation);
          need(
            gap.missing_range.from_generation ===
              (BigInt(before.cursor.after_generation) + 1n).toString() &&
              gap.missing_range.to_generation === to.toString() &&
              BigInt(gap.missing_range.from_generation) <= to,
            "Gap 缺口区间不符",
          );
        }
        const candidate = await this.collect(
          {
            identity: identityOf(gap.rebuild),
            mode: gap.rebuild.history_mode,
            generation: gap.rebuild.cut_generation,
            token: gap.snapshot_token,
            expectedFixed: gap.rebuild,
          },
          budget,
        );
        return this.#commit(candidate, budget, {
          kind: "gap",
          reason: gap.reason,
        });
      }
      need(sameIdentity(reply, before.cursor), "无 Gap 不得切换会话/化身");
      let candidate = before,
        applied = clone(before.state.snapshot);
      for (const delta of page.deltas) {
        await this.hooks.validateDelta(delta);
        need(
          delta.generation ===
            (BigInt(candidate.cursor.after_generation) + 1n).toString() &&
            delta.base_cut === candidate.cursor.base_cut &&
            delta.session_id === before.cursor.session_id &&
            delta.catalog_revision === before.cursor.catalog_revision &&
            delta.seq_range.from ===
              (BigInt(candidate.cursor.last_seq) + 1n).toString(),
          "Delta 代际/身份/cut/输入前沿断链",
        );
        applied = await this.applyBatch(applied, delta);
        candidate = await this.collect(
          {
            identity: before.cursor,
            mode: "RecomputedWithRevision",
            generation: delta.generation,
          },
          budget,
        );
        await this.hooks.comparePublication(delta, candidate.state.snapshot);
        need(
          equal(rowsOf(applied), candidate.projection.rows),
          "Delta 应用与同 cut 分页六族全字段/顺序不一致",
        );
      }
      if (!page.deltas.length) {
        candidate = await this.collect(
          {
            identity: before.cursor,
            mode: before.projection.fixed_cut.history_mode,
            generation: before.cursor.after_generation,
          },
          budget,
        );
        need(
          equal(candidate.projection, before.projection),
          "空增量下已提交完整切面改变",
        );
      }
      need(
        equal(page.next_cursor, candidate.cursor),
        "Watch 完整 next_cursor 不匹配应用末批",
      );
      const head = BigInt(page.observed_head.cut_generation),
        end = BigInt(candidate.cursor.after_generation);
      need(
        head >= end &&
          page.has_more === head > end &&
          (page.deltas.length > 0 || head === end),
        "Watch observed_head/has_more 或批次截断",
      );
      if (head === end) {
        const fixed = {
          ...candidate.projection.fixed_cut,
          history_mode: "RecomputedWithRevision",
          as_of_generation: null,
        };
        need(
          equal(page.observed_head, fixed),
          "Watch observed_head 与同代完整发布身份不符",
        );
      }
      return this.#commit(candidate, budget, {
        kind: "watch",
        batches: page.deltas.length,
        hasMore: page.has_more,
      });
    }
  }
  const publicAPI = {
    Client,
    DEFAULTS,
    FAMILIES,
    ORDER,
    orderFor,
    parseDiscovery,
    readDiscovery,
    TB02_CATALOG,
    canonical,
    parse,
    sha256,
    rowsOf,
    cursorOf,
  };
  const q3Build =
    typeof module === "object" && module.exports
      ? require("./q3-consumer.js")
      : globalThis.Q3Consumer;
  need(typeof q3Build === "function", "完整 Q3 消费者模块未加载");
  const Q3 = q3Build(publicAPI);
  publicAPI.Q3 = Q3;
  // #1404/#1414：类型清单与段合同的唯一来源在 q3-typed；页面 typedKinds 直接引用同一冻结数组。
  publicAPI.TB02_KINDS = Q3.typed.TB02_KINDS;
  publicAPI.validateBiPayload = Q3.typed.validateBiPayload;
  publicAPI.validateSegmentPayload = Q3.typed.validateSegmentPayload;
  publicAPI.validateTB02Catalog = Q3.validateTB02Catalog;
  publicAPI.validateTyped = Q3.typed.validateTyped;
  publicAPI.validateTB02ShapeRefs = Q3.typed.validateTB02ShapeRefs;
  publicAPI.validateTB02Axes = Q3.axesValidate;
  publicAPI.validateTB02Raw = Q3.rawValidate;
  publicAPI.validateTB02CutSources = Q3.bindSources;
  publicAPI.LegacyClient = Client;
  publicAPI.Client = function (options) {
    return options?.protocol === "s-session/3" ||
      options?.limits?.schema_revision === "s-query-client-resources/3"
      ? new Q3.Client(options)
      : new Client(options);
  };
  return Object.freeze(publicAPI);
});
