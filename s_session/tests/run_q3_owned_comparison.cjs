"use strict";
const fs = require("node:fs"), path = require("node:path");
const {spawn} = require("node:child_process"), {randomUUID} = require("node:crypto");
const {setTimeout: delay} = require("node:timers/promises");
const {ownedDeployment} = require("./q3_owned_deployment.cjs");
const {compareOwnedQuery} = require("./q3_owned_comparison.cjs");
const {measurement} = require("./q3_complete_fixture.cjs");
async function main(args) {
  if (args.length === 1 && args[0] === "--help") {
    console.log("node run_q3_owned_comparison.cjs BINARY DB Q_RESOURCES CLIENT_RESOURCES OUTPUT_DIR PORT EPOCH SOURCE_REF");
    return;
  }
  if (args.length !== 8) throw Error("要求八项显式参数，使用 --help 查看用法");
  const [binary, db, resources, clientResources, output, port, epoch, source] = args;
  if (!/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65535 || !/^[1-9]\d*$/.test(epoch))
    throw Error("要求明确的端口与正整数 epoch");
  for (const file of [binary, db, resources, clientResources])
    if (!path.isAbsolute(file) || !fs.statSync(file).isFile()) throw Error("输入必须为绝对文件路径");
  if (!path.isAbsolute(output)) throw Error("输出目录必须为新建绝对路径");
  const limits = JSON.parse(fs.readFileSync(clientResources, "utf8"));
  fs.mkdirSync(output);
  const endpoint = "http://127.0.0.1:" + port, controlInstanceId = randomUUID();
  const command = ["query-serve", "--protocol-revision", "s-session/3", "--db", db,
    "--resource-config", resources, "--producer-epoch", epoch, "--host", "127.0.0.1", "--port", port];
  fs.writeFileSync(path.join(output, "inputs.json"), JSON.stringify({binary, command, clientResources, source, limits}, null, 2));  const child = spawn(binary, command, {env:{...process.env, S_CONTROL_INSTANCE_ID:controlInstanceId}, stdio:["ignore","ignore","pipe"]});
  const owned = ownedDeployment({child, endpoint, controlInstanceId});
  let childError = null, closed = false;
  const exited = new Promise(resolve => child.once("close", () => {closed = true; resolve();}));
  child.on("error", error => {childError = error;});
  child.stderr.on("data", text => fs.appendFileSync(path.join(output, "q.stderr.log"), text));
  try {
    const until = Date.now() + 10000;
    while (!owned.evidence()) {
      if (childError) throw childError;
      if (closed) throw Error("Q 在冻结启动记录前退出");
      if (Date.now() >= until) throw Error("未在 10 秒内获得本次 Q 的冻结启动记录");
      await delay(20);
    }
    await compareOwnedQuery({owned, limits, allocationFactory:measurement,
      record:row => fs.appendFileSync(path.join(output, "comparison.jsonl"), JSON.stringify(row) + "\n")});
  } finally {
    if (!closed) {
      child.kill("SIGTERM");
      const force = setTimeout(() => child.kill("SIGKILL"), 10000);
      try { await exited; } finally {clearTimeout(force);}
    }
  }
}
if (require.main === module) main(process.argv.slice(2)).catch(error => {console.error(error.message); process.exitCode = 1;});
module.exports = {main};