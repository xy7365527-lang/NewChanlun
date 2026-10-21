// #1588 ④：沙盒层拦截本机 cargo / rustc。
// 成因：#1406 实装工蜂在 macOS 宿主上直接跑了 cargo check / cargo test（违「本机禁 cargo」约定）。
// 做法：每次运行在固定临时目录（$TMPDIR/nc-cargo-guard，原地刷新）生成同名垫片（cargo / rustc /
// rustup / cargo-* 等），前置到工蜂进程 PATH；
// 调用即打印 nc-remote 指引并以 LOCAL_CARGO_BLOCKED_EXIT 退出。nc-remote 本地只调 git/scp/ssh，
// cargo 在远端 WSL 里由 `source ~/.cargo/env` 解析，不受本机 PATH 影响。
// 另写一份 CLAUDE_ENV_FILE（Claude Code 每条 Bash 命令前 source），login shell 的 path_helper
// 若把 /opt/homebrew/bin 重排到前面，这里再把垫片目录顶回最前。
// 边界：用绝对路径（/opt/homebrew/bin/cargo）调用拦不住——那属于明知故犯，由提示词明令禁止。
import { chmodSync, lstatSync, mkdirSync, renameSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";

export const LOCAL_CARGO_BLOCKED_EXIT = 87;

/** 被垫的工具名：cargo 本体 + rustup（`rustup run <tc> cargo` 可绕 cargo 垫片）+ 编译器 + 常见 cargo 子命令二进制。 */
export const GUARDED_TOOLS = [
  "cargo",
  "rustc",
  "rustup",
  "rustdoc",
  "clippy-driver",
  "cargo-clippy",
  "cargo-fmt",
  "cargo-miri",
  "cargo-nextest",
  "cargo-llvm-cov",
] as const;

export const NC_REMOTE = "~/.local/bin/nc-remote";

export function guardScript(tool: string): string {
  return `#!/bin/sh
# #1588 本机 cargo 垫片（.sandcastle/cargo-guard.mts 生成，勿手改）。
cat >&2 <<'EOF'
[nc-local-cargo-guard #1588] 本机（macOS 宿主）禁止运行 ${tool}：本仓约定本机不跑 cargo build/test/check 等重型验证。
Rust 验证二选一：
  1) 远端（windows-5090 WSL）：先 git commit，再
       NC_REPO="$(git rev-parse --show-toplevel)" NC_TARGET=<工蜂唯一名，如 sandcastle-issue-N> \\
         ${NC_REMOTE} <已提交的 ref，如 sandcastle/issue-N> rust cargo check --locked
     日志拷回 /tmp/nc-remote/；ref 必须是已提交的提交。
  2) 推分支开 PR 走 CI（工蜂不 push 时交编排层）。
不要用绝对路径绕过本垫片。
EOF
exit ${LOCAL_CARGO_BLOCKED_EXIT}
`;
}

export interface CargoGuard {
  /** 垫片 bin 目录（前置到 PATH）。 */
  binDir: string;
  /** Claude Code CLAUDE_ENV_FILE：每条 Bash 命令前 source，把 binDir 顶回 PATH 最前。 */
  envFile: string;
}

// #1588 N4：垫片目录复用固定路径（每次运行原地重写），不再每次 mkdtemp 一份、长期 claim-loop 下
// 在 $TMPDIR 越积越多。不选「退出时删除」：main.mts 异常退出时 noSandbox 工蜂子进程可能还活着，
// 删掉垫片目录它的 cargo 就落回 /opt/homebrew/bin（拦截失效）；固定路径永不删除，无此窗口。
// 内容只取决于路径本身，多个 main.mts 并发写出的字节相同；每个文件先写临时名再 rename，
// 正在运行的工蜂不会读到写了一半的垫片。
export const CARGO_GUARD_DIR_NAME = "nc-cargo-guard";

export function defaultCargoGuardRoot(): string {
  return join(tmpdir(), CARGO_GUARD_DIR_NAME);
}

/** 原子写：同目录临时名写全 + chmod 后 rename 覆盖。 */
function writeFileAtomic(path: string, content: string, mode: number): void {
  const tmp = `${path}.${process.pid}.tmp`;
  writeFileSync(tmp, content, { mode });
  chmodSync(tmp, mode);
  renameSync(tmp, path);
}

/** 固定路径的垫片根目录须是当前用户自己的真实目录（/tmp 共享时防他人预占或软链劫持）。 */
function assertOwnDirectory(dir: string): void {
  const st = lstatSync(dir);
  const uid = process.getuid?.();
  if (!st.isDirectory() || st.isSymbolicLink() || (uid !== undefined && st.uid !== uid)) {
    throw new Error(`[FAIL-loud #1588] cargo 垫片目录 ${dir} 不是当前用户的真实目录（软链或属主不符），拒绝使用`);
  }
}

/** 生成 / 刷新垫片目录（root 缺省为固定路径 defaultCargoGuardRoot()，重复调用原地覆盖、不新建目录）。 */
export function installCargoGuard(root: string = defaultCargoGuardRoot()): CargoGuard {
  const binDir = join(root, "bin");
  mkdirSync(binDir, { recursive: true, mode: 0o700 });
  assertOwnDirectory(root);
  assertOwnDirectory(binDir);
  for (const tool of GUARDED_TOOLS) writeFileAtomic(join(binDir, tool), guardScript(tool), 0o755);
  const envFile = join(root, "claude-env.sh");
  writeFileAtomic(
    envFile,
    `# #1588 本机 cargo 垫片：保持垫片目录在 PATH 最前\nexport PATH=${shQuote(binDir)}:"$PATH"\n`,
    0o644,
  );
  return { binDir, envFile };
}

/** 工蜂进程环境增量：PATH 前置垫片目录 + CLAUDE_ENV_FILE + 显式标记。 */
export function cargoGuardEnv(guard: CargoGuard, basePath: string = process.env.PATH ?? ""): Record<string, string> {
  return {
    PATH: basePath ? `${guard.binDir}${delimiter}${basePath}` : guard.binDir,
    CLAUDE_ENV_FILE: guard.envFile,
    NC_LOCAL_CARGO_GUARD: "1",
  };
}

function shQuote(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`;
}
