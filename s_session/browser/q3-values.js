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
  function utf8(s) {
    need(typeof s === "string", "要求 UTF-8 文本");
    for (let i = 0; i < s.length; i++) {
      let c = s.charCodeAt(i);
      if (c >= 0xd800 && c <= 0xdbff) {
        c = s.charCodeAt(++i);
        need(c >= 0xdc00 && c <= 0xdfff, "孤立 Unicode 代理项");
      } else need(c < 0xdc00 || c > 0xdfff, "孤立 Unicode 代理项");
    }
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
