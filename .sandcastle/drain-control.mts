import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

export const DRAIN_ROOT = "/Volumes/AgentStorage/newchanlun-legacy-drain";
const HELPER = fileURLToPath(new URL("./drain-control.py", import.meta.url));

export class DrainFrozenError extends Error {}

export function drainRequested(root = DRAIN_ROOT): boolean {
  try {
    const state = JSON.parse(readFileSync(`${root}/state.json`, "utf8"));
    return state?.v !== 1 || !Number.isInteger(state.generation) || state.generation < 0 || state.mode !== "open";
  } catch (error) {
    return (error as NodeJS.ErrnoException).code !== "ENOENT";
  }
}

export async function withLegacyAdmission<T>(action: () => T | Promise<T>, root = DRAIN_ROOT): Promise<T> {
  return withAdmission(action, [HELPER, "hold", "--root", root]);
}

export async function withAdmission<T>(action: () => T | Promise<T>, args: string[]): Promise<T> {
  const child = spawn("python3", args, { stdio: ["pipe", "pipe", "pipe"] });
  let errors = "";
  child.stderr.on("data", (data) => { errors += String(data); });
  const ended = new Promise<number | null>((resolve, reject) => {
    child.once("error", reject);
    child.once("exit", resolve);
  });
  try {
    const admitted = await new Promise<boolean>((resolve, reject) => {
      let output = "";
      child.stdout.on("data", (data) => {
        output += String(data);
        if (output.includes("\n")) resolve(output.trim() === "admitted");
      });
      child.once("error", reject);
      child.once("exit", () => resolve(false));
    });
    if (!admitted) throw new DrainFrozenError(errors.trim() || "旧入口已冻结，未新增动作");
    return await action();
  } finally {
    child.stdin.end();
    await ended;
  }
}
