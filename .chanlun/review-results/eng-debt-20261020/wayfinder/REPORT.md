# #1588 工程债分片 wayfinder：证据与处置

> 名分：工作草稿（review-results），活期绑定 #1588。
> 分支：`eng-debt-1588/wayfinder`（基于 origin/main @ `df6e680930`）。
> 本机未跑 cargo（本分片零 Rust 改动）；TS 测试本机 `npx tsx --test` 跑（node_modules 临时 symlink 自 sandcastle-host，跑完即删，未入仓）。

## W-1 wayfinder_engine 常驻壳硬编码用户工作区

**现象**：`.sandcastle/run-wayfinder-engine.sh:11` 写死 `REPO=/Users/silencehan/Projects/NewChanlun`；仓内 plist 模板
（`.sandcastle/launchd/com.newchanlun.wayfinder-engine.plist`）的 ProgramArguments / WorkingDirectory / 日志路径、
`install-wayfinder-engine.sh:5` 的 `PLIST_SRC` 同样指用户工作区。launchd 因此执行的是**用户工作区当前检出分支上的引擎**。

**复现（只读）**：

```
$ plutil -p /Volumes/AgentStorage/wayfinder-engine.plist.bak-20261020   # 主控改前的备份
  ProgramArguments => [ "/bin/bash", "/Users/silencehan/Projects/NewChanlun/.sandcastle/run-wayfinder-engine.sh", "--live" ]
  WorkingDirectory => "/Users/silencehan/Projects/NewChanlun"
$ git -C /Users/silencehan/Projects/NewChanlun rev-parse --abbrev-ref HEAD
codex/1523-1525-launcher-riskgate-v1
$ git -C /Users/silencehan/Projects/NewChanlun rev-list --count HEAD..origin/main
167
$ git -C /Users/silencehan/Projects/NewChanlun diff --stat HEAD origin/main -- scripts/wayfinder_engine.mts
 scripts/wayfinder_engine.mts | 246 ++++++++++++++++++++++++++++++++++++++-----
```

用户工作区落后 main 167 个提交、引擎差 246 行（含 #1578 修复），另有 683 个未提交改动——launchd 跑的从来不是 main 上的引擎。

**根因**：常驻服务的宿主取了「交互主 worktree」，而该 worktree 是用户随手切分支的工作区；脚本与模板又把路径写死，换宿主只能逐处手改（#1578 合入 main 后没人改 plist，修复即「锁打好了没接进 gate」）。同仓 `run-claim-loop.sh` / `install-sandcastle-claimer.sh` 早已按脚本位置推导 + 模板指 sandcastle-host（#1182 后续），wayfinder 这一套没跟。

**处置 = 修**（对齐 claimer 先例）：

- `run-wayfinder-engine.sh`：`REPO="${SANDCASTLE_HOST_REPO:-$(cd "$SCRIPT_DIR/.." && pwd)}"`（变量名沿用 `run-claim-loop.sh`）；另把 `$HOME/.local/bin` 前置进 PATH（W-2 起草子代理调 `claude`，launchd PATH 不含它；主控现行 plist 也是这么补的）。
- plist 模板：五处路径改指 `/Users/silencehan/Projects/NewChanlun-sandcastle-host`，PATH 补 `~/.local/bin`，注释写明缘由。`plutil -lint` → OK。
- `install-wayfinder-engine.sh`：`PLIST_SRC` 按脚本位置取（`install-sandcastle-claimer.sh` 同款）。
- `.sandcastle/README.md`：wayfinder 段补「宿主仓 = sandcastle-host」「起草子代理模型口径」两段，文件地图行补推导说明。

**验证**：

```
$ bash -n .sandcastle/run-wayfinder-engine.sh && bash -n .sandcastle/install-wayfinder-engine.sh && bash -n .sandcastle/watcher.sh   → 通过
# 推导逻辑抽出到 /tmp/wf-derive/.sandcastle/ 下单测（不跑引擎）：
  cwd=/ 调用 → REPO=/tmp/wf-derive；SANDCASTLE_HOST_REPO=/x/override → REPO=/x/override；cwd=仓根相对调用 → REPO=/tmp/wf-derive
# 真壳从 /tmp 调起一轮 dry-run（只读 gh，不写 tracker）：
$ cd /tmp && bash <工位>/.sandcastle/run-wayfinder-engine.sh --once --dry-run   → exit=0
[wayfinder_engine] 启动：dry-run=true poll=240s orchestrator=xy7365527-lang spec 起草子代理=claude-cliproxy/claude-opus-5-5
  open 的 wayfinder:map 共 4 张 …… 扫描集 145 张：可拾取 0 / blocked 103 / claimed 6 / running 0 / 待释放 0 / 不放行 36
[wayfinder_engine] 一轮结束
（状态页落在工位 .sandcastle/logs/，证明 cwd 由脚本位置推导而非调用方 cwd）
```

**未做（交主控）**：本分片不改 `~/Library/LaunchAgents`、不 launchctl。主控现行 plist 是 `bash -c "cd …sandcastle-host && … exec npx tsx …"` 内联形态，功能等价；合入后若想回到仓内模板，在 sandcastle-host 跑 `bash .sandcastle/install-wayfinder-engine.sh install` 即可（会 unload/load）。现行 plist 的 StandardOut/ErrPath 仍写 `/Users/silencehan/Projects/NewChanlun/.sandcastle/logs/…`——实测 `sandcastle-host/.sandcastle/logs` 是指向用户工作区 logs 的 symlink（`readlink` → `/Users/silencehan/Projects/NewChanlun/.sandcastle/logs`），落点相同，无功能差异。

## W-2 两个用户工作区 plist 的核查（只读，未改、未 launchctl）

### sandcastle-watcher（`~/Library/LaunchAgents/com.newchanlun.sandcastle-watcher.plist`）

```
ProgramArguments => [ "/bin/bash", "/Users/silencehan/Projects/NewChanlun/.sandcastle/watcher.sh" ]   StartInterval => 600
```

- **读**：`.sandcastle/logs/workers.jsonl`、`.sandcastle/logs/main-loop-*.log`；**写**：`.sandcastle/logs/STATUS.md`（logs 目录两 worktree 共用一份，见上 symlink，数据面无差别），以及**仓内受版本控制的** `.chanlun/agent-roster-2026-08-16.md`（Python 每轮 `open(...,"w")` 整文件重写）。
- **在用户工作区运行的后果**：① 执行的是用户检出分支上的 watcher.sh——和 W-1 同一类病：main 上对 watcher.sh 的修复不生效（当前该文件恰好与 main 无差异，属侥幸）；② 每 10 分钟重写用户工作区里一个受控文件。实测本轮该写为字节等值（roster 里已无匹配 `| 沙盒工蜂 #N … running（2026-08-16/17 派生）|` 的行；workers.jsonl 76 张票均不命中），所以目前不产生脏改动，但用户切到 roster 内容不同的分支时会把该分支的 roster 按 registry 改写成脏文件。
- **结论 = 应改指 sandcastle-host**。依据：数据面经 symlink 共用，改宿主不丢数据；代码面改为跟 main。仓内 `watcher.sh:7` 同类硬编码已一并改为按脚本位置推导（`SANDCASTLE_HOST_REPO` 可覆盖）——合入后只需主控把 plist 的脚本路径换成 `/Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/watcher.sh`。
- **残留风险（建议转票）**：roster 回写是对受控文件的常驻写入，换到 sandcastle-host 后会落在 sandcastle-host 的工作树上（若真发生改写会使其变脏，可能挡 `git pull --ff-only`）。该 roster 是 2026-08-16 的日期快照、匹配行已清空，回写段实际已是死代码；建议另票退役 roster 回写段（或改写到 logs/ 下的非受控文件）。仓内无 watcher 的 plist 模板。

### issue933-sampler（`~/Library/LaunchAgents/com.newchanlun.issue933-sampler.plist`）

```
ProgramArguments => [ "/usr/bin/python3", "/Users/silencehan/Projects/NewChanlun/analysis/data_cache/issue933/sampler.py" ]
StandardOut/ErrPath => …/analysis/data_cache/issue933/sampler.log   StartInterval => 7200   RunAtLoad => false
```

- **脚本做什么**：每 2 小时对 BingX / Hyperliquid 公开 L2 端点各取 12 个标的快照（免签名、不下单），追加到 `sampler.py` 同目录的 `samples.jsonl`（路径由 `__file__` 推导）。不读仓内任何代码、不碰 git。
- **入仓状态**：`git check-ignore -v` → `.gitignore:82:analysis/data_cache/*`，`sampler.py` 与数据都不在版本控制里，只存在于用户工作区；`/Users/silencehan/Projects/NewChanlun-sandcastle-host/analysis/data_cache/issue933` 不存在。
- **结论 = 不应改指 sandcastle-host**：改过去会直接找不到脚本；它不依赖用户工作区的检出分支，不存在 W-1 式的「跑旧代码」问题；用户工作区在这里只是数据存放处。仓内无对应脚本，无可修。
- **但发现另一问题（建议主控裁）**：#933 已于 2026-08-18T06:37:01Z 关闭（`gh issue view 933` → CLOSED），采样器仍在跑——`sampler.log` 末行 `2026-10-20T14:19:29+00:00 采样 24 条`，`samples.jsonl` 已 5.8 MB 且持续增长。建议：若 #933 的后续（map #931）不再需要该序列，unload 并归档数据；若还要，把脚本与数据迁到 `/Volumes/AgentStorage` 并登记归属票。

## W-3 spec 起草子代理默认 deepseek（违「子代理只用 Claude 家族」）

**现象**：`scripts/wayfinder_engine.mts:51-52` `DEFAULT_SPEC_MODEL = "deepseek-v4-pro"`、`DEFAULT_SPEC_PROVIDER = "deepseek"`；`runSpecDrafter` 经 `prime-agent --provider deepseek` 起草 spec。图清时自动派出的子代理默认非 Claude，env 可任意覆盖、无校验。

**先核 prime-agent 能否走 CLIProxyLocal + Claude**（只读，密钥未打印）：

```
$ prime-agent --version → 0.10.0
$ python3 读 ~/.prime/agent/models.json（只打 provider 名/baseUrl/api/型号）：
  cliproxy-local | baseUrl= http://127.0.0.1:8318/v1 | api= openai-responses | models= ['gpt-6-astra', 'mf/claude-opus-5-5', 'yd/claude-opus-5-5']
$ prime-agent model list claude
  anthropic  claude-fable-5-1 / claude-opus-5-5 …（anthropic provider = 直连官方，不过 CLIProxy）
  cliproxy-local  mf/claude-opus-5-5 / yd/claude-opus-5-5
```

判定：prime-agent 有 cliproxy-local provider，但只登记了**带上游前缀**的 `mf/`/`yd/claude-opus-5-5`（钉死上游、绕开 CLIProxy 自动路由），没有无前缀 `claude-opus-5-5`，没有 `claude-fable-5-1`，且走 openai-responses 转译；该登记只在本机 `~/.prime/agent/models.json`、不在仓内。⟹ **不满足**，按要求改用 `.sandcastle/claude-route.mts` 同款：Claude Code CLI + `cliProxyEnv()`（`ANTHROPIC_BASE_URL=http://127.0.0.1:8318` + `access.api-keys` 首项作 `ANTHROPIC_AUTH_TOKEN`，只进子进程 env）。

**处置 = 修**：

- 默认 `DEFAULT_SPEC_PROVIDER = "claude-cliproxy"`、`DEFAULT_SPEC_MODEL = "claude-opus-5-5"`；白名单 `CLAUDE_FAMILY_SPEC_MODELS = ["claude-opus-5-5", "claude-fable-5-1"]`。
- 纯函数 `resolveSpecDrafter(env)`：provider ≠ `claude-cliproxy` 或 model 不在白名单 → throw `[FAIL-loud #1588]`，不回退默认；`parseArgs` 启动即调用（错配在启动时就炸，不等图清）。
- 纯函数 `specDrafterCommand(model)` → `claude -p --model <m> --output-format text --tools "" --no-session-persistence`（无工具、不落会话）；`runSpecDrafter` 用 `execFileSync`（不经 shell），prompt 走 stdin，`cwd = os.tmpdir()`（不让仓内 CLAUDE.md / 项目 hooks 混进起草上下文）。
- 白名单取「claude-opus-5-5 / claude-fable-5-1」两项字面，而非 `^claude-` 前缀：用户规定的括号即列举；`claude-haiku-4-5`、`mf/claude-opus-5-5` 一并被拒（测试覆盖）。若主控认为应放宽，改这一个常量即可。

**验证**：

```
$ npx tsx --test scripts/wayfinder_engine.test.mts
# tests 42  # pass 42  # fail 0          （原 37 + 新 5：默认值 / fable 可配 / 非 Claude 模型被拒 / 非 claude-cliproxy provider 被拒 / CLI 参数形状）
$ npx tsx --test scripts/wayfinder_engine.shell.test.mts
# tests 2  # pass 2  # fail 0            （假 gh 注入 timeout + 空队列，引擎带新 import 正常走完一轮）
$ WAYFINDER_SPEC_MODEL=deepseek-v4-pro npx tsx scripts/wayfinder_engine.mts --once --dry-run
Error: [FAIL-loud #1588] WAYFINDER_SPEC_MODEL=deepseek-v4-pro 不属于 Claude 家族白名单（claude-opus-5-5 / claude-fable-5-1）   exit=1
# 真路由冒烟一次（本机 CLIProxyLocal，非测试内；同一 specDrafterCommand + cliProxyEnv，提示「只回复 OK」）：
model=claude-opus-5-5 out="OK"
```

测试不触网、不含密钥：新测只测纯函数；`claude-route.mts` 的 import 只在调用 `cliProxyEnv()` 时读配置，测试不调用。

**CI 面说明**：`scripts/wayfinder_engine*.test.mts` 不在 `.github/workflows/ci.yml` 任何 job 里（`grep -n "tsx" .github/workflows/ci.yml` 无命中；`npm run typecheck` 的 tsconfig 只含 `.sandcastle/*`），PR 的 CI 绿不覆盖本分片的 TS 测试——上面本机跑的结果是唯一测试证据。建议转票把这两份测试接进 CI（形态一「锁没接进 gate」）。

## W-4 全仓其它「子代理默认非 Claude 家族」清单（只报告，不在本分片范围）

| 位置 | 现状 | 性质 |
|---|---|---|
| `.sandcastle/main-swarm.mts:17-21` | 蜂群 planner/implementer/reviewer/merger 四角色全 `k3`，`PROVIDER = "kimi-coding"`，经 prime-agent | **默认非 Claude**（#1002 时代常量，#1547 只迁了 main.mts） |
| `.sandcastle/image/prime-agent-settings.json` | `defaultProvider: kimi-coding`、`defaultModel: k3` | 镜像内 prime-agent 默认非 Claude |
| `.sandcastle/agent-workflows/shared/agent.ts:88-93` + `.github/workflows/agent-implement.yml:150` / `agent-implement-pr.yml:91` / `agent-review.yml:79` | 默认 Claude；打 `agent:model:deepseek-v4-pro` 标签即改走 prime-agent deepseek（`DEEPSEEK_API_KEY`） | 非默认，但保留了非 Claude 的显式通道 |
| `.sandcastle/agent-workflows/shared/agent.ts:41-49` | `AGENT_MODELS` 默认 `claude-sonnet-4-6` / `claude-opus-4-8` | Claude 家族但不在用户列举的 opus-5-5 / fable-5-1 内 |
| `.sandcastle/prime-agent-provider.ts:221` | 报错提示文案 `pass { provider: 'deepseek' }` | 仅文案 |
| `docs/agents/wayfinder-workflow.md:125` | 「工蜂 prime-agent」 | 文档口径已过时（#1547 后 main.mts 工蜂为 claudeCode） |

`.sandcastle/main.mts`（`claude-opus-5-5`）、`decision-claimer.mts`（fable-5-1 / opus-5-5）、`codex-child.ts`（`claude-opus-5-5`）合规。

## W-5 同类硬编码用户工作区路径（只报告）

`git grep -n "/Users/silencehan/Projects/NewChanlun" -- .sandcastle scripts`（排除 sandcastle-host）余下：
`.sandcastle/harvest.sh:9`、`.sandcastle/health-monitor.mts:23`（有 `HEALTH_MAIN_REPO` 覆盖）、`.sandcastle/run-claim-loop.sh:11` 与 `install-sandcastle-claimer.sh:12-13`（`GH_TOKEN_ROTATION_REQUIRED` 哨兵文件路径，经 logs symlink 等价）、`.sandcastle/launchd/com.newchanlun.{arch-hotzone,decision-claimer,health-monitor}.plist`、`scripts/wayfinder_session_brief.sh:13`、`scripts/_m1i_task*_completion_guard.sh`、`scripts/tests/test_ceremony_scan_goal*.py`、`.sandcastle/migrate-*/RUNBOOK.md`（历史手册，按 ADR 0012 口径不追改）。`launchd/com.newchanlun.arch-hotzone.plist:12` 注释「与 watcher.sh / harvest.sh / install-wayfinder-engine.sh 同款硬编码」本 PR 后部分过时。均不在本分片两项范围内。

## 希望评审重点看

1. `resolveSpecDrafter` 白名单取两项字面是否符合用户规定的本意（vs 放宽到 Claude 全家族）。
2. `runSpecDrafter` 的 `--tools ""` + `cwd=tmpdir()` 是否足够隔离（用户级 `~/.claude` 设置/hooks 仍会加载；未用 `--bare`，因为 `--bare` 只认 `ANTHROPIC_API_KEY`，与 #1547 验证过的 `ANTHROPIC_AUTH_TOKEN` 路由不一致）。
3. 引擎顶层 `import "../.sandcastle/claude-route.mts"` 会连带加载 `@ai-hero/sandcastle`——宿主需有 node_modules（sandcastle-host 有）；手工从无 node_modules 的 worktree 跑引擎会在 import 处失败（此前引擎零依赖）。
4. plist 模板 PATH 加 `~/.local/bin` 与壳内再前置一次是否冗余可接受。

## 第三轮处置（非作者评审 `reviews/1614-claude-fable-5.md`，PASS-WITH-NITS）

| 发现 | 处置 |
|---|---|
| 1 子代理 env 继承 `ANTHROPIC_API_KEY` 会绕开 CLIProxyLocal | **已修**：新增纯函数 `specDrafterEnv(base, proxyEnv)`，合并 env 后 `delete ANTHROPIC_API_KEY`（与 `decision-claimer.mts:513` 同款），`runSpecDrafter` 改用它；补夹具测试（剔除该键、路由变量覆盖宿主值、不改入参）。`npx tsx --test scripts/wayfinder_engine.test.mts` 43/43，`scripts/wayfinder_engine.shell.test.mts` 2/2。 |
| 4 `wayfinder-workflow.md:106` / `:125` 口径过时 | **已修**：:106 改为 Claude 起草子代理（Claude Code CLI 经 CLIProxyLocal，白名单 opus-5-5 / fable-5-1）；:125 工蜂改为 Claude Code CLI 经 CLIProxyLocal，并注明 `main-swarm.mts` 仍有 prime-agent 残留、已转票。 |
| 2 engine 顶层 import 带进 `@ai-hero/sandcastle` | 转票 #1617 §4 |
| 3 `--bare` + `ANTHROPIC_AUTH_TOKEN` 冒烟 | 转票 #1617 §3（上文「评审重点」第 2 条里「`--bare` 只认 `ANTHROPIC_API_KEY`」的说法和 decision-claimer 的生产用法相矛盾，以冒烟结论为准） |
| W-4 其它非 Claude 默认位置 | 转票 #1617 §1（`wayfinder-workflow.md:125` 已在本 PR 修掉，不再列入） |
| TS 单测没进 CI | 转票 #1617 §2。核实 PR #1615 的 `sandcastle-ts` job：test glob 是 `.sandcastle/*.test.{ts,mts}`，typecheck 的 include 只有 `.sandcastle/`，**不覆盖** `scripts/wayfinder_engine*.test.mts` |
| 5 二次校验冗余、6 PATH 两处前置冗余、7 白名单两项 | 评审判为可接受，不改 |
