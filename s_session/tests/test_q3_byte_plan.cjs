"use strict";
const test = require("node:test"), assert = require("node:assert/strict");
const C = require("../browser/s-content-v3.js"), V = require("../browser/q3-values.js");
const P = require("../browser/q3-byte-plan.js");
const {EventEmitter} = require("node:events"), {PassThrough} = require("node:stream");
const {ownedDeployment} = require("./q3_owned_deployment.cjs");
const zero = "0".repeat(64);
const identity = {session_id: "test", session_generation: "1"};
async function input(extent = 10000) {
  const encoded = await C.encode(C.fromPlain("x".repeat(extent)));
  const anchor = {kind: "projection", fixed_cut: identity,
    cut_projection_digest: "qcut3-" + zero, projection_ref: encoded.ref};
  return {anchor, path: [], target: encoded.ref, parents: [], identity,
    instance: "test-client", serial: 9n, pageSize: 31, replyBytes: 1000000,
    maxRequests: "100000", deployment: {replyBytes: "8388608", frameLimit: "1048576", frameBytes: n => n + 200n}};
}
test("缺证明、双端帽逆转与上界过宽均保持旧尺寸", async () => {
  const a = await input();
  assert.equal(P.choose({...a, deployment: null}), null);
  assert.equal(P.choose({...a, parents: null}), null);
  const p = P.choose(a); assert.equal(p.limit, "10000");
  assert.deepEqual(P.choose({...a, replyBytes: 20000}), P.choose({...a,
    deployment: {...a.deployment, replyBytes: "20000"}}));
  assert.equal(P.choose({...await input(1), replyBytes: 3000}), null);
  assert.equal(P.choose({...a, deployment: {...a.deployment, frameLimit: "1"}}), null);
});
test("完整帧计入续片 offset、serial 与 Content-Length 位数", async () => {
  const a = await input(104), frames = [];
  const measured = {...a.deployment, frameBytes: n => { frames.push(n); return n + 200n; }};
  const p = P.choose({...a, deployment: measured});
  assert.equal(p.count, 1n); assert.equal(frames.length, 1);
  const grow = P.choose({...a, serial: 999999999999999999999999n, deployment: measured});
  assert(grow.frameMax > p.frameMax);
  const b = await input(40000);
  const q = P.choose({...b, replyBytes: 40000});
  assert(q.count > 1n);
  assert.equal(q.serialEnd, b.serial + q.count);
});
function fakeChild() {
  const child = new EventEmitter();
  Object.assign(child, {exitCode: null, signalCode: null, killed: false, stderr: new PassThrough()});
  return child;
}
test("本次启动记录绑定 endpoint、Ready nonce、epoch 及进程生命周期", () => {
  const child = fakeChild(), owner = ownedDeployment({child, endpoint: "http://127.0.0.1:9999", controlInstanceId: "nonce"});
  const ready = {control_instance_id: "nonce", producer_epoch: "2", cut: identity};
  const args = {fetch: owner.fetch, ready, identity};
  assert.equal(owner.byteDeployment.snapshot(args), null);
  child.stderr.write('[q3] ' + JSON.stringify({listen: "127.0.0.1:9999", producer_epoch: "2",
    resource_config: {schema_revision: "s-query-resources/3", limits: {max_reply_bytes: "8000", max_frame_bytes: "2000"}}}) + '\n');
  assert.equal(owner.byteDeployment.snapshot(args).replyBytes, "8000");
  assert.equal(owner.byteDeployment.snapshot({...args, fetch: () => {}}), null);
  assert.equal(owner.byteDeployment.snapshot({...args, ready: {...ready, producer_epoch: "3"}}), null);
  assert.equal(owner.byteDeployment.snapshot({...args, ready: {...ready, control_instance_id: "old"}}), null);
  child.emit("exit", 0);
  assert.equal(owner.byteDeployment.snapshot(args), null);
});
test("endpoint 不匹配和重复启动记录均撤销部署证书", () => {
  for (const mismatch of [true, false]) {
    const child = fakeChild(), owner = ownedDeployment({child, endpoint:"http://127.0.0.1:9999", controlInstanceId:"nonce"});
    const startup = {listen:mismatch ? "127.0.0.1:9998" : "127.0.0.1:9999", producer_epoch:"2",
      resource_config:{schema_revision:"s-query-resources/3",limits:{max_reply_bytes:"8000",max_frame_bytes:"2000"}}};
    child.stderr.write("[q3] " + JSON.stringify(startup) + "\n");
    if (!mismatch) child.stderr.write("[q3] " + JSON.stringify(startup) + "\n");
    assert.equal(owner.byteDeployment.snapshot({fetch:owner.fetch,identity,
      ready:{control_instance_id:"nonce",producer_epoch:"2",cut:identity}}), null);
    assert.equal(owner.evidence(), null);
  }
});
test("真实编码 Reader 先取完整父键，固定尺寸并拒绝节点篡改且不重发", async () => {
  const encoded = await C.encode(C.fromPlain({["键".repeat(100)]: "x".repeat(1000)}));
  const graph = await C.GraphView.create(encoded.nodes);
  const anchor = {kind: "projection", fixed_cut: identity,
    cut_projection_digest: "qcut3-" + zero, projection_ref: encoded.ref};
  const requests = [], parents = [];
  let corrupt = false;
  async function request(p) {
    requests.push(p);
    const q = p.content_token || p, ref = graph.resolve(encoded.ref, q.path),
      offset = BigInt(q.offset), end = BigInt(ref.extent) < offset + BigInt(q.limit)
        ? BigInt(ref.extent) : offset + BigInt(q.limit);
    const done = end === BigInt(ref.extent);
    const next = done ? null : {token_schema: "s-content-token/1", anchor, path: q.path,
      target_ref: ref, unit: q.unit, offset: String(end), limit: q.limit,
      issued_capture_digest: zero};
    if (next) next.token_checksum = await C.tokenChecksum(next);
    const nodes = encoded.nodes.map(n => ({...n}));
    if (corrupt && q.unit === "bytes") nodes[0].body_hex += "00";
    return {anchor, path: q.path, target_ref: ref, unit: q.unit, offset: q.offset,
      end: String(end), total: ref.extent, nodes, done, next_token: next,
      validated_capture_digest: zero, ok: true,
      items: q.unit === "items" ? [...graph.entries(ref)].map(e => ({key:e.key,value_ref:e.value_ref}))
        : [...graph.byteChunks(ref, offset, end)].map(b => Buffer.from(b).toString("hex")).join("")};
  }
  const create = () => new V.Reader({anchor, request, check() {}, accept: async () => {}, store: async () => {}, limit:31,
    chooseBytes(x) { parents.push(x.parents); return {limit:"501", request}; }});
  const r = create(); await r.complete(r.value(encoded.ref)); await r.finish();
  assert.equal(parents[0][0].K, 300n);
  const bytes = requests.filter(p => (p.content_token || p).unit === "bytes");
  assert.deepEqual(bytes.map(p => (p.content_token || p).limit), ["501", "501"]);
  corrupt = true; const broken = create(), before = requests.length;
  await assert.rejects(broken.complete(broken.value(encoded.ref)), /node hash/);
  assert.equal(requests.length - before, 2);
});
test("受控 Node HTTP 完整请求帧与实际上线字节相等", async () => {
  const net = require("node:net");
  let captured = Buffer.alloc(0);
  const server = net.createServer(socket => socket.on("data", chunk => {
    captured = Buffer.concat([captured, chunk]);
    const text = captured.toString(), end = text.indexOf("\r\n\r\n");
    if (end >= 0 && captured.length === end + 4 + Number(/Content-Length: (\d+)/i.exec(text)[1]))
      socket.end('HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}');
  }));
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  try {
    const host = "127.0.0.1:" + server.address().port, child = fakeChild();
    const owner = ownedDeployment({child, endpoint: "http://" + host, controlInstanceId: "nonce"});
    child.stderr.write('[q3] ' + JSON.stringify({listen: host, producer_epoch: "2",
      resource_config: {schema_revision: "s-query-resources/3", limits: {max_reply_bytes: "8000", max_frame_bytes: "2000"}}}) + '\n');
    const deployment = owner.byteDeployment.snapshot({fetch:owner.fetch, identity,
      ready:{control_instance_id:"nonce", producer_epoch:"2", cut:identity}});
    const body = "x".repeat(100);
    const response = await owner.fetch("/api/v3/content", {method:"POST", body});
    await response.text();
    assert.equal(BigInt(captured.length), deployment.frameBytes(100n));
  } finally { await new Promise(resolve => server.close(resolve)); }
});
test("优化链保留 serial 区间，交错请求不改变链编号且失败不重发", async () => {
  const {API, CONFIG, measurement} = require("./q3_complete_fixture.cjs");
  const client = new API.Client({protocol:"s-session/3", fetch:async()=>{}, limits:{...CONFIG,replyBytes:40000},
    allocation:measurement(), byteDeployment:{snapshot:()=>({replyBytes:"1000000",frameLimit:"1000000",frameBytes:n=>n+200n})}});
  const b = await client.begin(), a = await input(40000), sent = [];
  b.ready = {producer_epoch: "2"};
  const plan = client.chooseBytes(a, identity, b), reservedEnd = client.serial;
  assert(plan && reservedEnd > 1n);
  client.serial += 1000n;
  client.request = async (_identity, payload, _budget, serial) => {sent.push(serial);return {...identity,producer_epoch:"2",payload};};
  await plan.request({op:"content"}); await plan.request({op:"content"});
  assert.deepEqual(sent,[1n,2n]); assert.equal(client.serial,reservedEnd+1000n);
  client.request = async () => ({...identity, producer_epoch:"3", payload:{}});
  await assert.rejects(plan.request({op:"content"}), /bytes部署纪元已变化/);
  let attempts=0;client.request=async()=>{attempts++;throw Error("transport failed");};
  await assert.rejects(plan.request({op:"content"}),/transport failed/); assert.equal(attempts,1);
});
test("E104 的 limit31到51 同位数仍因续片 offset93到102 增长而计入帧", async () => {
  const a = await input(104), full = P.choose(a);
  const cap = L => Number(full.replyBound - 2n * 104n + 2n * L);
  const old = P.choose({...a,pageSize:30,replyBytes:cap(31n)});
  const next = P.choose({...a,replyBytes:cap(51n)});
  assert.equal(old.limit,"31"); assert.equal(next.limit,"51");
  assert.equal((old.count-1n)*31n,93n);assert.equal((next.count-1n)*51n,102n);
  assert(next.frameMax > old.frameMax);
  assert.equal(P.choose({...a,replyBytes:cap(51n),deployment:{...a.deployment,frameLimit:String(old.frameMax)}}),null);
});
test("已冻结真实 onebar fixture 在同输入 Client 路径保持结果并减少 bytes 请求", async t => {
  const {API, CONFIG, measurement, makeServer} = require("./q3_complete_fixture.cjs");
  const legacy = await makeServer();
  const oldClient = new API.Client({protocol:"s-session/3",fetch:legacy.fetch,limits:CONFIG,allocation:measurement()});
  const oldResult = await oldClient.loadReady("AsKnown","1");
  const server = await makeServer();
  const client = new API.Client({protocol:"s-session/3",fetch:server.fetch,limits:CONFIG,allocation:measurement(),
    byteDeployment:{snapshot:({ready})=>ready ? {replyBytes:"8388608",frameLimit:"1048576",frameBytes:n=>n+200n}:null}});
  const result = await client.loadReady("AsKnown","1");
  assert.equal(result.fixed.cut_generation,"1");
  const reads = server.requests.filter(x=>x.body).map(x=>API.parse(x.body).payload)
    .map(x=>x.content_token||x).filter(x=>x.unit==="bytes");
  assert(reads.some(x=>BigInt(x.limit)>31n));
  assert.equal(result.projection.ref.root, oldResult.projection.ref.root);
  const legacyReads = legacy.requests.filter(x=>x.body).map(x=>API.parse(x.body).payload)
    .map(x=>x.content_token||x).filter(x=>x.unit==="bytes");
  assert(legacyReads.every(x=>x.limit==="31"));
  assert(legacyReads.length > reads.length);
  const oldJob = oldClient.lastJob.snapshot(), job = client.lastJob.snapshot();
  assert(BigInt(job.requests) < BigInt(oldJob.requests));
  assert(BigInt(job.wire_bytes) < BigInt(oldJob.wire_bytes));
  t.diagnostic(JSON.stringify({fixture:"q3-complete-onebar.json", projection_root:result.projection.ref.root,
    baseline:{requests:oldJob.requests,wire_bytes:oldJob.wire_bytes,bytes_requests:legacyReads.length},
    optimized:{requests:job.requests,wire_bytes:job.wire_bytes,bytes_requests:reads.length}}));
});
