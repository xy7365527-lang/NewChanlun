# PR #1614 非作者独审（#1588 wayfinder 分片）

- 评审模型：claude-fable-5（Claude Fable 5，非作者）
- 评审对象：https://github.com/xy7365527-lang/NewChanlun/pull/1614 ，head `5df98c6312`，base `main`@`df6e680930`
- 评审日期：2026-10-21
- 方式：只读。读 PR diff / 描述 / `REPORT.md` / 相关源码，本机重跑关键命令；未改代码、未推送、未评论 PR、未合入、未 launchctl。

## 结论：**PASS-WITH-NITS**

两项改动（常驻壳宿主推导 + spec 起草子代理改 Claude 白名单）都是真修，不掩盖问题、不降覆盖、不误改既有行为；测试 +5 条全绿；证据与 PR 描述一致。
有 1 条值得在合入前或紧随其后补的加固（继承 `ANTHROPIC_API_KEY` 未清），其余为 nit / 转票项。

## 轴①：正确性 / 风险

| # | 严重度 | 位置 | 发现 | 建议 |
|---|---|---|---|---|
| 1 | **低（加固）** | `scripts/wayfinder_engine.mts:959` | `env: { ...process.env, ...cliProxyEnv() }` 只追加 `ANTHROPIC_BASE_URL` / `ANTHROPIC_AUTH_TOKEN`，不清继承的 `ANTHROPIC_API_KEY`。Claude Code 以 `ANTHROPIC_API_KEY` 优先，若宿主 shell 带官方 key，起草会绕开 CLIProxyLocal（或在代理端鉴权失败）。同仓先例 `.sandcastle/decision-claimer.mts:512-513` 构造同类 env 后显式 `delete env.ANTHROPIC_API_KEY`。**生产路径不受影响**：plist 模板 `EnvironmentVariables` 只有 PATH/GH_REPO/颜色变量，launchd 不 source shell profile；本机交互 shell 也未设该变量。只影响人工从带 key 的 shell 跑 `--live`。Codex 机审同样提到这条（P2）。 | 对齐 decision-claimer：`const env = { ...process.env, ...cliProxyEnv() }; delete env.ANTHROPIC_API_KEY;`。一行改动，可本 PR 顺手补或转小票。 |
| 2 | nit | `scripts/wayfinder_engine.mts:44` | 顶层 `import { cliProxyEnv } from "../.sandcastle/claude-route.mts"`，而该模块顶层 `import { claudeCode } from "@ai-hero/sandcastle"`。引擎与 `wayfinder_engine.test.mts` 从此必须有 `node_modules`（本次我在工位跑测试就得临时软链 sandcastle-host 的 node_modules）。此前引擎零依赖。sandcastle-host 有 node_modules，生产不受影响。作者在 REPORT「希望评审重点看 3」已自述。 | 可接受。若想保持引擎零依赖，把 `readCliProxyAccessKey` / `cliProxyEnv` 抽到无 `@ai-hero/sandcastle` 依赖的小模块（如 `.sandcastle/cliproxy-env.mts`），`claude-route.mts` 再 re-export。不阻塞。 |
| 3 | nit | `scripts/wayfinder_engine.mts:938-941, 949-961` | 隔离面：`cwd=tmpdir()` 只避开项目级 CLAUDE.md / hooks；用户级 `~/.claude`（launchd 下 `CLAUDE_CONFIG_DIR` 未设）的 settings / hooks / CLAUDE.md 仍加载（本机现查 `~/.claude/settings.json` hooks 为空，今天没事）。作者未用 `--bare` 的理由是「`--bare` 只认 `ANTHROPIC_API_KEY`」，但同仓 `decision-claimer.mts:514-516` 生产上正是 `--bare` + `ANTHROPIC_AUTH_TOKEN`，与该理由相左；`claude --help` 对 `--bare` 的措辞是「strictly ANTHROPIC_API_KEY or apiKeyHelper」，AUTH_TOKEN 是否可用需一次冒烟定论。 | 转小票：冒烟 `--bare` + AUTH_TOKEN；若通，起草子代理改 `--bare`（顺带跳过 hooks、CLAUDE.md 自发现、keychain，隔离更干净，也与 decision-claimer 口径统一）。不阻塞。 |
| 4 | nit（文档漂移） | `docs/agents/wayfinder-workflow.md:106` | 「spec 草案 = rlm 子代理起草」——这行就是对引擎起草子代理的描述，本 PR 后已过时。作者 W-4 只列了 :125（工蜂 prime-agent），漏了 :106。 | 一行改为「Claude 起草子代理（经 CLIProxyLocal，#1588）」，本 PR 顺手改或并入 W-4 转票。 |
| 5 | nit | `scripts/wayfinder_engine.mts:950-953` | `resolveSpecDrafter` 在 `parseArgs` 启动时已校验，`runSpecDrafter` 里用 cfg 再拼回 env 二次校验。防御性冗余，无害。 | 可不改。 |
| 6 | 信息 | `.sandcastle/launchd/com.newchanlun.wayfinder-engine.plist` + `run-wayfinder-engine.sh` | PATH 两处都前置 `~/.local/bin`，冗余但各自独立成立（壳可被手工调用、plist 可被内联形态替代）。作者「重点看 4」已自述。 | 接受。 |
| 7 | 信息 | `scripts/wayfinder_engine.mts:56` | 白名单取 `claude-opus-5-5` / `claude-fable-5-1` 两个字面而非 `^claude-` 前缀，`claude-haiku-4-5`、`mf/claude-opus-5-5` 被拒（有测试）。与用户「子代理只用 Claude 家族（claude-opus-5-5 / claude-fable-5-1）」的列举一致。 | 同意现状；放宽只改一个常量。 |

**不掩盖 / 不降覆盖 / 不误改**：
- 测试只增不减（37→42），新测全是纯函数、不触网、不含密钥；shell 测试 2 条仍过。
- 宿主推导逻辑与 `run-claim-loop.sh` 逐字同款；launchd 若仍指用户工作区的 `watcher.sh`，推导结果与改前完全相同（无回归）。
- 旧 `prime-agent --provider deepseek` 路径被白名单堵死且有测试锁（`provider=deepseek/cliproxy-local/anthropic` 均 throw）。
- W-2 对两个用户工作区 plist 的判断我认同：watcher 应改指 host（数据面经 logs 软链共用）；issue933-sampler 脚本只在用户工作区且 gitignored，改指 host 会找不到脚本，不应动。
- CI 不跑 `scripts/wayfinder_engine*.test.mts`（`grep tsx .github/workflows/ci.yml` 无命中）——作者如实披露，本机结果是唯一测试证据，我已独立复跑。建议转票接入 CI（锁没进 gate）。

## 轴②：#1588 票面验收

- **「逐条 现象、复现命令与输出、根因、处置」**：`REPORT.md` W-1 / W-3 四段齐全，W-2 / W-4 / W-5 为核查与清单。票面要求汇总到 `.chanlun/review-results/eng-debt-20261020/LEDGER.md`——main 上尚无该文件，兄弟分片（#1592/#1593/#1594/#1596/#1611）同样各写自己目录下的报告，LEDGER 应由主控汇编，本分片做法与惯例一致。
- **「每个修复非作者独审」**：本文即独审。
- **「按 squash 规则合入（走用户合入闸）」**：PR 标注「不合入；需非作者独审」，未越闸。
- DevSkim / 基线测试 / CI 时长 / 内存四项不在本分片范围；PR 描述对 DevSkim 红的归因（与 main push 同为 55148 条、本 PR 文件 0 命中）属环境/存量问题判断，与 #1593 分片处置一致。
- 本分片属票面第 5 项「当前就错」的部分：#1578 修复合入 main 后从未在 launchd 生效，是真实的「锁打好了没接进 gate」。

## 我实际复核过的事实与命令

均在工位 `/Volumes/AgentStorage/eng-debt-20261020/wayfinder`（已检出 `eng-debt-1588/wayfinder`，`git status` 干净）或 `/tmp` 执行；跑测试时临时 `ln -s <sandcastle-host>/node_modules node_modules`，跑完即 `rm`，`git status` 仍干净。

1. `gh pr view 1614 --json files,commits,body,reviews` / `gh pr diff 1614` / `gh issue view 1588` —— 单提交 `5df98c6312`，8 文件；机审只有 Codex 1 条 P2（即上表 #1）。
2. `npx tsx --test scripts/wayfinder_engine.test.mts` → `# tests 42 # pass 42 # fail 0`（与 PR 一致）。
3. `npx tsx --test scripts/wayfinder_engine.shell.test.mts` → `# tests 2 # pass 2 # fail 0`。
4. `WAYFINDER_SPEC_MODEL=deepseek-v4-pro npx tsx scripts/wayfinder_engine.mts --once --dry-run` → `Error: [FAIL-loud #1588] WAYFINDER_SPEC_MODEL=deepseek-v4-pro 不属于 Claude 家族白名单（claude-opus-5-5 / claude-fable-5-1）`；`WAYFINDER_SPEC_PROVIDER=deepseek …` 同样 FAIL-loud。
5. `cd /tmp && bash <工位>/.sandcastle/run-wayfinder-engine.sh --once --dry-run` → 启动行 `spec 起草子代理=claude-cliproxy/claude-opus-5-5`，`open 的 wayfinder:map 共 4 张`；状态页落在**工位** `.sandcastle/logs/wayfinder-status.md`（证明仓根由脚本位置推导，非调用方 cwd）。
6. `claude --version` → 2.1.221；`claude --help`：`--tools <tools...>` 明示 `Use "" to disable all tools`；`--no-session-persistence`、`--output-format`、`--model` 均存在；`--bare` 文案「Anthropic auth is strictly ANTHROPIC_API_KEY or apiKeyHelper」。
7. `plutil -p ~/Library/LaunchAgents/com.newchanlun.wayfinder-engine.plist`（只读）→ 现行为 `bash -c "cd …/NewChanlun-sandcastle-host && … PATH=/Users/silencehan/.local/bin:$PATH && exec npx tsx scripts/wayfinder_engine.mts --live"`，WorkingDirectory 已指 host；StandardOut/ErrPath 仍写用户工作区 logs——与 REPORT W-1「未做」段描述一致。
8. `readlink /Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/logs` → `/Users/silencehan/Projects/NewChanlun/.sandcastle/logs`（logs 软链主张属实）。
9. `git -C …/NewChanlun-sandcastle-host rev-parse --abbrev-ref HEAD` → `main`；`git -C …/NewChanlun rev-parse --abbrev-ref HEAD` → `codex/1523-1525-launcher-riskgate-v1`；`rev-list --count HEAD..origin/main` → `167`（PR「落后 167 提交」属实）。
10. 对照 `.sandcastle/run-claim-loop.sh:15-17`、`install-sandcastle-claimer.sh:6-8`、`launchd/com.newchanlun.sandcastle-claimer.plist` —— 本 PR 的推导写法与模板路径与 claimer 先例逐字同款。
11. `grep -n ANTHROPIC_API_KEY .sandcastle/decision-claimer.mts` → `:513 delete env.ANTHROPIC_API_KEY`（上表 #1 先例属实）；plist 模板 `EnvironmentVariables` 键只有 PATH/GH_REPO/NO_COLOR/CLICOLOR/FORCE_COLOR/CLICOLOR_FORCE；本 shell `ANTHROPIC_API_KEY` 未设。
12. `~/.claude/settings.json` hooks 为空（上表 #3 的「今天没事」依据）；`grep -n "rlm 子代理" docs/agents/wayfinder-workflow.md` → :106 命中（上表 #4）。
13. `grep -n "tsx\|wayfinder_engine" .github/workflows/ci.yml` → 无命中（CI 不跑这批 TS 测试，作者披露属实）。
14. 未做：未跑真路由冒烟（作者已跑 `out="OK"`，我不重复消耗代理额度）；未动 `~/Library/LaunchAgents`；本机禁 cargo，本分片零 Rust 改动无需远端复核。
