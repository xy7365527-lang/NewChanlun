"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const crypto = require("node:crypto");
const path = require("node:path");
const vectorsPath = path.join(
  __dirname,
  "../../.chanlun/review-results/issue1394-content-v3-20260916/inputs/content-format-vectors.json",
);
const raw = fs.readFileSync(vectorsPath);
const vectors = JSON.parse(raw);
const C = require("../browser/s-content-v3.js");

test("independent 23 value roots, 22 renders and 96 reachable node bytes", async () => {
  assert.equal(
    crypto.createHash("sha256").update(raw).digest("hex"),
    "78984501cbb465be8251e3d212e34c0cb6fb33a5aab9a4999a3f1db592ce71f0",
  );
  const all = new Map();
  for (const v of vectors.positive) {
    const result = await C.encode(v.expression, { allowTemplate: true });
    assert.equal(result.ref.root, v.root, v.name);
    for (const n of result.nodes) all.set(n.root, n);
    const g = await C.GraphView.create(result.nodes);
    if (v.expression.t === "batch_self")
      await assert.rejects(g.verifyComplete(result.ref));
    else await g.verifyComplete(result.ref);
    if (v.expression.t !== "batch_self") {
      let rendered = "";
      for await (const chunk of g.renderChunks(result.ref)) rendered += chunk;
      assert.equal(
        Buffer.from(rendered).toString("hex"),
        v.canonical_render_hex,
        v.name,
      );
    }
  }
  assert.equal(all.size, 96);
  for (const [root, node] of all)
    assert.deepEqual(
      node,
      vectors.nodes.find((n) => n.root === root),
    );
});

// Captured formal HTTP payloads; original request SHA256 dd0cf472bf757ade44d6995bf5872138010388b22b3ea9f5778e9916066aa78b; response SHA256 b396eaa69d63964d394c1041380da024036aae3cf1ebd2e8470eda39919e5c83.
const httpRequest = {
  anchor: {
    cut_projection_digest:
      "qcut3-690ad95b6dca8053c7724399a30e62ea6611427217fd49a8db87f84e1f6aa408",
    fixed_cut: {
      as_of_generation: "1",
      catalog_evidence: {
        extent: "16",
        format: "s-content/1",
        kind: "object",
        root: "sc1-75b704e1a16f9da0605d787b56948feac69766715943f7953accb7ddd547fd8e",
      },
      catalog_revision:
        "s2-axis-quantifiers (SPEC-COVERAGE-INPUT.json sha256=76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c)",
      catalog_run_status: "run",
      cut_generation: "1",
      history_mode: "AsKnown",
      index_frontier:
        "batch3-529bec7f18eb81627c1acfd1d7d39b2391b67168e810a1684723c9a07853ca13",
      input_frontier: "0",
      profile_hash:
        "1c11d398cd9ac43a161bc6494b196771f131fba74c42c0bf1ccb6474a8fc4d57",
      profile_id: "ohlc_integer_tb02a_v1",
      rule_revision: "s2-axis-quantifiers+new-bi/1",
      scope: { economic: "not_started", structure: "CompleteCut" },
      seq_range: { from: "0", to: "0" },
      session_generation: "1",
      session_id: "issue1394-financial-small-20260930",
      structure_cut: "cut-1",
    },
    kind: "projection",
    projection_ref: {
      extent: "7",
      format: "s-content/1",
      kind: "object",
      root: "sc1-03595ea19f446c232679e216a8146bc08fa639007ae41e12830a60e4dd562114",
    },
  },
  limit: "1",
  offset: "0",
  op: "content",
  path: [{ field: "rows" }],
  unit: "items",
};
const httpResponse = {
  anchor: {
    cut_projection_digest:
      "qcut3-690ad95b6dca8053c7724399a30e62ea6611427217fd49a8db87f84e1f6aa408",
    fixed_cut: {
      as_of_generation: "1",
      catalog_evidence: {
        extent: "16",
        format: "s-content/1",
        kind: "object",
        root: "sc1-75b704e1a16f9da0605d787b56948feac69766715943f7953accb7ddd547fd8e",
      },
      catalog_revision:
        "s2-axis-quantifiers (SPEC-COVERAGE-INPUT.json sha256=76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c)",
      catalog_run_status: "run",
      cut_generation: "1",
      history_mode: "AsKnown",
      index_frontier:
        "batch3-529bec7f18eb81627c1acfd1d7d39b2391b67168e810a1684723c9a07853ca13",
      input_frontier: "0",
      profile_hash:
        "1c11d398cd9ac43a161bc6494b196771f131fba74c42c0bf1ccb6474a8fc4d57",
      profile_id: "ohlc_integer_tb02a_v1",
      rule_revision: "s2-axis-quantifiers+new-bi/1",
      scope: { economic: "not_started", structure: "CompleteCut" },
      seq_range: { from: "0", to: "0" },
      session_generation: "1",
      session_id: "issue1394-financial-small-20260930",
      structure_cut: "cut-1",
    },
    kind: "projection",
    projection_ref: {
      extent: "7",
      format: "s-content/1",
      kind: "object",
      root: "sc1-03595ea19f446c232679e216a8146bc08fa639007ae41e12830a60e4dd562114",
    },
  },
  done: false,
  end: "1",
  items: [
    {
      extent: "3",
      format: "s-content/1",
      kind: "object",
      root: "sc1-c2f73b9b8a9667aa03c14e9c01633af41a00e0c8320e427341d036bfe92a67d8",
    },
  ],
  next_token: {
    anchor: {
      cut_projection_digest:
        "qcut3-690ad95b6dca8053c7724399a30e62ea6611427217fd49a8db87f84e1f6aa408",
      fixed_cut: {
        as_of_generation: "1",
        catalog_evidence: {
          extent: "16",
          format: "s-content/1",
          kind: "object",
          root: "sc1-75b704e1a16f9da0605d787b56948feac69766715943f7953accb7ddd547fd8e",
        },
        catalog_revision:
          "s2-axis-quantifiers (SPEC-COVERAGE-INPUT.json sha256=76019aba67712e9140a95f8eaf2495a9b745707fce1ce5684de7df4334fa2b4c)",
        catalog_run_status: "run",
        cut_generation: "1",
        history_mode: "AsKnown",
        index_frontier:
          "batch3-529bec7f18eb81627c1acfd1d7d39b2391b67168e810a1684723c9a07853ca13",
        input_frontier: "0",
        profile_hash:
          "1c11d398cd9ac43a161bc6494b196771f131fba74c42c0bf1ccb6474a8fc4d57",
        profile_id: "ohlc_integer_tb02a_v1",
        rule_revision: "s2-axis-quantifiers+new-bi/1",
        scope: { economic: "not_started", structure: "CompleteCut" },
        seq_range: { from: "0", to: "0" },
        session_generation: "1",
        session_id: "issue1394-financial-small-20260930",
        structure_cut: "cut-1",
      },
      kind: "projection",
      projection_ref: {
        extent: "7",
        format: "s-content/1",
        kind: "object",
        root: "sc1-03595ea19f446c232679e216a8146bc08fa639007ae41e12830a60e4dd562114",
      },
    },
    issued_capture_digest:
      "35fcd1e0fb2179aa830fa4ded618552d44a855c78e3ff0f10414a02b8b4cd5ef",
    limit: "1",
    offset: "1",
    path: [{ field: "rows" }],
    target_ref: {
      extent: "7",
      format: "s-content/1",
      kind: "array",
      root: "sc1-47cc2a061a36b979b49a1a826fc9f576890c59194fbc6bd06d35d2bb74694d4a",
    },
    token_checksum:
      "910824a4bc6f9e4e2fe47c98e1c91e4d05abc6790ad18b6b9ac4ae3b2008306b",
    token_schema: "s-content-token/1",
    unit: "items",
  },
  nodes: [
    {
      body_hex:
        "00000000000000021f05bb04c47b2ab1225e9552eda2478d6431cf4d2c5d750b5d38b7f24b04f3663b5a65d90158e7bfe10529048955a2ef889122e264f2c079857021f30b39c324",
      root: "sc1-0131d37a42e7faa2e954dc7a754b464b7b5582bd1ccd84bd34d40940267bb344",
      tag: "13",
    },
    {
      body_hex:
        "00000000000000076b7181895c99348b5adc2ec4fcde689c6219ec0e62efab1a9084dc9b259186f7",
      root: "sc1-03595ea19f446c232679e216a8146bc08fa639007ae41e12830a60e4dd562114",
      tag: "06",
    },
    {
      body_hex:
        "c62053feefbd8481352eaa39b83653da8e1a10905a2a197f9747e34bd9a13724",
      root: "sc1-04027d1b6303a092d61120d0352ed269298c9f3831bbba87eb1403ed510203a9",
      tag: "12",
    },
    {
      body_hex: "6f726465725f76657273696f6e",
      root: "sc1-07108bdfa86ec1e68d0569fbaeeba1da6f615eb817d0a1deba794c99d1deda9d",
      tag: "10",
    },
    {
      body_hex:
        "000000000000000d88d7fc4032914b4c55ff47eb328330428bb59da34587b19bd8bdded057e893f1",
      root: "sc1-073745543f7a47e884e1e463f91e7d2b73809356626d49d894dfd2d1bf5efaa5",
      tag: "03",
    },
    {
      body_hex:
        "ac7e10d4763bb4db5185cb82144476fa88a9e8ecb5b06ed4f29daed85c739fbd",
      root: "sc1-088e3d6e753293a949aaab4a69e29594a2d978581ad96c48782f8a353cd5e3d8",
      tag: "12",
    },
    {
      body_hex:
        "073745543f7a47e884e1e463f91e7d2b73809356626d49d894dfd2d1bf5efaa5e94f8443b2a06055ec91c56de9b7d0ba97ae72751504dd791b516f3b9de6af24",
      root: "sc1-0df7e1d7c30064f7afe130b424dc7c042c434b633b517ee81bbb6594019ef53b",
      tag: "14",
    },
    {
      body_hex:
        "0000000000000005088e3d6e753293a949aaab4a69e29594a2d978581ad96c48782f8a353cd5e3d8",
      root: "sc1-119d536aedee6f82749dad80daf786e04612539fe174b5764ba5f1a72996998e",
      tag: "03",
    },
    {
      body_hex:
        "f133d24e09bf5ecdea5b6dd57eb6401edd7f73afe6fe3ef4334ca774de1424dd",
      root: "sc1-1f05bb04c47b2ab1225e9552eda2478d6431cf4d2c5d750b5d38b7f24b04f366",
      tag: "12",
    },
    {
      body_hex:
        "000000000000000904027d1b6303a092d61120d0352ed269298c9f3831bbba87eb1403ed510203a9",
      root: "sc1-38a53886188b7a6ed401bcc219232529b98bcc83fcd72d3453ca4ffe7d018a78",
      tag: "03",
    },
    {
      body_hex:
        "ccd7296d32c5a631ca48a982f17c5beca4365a1cc509364ba6454240e1281fad",
      root: "sc1-3b5a65d90158e7bfe10529048955a2ef889122e264f2c079857021f30b39c324",
      tag: "12",
    },
    {
      body_hex:
        "00000000000000076291651978ef79e6bca6f0b7ad68b071b05e2bbf2542caf079e09d2d7625b77a",
      root: "sc1-47cc2a061a36b979b49a1a826fc9f576890c59194fbc6bd06d35d2bb74694d4a",
      tag: "05",
    },
    {
      body_hex:
        "c2f73b9b8a9667aa03c14e9c01633af41a00e0c8320e427341d036bfe92a67d8",
      root: "sc1-4a218b3dd05c5bd3e078f9d13cbdce108308b8d0990c953d07c6751655832998",
      tag: "12",
    },
    {
      body_hex:
        "0000000000000004d3180614fbddeaecc5622c58d1df862095135d5e8c3e6764a52013936825bc3c0131d37a42e7faa2e954dc7a754b464b7b5582bd1ccd84bd34d40940267bb344",
      root: "sc1-4ad1fb39f310ccd03c02b9afa27ea8158c953ea7c56bc00dbe5512ecc21c746a",
      tag: "13",
    },
    {
      body_hex:
        "0000000000000007f84d55da5d7b344e5944db3a70a4680d43e3110835dd5414f95375dbb69e11f6a871bc16c3386f902b4e5056d391ea07371486a895ce6fa652705ae21df6f09e",
      root: "sc1-6291651978ef79e6bca6f0b7ad68b071b05e2bbf2542caf079e09d2d7625b77a",
      tag: "13",
    },
    {
      body_hex:
        "00000000000000037280799afd24807023aa1fe0fc345464a6fa655fd8f0d508104d851a2969778578b05fc3a4ccedc4521d4f9dd2a658abe88f6392739ae6f9ffa38d658368ef71",
      root: "sc1-6b4c6148c5d3859bbcfc06af6eb6f034463140fe782c6e32af153136a9af4f15",
      tag: "13",
    },
    {
      body_hex:
        "00000000000000074ad1fb39f310ccd03c02b9afa27ea8158c953ea7c56bc00dbe5512ecc21c746a6b4c6148c5d3859bbcfc06af6eb6f034463140fe782c6e32af153136a9af4f15",
      root: "sc1-6b7181895c99348b5adc2ec4fcde689c6219ec0e62efab1a9084dc9b259186f7",
      tag: "13",
    },
    {
      body_hex:
        "0000000000000002d1285727a6307eb014883dc9e0dfa1dccfa0ee8f7d8cca9803fbb5c18681ae8ef01f8a6964968d9c57449a7262d3f1b888a0e25232ceb1c8d9311d1f41209df9",
      root: "sc1-7280799afd24807023aa1fe0fc345464a6fa655fd8f0d508104d851a29697785",
      tag: "13",
    },
    {
      body_hex:
        "9f910373d2f14afce0f8b31312ec45fa9d7940b9094896f2af3f14b74270ec9d",
      root: "sc1-78b05fc3a4ccedc4521d4f9dd2a658abe88f6392739ae6f9ffa38d658368ef71",
      tag: "12",
    },
    {
      body_hex: "726f7773",
      root: "sc1-79454315d141f8ce303d47a39bc20881bee637cc69fcbab0bb1f39ee828016d5",
      tag: "10",
    },
    {
      body_hex:
        "0000000000000004afb1d5b968f9a6840f8075945f749663afbbd59b1bf879063eddb7ede9bea0b7",
      root: "sc1-882bb87b1e47c404429b9d2ec39852d5fb7e058b1a43941cd28b04fc2d4c8952",
      tag: "03",
    },
    {
      body_hex:
        "07108bdfa86ec1e68d0569fbaeeba1da6f615eb817d0a1deba794c99d1deda9d",
      root: "sc1-88d7fc4032914b4c55ff47eb328330428bb59da34587b19bd8bdded057e893f1",
      tag: "12",
    },
    {
      body_hex:
        "119d536aedee6f82749dad80daf786e04612539fe174b5764ba5f1a72996998e49bf4520f3c200bc357334fe4892fc58847f73eedd6449172aca53f8c2e537ee",
      root: "sc1-9f910373d2f14afce0f8b31312ec45fa9d7940b9094896f2af3f14b74270ec9d",
      tag: "14",
    },
    {
      body_hex:
        "0000000000000003a04e1d09e52814432c71ab39fd1218de944115c0a72c45690fc8256cc514873b638910759bc3a2a8d7811306147e4aaf76f95991faafd6b8a80c7ef2ead40471",
      root: "sc1-a871bc16c3386f902b4e5056d391ea07371486a895ce6fa652705ae21df6f09e",
      tag: "13",
    },
    {
      body_hex:
        "00000000000000024a218b3dd05c5bd3e078f9d13cbdce108308b8d0990c953d07c6751655832998dd6116ccae79f41600aae484ff3994039f58aa6a477f30092b6bbb1951eeab20",
      root: "sc1-ab83268d8e1a0f9d2f65eb22b7ea79cffd67c138996ce849d440865c5a17a055",
      tag: "13",
    },
    {
      body_hex: "73636f7065",
      root: "sc1-ac7e10d4763bb4db5185cb82144476fa88a9e8ecb5b06ed4f29daed85c739fbd",
      tag: "10",
    },
    {
      body_hex:
        "79454315d141f8ce303d47a39bc20881bee637cc69fcbab0bb1f39ee828016d5",
      root: "sc1-afb1d5b968f9a6840f8075945f749663afbbd59b1bf879063eddb7ede9bea0b7",
      tag: "12",
    },
    {
      body_hex:
        "0000000000000003ee6b1a904a3f0de4a7056fdaac4155d49e8714e3a994931254e102eaba4790cc",
      root: "sc1-c2f73b9b8a9667aa03c14e9c01633af41a00e0c8320e427341d036bfe92a67d8",
      tag: "06",
    },
    {
      body_hex: "6f6d697373696f6e73",
      root: "sc1-c62053feefbd8481352eaa39b83653da8e1a10905a2a197f9747e34bd9a13724",
      tag: "10",
    },
    {
      body_hex:
        "38a53886188b7a6ed401bcc219232529b98bcc83fcd72d3453ca4ffe7d018a780cbb888583ed1533282a1eb0f066cb8c28d58b0acacde146fc823f4a3662adb7",
      root: "sc1-ccd7296d32c5a631ca48a982f17c5beca4365a1cc509364ba6454240e1281fad",
      tag: "14",
    },
    {
      body_hex:
        "0df7e1d7c30064f7afe130b424dc7c042c434b633b517ee81bbb6594019ef53b",
      root: "sc1-d1285727a6307eb014883dc9e0dfa1dccfa0ee8f7d8cca9803fbb5c18681ae8e",
      tag: "12",
    },
    {
      body_hex:
        "0000000000000002e083a5516873b1ab1ce22ac51b39b6ae5938c24141b9309c4824525398beefefa4f288eb769918aa24b7c72baf47f9b7d8d3cb2d2dfcf92ab7d3476c2db03b58",
      root: "sc1-d1bbbde0bca08eb1966dd842d2458fbe10a6ec04cdcaea4aa17d1ef658327d2a",
      tag: "13",
    },
    {
      body_hex:
        "0000000000000002a549f686c6c72982f33bcb1243106f381c307ce131268a80b405106766da354775d68d09157ad52f93e7b01f858734328b10e7a3d4f5eec3adcd0d842d03f0a5",
      root: "sc1-d3180614fbddeaecc5622c58d1df862095135d5e8c3e6764a52013936825bc3c",
      tag: "13",
    },
    {
      body_hex:
        "882bb87b1e47c404429b9d2ec39852d5fb7e058b1a43941cd28b04fc2d4c895247cc2a061a36b979b49a1a826fc9f576890c59194fbc6bd06d35d2bb74694d4a",
      root: "sc1-dcab8e838c170b5e6ee459528db7feb4946914fdb72e31a1890b56802217c89d",
      tag: "14",
    },
    {
      body_hex:
        "d85db10dd155a4cc9012035a031e37782e2169ee509109a5bf0aaa68f495a485",
      root: "sc1-dd6116ccae79f41600aae484ff3994039f58aa6a477f30092b6bbb1951eeab20",
      tag: "12",
    },
    {
      body_hex:
        "dcab8e838c170b5e6ee459528db7feb4946914fdb72e31a1890b56802217c89d",
      root: "sc1-f01f8a6964968d9c57449a7262d3f1b888a0e25232ceb1c8d9311d1f41209df9",
      tag: "12",
    },
    {
      body_hex:
        "0000000000000004ab83268d8e1a0f9d2f65eb22b7ea79cffd67c138996ce849d440865c5a17a055d1bbbde0bca08eb1966dd842d2458fbe10a6ec04cdcaea4aa17d1ef658327d2a",
      root: "sc1-f84d55da5d7b344e5944db3a70a4680d43e3110835dd5414f95375dbb69e11f6",
      tag: "13",
    },
  ],
  offset: "0",
  ok: true,
  path: [{ field: "rows" }],
  target_ref: {
    extent: "7",
    format: "s-content/1",
    kind: "array",
    root: "sc1-47cc2a061a36b979b49a1a826fc9f576890c59194fbc6bd06d35d2bb74694d4a",
  },
  total: "7",
  unit: "items",
  validated_capture_digest:
    "35fcd1e0fb2179aa830fa4ded618552d44a855c78e3ff0f10414a02b8b4cd5ef",
};

function reachable(root) {
  const pool = new Map(vectors.nodes.map((n) => [n.root, n]));
  const result = new Map(),
    pending = [root];
  while (pending.length) {
    const r = pending.pop();
    if (result.has(r)) continue;
    const n = pool.get(r);
    if (!n) continue;
    result.set(r, n);
    const offsets =
      {
        "03": [16],
        "04": [16],
        "05": [16],
        "06": [16],
        "07": [0],
        12: [0],
        13: [16, 80],
        14: [0, 64],
      }[n.tag] || [];
    for (const off of offsets)
      pending.push("sc1-" + n.body_hex.slice(off, off + 64));
  }
  return [...result.values()].sort((a, b) => a.root.localeCompare(b.root));
}

test("all 10 independently encoded bad roots reject full validation", async () => {
  for (const v of vectors.invalid_encoded_roots) {
    await assert.rejects(
      async () => {
        const g = await C.GraphView.create(reachable(v.root));
        await g.verifyComplete(g.header(v.root));
      },
      undefined,
      v.name,
    );
  }
});

test("formal HTTP response is an independent range proof, not a full graph", async () => {
  const cache = new Map();
  const proof = await C.verifyRange(httpRequest, httpResponse, { cache });
  assert.equal(proof.rangeValidated, true);
  assert.equal(proof.semanticValidated, false);
  assert.equal(proof.metrics.uniqueNodes, 37);
  assert.equal(cache.size, 37);
  await assert.rejects(
    proof.graph.verifyComplete(httpRequest.anchor.projection_ref),
  );
});

const clone = (v) => JSON.parse(JSON.stringify(v));
async function render(g, ref) {
  let s = "";
  for await (const c of g.renderChunks(ref)) s += c;
  return s;
}
function reference(root, kind, extent = null) {
  return { format: "s-content/1", root, kind, extent };
}
function fixtureRef(name) {
  const v = vectors.positive.find((v) => v.name === name),
    t = v.expression.t;
  const tag = {
    null: "null",
    bool: "bool",
    string: "string",
    number: "number",
    array: "array",
    object: "object",
    json_text: "json_text",
    batch_self: "template_batch",
  }[t];
  const extent =
    t === "array"
      ? String(v.expression.items.length)
      : t === "object"
        ? String(v.expression.fields.length)
        : t === "string"
          ? String(Buffer.byteLength(v.expression.value))
          : t === "number"
            ? String(Buffer.byteLength(v.expression.token))
            : null;
  return reference(v.root, tag, extent);
}
function anchorFor(ref) {
  return {
    kind: "projection",
    fixed_cut: { session_id: "fixture", structure_cut: "cut-1" },
    cut_projection_digest: "qcut3-" + "a".repeat(64),
    projection_ref: ref,
  };
}
async function caseFor(expr, path, unit, offset, limit, items, total) {
  const e = await C.encode(expr),
    g = await C.GraphView.create(e.nodes),
    target = g.resolve(e.ref, path);
  const request = {
    op: "content",
    anchor: anchorFor(e.ref),
    path,
    unit,
    offset,
    limit,
  };
  const end = String(
    BigInt(offset) + BigInt(limit) > BigInt(total)
      ? BigInt(total)
      : BigInt(offset) + BigInt(limit),
  );
  const response = {
    anchor: clone(request.anchor),
    path: clone(path),
    target_ref: target,
    unit,
    offset,
    end,
    total,
    items,
    nodes: e.nodes,
    done: end === total,
    next_token: null,
    validated_capture_digest: "b".repeat(64),
    ok: true,
  };
  if (!response.done) {
    const t = {
      token_schema: "s-content-token/1",
      anchor: clone(request.anchor),
      path: clone(path),
      target_ref: target,
      unit,
      offset: end,
      limit,
      issued_capture_digest: "b".repeat(64),
    };
    t.token_checksum = await C.tokenChecksum(t);
    response.next_token = t;
  }
  return { request, response, e, g };
}

test("token lexemes survive parse/decode/render and plain Numbers cannot guess them", async () => {
  const text = '{"a":[1,1.0,1e0,-0,1E+9],"b":"汉\\n"}';
  const e = await C.encode(C.parseJSON(text)),
    g = await C.GraphView.create(e.nodes);
  assert.equal(await render(g, e.ref), text);
  assert.deepEqual(await g.decodedValue(e.ref), C.parseJSON(text));
  assert.throws(() => C.fromPlain(1), /Number/);
  assert.throws(() => C.parseJSON('{"a":1,"a":2}'), /duplicate/);
  for (const token of ["01", "+1", "NaN", "Infinity", "1.", "1e", "--1", " 1"])
    await assert.rejects(C.encode({ t: "number", token }));
  for (const text of ["[1,]", '{"a":1,}', "true false", '"\\ud800"'])
    assert.throws(() => C.parseJSON(text));
  const nfc = await C.encode({ t: "string", value: "é" }),
    decomposed = await C.encode({ t: "string", value: "e\u0301" });
  assert.notEqual(nfc.ref.root, decomposed.ref.root);
});

test("exact old JSON text is preserved; ordinary ref-looking objects stay objects", async () => {
  for (const text of [
    '{"a":1.0}',
    '["汉",true,null]',
    '{"source":"x/y","z":"\\u0001"}',
  ]) {
    const e = await C.encodeJSONText(text),
      g = await C.GraphView.create(e.nodes);
    assert.equal(await render(g, e.ref), JSON.stringify(text));
  }
  for (const text of ['{ "a":1}', '{"z":1,"a":2}', '"\\/"', '"\\u0061"'])
    await assert.rejects(C.encodeJSONText(text));
  const plain = fixtureRef("number_one"),
    normal = await C.encode(C.fromPlain(plain)),
    edge = await C.encode(C.refEdge(plain));
  assert.equal(normal.ref.kind, "object");
  assert.notEqual(normal.ref.root, plain.root);
  assert.deepEqual(edge.ref, plain);
  assert.equal(edge.semanticValidated, false);
  assert.equal(edge.nodes.length, 0);
  const provisional = await C.encode({
    t: "object",
    fields: [["a", C.refEdge(plain)]],
  });
  assert.equal(provisional.semanticValidated, false);
  const g = await C.GraphView.create(provisional.nodes);
  await assert.rejects(g.verifyComplete(provisional.ref), /missing/);
  await assert.rejects(
    C.encode({
      t: "object",
      fields: [
        ["x", { t: "null" }],
        ["x", { t: "bool", value: true }],
      ],
    }),
    /duplicate/,
  );
});

test("template is hashable alone but never publicly readable; json_text has ordinary descendants", async () => {
  const e = await C.encode({ t: "batch_self" }, { allowTemplate: true }),
    g = await C.GraphView.create(e.nodes);
  assert.equal(e.semanticValidated, false);
  await assert.rejects(g.verifyComplete(e.ref, { allowTemplate: true }));
  await assert.rejects(g.decodedValue(e.ref));
  await assert.rejects(render(g, e.ref));
  assert.throws(() => g.resolve(e.ref, []));
  await assert.rejects(
    C.encode(
      { t: "array", items: [{ t: "batch_self" }] },
      { allowTemplate: true },
    ),
  );
  for (const value of [
    { t: "json_text", value: { t: "null" } },
    { t: "array", items: [{ t: "batch_self" }] },
    { t: "object", fields: [["x", { t: "json_text", value: { t: "null" } }]] },
  ])
    await assert.rejects(
      C.encode({ t: "json_text", value }, { allowTemplate: true }),
    );
});

test("independent proofs reject cached missing ancestors, duplicate/order/hash/exact-field errors", async () => {
  const cache = new Map();
  await C.verifyRange(httpRequest, httpResponse, { cache });
  const original = [...cache.entries()];
  const mutations = [
    (r) => {
      r.nodes = r.nodes.filter((n) => n.root !== r.anchor.projection_ref.root);
    },
    (r) => {
      r.nodes.splice(1, 0, r.nodes[0]);
    },
    (r) => {
      r.nodes.reverse();
    },
    (r) => {
      r.nodes[0].body_hex += "00";
    },
    (r) => {
      r.nodes[0].body_hex = r.nodes[0].body_hex.toUpperCase();
    },
    (r) => {
      r.nodes[0].extra = true;
    },
    (r) => {
      r.target_ref.extent = "8";
    },
    (r) => {
      r.items[0] = r.target_ref;
    },
    (r) => {
      r.items.push(r.items[0]);
    },
    (r) => {
      r.path = [{ field: "counts" }];
    },
    (r) => {
      r.anchor.fixed_cut.structure_cut = "cut-2";
    },
    (r) => {
      r.done = true;
    },
    (r) => {
      r.end = "2";
    },
    (r) => {
      r.total = "07";
    },
    (r) => {
      r.extra = true;
    },
    (r) => {
      r.next_token.offset = "2";
    },
    (r) => {
      r.next_token.target_ref = r.items[0];
    },
  ];
  for (const mutate of mutations) {
    const r = clone(httpResponse);
    mutate(r);
    await assert.rejects(C.verifyRange(httpRequest, r, { cache }));
    assert.deepEqual([...cache.entries()], original);
  }
  const conflict = new Map([
    [httpResponse.nodes[0].root, { ...httpResponse.nodes[0], body_hex: "00" }],
  ]);
  await assert.rejects(
    C.verifyRange(httpRequest, httpResponse, { cache: conflict }),
    /conflict/,
  );
  assert.equal(conflict.size, 1);
});

test("position proofs, byte identity, endpoints and overflow are independently checked", async () => {
  const nullRef = fixtureRef("null"),
    one = fixtureRef("number_one");
  const expr = {
    t: "object",
    fields: [
      [
        "a",
        { t: "array", items: [{ t: "null" }, { t: "number", token: "1" }] },
      ],
      ["s", { t: "string", value: "x".repeat(4095) + "汉尾" }],
    ],
  };
  const array = await caseFor(
    expr,
    [{ field: "a" }],
    "items",
    "1",
    "1",
    [one],
    "2",
  );
  await C.verifyRange(array.request, array.response);
  const wrong = clone(array.response);
  wrong.items = [nullRef];
  await assert.rejects(C.verifyRange(array.request, wrong));
  const bytes = await caseFor(
    expr,
    [{ field: "s" }],
    "bytes",
    "4095",
    "3",
    "e6b189",
    "4101",
  );
  await C.verifyRange(bytes.request, bytes.response);
  const different = clone(bytes.response);
  different.items = "e6b188";
  await assert.rejects(C.verifyRange(bytes.request, different));
  const terminal = await caseFor(
    expr,
    [{ field: "s" }],
    "bytes",
    "4101",
    "1",
    "",
    "4101",
  );
  assert.equal(
    (await C.verifyRange(terminal.request, terminal.response)).done,
    true,
  );
  const over = clone(terminal.request);
  over.offset = "9223372036854775807";
  over.limit = "1";
  const overResponse = clone(terminal.response);
  overResponse.offset = over.offset;
  await assert.rejects(C.verifyRange(over, overResponse), /overflow/);
  const badPath = clone(array.request);
  badPath.path = [{ field: "a" }, { index: "01" }];
  await assert.rejects(C.verifyRange(badPath, array.response));
  const item = await caseFor(
    expr,
    [{ field: "a" }, { index: "0" }],
    "value",
    "0",
    "1",
    [nullRef],
    "1",
  );
  await C.verifyRange(item.request, item.response);
  const end = await caseFor(
    expr,
    [{ field: "a" }, { index: "0" }],
    "value",
    "1",
    "1",
    [],
    "1",
  );
  await C.verifyRange(end.request, end.response);
  const empty = clone(end.response);
  empty.items = [nullRef];
  await assert.rejects(C.verifyRange(end.request, empty));
  const object = await caseFor(
    expr,
    [],
    "items",
    "0",
    "1",
    [{ key: "a", value_ref: array.response.target_ref }],
    "2",
  );
  await C.verifyRange(object.request, object.response);
  const badKey = clone(object.response);
  badKey.items[0].key = "b";
  await assert.rejects(C.verifyRange(object.request, badKey));
});

test("token checksum is not authority to change an existing stream anchor/target/offset", async () => {
  await C.validateToken(httpResponse.next_token);
  const changed = clone(httpResponse.next_token);
  changed.anchor.fixed_cut.structure_cut = "cut-2";
  changed.token_checksum = await C.tokenChecksum(changed);
  await C.validateToken(changed); // A checksum alone cannot assert publication authority.
  const request = { op: "content", content_token: changed };
  const response = clone(httpResponse);
  response.anchor = clone(changed.anchor);
  response.offset = "1";
  response.end = "2";
  response.items = clone(httpResponse.items);
  response.next_token = clone(changed);
  response.next_token.offset = "2";
  response.next_token.token_checksum = await C.tokenChecksum(
    response.next_token,
  );
  await assert.rejects(
    C.verifyRange(request, response, {
      previousToken: httpResponse.next_token,
    }),
    /continuity/,
  );
  const wrongTarget = clone(httpResponse);
  wrongTarget.next_token.target_ref = wrongTarget.items[0];
  wrongTarget.next_token.token_checksum = await C.tokenChecksum(
    wrongTarget.next_token,
  );
  await assert.rejects(C.verifyRange(httpRequest, wrongTarget), /identity/);
  const skip = clone(httpResponse);
  skip.next_token.offset = "2";
  skip.next_token.token_checksum = await C.tokenChecksum(skip.next_token);
  await assert.rejects(C.verifyRange(httpRequest, skip), /extent/);
});

test("Browser WebCrypto runs the same core without require, DOM or network", async () => {
  const vm = require("node:vm");
  const context = {
    TextEncoder,
    TextDecoder,
    Uint8Array,
    crypto: crypto.webcrypto,
  };
  vm.createContext(context);
  vm.runInContext(
    fs.readFileSync(path.join(__dirname, "../browser/s-content-v3.js"), "utf8"),
    context,
  );
  const browser = context.SContentV3;
  context.requestText = JSON.stringify(httpRequest);
  context.responseText = JSON.stringify(httpResponse);
  const proof = await vm.runInContext(
    "SContentV3.verifyRange(JSON.parse(requestText),JSON.parse(responseText))",
    context,
  );
  assert.equal(proof.rangeValidated, true);
  assert.equal(proof.metrics.uniqueNodes, 37);
  for (const v of vectors.positive) {
    const e = await browser.encode(v.expression, { allowTemplate: true });
    assert.equal(e.ref.root, v.root);
  }
});

test("no implicit nesting ceiling and full graph APIs agree on deep values", async () => {
  let ast = { t: "number", token: "1.0" };
  for (let i = 0; i < 1200; i++) ast = { t: "array", items: [ast] };
  const e = await C.encode(ast),
    g = await C.GraphView.create(e.nodes);
  assert.equal(
    await render(g, e.ref),
    "[".repeat(1200) + "1.0" + "]".repeat(1200),
  );
  const decoded = await g.decodedValue(e.ref);
  let a = decoded;
  for (let i = 0; i < 1200; i++) a = a.items[0];
  assert.equal(a.token, "1.0");
  const plain = [];
  plain.push(plain);
  assert.throws(() => C.fromPlain(plain), /cycle/);
});

test("checked references, header framing and all node roles reject malformed transport", async () => {
  const valid = fixtureRef("number_one");
  for (const ref of [
    { ...valid, extra: true },
    { ...valid, extent: "01" },
    { ...valid, extent: "9223372036854775808" },
    { ...valid, format: "s-content/0" },
    { ...valid, root: valid.root.toUpperCase() },
    { ...valid, kind: "bytes" },
    { ...fixtureRef("null"), extent: "0" },
  ])
    assert.throws(() => C.validateRef(ref));
  const g = await C.GraphView.create(reachable(valid.root));
  assert.throws(() => g.checkedRef({ ...valid, extent: "2" }));
  const domain = Buffer.from("NewChanlun/SContent/1\0");
  function raw(tag, body) {
    const size = Buffer.alloc(8);
    size.writeBigUInt64BE(BigInt(body.length));
    return {
      root:
        "sc1-" +
        crypto
          .createHash("sha256")
          .update(Buffer.concat([domain, Buffer.from([tag]), size, body]))
          .digest("hex"),
      tag: tag.toString(16).padStart(2, "0"),
      body_hex: body.toString("hex"),
    };
  }
  for (const n of [
    raw(0, Buffer.from([0])),
    raw(3, Buffer.alloc(39)),
    raw(
      3,
      Buffer.concat([Buffer.from("8000000000000000", "hex"), Buffer.alloc(32)]),
    ),
    raw(0x10, Buffer.alloc(0)),
    raw(0x10, Buffer.alloc(4097)),
    raw(0x13, Buffer.alloc(72)),
    raw(9, Buffer.alloc(0)),
    raw(0x14, Buffer.alloc(63)),
  ])
    await assert.rejects(C.GraphView.create([n]));
  const sourceNode = reachable(valid.root)[0],
    mutable = { ...sourceNode };
  const pending = C.GraphView.create([mutable]);
  mutable.body_hex = "00";
  const safe = await pending;
  assert.deepEqual(safe.nodes(), [sourceNode]);
});

test("ordered continuation verifies the same target with independent proof on every page", async () => {
  const v = vectors.positive.find((v) => v.name === "array_5");
  const encoded = { ref: fixtureRef("array_5"), nodes: reachable(v.root) };
  const root = await C.encode({
    t: "object",
    fields: [["a", C.refEdge(encoded.ref)]],
  });
  const nodes = [...root.nodes, ...encoded.nodes].sort((a, b) =>
    a.root.localeCompare(b.root),
  );
  const knownRef = (i) => {
    const e = vectors.positive.find((v) => v.name === "array_" + (i + 1));
    const list = reachable(e.root);
    const leaf = list.find(
      (n) =>
        n.tag === "03" &&
        Buffer.from(n.body_hex.slice(0, 16), "hex").readBigUInt64BE() === 1n &&
        list.some(
          (c) =>
            c.tag === "10" &&
            c.body_hex === Buffer.from(String(i)).toString("hex"),
        ),
    );
    // Header identities are taken from independent array vector leaf nodes.
    // Find the exact string via independently framed 0x10/0x12 paths.
    const block = nodes.find(
      (n) =>
        n.tag === "10" && n.body_hex === Buffer.from(String(i)).toString("hex"),
    );
    const seq = nodes.find(
      (n) => n.tag === "12" && n.body_hex === block.root.slice(4),
    );
    const header = nodes.find(
      (n) =>
        n.tag === "03" && n.body_hex === "0000000000000001" + seq.root.slice(4),
    );
    assert.ok(leaf);
    return reference(header.root, "string", "1");
  };
  const anchor = anchorFor(root.ref),
    path = [{ field: "a" }];
  let previousToken;
  for (let i = 0; i < 5; i++) {
    const request = i
      ? { op: "content", content_token: previousToken }
      : { op: "content", anchor, path, unit: "items", offset: "0", limit: "1" };
    const next =
      i === 4
        ? null
        : {
            token_schema: "s-content-token/1",
            anchor,
            path,
            target_ref: encoded.ref,
            unit: "items",
            offset: String(i + 1),
            limit: "1",
            issued_capture_digest: "c".repeat(64),
          };
    if (next) next.token_checksum = await C.tokenChecksum(next);
    const response = {
      anchor,
      path,
      target_ref: encoded.ref,
      unit: "items",
      offset: String(i),
      end: String(i + 1),
      total: "5",
      items: [knownRef(i)],
      nodes,
      done: i === 4,
      next_token: next,
      validated_capture_digest: "d".repeat(64),
      ok: true,
    };
    if (i === 0) {
      response.validated_capture_digest = "c".repeat(64);
    }
    const r = await C.verifyRange(request, response, { previousToken });
    assert.equal(r.end, String(i + 1));
    if (i === 1) {
      const missing = clone(response);
      missing.nodes = missing.nodes.filter((n) => n.root !== root.ref.root);
      await assert.rejects(
        C.verifyRange(request, missing, {
          previousToken,
          cache: new Map(nodes.map((n) => [n.root, n])),
        }),
      );
    }
    previousToken = next;
  }
});

test("json_text explicit path and sparse known forbidden descendants enforce ordinary-tree roles", async () => {
  const expression = {
    t: "object",
    fields: [
      [
        "json",
        {
          t: "json_text",
          value: { t: "array", items: [{ t: "number", token: "1.0" }] },
        },
      ],
    ],
  };
  const value = await caseFor(
    expression,
    [{ field: "json" }, { json_text: true }, { index: "0" }],
    "bytes",
    "0",
    "3",
    "312e30",
    "3",
  );
  await C.verifyRange(value.request, value.response);
  const publicValue = await caseFor(
    expression,
    [{ field: "json" }],
    "value",
    "0",
    "1",
    null,
    "1",
  );
  publicValue.response.items = [publicValue.response.target_ref];
  await C.verifyRange(publicValue.request, publicValue.response);
  const e = await C.encode({ t: "string", value: "\ufeff汉" }),
    g = await C.GraphView.create(e.nodes);
  assert.equal(await render(g, e.ref), '"\ufeff汉"');
});

function rawBuilder() {
  const nodes = new Map(),
    domain = Buffer.from("NewChanlun/SContent/1\0");
  function add(tag, body) {
    const size = Buffer.alloc(8);
    size.writeBigUInt64BE(BigInt(body.length));
    const root =
      "sc1-" +
      crypto
        .createHash("sha256")
        .update(Buffer.concat([domain, Buffer.from([tag]), size, body]))
        .digest("hex");
    nodes.set(root, {
      root,
      tag: tag.toString(16).padStart(2, "0"),
      body_hex: body.toString("hex"),
    });
    return root;
  }
  const hash = (r) => Buffer.from(r.slice(4), "hex"),
    u64 = (n) => {
      const b = Buffer.alloc(8);
      b.writeBigUInt64BE(BigInt(n));
      return b;
    };
  return { nodes, add, hash, u64 };
}

function jsonTextHeaderProof(innerTag, innerBody, offset = "0") {
  const b = rawBuilder(),
    inner = b.add(innerTag, innerBody),
    root = b.add(7, b.hash(inner)),
    keyChunk = b.add(0x10, Buffer.from("view")),
    keySeq = b.add(0x12, b.hash(keyChunk)),
    key = b.add(3, Buffer.concat([b.u64(4), b.hash(keySeq)])),
    field = b.add(0x14, Buffer.concat([b.hash(key), b.hash(root)])),
    fields = b.add(0x12, b.hash(field)),
    object = b.add(6, Buffer.concat([b.u64(1), b.hash(fields)])),
    ref = reference(root, "json_text"),
    anchor = anchorFor(reference(object, "object", "1")),
    path = [{ field: "view" }];
  return {
    inner,
    request: { op: "content", anchor, path, unit: "value", offset, limit: "1" },
    response: {
      anchor: clone(anchor),
      path: clone(path),
      target_ref: ref,
      unit: "value",
      offset,
      end: "1",
      total: "1",
      items: offset === "0" ? [ref] : [],
      nodes: [...b.nodes.values()].sort((a, b) => a.root.localeCompare(b.root)),
      done: true,
      next_token: null,
      validated_capture_digest: "b".repeat(64),
      ok: true,
    },
  };
}

test("json_text value proves only its direct inner header and exposes the authenticated next unit", async () => {
  for (const [tag, kind, extent, unit] of [
    [0, "null", null, "value"],
    [2, "bool", null, "value"],
    [3, "string", "12345", "bytes"],
    [4, "number", "12345", "bytes"],
    [5, "array", "12345", "items"],
    [6, "object", "12345", "items"],
  ]) {
    const body = tag < 3 ? Buffer.alloc(0) :
      Buffer.concat([rawBuilder().u64(12345), Buffer.alloc(32)]);
    for (const offset of ["0", "1"]) {
      const { inner, request, response } = jsonTextHeaderProof(tag, body, offset),
        proof = await C.verifyRange(request, response),
        innerRef = proof.graph.resolve(proof.target_ref, [{ json_text: true }]);
      assert.deepEqual(innerRef, reference(inner, kind, extent));
      const nextUnit = ["array", "object"].includes(innerRef.kind) ? "items" :
        ["string", "number"].includes(innerRef.kind) ? "bytes" : "value";
      assert.equal(nextUnit, unit);
      assert.equal(proof.metrics.uniqueNodes, 8);
      // Eight frames plus fixed wrapper bodies total 492 bytes before inner body.
      assert.equal(proof.metrics.framedBytes, String(492 + body.length));
      assert.equal(proof.semanticValidated, false);
      assert.deepEqual(response.items, offset === "0" ? [response.target_ref] : []);
      if (tag >= 3)
        await assert.rejects(proof.graph.verifyComplete(proof.target_ref), /missing node/);
    }
  }
});

test("json_text value proof cannot omit direct inner header even if cache already has it", async () => {
  for (const offset of ["0", "1"]) {
    const { inner, request, response } = jsonTextHeaderProof(0, Buffer.alloc(0), offset),
      cache = new Map(response.nodes.map(n => [n.root, n]));
    response.nodes = response.nodes.filter(n => n.root !== inner);
    await assert.rejects(C.verifyRange(request, response, { cache }), /missing node/);
    assert.equal(cache.size, 8);
  }
});

test("json_text value rejects direct inner wrong hash shape and JsonInner roles", async () => {
  for (const [tag, body] of [
    [7, Buffer.alloc(32)], [8, Buffer.alloc(0)], [0x11, Buffer.alloc(0)],
    [5, Buffer.alloc(39)], [0, Buffer.from([0])],
  ]) {
    const { request, response } = jsonTextHeaderProof(tag, body);
    await assert.rejects(C.verifyRange(request, response),
      /json_text descendant|internal node|header length|empty body/);
  }
  const { inner, request, response } = jsonTextHeaderProof(0, Buffer.alloc(0));
  response.nodes.find(n => n.root === inner).tag = "02";
  await assert.rejects(C.verifyRange(request, response), /node hash/);
});

function sharedGraph(kind, power, jsonText = false) {
  const b = rawBuilder();
  const leaf =
    kind === "array"
      ? b.add(0, Buffer.alloc(0))
      : b.add(0x10, Buffer.alloc(4096, kind === "number" ? 49 : 97));
  let seq = b.add(0x12, b.hash(leaf));
  for (let i = 1; i <= power; i++)
    seq = b.add(
      0x13,
      Buffer.concat([b.u64(1n << BigInt(i)), b.hash(seq), b.hash(seq)]),
    );
  const extent = (kind === "array" ? 1n : 4096n) << BigInt(power);
  let root = b.add(
    { array: 5, string: 3, number: 4 }[kind],
    Buffer.concat([b.u64(extent), b.hash(seq)]),
  );
  if (jsonText) root = b.add(7, b.hash(root));
  return { nodes: [...b.nodes.values()], root, extent };
}

test("F1 immutable checked inputs survive await and stream yield mutations", async () => {
  const e = await C.encode({ t: "null" }),
    g = await C.GraphView.create(e.nodes),
    r = { ...e.ref };
  const p = g.decodedValue(r);
  r.kind = "bool";
  assert.deepEqual(await p, { t: "null" });
  assert.equal(Object.isFrozen(r), false);
  assert.throws(() => {
    g.metrics = { uniqueNodes: 0, framedBytes: "0" };
  });
  assert.throws(() => new C.GraphView(new Map(), new Map(), {}), /create/);
  const rr = { ...e.ref },
    it = g.renderChunks(rr),
    first = it.next();
  rr.kind = "bool";
  assert.equal((await first).value, "null");
  await it.return();
  const s = await C.encode({ t: "string", value: "abc" }),
    sg = await C.GraphView.create(s.nodes),
    sr = { ...s.ref },
    stream = sg.renderChunks(sr);
  let text = (await stream.next()).value + (await stream.next()).value;
  sr.kind = "number";
  sr.root = e.ref.root;
  for await (const c of stream) text += c;
  assert.equal(text, '"abc"');
  const token = clone(httpResponse.next_token),
    expected = clone(token),
    pending = C.validateToken(token);
  token.offset = "900";
  token.anchor.fixed_cut.structure_cut = "changed";
  const checked = await pending;
  assert.deepEqual(checked, expected);
  assert.equal(checked.token_checksum, await C.tokenChecksum(checked));
  assert.equal(Object.isFrozen(token), false);
  assert.equal(Object.isFrozen(checked.anchor.fixed_cut), true);
  const options = {
    previousToken: { ...expected, token_checksum: "0".repeat(64) },
  };
  const request = { op: "content", content_token: expected },
    response = clone(httpResponse);
  response.offset = "1";
  response.end = "2";
  response.next_token.offset = "2";
  response.next_token.token_checksum = await C.tokenChecksum(
    response.next_token,
  );
  const proof = C.verifyRange(request, response, options);
  options.previousToken = expected;
  await assert.rejects(proof, /checksum|continuity/);
  const accepted = await C.verifyRange(httpRequest, httpResponse);
  assert.equal(Object.isFrozen(accepted.next_token.path), true);
  assert.throws(() => {
    accepted.next_token.offset = "900";
  });
  const ast = {
    t: "array",
    items: [{ t: "null" }, { t: "bool", value: true }],
  };
  const encoding = C.encode(ast);
  ast.items.reverse();
  ast.items[0].value = false;
  const encoded = await encoding,
    ag = await C.GraphView.create(encoded.nodes);
  assert.equal(await render(ag, encoded.ref), "[null,true]");
  assert.equal(Object.isFrozen(ast), false);
});

test("F2 nested json_text renders valid arrays and objects at every position", async () => {
  const json = (t) => ({ t: "json_text", value: C.parseJSON(t) });
  const array = await C.encode({
    t: "array",
    items: [
      json("null"),
      json("[]"),
      json('"x"'),
      json('[1.0,"汉\\n"]'),
      { t: "bool", value: true },
    ],
  });
  const g = await C.GraphView.create(array.nodes);
  assert.equal(
    await render(g, array.ref),
    '["null","[]","\\"x\\"","[1.0,\\"汉\\\\n\\"]",true]',
  );
  const object = await C.encode({
    t: "object",
    fields: [
      ["c", json('{"x":1e0}')],
      ["a", json("false")],
      ["b", { t: "null" }],
      ["d", json('"\\u0001"')],
    ],
  });
  const og = await C.GraphView.create(object.nodes);
  assert.equal(
    await render(og, object.ref),
    '{"a":"false","b":null,"c":"{\\"x\\":1e0}","d":"\\"\\\\u0001\\""}',
  );
});

test("F3 compact validation never expands shared logical items or byte positions before output", async () => {
  for (const [kind, power] of [
    ["array", 18],
    ["string", 10],
    ["number", 10],
  ])
    for (const jsonText of [false, true]) {
      const b = sharedGraph(kind, power, jsonText),
        g = await C.GraphView.create(b.nodes);
      let visits = 0;
      for (const name of ["entries", "byteChunks"]) {
        const original = g[name];
        g[name] = function* (...args) {
          for (const e of original.apply(this, args)) {
            visits++;
            yield e;
          }
        };
      }
      await g.verifyComplete(g.header(b.root));
      assert.equal(visits, 0, kind + " complete closure");
      const iterator = g.renderChunks(g.header(b.root));
      await iterator.next();
      assert.equal(
        visits,
        kind === "number" && !jsonText ? 1 : 0,
        kind + " first output",
      );
      await iterator.return();
    }
  let ast = { t: "null" };
  for (let i = 0; i < 12; i++) ast = { t: "array", items: [ast, ast] };
  const original = crypto.createHash;
  let calls = 0;
  crypto.createHash = function (...args) {
    calls++;
    return original.apply(this, args);
  };
  let e;
  try {
    e = await C.encode(ast);
  } finally {
    crypto.createHash = original;
  }
  assert.equal(e.metrics.uniqueNodes, 37);
  assert.ok(calls <= 4 * e.metrics.uniqueNodes, "actual SHA256 calls=" + calls);
});

function rawBytesGraph(kind, bytes, parts = null) {
  const b = rawBuilder(),
    chunks =
      parts ||
      Array.from({ length: Math.ceil(bytes.length / 4096) }, (_, i) =>
        bytes.subarray(i * 4096, (i + 1) * 4096),
      ),
    roots = chunks.map((c) => b.add(0x10, c));
  function m(r) {
    if (!r.length) return b.add(0x11, Buffer.alloc(0));
    if (r.length === 1) return b.add(0x12, b.hash(r[0]));
    let k = 1;
    while (k * 2 < r.length) k *= 2;
    return b.add(
      0x13,
      Buffer.concat([
        b.u64(r.length),
        b.hash(m(r.slice(0, k))),
        b.hash(m(r.slice(k))),
      ]),
    );
  }
  const root = b.add(
    kind === "number" ? 4 : 3,
    Buffer.concat([b.u64(bytes.length), b.hash(m(roots))]),
  );
  return { nodes: [...b.nodes.values()], root, b };
}
async function acceptsRaw(kind, bytes, expected, parts = null) {
  const data = rawBytesGraph(kind, bytes, parts),
    g = await C.GraphView.create(data.nodes);
  if (expected) await g.verifyComplete(g.header(data.root));
  else await assert.rejects(g.verifyComplete(g.header(data.root)));
  return { data, g };
}

test("composed UTF8 states reject overlong, surrogate, out-of-domain and unfinished block seams", async () => {
  for (let b = 0; b <= 255; b++)
    await acceptsRaw("string", Buffer.from([b]), b <= 127);
  const cases = [
    ["c280", true],
    ["dfbf", true],
    ["e0a080", true],
    ["ed9fbf", true],
    ["ee8080", true],
    ["f0908080", true],
    ["f48fbfbf", true],
    ["c080", false],
    ["c180", false],
    ["e08080", false],
    ["eda080", false],
    ["f0808080", false],
    ["f4908080", false],
    ["f5808080", false],
    ["8080", false],
    ["c2", false],
    ["e180", false],
    ["f18080", false],
    ["c241", false],
  ];
  for (const [bytes, valid] of cases) {
    const raw = Buffer.from(bytes, "hex");
    await acceptsRaw("string", raw, valid);
    await acceptsRaw(
      "string",
      Buffer.concat([Buffer.alloc(4095, 120), raw]),
      valid,
    );
  }
});

test("composed number grammar retains exponent/fraction entry states and exact lexemes", async () => {
  for (const token of [
    "0",
    "-0",
    "1.0",
    "1e0",
    "1E+0",
    "-1.2e-3",
    "1".repeat(4095) + "e+0",
    "1".repeat(4095) + ".0",
    "-" + "1".repeat(4095) + "e-2",
  ]) {
    const { data, g } = await acceptsRaw("number", Buffer.from(token), true);
    assert.equal(await render(g, g.header(data.root)), token);
  }
  for (const token of [
    "",
    "01",
    "-01",
    "+1",
    "0x1",
    "1.",
    "1e",
    "1e+",
    "1e-",
    "1e01x",
    "1".repeat(4095) + "e+",
    "1".repeat(4095) + ".",
    "0".repeat(8192),
    "-" + ".1",
  ])
    await acceptsRaw("number", Buffer.from(token), false);
  const raw = rawBytesGraph("string", Buffer.from("01")),
    number = raw.b.add(
      4,
      Buffer.concat([
        raw.b.u64(2),
        Buffer.from(raw.b.nodes.get(raw.root).body_hex.slice(16), "hex"),
      ]),
    ),
    g = await C.GraphView.create([...raw.b.nodes.values()]);
  await g.verifyComplete(g.header(raw.root));
  await assert.rejects(g.verifyComplete(g.header(number)), /number/);
});

test("memo summaries retain role, ordinary-tree and non-final block obligations", async () => {
  const b = rawBuilder(),
    nullRoot = b.add(0, Buffer.alloc(0)),
    seq = b.add(0x12, b.hash(nullRoot)),
    array = b.add(5, Buffer.concat([b.u64(1), b.hash(seq)])),
    object = b.add(6, Buffer.concat([b.u64(1), b.hash(seq)]));
  const graph = await C.GraphView.create([...b.nodes.values()]);
  await graph.verifyComplete(graph.header(array));
  await assert.rejects(
    graph.verifyComplete(graph.header(object)),
    /field role/,
  );
  const short = rawBytesGraph("string", Buffer.from("a")),
    shortSeq = "sc1-" + short.b.nodes.get(short.root).body_hex.slice(16),
    full = short.b.add(0x10, Buffer.alloc(4096, 120)),
    fullSeq = short.b.add(0x12, short.b.hash(full)),
    wrongSeq = short.b.add(
      0x13,
      Buffer.concat([
        short.b.u64(2),
        short.b.hash(shortSeq),
        short.b.hash(fullSeq),
      ]),
    ),
    wrong = short.b.add(
      3,
      Buffer.concat([short.b.u64(4097), short.b.hash(wrongSeq)]),
    );
  const bg = await C.GraphView.create([...short.b.nodes.values()]);
  await bg.verifyComplete(bg.header(short.root));
  await assert.rejects(bg.verifyComplete(bg.header(wrong)), /boundary/);
  const ordinary = {
      t: "array",
      items: [{ t: "json_text", value: { t: "null" } }],
    },
    e = await C.encode(ordinary),
    raw = rawBuilder(),
    nested = raw.add(7, raw.hash(e.ref.root)),
    ng = await C.GraphView.create([...e.nodes, ...raw.nodes.values()]);
  await ng.verifyComplete(e.ref);
  await assert.rejects(ng.verifyComplete(ng.header(nested)), /nonordinary/);
  await assert.rejects(
    C.encode({
      t: "array",
      items: [ordinary, { t: "json_text", value: ordinary }],
    }),
    /descendant/,
  );
});

test("compressed UTF8 key boundaries prove strict order without materializing huge common prefixes", async () => {
  const b = rawBuilder(),
    nullRoot = b.add(0, Buffer.alloc(0)),
    block = b.add(0x10, Buffer.alloc(4096, 120));
  let prefix = b.add(0x12, b.hash(block));
  const power = 20,
    n = 1n << 20n;
  for (let i = 1; i <= power; i++)
    prefix = b.add(
      0x13,
      Buffer.concat([b.u64(1n << BigInt(i)), b.hash(prefix), b.hash(prefix)]),
    );
  function field(tail) {
    const bytes = b.add(0x10, Buffer.from(tail)),
      tailSeq = b.add(0x12, b.hash(bytes)),
      keySeq = b.add(
        0x13,
        Buffer.concat([b.u64(n + 1n), b.hash(prefix), b.hash(tailSeq)]),
      ),
      key = b.add(3, Buffer.concat([b.u64(n * 4096n + 1n), b.hash(keySeq)])),
      field = b.add(0x14, Buffer.concat([b.hash(key), b.hash(nullRoot)]));
    return b.add(0x12, b.hash(field));
  }
  const a = field("a"),
    z = field("b");
  function object(left, right) {
    const seq = b.add(
      0x13,
      Buffer.concat([b.u64(2), b.hash(left), b.hash(right)]),
    );
    return b.add(6, Buffer.concat([b.u64(2), b.hash(seq)]));
  }
  const good = object(a, z),
    reverse = object(z, a),
    duplicate = object(a, a),
    g = await C.GraphView.create([...b.nodes.values()]);
  g.text = () => {
    throw Error("key unexpectedly materialized");
  };
  const result = await g.verifyComplete(g.header(good));
  assert.ok(
    result.validationWork.contextsComputed <= 4 * g.metrics.uniqueNodes,
  );
  assert.ok(BigInt(result.validationWork.transitionByteReads) < 100000n);
  await assert.rejects(g.verifyComplete(g.header(reverse)), /order/);
  await assert.rejects(g.verifyComplete(g.header(duplicate)), /order/);
  const stream = g.renderChunks(g.header(good));
  assert.equal((await stream.next()).value, "{");
  await stream.return();
});

test("large legal virtual extents cost unique contexts, with streaming output charged separately", async () => {
  for (const [kind, power] of [
    ["array", 62],
    ["string", 50],
    ["number", 50],
  ]) {
    const data = sharedGraph(kind, power, true),
      g = await C.GraphView.create(data.nodes);
    const r = await g.verifyComplete(g.header(data.root));
    assert.ok(r.validationWork.contextsComputed <= 4 * g.metrics.uniqueNodes);
    assert.ok(BigInt(r.validationWork.transitionByteReads) < 100000n);
    const iterator = g.renderChunks(g.header(data.root));
    assert.equal((await iterator.next()).value, '"');
    await iterator.return();
  }
});

test("Browser WebCrypto snapshot and nested-view fixes hold across real async digest boundaries", async () => {
  const vm = require("node:vm"),
    context = {
      TextEncoder,
      TextDecoder,
      Uint8Array,
      crypto: crypto.webcrypto,
    };
  vm.createContext(context);
  vm.runInContext(
    fs.readFileSync(path.join(__dirname, "../browser/s-content-v3.js"), "utf8"),
    context,
  );
  const result = await vm.runInContext(
    `(async()=>{const C=SContentV3,e=await C.encode({t:'null'}),g=await C.GraphView.create(e.nodes),r={...e.ref},pending=g.decodedValue(r);r.kind='bool';const decoded=await pending;const ast={t:'array',items:[{t:'json_text',value:{t:'number',token:'1.0'}},{t:'null'}]},encode=C.encode(ast);ast.items.reverse();const a=await encode,ag=await C.GraphView.create(a.nodes);let text='';for await(const s of ag.renderChunks(a.ref))text+=s;return {decoded,text};})()`,
    context,
  );
  assert.equal(result.decoded.t, "null");
  assert.equal(result.text, '["1.0",null]');
});
