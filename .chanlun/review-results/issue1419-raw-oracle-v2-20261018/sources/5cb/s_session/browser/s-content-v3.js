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
