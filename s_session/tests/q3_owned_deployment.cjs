/* #1789：本次 Q 子进程的私有测试装配，资源来自启动 stderr。 */
"use strict";
const http = require("node:http"), {Readable} = require("node:stream");
function ownedDeployment({child, endpoint, controlInstanceId}) {
  const url = new URL(endpoint);
  if (url.protocol !== "http:" || url.pathname !== "/" || url.search || url.username || url.password)
    throw Error("受控传输要求无附加路径或凭据的 HTTP endpoint");
  let startup = null, pending = "", invalid = false;
  const live = () => !invalid && child.exitCode === null && child.signalCode === null && !child.killed;
  child.once("exit", () => { invalid = true; });
  child.once("error", () => { invalid = true; });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", text => {
    pending += text;
    if (pending.length > 65536) { invalid = true; pending = ""; return; }
    for (let end; (end = pending.indexOf("\n")) >= 0;) {
      const line = pending.slice(0, end); pending = pending.slice(end + 1);
      if (!line.startsWith("[q3] ")) continue;
      try {
        if (startup) { invalid = true; continue; }
        startup = JSON.parse(line.slice(5));
        if (startup.listen !== url.host) invalid = true;
      } catch { invalid = true; }
    }
  });
  const headers = bytes => ({Host: url.host, "Content-Type": "application/json",
    "Content-Length": String(bytes), Connection: "close"});
  const frameBytes = bytes => bytes + BigInt(Buffer.byteLength(
    "POST /api/v3/content HTTP/1.1\r\n" + Object.entries(headers(bytes))
      .map(([k, v]) => k + ": " + v + "\r\n").join("") + "\r\n"));
  function fetch(path, options = {}) {
    if (!live()) return Promise.reject(Error("本次 Q 子进程已退出"));
    if (!/^\/api\/(?:ready|v3\/[a-z]+)$/.test(path)) return Promise.reject(Error("未知 Q 路径"));
    const body = options.body || "", bytes = Buffer.byteLength(body);
    return new Promise((resolve, reject) => {
      const req = http.request(new URL(path, url), {method: options.method || "GET",
        headers: headers(bytes), setHost: false, agent: false, signal: options.signal}, res => {
        resolve(new Response(Readable.toWeb(res), {status: res.statusCode}));
      });
      const exited = () => req.destroy(Error("本次 Q 子进程已退出"));
      child.once("exit", exited);
      req.once("close", () => child.removeListener("exit", exited));
      req.on("error", reject);
      req.end(body);
    });
  }
  const byteDeployment = Object.freeze({snapshot({fetch: candidate, ready, identity}) {
    if (candidate !== fetch || !live() || !startup || !ready ||
      ready.control_instance_id !== controlInstanceId || !controlInstanceId ||
      ready.producer_epoch !== startup.producer_epoch ||
      ready.cut.session_id !== identity.session_id ||
      ready.cut.session_generation !== identity.session_generation) return null;
    if (startup.resource_config?.schema_revision !== "s-query-resources/3") return null;
    const resources = startup.resource_config.limits;
    if (!["max_reply_bytes", "max_frame_bytes"].every(k =>
      typeof resources?.[k] === "string" && /^[1-9][0-9]*$/.test(resources[k]))) return null;
    return Object.freeze({replyBytes: resources.max_reply_bytes,
      frameLimit: resources.max_frame_bytes, frameBytes});
  }});
  function evidence() {
    if (!live() || !startup) return null;
    return {endpoint: url.origin, control_instance_id: controlInstanceId,
      startup: JSON.parse(JSON.stringify(startup))};
  }
  return Object.freeze({fetch, byteDeployment, evidence});
}
module.exports = {ownedDeployment};
