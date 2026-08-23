# RFC 独立评审报告：`prime-agent.remote-child/v1`

- 评审对象：`/Users/silencehan/Projects/NewChanlun/.chanlun/review-results/rfc-prime-remote-child-v1-draft-20260823.md`
- 对照基线：`gh issue view 1126 --repo xy7365527-lang/NewChanlun --json body` 的 Acceptance 1–7
- 结论：**PASS**（BLOCKING 0 / MAJOR 0 / MINOR 5）
- 评审日期：2026-08-23

## 评审方法与一手证据

1. 读取 #1126 Acceptance 原文，逐条核对草稿。
2. AC-1 查重抽查实际内容：
   - `gh issue view 739`：标题 *Feature: ACP client-backed external child agents*，正文为配置本地 ACP command（`codex acp` / `claude acp`）作为 child backend 的 spawn API；`CLOSED / NOT_PLANNED`，关闭评论明确是“closing the current Issue queue and moving to a discussion-first process”。草稿对 #739 的定性与关闭原因属实。
   - `gh issue view 1182`：*Prime Agent v0.8: five-stack integration tracker*，`OPEN / REOPENED`；正文是 Core/MCP/Release/Prompts/ACP-Evaluation checklist，“Remote reviewed refs”确为 Git SHA 列表；正文明确上游 prerequisites #838/#850/#851/#852 分开。草稿对 #1182 的定性属实。
   - Discussion #1571（REST + GraphQL）：仍 `open`，分类 *Feature requests*，标题 *RFC: authenticated remote RLM children (prime-agent.remote-child/v1)*，作者 `xy7365527-lang`（与 #1126 作者一致），0 comments。草稿“同主题、同作者、仍打开、应更新/回复而非另开”属实。
   - 抽查相关 Discussions #1488/#1470/#1421/#1529/#1506/#1503 正文：除下文 F-01 外，草稿对“为何不承接”的描述与原文一致。
   - 数量核实：GitHub Search `type:issue` = **312**；GraphQL discussions totalCount = **144**，与草稿 §13 一致。
3. AC-6 引用核实：
   - `git clone --depth 1 https://github.com/PrimeIntellect-ai/prime-agent /tmp/prime-agent-review`，HEAD = `e319a66d7351c75abe7f040d02d9a8d6e25028e9`；GitHub API 确认该 SHA 即当前 `main` head（2026-08-21T23:32:39Z）。
   - 解析草稿全部 `file#L`/`#L` 引用，去重后 **57 个 file:line range**，逐一以 `git show HEAD:<path>` 核对：**57/57 行号范围真实存在**，路径解析后均落在 `packages/coding-agent/{src,docs,test}` 或 `prime-agent-runtime/src/rlm/__init__.py`。关键抽查包括：`daemon-socket.ts#L9-L43/#L216-L239`（0700 目录/0600 socket/Windows named pipe）、`daemon-supervisor.ts#L1053-L1065`（`authenticated: true`）、`daemon-protocol.ts#L44-L50`（“local daemon JSONL … not the final remote gateway protocol”）、`agent-session.ts#L10355-L10361`（detached task）、`agent-session-recursion.test.ts#L1284-L1299`（post-admission startup failure）、`rlm-ledger.ts#L699-L721`（`fsyncSync`）、`daemon-protocol.ts#L53-L70`（v7/schema 22）、`prime-agent-runtime/src/rlm/__init__.py#L43-L50`（六字段 dataclass）。
   - 源码负证据抽查：`git grep -n -i` 在 `packages/coding-agent/{src,docs,test}` 与 `prime-agent-runtime/src` 中无 `RlmChildRuntime` / `RemoteRlmChildRuntime` / `remote-child` / `child lease` / `outbound broker` 实现，支持 §13 负结论。
4. AC-7 扫描：草稿仅含 1 个公开链接（Prime Discussion #1571）；无 `NewChanlun`、`xy7365527-lang`、`/Users`、`#1114`、`#1115`、私有 blob/SHA、API key、PAT 或本机路径。

## Acceptance 逐项结论

| AC | 结论 | 说明 |
|---|---|---|
| AC-1 查重 | PASS | #739/#1182/#1571 实际内容与草稿结论一致；未发现需另开新 Discussion 的理由；仅 F-01 一处相关 Discussion 描述错误。 |
| AC-2 十三要素 | PASS | §1–§7 逐项覆盖 use case、local socket 信任边界、extension/provider 不足、durable admission barrier、`RlmChildRuntime` local/remote 抽象、remote record/state/ledger、family `message/list/observe/delete/steer/stop/resume` 语义；无空洞段。 |
| AC-3 协议细节 | PASS（附 MINOR） | §8 + §4/§7 给出 invitation、child lease、epoch/seq replay fence、heartbeat/checkpoint、cancellation/restart、family isolation、audit/quotas，语义层可评审；边界数值与 PoP 机制有 F-02 缺口。 |
| AC-4 分层 | PASS | §9 明确 application protocol 与 transport/broker 分层；broker 仅为推荐 transport，Core MVP in-memory，不假定托管服务/入口/provider credential issuer；禁止 raw daemon v7 relay。 |
| AC-5 mock/version/PR | PASS（附 MINOR） | §10 矩阵 15 类 case 均有可执行断言；§11 version/兼容策略有源码依据；§12 四段 PR 切分基本独立可审；F-03/F-04 为矩阵完整性与 PR2 可执行性细节。 |
| AC-6 引用真实性 | PASS（附 MINOR） | 57 个 file:line range 全部在 `main @ e319a66` 存在；F-05 仅一处引用范围未覆盖到所列举的 `shutdown` 行。 |
| AC-7 自包含 | PASS | 无私仓链接/路径/凭据，不依赖私有 NewChanlun 材料，自包含于公开 Prime 材料。 |

## 发现明细

### F-01 · MINOR · AC-1 / §13
**条款**：AC-1“说明现有 Issue #739/#1182 及相关 Discussions 为何不承接本题”。
**证据**：草稿写 “#1529/#1506/#1503 等分别涉及 reply doctrine、heartbeat model、worker heap”。实际 Discussion #1506 标题为 *[Feature] Set reasoning effort per subagent in rlm()*，正文全篇讨论 `rlm.run` 增加 per-child `thinking/effort` kwarg，grep 正文无 `heartbeat`；真正的 worker heap 是 #1503。把 #1506 标成 “heartbeat model” 是对公开证据的错误转述。
**影响**：该错误不影响“均不含 remote admission”的最终判断，但上游 maintainer 核对时会降低 §13 证据可信度。建议提交前把 #1506 更正为 “per-subagent reasoning effort”，或换成真正的 heartbeat 相关 Discussion。

### F-02 · MINOR · AC-3 / §8
**条款**：AC-3“协议细节（invitation/lease/replay fence/audit/quotas）具体到可评审”。
**证据**：§8 只有类别级约束，未给边界数值或机制：invitation TTL 仅写“分钟级”；child lease 的 proof-of-possession key 未定算法/生成/轮换；heartbeat 间隔、lease 过期时间、续期上限未给默认/范围；quotas 只列 frame size、rate、buffer bytes、retries、并发 children 维度，无数值；audit 无保留期。
**影响**：语义边界可评审，但无法判断具体参数是否满足安全目标。建议在 §8 给出 v1 默认值/范围（如 TTL=5min、heartbeat grace=3×interval、帧上限 256 KiB、audit 保留 ≥ N 天）或显式声明为可配置项及其安全下限。

### F-03 · MINOR · AC-5 / §10
**条款**：AC-5“不得伪造 transcript/usage”。
**证据**：§1.6 与 §5 声明 remote transcript/kernel/usage 由 W 持有、parent 不伪造；但 §10 mock 矩阵的 Visibility 行只断言 “不伪造本地 transcript/tool rows”，没有针对 forged usage/checkpoint hash、stale-attempt completion、provider-native usage 不被覆盖的测试行。
**影响**：原则在正文，验收矩阵未锁定。建议增加一行 Provenance：伪造 usage/checkpoint → mark unverified or reject；旧 attempt 的 completion/usage → 拒绝；parent 侧不写 provider-native usage。

### F-04 · MINOR · AC-5 / §12
**条款**：AC-5“PR 切分可执行；每段独立可 review/验收”。
**证据**：§12.2 Security/transport 把“第一个真实 transport（双方出站 broker client）”放入 PR2，而 §12.4 把 Broker 列为可选（“若 Prime 团队不提供，协议仍可运行”），§9 又声明不假定已有托管服务。文中未说明 PR2 的 broker client 用什么对端做集成验收。
**影响**：PR2 作为“每段独立可验收”存在对可选 PR4/外部 broker 的隐含依赖。建议明确 PR2 自带 in-repo 测试 relay/回环对端，或把最小自托管 broker server 与 client 同段交付，保持 PR2 可独立验证。

### F-05 · MINOR · AC-6 / §2
**条款**：AC-6“引用的公开 file:line 真实”。
**证据**：草稿称 public JSONL 命令面含 `shutdown` 并引 `daemon-protocol.ts#L370-L525`；该范围到 525 行（`delete_rlm_subagent`）结束，而 `type: "shutdown"` 实际在 **658 行**（`git show e319a66:packages/coding-agent/src/modes/daemon/daemon-protocol.ts`）。其余引用范围全部真实。
**影响**：结论本身正确（命令面确有 shutdown），但引用范围不完整。建议改为 `#L370-L658` 或单独引 `#L658`。

## 最终结论

- **PASS**，阻断数 **0**。
- 草稿在 AC-1/2/4/7 上证据扎实，57 个 file:line 引用全部在最新 `main @ e319a66` 真实存在，查重结论经抽查 #739/#1182/#1571 原文成立。
- 提交上游前建议修复 F-01（公开证据描述错误）与 F-05（引用范围不完整）；F-02/F-03/F-04 建议在提交前或 Core MVP 开工会上一并收口，避免实现阶段返工。
- 提交路径方面，草稿选择“更新/回复已存在的 #1571”符合“不重复提交”且优于另开新 Discussion；实际提交时使用 GitHub Discussion 官方入口/GraphQL update/comment 即可，随后把 #1571 URL 回写 #1126。
