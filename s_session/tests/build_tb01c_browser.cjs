/* #1394 私有静态入口生成/漂移锁，不修改HTTP路由。 */
"use strict";
const fs = require("node:fs"),
  path = require("node:path"),
  crypto = require("node:crypto");
const browser = path.resolve(__dirname, "../browser"),
  target = path.join(browser, "tb01c-client.js");
const begin = "/* #1394 generated browser dependencies begin */",
  end = "/* #1394 generated browser dependencies end */";
const inputs = [
  "s-content-v3.js",
  "s-content-job-v3.js",
  "q3-values.js",
  "q3-center.js",
  "q3-typed.js",
  "q3-consumer.js",
];
function hash(text) {
  return crypto.createHash("sha256").update(text).digest("hex");
}
function expected() {
  const current = fs.readFileSync(target, "utf8"),
    at = current.indexOf(end);
  if (!current.startsWith(begin) || at < 0) throw Error("既有入口缺生成段边界");
  const runtime = current.slice(at + end.length).replace(/^\n+/, "");
  const parts = [begin, "if (typeof module !== 'object' || !module.exports) {"],
    sha = {};
  for (const name of inputs) {
    const text = fs.readFileSync(path.join(browser, name), "utf8");
    sha[name] = hash(text);
    parts.push(
      "/* exact source " + name + " SHA256 " + sha[name] + " */",
      text,
    );
  }
  parts.push("}", end, runtime);
  return { text: parts.join("\n"), inputs: sha, runtime_sha256: hash(runtime) };
}
function run(write = false) {
  const output = expected();
  if (write) fs.writeFileSync(target, output.text);
  else if (fs.readFileSync(target, "utf8") !== output.text)
    throw Error(
      "Browser bundle输入漂移：运行 node s_session/tests/build_tb01c_browser.cjs --write",
    );
  return {
    status: "passed",
    inputs: output.inputs,
    runtime_sha256: output.runtime_sha256,
    bundle_sha256: hash(output.text),
  };
}
if (require.main === module) {
  try {
    if (
      process.argv.length > 3 ||
      (process.argv[2] && !["--check", "--write"].includes(process.argv[2]))
    )
      throw Error("只允许--check或--write");
    console.log(JSON.stringify(run(process.argv[2] === "--write"), null, 2));
  } catch (e) {
    console.error(e.message);
    process.exitCode = 1;
  }
}
module.exports = { run, expected };
