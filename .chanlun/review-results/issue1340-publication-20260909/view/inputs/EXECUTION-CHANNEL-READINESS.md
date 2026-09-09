> 仅转换链接的阅读版；[原字节](../../payload/inputs/EXECUTION-CHANNEL-READINESS.md) SHA256 `2a1317488a871ef2c728d13c7645513189016563778c209f0d471e001269c45f`。原稿状态是发生时记录，当前发布范围与限制见归档根 README。

# #1323 / #1340 执行通道只读就绪审查

> 核验时间：2026-09-09T05:21:12.691746+00:00。本报告属于当前图的执行准备输入，不是 TB01 实装验收。
> 只写本报告与同名 JSON；未启动工蜂或容器、未启停服务、未改标签/配置、未读密钥、未迁移 Prime。

## 结论

**暂不把 TB01-A 加入常驻 sandcastle 队列。** 实际 claimer 在独立 clone `/Users/silencehan/Projects/NewChanlun-sandcastle-host`，当前本地 `main` 仍为 `44917408b0f83203fe8eb3ace49172313291ec9c`，而实查远端 `main` 为 `644075a6abcbfcda74815daed68e06eff6ddaf63`。自动更新配置在安装版 plist 和 launchctl 环境里均未找到；该 clone 还有 `uv.lock` 未提交改动。新分支会从该旧本地 `main` 创建，SDK 不替新分支 fetch。

**不必改宿主脏 main，也不必迁移 Prime 服务。** 已安装 Sandcastle SDK 允许从干净隔离 W 指定 `cwd` 与完整 SHA 创建单票沙盒；用临时 one-shot 驱动可保持现役 `prime-agent / deepseek / deepseek-v4-pro`，显式运行两个独立阶段、只留下分支，随后由编排者验证。此路径的 API/解析/基线行为已由现装代码核实，但还没有 fresh 容器与模型往返证据，故状态是“可准备执行，尚未 smoke 验收”。Docker/Colima/镜像现状不支持“沙盒不可行”例外。

常驻 `claimer -> main.mts` 没有调用 harvest；两个工蜂 prompt 都明禁 push/merge/动 main/关票。安装的 launchd、cron、当前进程未发现 harvest 调用。**单独运行 `harvest.sh` 则会自动 push main 并关票，且扫描所有 `sandcastle/issue-*`，不核用户批准或独立评审结论。** 本次执行不能接这个脚本。

## 实际运行面与队列

| 对象 | 当前证据 | 含义 |
|---|---|---|
| claimer | LaunchAgent `com.newchanlun.sandcastle-claimer`，主 PID 31156 / node 31193；cwd 为独立 host；`--live` | 常驻队列当前仍有效，给叶票加标签可能即时从旧基线开工 |
| wayfinder engine | 主 PID 1116 / node 1393；cwd `/Users/silencehan/Projects/NewChanlun`；`--live` | 与 claimer 不在同一 clone |
| 实际 main agent | host `main.mts:36-38,309,326`；两阶段均 `primeAgent(deepseek-v4-pro, {provider: deepseek})` | 交互目录的 Claude 配置不能代表当前 daemon 固定通道 |
| 主机状态 | Colima running；Docker 29.5.2 linux/aarch64；运行容器 0 | 底座目前可查询；未验 provider/auth/bootstrap |
| 镜像 | `sandcastle:newchanlun`；`sha256:bfe1f69550e32dd6ef557c2972621ad010cd2e04c1129ccb7f2488319b9b63d8` | 创建于 2026-08-31，arm64 |
| 专属队列 | 只有 #1237，未 assign；唯一 open blocker #1240；#1235/#1238 已 CLOSED | 当前 runnable=0；#1240 以后关闭则旧票可能立即被拾取 |
| 只读 fresh claimer 探针 | host 执行 `npx tsx .sandcastle/claim-loop.mts --once --dry-run`，exit 0，队列 1、无可拾取、未打印 stale rollback | 没有触发 worker、写日志、回滚认领 |
| worker 日志 | 共 532 事件；最后 2026-08-31T21:19:30.380Z，#1235 `sandbox/closed` | 当前没有已登记运行工蜂；日志目录为两 clone 共享链接 |

claimer 每轮先检查任何 `.sandcastle/main.mts` 进程，再对 `workers.jsonl` 的陈旧 claim 做回滚，最后查全仓 `sandcastle` 队列（limit 20）。判据只是 open、未 assign、原生 open blocker 为零；不核票型、SPEC、祖先图或 `ready-for-agent`。`main.mts` 再独立查询一次队列，并不是按 claimer 上轮返回的指定票号执行，因此“MAX_TICKETS=1”只限制数量，不限制票身份。现状没有旧票可运行；这一结论不能当成之后的单票 allowlist。

## 真实 graphwalker：只读直接子票，不递归

`scripts/wayfinder_engine.mts:584-602` 全仓查询 open `wayfinder:map`（limit 100），对每张 map 只调用一次 `/issues/N/sub_issues`。`:552-570` 无递归；`:428-448` 只把该 map 的**首个已识别、OPEN 且评论批准的 SPEC**的批准态赋给直接孩子。

- SPEC 只按标题开头 `📋 SPEC` 或 `[spec]` 识别；`SPEC：…` 不识别。实查 #1340 标题为 `SPEC：完整生产程序、结构全分类与逐重经营（#1323，R2）`，所以当前机器不读取其批准评论。
- impl 只按标题开头 `[impl]` 识别，不按 `wayfinder:impl` 标签。
- `ready-for-agent` 是自动释放器的输入集合；无 open blocker、未 assign、`[impl]` 且直接 map 的 SPEC 已批准才会自动加 `sandcastle`。
- 已带 `sandcastle` 的票直接归 released；claimer 的实际领取不要求 `ready-for-agent`。聚合票不加 ready 还不够，亦不能加 sandcastle。
- 现有 `1323 -> 1340 -> TB -> 纵片` 可以保留作验收结构，但批准不会递归到纵片。只给深层纵片 ready 会 held。建议保留图的真实层级，以明确的单票 one-shot 驱动已解依赖纵片；不因机器限制把验收聚合假标成 map。
- 当前 26 张 ready 中，唯一标题 `[impl]` 的 #1045 仍被 #1046 阻塞，未发现此次 SPEC 批准即会释放旧 impl 的活体证据。以后重新核查。

## 从干净 W 调现役 main.mts：为何不能直接用

W=`/Users/silencehan/Projects/NewChanlun-1323-spec-publication` 当前分支 `codex/1323-spec-publication-20260909`、HEAD `efc1ddc4d6c015f4f8c6d44f904d704a88308b46`，工作区干净；它与交互仓共用 `/Users/silencehan/Projects/NewChanlun/.git`，W 没有 `node_modules` 或 `.sandcastle/.env`。

1. 实际 host `main.mts:62-79,245` 启动即强制 `cwd` 当前分支为 main；从 W 启动会在取票前退出。
2. 它没有 `--issue`、`--base-ref`、`--dry-run`、`--help` 处理。`baseBranch:'main'` 写死，`pickIssue` 查全队列。**不得执行 `main.mts --help` 当只读探针**；它仍然可能取票、读凭据和开工。
3. SDK 的 `promptFile` 内置 `TARGET_BRANCH` 来自 `getCurrentBranch(hostRepoDir)`，并禁止用 `promptArgs` 覆盖。把 W 强行冒充 main 会丢失正确的审核边界。
4. 包导入按入口模块路径解析，`cwd` 不改 ESM 包解析。host 的 `node_modules` 实际 symlink 到交互仓 `/Users/silencehan/Projects/NewChanlun/node_modules`；W 没有依赖。独立脚本可直接绝对导入已安装的 `dist/index.js`、`dist/sandboxes/docker.js` 和 host 的 `prime-agent-provider.ts`，由已安装 tsx 执行。
5. SDK `cwd` 明确锚定 `.sandcastle/worktrees`、`.sandcastle/.env` 和 git 操作。新 branch 从 `baseBranch` 指定 ref 创建；**已有 branch 会忽略 `baseBranch`**，所以 fresh smoke/首个 dispatch 必须断言分支尚不存在。

## 可执行的最小 fresh smoke 路径（未运行）

先把本次归档合适的**干净、可追溯完整 SHA**钉为基线。下面使用 W 的已提交 HEAD；若 W 后续又有新提交，须由调用者重取并记录，不能沿用本报告旧 SHA。分支用独立 `codex/…` 名称，避免被 `harvest.sh` 的 `sandcastle/issue-*` 扫描器顺带接受。所有工作树落 `/Users`，符合本仓 Colima bind-mount 已知约束。

为让 SDK 读取现役凭据，可在 W 当前缺失的位置创建一个**临时 ignored symlink**，不打印或复制凭据、不修改现役文件。这只是调用者后续执行前置，本次未创建：

```bash
ln -s /Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/.env /Users/silencehan/Projects/NewChanlun-1323-spec-publication/.sandcastle/.env
```

现状检查必须仍是该路径不存在；若已存在，先核它的来源，不覆盖。SDK 只解析 `cwd/.sandcastle/.env` 已声明的键，缺该文件时，单独设置普通宿主环境变量不保证会注入容器。认证是否有效只能由 fresh smoke 判定。

下列脚本可另存临时 `tb01-channel-smoke.mts`。它不调用 main/claimer/harvest、不领票、不使用 gh token，保留当前固定 provider/model，并要求一次真实模型工具读取 repo HEAD。提供的是 **SDK 支持的可执行准备稿**，未启动或宣称通过：

```typescript
import { createSandbox } from '/Users/silencehan/Projects/NewChanlun/node_modules/@ai-hero/sandcastle/dist/index.js';
import { docker } from '/Users/silencehan/Projects/NewChanlun/node_modules/@ai-hero/sandcastle/dist/sandboxes/docker.js';
import { primeAgent } from '/Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/prime-agent-provider.ts';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';

const cwd = '/Users/silencehan/Projects/NewChanlun-1323-spec-publication';
const git = (...args: string[]) => execFileSync('git', args, {cwd, encoding: 'utf8'}).trim();
if (git('status', '--porcelain')) throw new Error('W is dirty');
const base = git('rev-parse', 'HEAD');
const branch = `codex/1323-tb01-channel-smoke-${Date.now()}`;
if (spawnSync('git', ['show-ref','--verify','--quiet',`refs/heads/${branch}`], {cwd}).status === 0)
  throw new Error('branch exists; baseBranch would be ignored');
const evidence = `${cwd}/.sandcastle/logs/${branch.replaceAll('/', '-')}`;
mkdirSync(evidence, {recursive: true});
const sandbox = await createSandbox({
  cwd, branch, baseBranch: base,
  sandbox: docker({imageName: 'sandcastle:newchanlun'}),
});
let summary: unknown;
try {
  const probe = await sandbox.exec('git rev-parse HEAD');
  if (probe.exitCode !== 0 || probe.stdout.trim() !== base) throw new Error('sandbox base mismatch');
  const result = await sandbox.run({
    name: 'tb01-fixed-channel-smoke', maxIterations: 1,
    agent: primeAgent('deepseek-v4-pro', {
      provider: 'deepseek', sessionStorage: {hostSessionsDir: `${evidence}/sessions`},
    }),
    prompt: `当前获批 #1323 / #1340 执行准备只读 smoke。调用一次工具执行 git rev-parse HEAD；预期 ${base}。不修改文件，不提交，不调用 gh，不评论或操作 issue。完成后输出 TB01_FIXED_CHANNEL_OK 和实际 SHA。`,
    completionSignal: 'TB01_FIXED_CHANNEL_OK', idleTimeoutSeconds: 120,
    completionTimeoutSeconds: 30, signal: AbortSignal.timeout(180000),
    logging: {type: 'file', path: `${evidence}/smoke.log`},
  });
  if (result.commits.length || result.completionSignal !== 'TB01_FIXED_CHANNEL_OK')
    throw new Error('smoke missing completion or unexpectedly committed');
  const after = await sandbox.exec('git status --porcelain');
  if (after.exitCode !== 0 || after.stdout.trim()) throw new Error('smoke changed worktree');
  summary = {base, branch, worktreePath: sandbox.worktreePath,
    sessions: result.iterations.map(i => i.sessionId), commits: result.commits,
    completionSignal: result.completionSignal, logFilePath: result.logFilePath};
  writeFileSync(`${evidence}/result.json`, JSON.stringify(summary, null, 2));
  console.log(JSON.stringify(summary));
} finally { await sandbox.close(); }
```

调用使用现装 tsx，不依赖 W 的 node_modules，不在调用前重建镜像或迁移服务：

```bash
node /Users/silencehan/Projects/NewChanlun/node_modules/tsx/dist/cli.mjs /绝对临时路径/tb01-channel-smoke.mts
```

smoke 通过只证明该固定通道此刻可建沙盒、同基线、bootstrap/模型往返/工具执行/会话捕获可用；日志还需核实际 tool invocation，不能把模型写回 marker 当成工具证据。烟测不替代 TB01 的编译、契约、独立评审或生产验收。

## 单票实装 dispatch 的最小约束

无需更新脏宿主 clone。复用上面绝对 SDK/provider 导入，临时驱动仅吃一个已发布**叶票号**和一个完整 `BASE_SHA`。先对该票做 REST read：OPEN、原生 open blocker=0、尚未他人认领、当前无 sandcastle 标签。人工定向认领只改该票 assign（不加 sandcastle），随后在代码里用该常量票号，完全不调用 `pickIssue`。这与常驻全局队列分离，不会触发旧基线常驻领取。

- 创建：`createSandbox({cwd: W, branch: 'codex/1323-tb01-a-<叶票号>', baseBranch: BASE_SHA, sandbox: docker(...)})`。首跑断言分支不存在；续跑必须核已有分支祖先与本票工件后显式恢复，不能假设 baseBranch 会重设。
- 上下文：host 侧获取该叶票、指定父票/已批准 SPEC 的文本；以 inline prompt 传入明确票号/输入/出入口/验收和 BASE_SHA。inline prompt 不执行 `!` 命令扩展，故不得把带 `!` 和未替换 `{{…}}` 的原模板原样当 inline prompt。
- 实装：当前固定 `primeAgent('deepseek-v4-pro', {provider:'deepseek'})`；每轮 `maxIterations:1`，至多沿当前同一 session 显式 resume 3 轮（原 main 的 runWithResume 口径）。只允许该叶片的改动；按原模板保留 commit、必要测试以及禁 push/merge/关票/main 的约束。
- 独立评审：同一沙盒再开**不带 implementer resumeSession 的独立 invocation**，同 provider/model；inline 明确 `git diff BASE_SHA...BRANCH` 与提交列表，避免 SDK TARGET_BRANCH 对 W 名字的隐式绑定。不能把“阶段返回”记成通过；必须核最终评审结论及适用检查，配额/超时/无 commit 留未完成。
- 合入：驱动仅输出 branch、base、commit、session、日志/测试/评审证据后 close。不导入或调用 harvest，不 push main，不关叶票/TB/图。默认分支合入仍是后续人工批准门。
- 临时 registry 应放 W 自己的日志目录，并在当日 roster 明确本票/branch/runtime；不能只向共享 workers 日志写一个 claim/claimed 后长期无 implementer 事件，否则现役看门狗可能把它当 stale 认领，自动加 sandcastle。
- 若需要容器内读 GitHub，沿现役 `resolveSandboxGithubEnv` 的**仅内存** keyring-to-Docker-env 路径加入 GH_REPO/GH_TOKEN；不打印、不写文件。本只读核查与 smoke 稿未读取该 token。

只有坚持复用现役 `main.mts` 原入口才需要另建干净 main clone 或修正宿主基线，并处理单票队列约束。把当前 host 直接 reset 到 origin/main 会顺带把其现役 Prime 脚本换成另一套通道，不能把此行为包装成纯基线更新；本报告建议的 SDK one-shot 不需要这一步，因此没有提供执行 reset 的路径。

## 复核命令与证据边界

已运行只读命令包括：`git status/rev-parse/ls-remote`；安装版 plist 限定字段读取；`lsof -p <pid> -d cwd`；进程名查找；`colima status`；`docker info`；`docker image inspect` 仅 ID/日期/架构；`docker ps`；`gh issue list`；原生 dependencies GET；claimer `--once --dry-run`；现役 main 的 esbuild **内存 transform**（`MAIN_TRANSPILE_OK`，不执行模块）。没有读取任何 `.env` / OAuth token 的内容，没有模型 API 请求。

`main.mts --help` 不是只读入口；`wayfinder_engine --dry-run` 虽不写 tracker 仍写状态页；Sandcastle CLI 0.12.0 的 help 只有 init 与 image 管理，没有可用的 `list/status/run` 子命令，不能照旧文档臆用。fresh smoke 阻碍必须按实际报错记录后再定例外，现在没有其不可行实证。

详细路径、行号、内容 SHA-256、队列与进程快照见 [EXECUTION-CHANNEL-READINESS.json](../../payload/inputs/EXECUTION-CHANNEL-READINESS.json)。
