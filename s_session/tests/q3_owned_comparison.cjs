/* #1789：由本次 Q 启动 owner 调用。同一进程、同配置、逐代对照。 */
"use strict";
const assert = require("node:assert/strict");
const API = require("../browser/tb01c-client.js");
async function compareOwnedQuery({owned, limits, allocationFactory, generations = ["60", "106"], record}) {
  if (typeof allocationFactory !== "function" || typeof record !== "function")
    throw Error("要求具名分配适配器工厂及结果记录函数");
  for (const generation of generations) {
    let baseline = null;
    for (const optimized of [false, true]) {
      const allocation = allocationFactory();
      const client = new API.Client({protocol: "s-session/3", fetch: owned.fetch,
        byteDeployment: optimized ? owned.byteDeployment : null, limits, allocation});
      const started = performance.now();
      try {
        const result = await client.loadReady("AsKnown", generation);
        const row = {generation, optimized, status: "passed", ms: performance.now() - started,
          projection_root: result.projection.ref.root, resources: limits,
          measurement_source: allocation.measurementSource, test_only_allocation: allocation.testOnly === true,
          deployment: owned.evidence(), job: client.lastJob.snapshot()};
        if (!optimized) baseline = row;
        else {
          assert.equal(row.projection_root, baseline.projection_root, "同输入投影根不同");
          assert(BigInt(row.job.requests) < BigInt(baseline.job.requests), "请求数未下降");
          assert(BigInt(row.job.wire_bytes) < BigInt(baseline.job.wire_bytes), "wire未下降");
        }
        await record(row);
      } catch (error) {
        await record({generation, optimized, status: "failed", ms: performance.now() - started,
          error: String(error), job: client.lastJob?.snapshot()});
        throw error;
      }
    }
  }
}
module.exports = {compareOwnedQuery};
