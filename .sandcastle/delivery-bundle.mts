#!/usr/bin/env -S npx tsx
import { readFileSync } from "node:fs";
import { preflightDelivery } from "./delivery-bundle.ts";

const [repo, manifest, ...extra] = process.argv.slice(2);
if (!repo || !manifest || extra.length) {
  process.stderr.write("用法：npx tsx .sandcastle/delivery-bundle.mts <工作树根目录> <仓外交付-manifest.json>\n");
  process.exitCode = 2;
} else {
  try {
    const result = preflightDelivery(repo, JSON.parse(readFileSync(manifest, "utf8")));
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    process.exitCode = result.status === "BLOCKED" ? 1 : 0;
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 2;
  }
}
