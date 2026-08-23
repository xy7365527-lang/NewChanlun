# #1128 独立双轴评审：Sandcastle 显式模型 registry（map #1100 线）

- 评审日期：2026-08-23；只读评审；本报告为唯一写入。
- 评审对象（本地 `main`，`b032f038b9` 可达）：
  - `53c1200cc8` feat(sandcastle): #1128——显式模型 registry：Claude 默认 + DeepSeek v4-pro 可选实装
  - `85e0078f6a` fix(sandcastle): #1128 P1+P2 精确 label 路由 + 最小 EVENT_PAYLOAD
- 评审基线：`git show <commit>:<path>` 的最终态（`85e0078f6a` 与 `main` 树一致）；另行在 `/tmp/issue1128-review-85e0078` 解包该 commit 并执行本地验证。
- 轴一 Standards：`.sandcastle/CODING_STANDARDS.md` + 仓内 workflow 惯例（fail-loud、allowlist 闭合、事件 fixture）。
- 轴二 Spec：`gh issue view 1128 --repo xy7365527-lang/NewChanlun` 的 Acceptance 7 条逐条对拍（重点 AC-2..AC-6）。

## 0. 结论速览

**FAIL。BLOCKING 2 / MAJOR 2 / MINOR 4。**

安全不变量（AC-2/AC-3/AC-5 的 TS 侧）实现基本正确；两个 BLOCKING 都在 DeepSeek 运行面：
- B-1：GitHub-hosted runner 的 DeepSeek 分支只装 Prime Agent CLI、不装其 kernel bootstrap 依赖 `uv`，DeepSeek 工蜂实际无法调用工具（仓内 Dockerfile 明确写了必须预装）。
- B-2：`agent-implement-pr.yml` 的互斥路由继续依赖 `pull_request_target` label 事件快照在 `if:` 里求值，该表达式形态在仓内已有实测记录证明不可靠，DeepSeek PR 修正路线无法保证正确选择。

AC-4（remote-child invitation/lease 与模型凭据分离）与 AC-7（真实 smoke）没有可闭合证据：**未验证，需 smoke**。

## 1. 验证记录（本地可复现）

| 项 | 命令/结果 |
|---|---|
| TS 类型 | 在 `85e0078f6a` 解包树 `npm ci && npm run typecheck`：退出码 0 |
| registry 单测 | `tsx --test .sandcastle/agent-workflows/shared/shared.test.ts`：24 pass / 0 fail |
| 机械锁 | `python3 .sandcastle/verify-workflows.py`：全过（5 workflow + 4 形态对拍 + 禁入扫描 + 12 fixture + registry 锁） |
| CI 接线 | `git grep -n 'verify-workflows\|shared.test\|tsx --test' main -- '.github/workflows/*.yml'`：无命中（见 MINOR-4） |

---

## 2. Findings

### BLOCKING-1 ｜ DeepSeek 分支缺 `uv`：Prime Agent 工具调用会被 kernel bootstrap 闸死

- `.github/workflows/agent-implement.yml:147-149`、`.github/workflows/agent-implement-pr.yml:83-86`：DeepSeek 分支只执行 `npm install -g prime-agent-0.7.2.tgz`，没有 uv 安装步。
- `.sandcastle/Dockerfile:35-37`：同一 CLI 的镜像路线明确注释「uv：prime-agent 的 IPython kernel 自举依赖；沙盒无 TTY，不会交互安装，必须预装」。但两个 workflow 跑的是裸 `ubuntu-latest` + `noSandbox()`（`.sandcastle/agent-workflows/implement/implement.ts:22`、`implement-pr/implement-pr.ts:32`），不是该 Docker 镜像。
- 仓内补充证据：`origin/main` 的后续修复 `e11a218e77`（#1173）为 `agent-implement.yml` DeepSeek 分支补了 `Install uv` 步，commit message 记录真实 smoke 失败根因：「无 uv → 所有工具调用被 kernel bootstrap 闸挡住 → 无 commit」。
- 影响：显式 `agent:model:deepseek-v4-pro` 的 issue implement 与 PR implement 两条线都无法产生工具调用/commit；AC-1「DeepSeek 实装」与 AC-5「显式标签启用 DeepSeek」在运行面不成立。

### BLOCKING-2 ｜ `agent-implement-pr.yml` 用 `pull_request_target` label 快照做互斥路由，已被同型实测证伪

- `.github/workflows/agent-implement-pr.yml:80,85,91,99`：四处 `if:` 都直接对 `github.event.pull_request.labels.*.name` 求值来选择安装/注入/运行分支。
- 仓内补充证据：`origin/main` 的 `ab76975a60`（#1173）记录两次真实运行——同一 `pull_request_target` + labeled 快照在 `if:` 中求值不可靠：「快照含模型标签但互斥步都执行、DeepSeek run 步被 skip」，随后改成运行时 `gh pr view` 实读标签路由。目标 commit 的 `agent-implement-pr.yml` 仍是旧的静态快照写法（且本地 `main` 未含该修复）。
- 影响：DeepSeek `implement-pr` 可能出现两个安装步都执行、DeepSeek run 步被跳过；由于 `EVENT_PAYLOAD` 里标签与步选择不一致，TS 侧会按 DeepSeek 要 key 而步环境只给了 Claude token（或反向），最终 fail/blocked。AC-2 的「确定路由 + blocked」与 AC-5 的 PR 修正路线不可靠。
- 建议：仿照后续修复，先以一个运行时步骤实读 PR labels 输出 `use_deepseek`，互斥步只消费该输出。

### MAJOR-1 ｜ remote-child identity allowlist 是死代码；进程边界没有真正过滤模型凭据/identity

- `.sandcastle/agent-workflows/shared/agent.ts:270-279`：`remoteChildIdentityEnv()` 定义了 invitation/lease 白名单，但 `git grep` 显示生产代码零调用（唯一调用在 `shared.test.ts:277-290`）。
- `.sandcastle/agent-workflows/shared/agent.ts:325-327` 的注释「Claude token / remote-child identity 一律不进 provider env」只在 `provider.env` 层成立；运行层不成立：
  - `package-lock.json:16-19` 锁定 `@ai-hero/sandcastle@0.12.0`；
  - `node_modules/@ai-hero/sandcastle/dist/index.js:627-639` 会自动读宿主 `.sandcastle/.env` 并合入 provider env；
  - `node_modules/@ai-hero/sandcastle/dist/chunk-62WN33RK.js:11-13` 的 `noSandbox.create` 以 `{ ...process.env, ...createOptions.env }` 起子进程。
- 影响：当前两个 workflow 的 step env 互斥，所以今天没有实际泄漏；但 AC-4 的「严格分离」靠的是 YAML env 形状，不是代码边界；`remoteChildIdentityEnv` 测了却没接，broker/admission 代码也不在本仓。AC-4 只能判「代码侧部分 + 未验证，需 smoke」。

### MAJOR-2 ｜ #1002 Kimi 默认回退仍在被 import 的 provider 里，禁入扫描盲区造成假绿

- `.sandcastle/prime-agent-provider.ts:32-33` 与 `:150`：`provider?: string` 的缺省仍是 `"kimi-coding"`；`agent.ts` 新代码直接 import 该函数。
- `.sandcastle/verify-workflows.py:275-276`：`all_text` 只扫 5 个 workflow + `agent-workflows/**`，不包含 `.sandcastle/prime-agent-provider.ts`；因此 `:288` 的 `kimi-coding` 禁入项在本仓存在命中的情况下仍报全过。
- `.sandcastle/agent-workflows/shared/agent.ts:66` 把 `primeProvider` 类型放宽为 `string`；`:319-327` 的构造分支只判 `provider === "prime-agent" && secretName === DEEPSEEK_SECRET`，没有断言 `entry.primeProvider === "deepseek"`。
- 影响：当前 `selectedAgent()` 确实显式传了 `"deepseek"`（`agent.ts:323-324`），今天不会真调 Kimi；但 allowlist 不是类型级闭合，且「不复活 #1002 Kimi」的机械锁有盲区。AC-6 当前行为通过，回归防护不通过。

### MINOR-1 ｜ `EVENT_PAYLOAD` 缺失时 fail-open 为「无标签默认 Claude」

- `.sandcastle/agent-workflows/shared/agent.ts:208-214`：`parseEventPayload()` 声明缺失 `EVENT_PAYLOAD` 必须抛错。
- `.sandcastle/agent-workflows/shared/agent.ts:222-228`：`readModelLabelsFromEnv()` 在 `if (!env.EVENT_PAYLOAD) return []` 直接短路，绕过上面的 fail-loud；`shared.test.ts:324-344` 还把这一行为固化为测试。
- 影响：固定 Claude 角色（review/explore/update-branch）在 workflow 接线回归导致 `EVENT_PAYLOAD` 丢失时会静默按无标签走，违背 fail-loud 惯例。建议删除短路或只允许显式 `SANDCASTLE_LOCAL=1` 时放行。

### MINOR-2 ｜ AC-5 事件 fixture 缺 Draft PR 与 resume 形态

- `.sandcastle/agent-workflows/shared/fixtures/pr-labeled-review.json:4-11`：`pull_request` 只有 `state: "open"`，没有 `draft: true`；整个 fixture 目录没有「Issue→Draft PR→review」链式 fixture。
- 只有 `.sandcastle/agent-workflows/shared/fixtures/issue-labeled-deepseek-retry.json` 覆盖 retry；没有任何 resume/session 续跑 fixture（`run-with-extraction.ts:47` 的 extraction resume 也没有事件级 fixture）。
- 影响：AC-5 字面要求「模型选择、Issue→Draft PR→review、retry/resume 均有事件 fixture」，目前只覆盖模型选择/retry/分离的 PR review 事件，Draft 与 resume 两格证据缺失（代码上 Draft PR 在 `agent-implement.yml:177-192` 存在）。

### MINOR-3 ｜ workflow `contains` 大小写不敏感与 TS `startsWith` 大小写敏感不一致

- `.github/workflows/agent-implement.yml:142,148,154,163`、`.github/workflows/agent-implement-pr.yml:80,85,91,99` 用 `contains(...)` 选路由；GitHub Actions 表达式文档明确 `contains` 比较字符串时大小写不敏感。
- `.sandcastle/agent-workflows/shared/agent.ts:125-127` 用大小写敏感的 `startsWith("agent:model:")` 过滤。
- 影响：例如未知标签 `agent:model:DEEPSEEK-V4-PRO` 会让 workflow 走 DeepSeek 分支，但 TS 侧当成无模型标签默认 Claude；当前因步 env 互斥而 fail-closed（DeepSeek 步里没有 Claude token），固定 Claude 角色却会静默忽略这个未知大小写变体。P1 敌对 fixture 只覆盖 `not-*` 和 `-legacy`，未覆盖大小写变体。建议补 case-variant fixture，并让 YAML 路由与 TS 用同一规范化判据。

### MINOR-4 ｜ 测试与机械锁未接 CI

- `git grep -n 'verify-workflows\|shared.test\|tsx --test' main -- '.github/workflows/*.yml'` 无命中；五个 agent workflow 只跑 `npm run typecheck`（例：`agent-implement.yml:137-139`）。
- 影响：本次本地 24 项单测与 `verify-workflows.py` 全过，但未来改动可让 fixture/allowlist/路由锁漂移而不被 CI 拦住。建议在 `ci.yml` 增加 `tsx --test` + `python3 .sandcastle/verify-workflows.py`。

---

## 3. 轴二：Spec Acceptance 逐条

| AC | 判 | 证据/缺口 |
|---|---|---|
| AC-1 默认 Claude Sonnet 实装、Claude Opus 评审、`agent:model:deepseek-v4-pro` 实装；评审独立 session/model | ⚠️ 代码 PASS / 运行面 BLOCKED | `agent.ts:33-43`（AGENT_MODELS）、`:81-90`（registry，`appliesTo: ["implement","implement-pr"]`）；`agent-review.yml:66-74` + `review.ts:29-36` 固定 `claudeAgent("review")`；`shared.test.ts:126-148,238-251`。DeepSeek 实装运行面见 BLOCKING-1/2。 |
| AC-2 label→provider→model→Secret 代码内 allowlist；任意字符串不得变 key；未知/冲突 fail-loud + blocked | ✅ 核心 PASS（边角 MINOR-3） | `agent.ts:50-58`（`MODEL_SECRET_NAMES`）、`:81-90`（registry）、`:122-155`（未知/冲突抛 `ModelRegistryError`）、`:250-264`（固定 env 分支）；`verify-workflows.py:338-343`（secrets allowlist）、`:424-432`（票面字符串不入 key）；fixture `issue-labeled-unknown.json` 用 `agent:model:DEEPSEEK_API_KEY` 做反例。工作流 `Mark blocked on failure` 在五个 workflow 均有。大小写变体见 MINOR-3。 |
| AC-3 DeepSeek=Prime Agent deepseek/deepseek-v4-pro；仅 `DEEPSEEK_API_KEY`；缺 key 任何模型调用前失败；禁复制 `.sandcastle/.env`/本机 key | ⚠️ 守卫 PASS / 运行面 BLOCKED | `agent.ts:318-327` 先 `modelCredential` 后构造 provider，缺 key 抛错不回退 Claude（`shared.test.ts:222-236`）；workflow 只注 `secrets.DEEPSEEK_API_KEY`（`agent-implement.yml:162-169`、`agent-implement-pr.yml:98-104`）；`git grep` 未见 workflow 读 `.sandcastle/.env`/本机 key（`verify-workflows.py:278-287` 亦有禁入扫描）。但 BLOCKING-1 使 key 存在时模型调用后工具全挂。 |
| AC-4 invitation/lease 与 `DEEPSEEK_API_KEY` 严格分离；identity 不携带模型凭据；broker 不读模型 secret | ❓ 未验证，需 smoke | 代码侧：`agent.ts:97-100` allowlist 分离、`:270-279` helper、`shared.test.ts:277-290` 锁定；但 helper 零生产调用、broker/admission 代码不在仓内、运行层进程 env 合并见 MAJOR-1。无法在本次静态评审闭合。 |
| AC-5 默认无标签走 Claude；DeepSeek 仅显式标签；模型选择/Issue→Draft PR→review/retry/resume 有事件 fixture | ⚠️ 代码 PASS / fixture 缺口 | `agent.ts:122-131`（无标签→默认 Claude）、`:303-337`（显式才 DeepSeek）；fixture 覆盖默认/显式/retry/PR review（`shared.test.ts:347-469`）。Draft/resume fixture 缺失见 MINOR-2；PR implement 路由运行面见 BLOCKING-2。 |
| AC-6 #1002 旧 Kimi 常量/票面不声明模型不复活 | ✅ 行为 PASS / 防护有洞 | 新 registry 无 Kimi、`shared.test.ts:150-162,471-480` 锁定未知 kimi 标签、`verify-workflows.py:288-290` 禁入。但 `prime-agent-provider.ts` 的 `kimi-coding` 默认回退在扫描范围外（MAJOR-2）。 |
| AC-7 依赖 #1110/remote-child core，独立双轴评审 + 真实 smoke 后方可关闭 | ❓ 未验证，需 smoke | issue 状态 OPEN；`53c1200cc8` message 明确「未 push、未真实 smoke、未创建/迁移任何 Secret」。本报告为轴二（Spec）；BLOCKING-1/2 就是 smoke 前必须处置的两项。 |

## 4. 轴一：Standards 小结

- commit 格式：`<type>(<scope>): #1128——…` 两 commit 均合规；注释/文档中文、票号登记到位。
- fail-loud：TS 侧未知/冲突/缺 key 均在构造 provider 前抛错并落 `failure_reason.txt`，workflow 回写 `agent:blocked`——主路径合规；`EVENT_PAYLOAD` 缺失 fail-open 是例外（MINOR-1）。
- allowlist 闭合：Secret 名、workflow secrets 引用、registry 均有白名单，`verify-workflows.py` 机械锁存在；但 provider 缺省值在锁外（MAJOR-2），进程边界过滤未代码化（MAJOR-1）。
- 事件 fixture：12 枚 fixture + 24 项单测覆盖默认/DeepSeek/冲突/未知/缺 key/retry/P1 敌对标签，质量较好；Draft/resume 两格缺失（MINOR-2）。
- 单次判定不双实现：workflow YAML 路由与 TS registry 仍是两处判定，P1 锁靠字符串计数；`pull_request_target` 下已被实测证伪（BLOCKING-2），大小写语义也不一致（MINOR-3）。

## 5. 处置建议

1. 先补 BLOCKING-1：DeepSeek 分支加 `uv` 安装（或改跑带 uv 的容器沙箱）。
2. 再补 BLOCKING-2：`agent-implement-pr.yml` 改运行时实读 PR labels 的单一路由步骤。
3. 修 MAJOR-1/2 后重跑 `shared.test.ts` + `verify-workflows.py`，并把二者接进 `ci.yml`。
4. AC-4/AC-7 按票面纪律走 #1110/remote-child 真实 smoke；smoke 前不得关闭 #1128。
