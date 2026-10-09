// 监督器自己的 git 不能执行仓库或用户配置里的外部命令。
// 工蜂与管理者共享 HOME，也能写 checkout 的本地配置；这些命令会在
// snapshotCheckout / 交付预检的 git status 里带着管理者环境跑起来。
import { execFileSync } from "node:child_process";

const EXECUTABLE_CONFIG_KEY = /^(?:filter\..+\.(?:clean|smudge|process)|diff\..+\.(?:command|textconv)|diff\.external)$/i;

export function supervisorGitEnv(source: NodeJS.ProcessEnv = process.env): NodeJS.ProcessEnv {
  return {
    ...source,
    GIT_OPTIONAL_LOCKS: "0",
    GIT_CONFIG_GLOBAL: "/dev/null",
    GIT_CONFIG_SYSTEM: "/dev/null",
    GIT_CONFIG_NOSYSTEM: "1",
  };
}

function localExecutableOverrides(root: string, env: NodeJS.ProcessEnv): string[] {
  const raw = execFileSync("git", [
    "--no-replace-objects",
    "-c", "core.fsmonitor=false",
    "-c", "core.pager=cat",
    "-c", "core.hooksPath=/dev/null",
    "-C", root,
    "config", "--local", "--list", "-z",
  ], {
    cwd: root,
    encoding: "buffer",
    stdio: ["ignore", "pipe", "pipe"],
    env,
    maxBuffer: 8 * 1024 * 1024,
  });
  const overrides = ["-c", "core.fsmonitor=false", "-c", "core.pager=cat", "-c", "core.hooksPath=/dev/null"];
  const seen = new Set<string>();
  let start = 0;
  for (let i = 0; i <= raw.length; i++) {
    if (i !== raw.length && raw[i] !== 0) continue;
    const chunk = raw.subarray(start, i);
    start = i + 1;
    const newline = chunk.indexOf(0x0a);
    const key = (newline === -1 ? chunk : chunk.subarray(0, newline)).toString("utf8").toLowerCase();
    if (!key || !EXECUTABLE_CONFIG_KEY.test(key) || seen.has(key)) continue;
    seen.add(key);
    // 命令行空值覆盖本地 clean/process/textconv，属性仍可指向该驱动，但没有命令可执行。
    overrides.push("-c", `${key}=`);
  }
  return overrides;
}

/** 只读 git。忽略全局/系统配置，并覆盖本地可执行过滤器与 fsmonitor。 */
export function supervisorGit(root: string, args: string[], source: NodeJS.ProcessEnv = process.env): Buffer {
  const env = supervisorGitEnv(source);
  return execFileSync("git", [
    "--no-replace-objects",
    "--literal-pathspecs",
    "-c", "core.filemode=true",
    "-c", "diff.ignoreSubmodules=none",
    ...localExecutableOverrides(root, env),
    "-C", root,
    ...args,
  ], {
    cwd: root,
    encoding: "buffer",
    stdio: ["ignore", "pipe", "pipe"],
    env,
    maxBuffer: 64 * 1024 * 1024,
  });
}
