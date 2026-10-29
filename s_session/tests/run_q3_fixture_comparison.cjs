/* #1789：冻结真实 S3 W 夹具上的 bytes 分页双臂对照（JS 请求 oracle，不是活 Q 部署）。
 * 一次进程只读一代一臂，结果追加一行 JSON。重型窗口逐个独立进程串行跑：
 *   node --max-old-space-size=16000 s_session/tests/run_q3_fixture_comparison.cjs <60|106> <baseline|optimized|unbound> <out.jsonl>
 * baseline 无部署证书；optimized 用与 onebar 对照同一张模拟证书（帽取 a5s6 q3-frozen.json 的 max_reply_bytes 与
 * max_frame_bytes）；unbound 是认证负控：证书经 ownedDeployment 但启动记录 endpoint 不符，必须与 baseline 逐项相同。 */
"use strict";
const fs = require("node:fs"), {EventEmitter} = require("node:events"), {PassThrough} = require("node:stream");
const {API, CONFIG, measurement, makeServer, makeS16Base} = require("./q3_complete_fixture.cjs");
const {ownedDeployment} = require("./q3_owned_deployment.cjs");
const FILE = "q3-real-s3-w-recover-revise.json";
// 与 test_q3_s3_real_cut.cjs REAL_CONFIG 相同：两臂同一份 Client 限额。
const LIMITS = {...CONFIG, timeoutMs: 4 * 3600 * 1000,
  job_limits: {...CONFIG.job_limits, max_elapsed_ms: String(4 * 3600 * 1000), max_total_wire_bytes: "2993377103"}};
const CERT = {replyBytes: "8388608", frameLimit: "1048576", frameBytes: n => n + 200n};

function owned(listen) {
  const child = new EventEmitter();
  Object.assign(child, {exitCode: null, signalCode: null, killed: false, stderr: new PassThrough()});
  const owner = ownedDeployment({child, endpoint: "http://127.0.0.1:9999", controlInstanceId: "test-controller"});
  child.stderr.write("[q3] " + JSON.stringify({listen, producer_epoch: "1",
    resource_config: {schema_revision: "s-query-resources/3", limits: {max_reply_bytes: CERT.replyBytes, max_frame_bytes: CERT.frameLimit}}}) + "\n");
  return owner;
}
// chooseBytes 吞掉证书内部异常，故负控的判定记在 control 上，由 main 在读完后核验。
function deploymentFor(arm, control) {
  if (arm === "baseline") return null;
  if (arm === "optimized") return {snapshot: ({ready}) => ready ? CERT : null};
  if (arm !== "unbound") throw Error("未知臂 " + arm);
  // 同一 Ready 下仅启动记录 listen 不同：孪生证书必须成立，负控证书必须为 null，否则负控不纯。
  const twin = owned("127.0.0.1:9999"), bad = owned("127.0.0.1:9998");
  return {snapshot(args) {
    control.calls++;
    if (args.ready && twin.byteDeployment.snapshot({...args, fetch: twin.fetch})) control.twin++;
    const cert = bad.byteDeployment.snapshot({...args, fetch: bad.fetch});
    if (cert) control.leaked++;
    return cert;
  }};
}

async function main([generation, arm, out]) {
  if (!["60", "106"].includes(generation) || !out) throw Error("用法：<60|106> <baseline|optimized|unbound> <out.jsonl>");
  const server = await makeServer(await makeS16Base(generation, FILE));
  const control = {calls: 0, twin: 0, leaked: 0};
  const client = new API.Client({protocol: "s-session/3", fetch: server.fetch, limits: LIMITS,
    allocation: measurement(), byteDeployment: deploymentFor(arm, control)});
  const started = Date.now();
  const result = await client.loadReady("AsKnown", generation);
  const elapsed = Date.now() - started;
  if (result.fixed.cut_generation !== generation) throw Error("读到的代不符");
  const reads = server.requests.filter(x => x.body).map(x => API.parse(x.body).payload).map(x => x.content_token || x);
  const limits = {};
  for (const r of reads.filter(x => x.unit === "bytes")) limits[r.limit] = (limits[r.limit] || 0) + 1;
  const job = client.lastJob.snapshot();
  if (arm === "unbound" && !(control.calls > 0 && control.twin === control.calls && control.leaked === 0))
    throw Error("认证负控不纯：" + JSON.stringify(control));
  if (arm === "optimized" && !Object.keys(limits).some(l => BigInt(l) > 31n)) throw Error("优化臂未启用证书");
  if (arm !== "optimized" && Object.keys(limits).some(l => l !== "31")) throw Error("无证书臂改变了尺寸");
  const row = {fixture: FILE, evidence: "frozen-fixture-js-oracle", generation, arm,
    projection_root: result.projection.ref.root, http_requests: server.requests.length,
    bytes_requests: reads.filter(x => x.unit === "bytes").length, bytes_limits: limits,
    control: arm === "unbound" ? control : null,
    job, elapsed_ms: elapsed, rss: process.memoryUsage().rss, node: process.version};
  fs.appendFileSync(out, JSON.stringify(row) + "\n");
  console.log(JSON.stringify({generation, arm, requests: job.requests, wire_bytes: job.wire_bytes,
    bytes_requests: row.bytes_requests, root: row.projection_root, elapsed_ms: elapsed}));
}
main(process.argv.slice(2)).catch(e => { console.error(e); process.exitCode = 1; });
