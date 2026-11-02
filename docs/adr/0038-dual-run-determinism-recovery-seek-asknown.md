> 正本：本文件是双跑确定性、恢复语义、seek 不偷未来、AsKnown 分证协议的教义正本。
> 最后裁定：#1537（2026-10-19）；修订：#1539（2026-11-02，见文末「修订」节）
> 受影响代码清单：见「受影响票与 blocking 边变化」节

## 问题

本票 #1537（FU-15）要求裁决「双跑确定性、恢复、seek 不偷未来、AsKnown 分证」的完整协议。票面列出两项验收：固定输入下双跑全状态等价；恢复/seek/AsKnown 反例通过。当前只有比较维度已列，但独立进程杀停/重启、seek 语义、AsKnown 分证的机制缺。

## 权威依据

1. **图内技术合同权威性**：`.sandcastle/docs/graph-engineering.md:23`【裁决面分级】定义「T 类（技术取舍）：数据结构、接口与 seam、owner 落点、状态机与事件 schema、测试锁、迁移步骤、恢复语义。有权威源或本仓判例可援引。→ 自动裁决」，明确将恢复语义列为 T 类判据。

2. **确定性回放的既有模型**：ADR 0033:12【裁定一】定义「第 t 根 bar 适用集合 S_{t−1}（t−1 收盘时已输出并经准入），bar t 的外效构成批 B_t，批内各重独立判定，`intra_batch_order: Unknown`」，v1 确定性标注 `seed: none`。

3. **owner 与写权分离**：ADR 0027:23【裁定一标题】定义「两个域各有一个 owner，所有权互不重叠」，ADR 0027:25-28【裁定一表格】将结构判定域与重操作程序的所有权明确分列。

4. **分档事实不回填**：`CONTEXT.md:435`【阶段事件史/扩量资格】定义「`TStage` latch 只投影 campaign 内达标事实史（AsKnown 不回填）；扩量资格由唯一谓词按当前真实成本 K 与已退本金计算，latch 不得作其输入」。

5. **恢复路径不偷未来**：ADR 0033:14【裁定一】援引 ADR 0018 裁定四「撤单不得用预算到期 bar」（锚 `049-第49课.md:62`【正文】「其后，中枢已经完成就无所谓中枢震荡了」），在多重上的形式即前沿规则。

6. **历史模拟与真实环境同核**：ADR 0027:44【裁定三标题】与 ADR 0027:46-47【裁定三正文】定义「历史回放、paper 与 live 必须共用同一个结构判定域实现和同一个重操作程序实现；环境差异只允许落在行情、撮合和场所 adapter」。

7. **场所 profile 版本化证据射程**：ADR 0028:13【裁定一表格】定义 profile 字段含 `environment`、`execution_form`、`evidence_class`、`credential_scope`，「任一字段变更即新版本；`environment` 与 `execution_form` 不得在版本内切换」。

## 候选

### 候选 A：固定输入下全状态确定性 + 幂等恢复 + AsKnown latch

1. **双跑确定性协议**：固定（input bar 序列，policy 版本，model id/revision，seed，launcher scheduler）五元组，同步在所有检查点比较六项全状态（S 结构快照与变化，各重 B 账与 TStage latch，E 责任与准入历史，订单/成交原件，物理持仓，外效历史）；批内顺序未知不作比较面。
2. **恢复与 seek 语义**：seek(t) 取 S/B/E 最后不晚于 t 的快照，从该快照重放至 t，`as_of` 输出只含 ≤t 的事实；seek 不拉未来数据，不改写 ≤t 的已持久事实，不提前触发 >t 的阶段转换。杀停后恢复按最后持久 checkpoint 重放。
3. **AsKnown 分证机制**：TStage latch（降成本闸、退本金闸、增股闸）只投影 campaign 内达标事实，存储值为「首次达标的 bar 索引+证据指纹」，后续 bar 不回填；回放/恢复时 latch 值按原 campaign 还原，不用当前成本重算；profile 证据类（simulation/experiment/production）进 schema，分证不交叉引用。

### 候选 B：弱确定性 + 可选快照策略

1. 双跑只比较端态（最后一 bar 的六项全状态），中间快照可选取；批内顺序明确排序（by priority/资金剩余/操作级别），以换取单线程确定性。
2. 恢复时允许用当前政策重算 TStage latch，以兼容政策热更新；seek 可看到未来数据做「追溯修订」。
3. AsKnown 只作标签，不作门；回放可用更新的阈值重判历史。

## 裁定

采纳候选 A。双跑确定性、恢复语义、seek 不偷未来、AsKnown 分证是图内技术合同（graph-engineering.md 明文列举「恢复语义」与「状态机」为 T 类），本票有权援引 #799/#804 判据、决策地图 FU-11/14 前置定义及 ADR 0027/0033/0028 已裁的 owner、模型与 profile 条款裁定技术协议。

正本落点：docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md

### 一、双跑确定性协议

固定五元组（input, policy_version, model_id/revision, seed, scheduler）下，两次独立运行在每个 checkpoint 的全状态必须逐位等价。全状态六面：

1. S 结构快照（Classification 全部级别的结构对象、修订链、撤回记录）；
2. 各重 B 账（Voice、TStage latch 值与转换历史、筹码、毛账、cost_remaining）；
3. E 责任（共同占用、在途订单、最大责任保留）；
4. 订单/成交原件（epoch、身份、前后状态、cause 链）；
5. 物理持仓（场所回报与内部投影对账）；
6. 外效历史（已发送命令、回报、更正、bust 的完整 journal）。

批内顺序声明为 `Unknown`（ADR 0033 裁定一），不作比较维度；checkpoint 取每 bar 收盘后、下一 bar 开盘前，不取 bar 内中间态。v1 `seed: none` 确定性；引入随机化时另开版本。

### 二、恢复与 seek 语义

**恢复**：launcher 启动时检测上次 checkpoint，从该点加载六面快照并重放至当前 bar；杀停等同未完成 bar 的全部外效未发生，S/B/E 回退到上一 checkpoint。

**seek(t) 语义**：
1. 取 S/B/E 最后不晚于 t 的快照；
2. 从该快照向前重放至 t 的收盘；
3. `as_of=t` 输出只含索引 ≤t 的事实，不拉 >t 的未来数据；
4. TStage latch 值按原 campaign 的达标历史还原，不用 seek 时的当前成本重算；
5. 不改写 ≤t 的已持久事实，不提前触发 >t 的阶段转换。

**不偷未来**：seek/恢复路径不得利用当下已知而 t 时刻未知的信息（阶段达标、policy 版本更新、后续成交）改写 ≤t 的决策或状态；这是 ADR 0033 裁定一「前沿规则」在时间轴上的不变式。

### 三、AsKnown 分证机制

1. **TStage latch 只投影 campaign 内达标事实**（CONTEXT.md:435）：latch 存储「首次达标 bar 索引 + 证据指纹（成本/频率/速率下界的原始数值）」，后续 bar 不回填，恢复/seek 时按原 campaign 还原该值；
2. **profile 证据类进 schema**：`evidence_class ∈ {simulation, experiment, production}` 作为 profile 版本字段（ADR 0028 裁定一），实验证据不得读为生产有效，simulation 产出不进 TStage 触闸计数；
3. **分证不交叉引用**：回放验证用的 `simulation` profile 不共享 `experiment`/`production` 的 latch 状态，各自独立累计；同一历史窗口在不同 profile 下可有不同 TStage；
4. **政策版本不回退覆盖**：历史 bar 的 latch 值与当时适用的 policy 版本绑定，后续政策更新不追溯修改历史 latch，新 campaign 从新版本起算。

### 四、验收 oracle 与反例机制

1. **H1 确定性对拍**：同一输入五元组在两个独立进程跑至同一 t，diff 六面快照，逐字段 assert 等值；
2. **H2 杀停恢复**：t=1000 时 SIGKILL，重启从最后 checkpoint（如 t=950）恢复并续至 t=1050，六面快照与未杀停的连续运行在 t=1050 相同；
3. **H3 seek 不偷未来**：seek(800) 后 `as_of=800` 输出不含 bar>800 的任何事实；TStage latch 值等于原运行在 t=800 的值，不等于 seek 时的当前成本重算值；
4. **H4 AsKnown 分证**：同一历史窗口分别以 `simulation` 与 `experiment` profile 跑，latch 触发 bar 可不同，各自独立，`simulation` 闸门通过不自动使 `experiment` 通过；
5. **反例通过判据**：任一 H1–H4 的 diff 非空 ⟹ FAIL 并留档，说明违反确定性/恢复/seek/分证之一；四项全空 ⟹ PASS。

## 验收清单

- [ ] `cargo test dual_run_determinism_h1` 通过（同输入双跑六面逐位相同）
- [ ] `cargo test recovery_after_kill_h2` 通过（杀停恢复后续接与连续运行等价）
- [ ] `cargo test seek_no_future_leak_h3` 通过（seek(t) 输出不含 >t 事实，latch 值按原 campaign）
- [ ] `cargo test asknown_evidence_segregation_h4` 通过（simulation/experiment 分证独立）
- [ ] 文件存在 `docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md`
- [ ] TB-07 实施票（#1381/1399/1411/1418）引用本 ADR 的确定性协议
- [ ] TB-11 整图验收包含 H1–H4 四项 oracle 的实际运行证据

## 受影响票与 blocking 边变化

**受影响票**：
- #1534（FU-11，事件 schema 须含 epoch、policy/model hash、前后状态以支持恢复）
- #1536（FU-14，完整历史 profile 须含 model_id/revision、seed、scheduler）
- #1381/1399/1411/1418（TB-07，历史双跑实现须实装 H1–H4 四项 oracle）
- #1387（TB-11，整图验收须核 H1–H4 证据）
- #1340（SPEC，C10.3/AT-14/AT-17 恢复与双跑验收引用本 ADR）

**blocking 边变化**：
- 新增：本票 #1537 blocks #1381（TB-07-A 双跑实现依赖确定性协议）
- 新增：本票 #1537 blocks #1387（TB-11 整图验收依赖 H1–H4 oracle 定义）
- 保持：#1534（FU-11）blocks #1537（依赖既存）
- 保持：#1536（FU-14）blocks #1537（依赖既存）

## 授权引用

本条依 #1323 Notes N-1 预授权判定。用户 2026-10-18 原话：「其实很多技术的东西 完全不需要我来做 我完全信任你的判断」。双跑确定性、恢复语义、seek 不偷未来、AsKnown 分证均属图内技术合同（`.sandcastle/docs/graph-engineering.md:23` 明文列举「恢复语义」与「状态机」为 T 类），本裁决援引 #799/#804 收敛通则、决策地图 FU-11/14 前置定义、ADR 0027（owner）/0033（bar 批模型）/0028（profile 分证）已裁条款，在技术取舍权限内裁定。

## 引用自证

| 引用 | 核实命令 | 结果摘要 |
|---|---|---|
| `.sandcastle/docs/graph-engineering.md:23` | `sed -n '23p' .sandcastle/docs/graph-engineering.md` | 第 23 行：「- **T 类（技术取舍）**：数据结构、接口与 seam、owner 落点、状态机与事件 schema、测试锁、迁移步骤、恢复语义。有权威源或本仓判例可援引。→ 自动裁决。」 |
| ADR 0033:12【裁定一】 | `sed -n '12p' docs/adr/0033-history-simulation-bar-batch-model.md` | 第 12 行：「模型 id `history-sim/bar-batch-known-frontier/1`。第 t 根 bar 适用集合 `S_{t−1}`（t−1 收盘时已输出并经 RD-11 准入）；bar t 的外效构成批 `B_t`；批内各重订单各自独立对 OHLC 判定（严格穿越、开盘取优），互不排序、互不占用对方资金，`intra_batch_order: Unknown`」 |
| ADR 0027:23【裁定一标题】 | `sed -n '23p' docs/adr/0027-classification-chong-operating-program-ownership.md` | 第 23 行：「### 一、两个域各有一个 owner，所有权互不重叠」 |
| ADR 0027:25-28【裁定一表格】 | `sed -n '25,28p' docs/adr/0027-classification-chong-operating-program-ownership.md` | 第 25-28 行：表格分两行，结构判定域拥有 Classification，重操作程序拥有 TStage/Voice/筹码/毛账/交易意图 |
| `CONTEXT.md:435` | `sed -n '435p' CONTEXT.md` | 第 435 行：「**阶段事件史 / 扩量资格**（同上，裁定三）:`TStage` latch 只投影 campaign 内达标事件史（AsKnown 不回填）」 |
| ADR 0033:14【裁定一】| `sed -n '14p' docs/adr/0033-history-simulation-bar-batch-model.md` | 第 14 行含教义依据援引链，指向 ADR 0018 裁定四「撤单不得用预算到期 bar」与 `049-第49课.md:62` |
| ADR 0027:44【裁定三标题】 | `sed -n '44p' docs/adr/0027-classification-chong-operating-program-ownership.md` | 第 44 行：「### 三、历史、paper 与 live 共用两个核心 owner」 |
| ADR 0027:46-47【裁定三正文】 | `sed -n '46,47p' docs/adr/0027-classification-chong-operating-program-ownership.md` | 第 46-47 行：「历史回放、paper 与 live 必须共用同一个结构判定域实现和同一个重操作程序实现。不得为任一运行环境保留特有的结构判定、重状态推进或交易决策程序。」 |
| ADR 0028:13【裁定一表格】| `rg -n "evidence_class" docs/adr/0028-first-capability-verification-profile.md` | 第 13 行起 profile schema 表含 `environment`、`execution_form`、`evidence_class`、`credential_scope` 四字段 |
| `049-第49课.md:62` | `sed -n '62p' docs/chanlun/text/blog/049-第49课.md` | 第 62 行：「其后，中枢已经完成就无所谓中枢震荡了」 |

## 修订（#1539，恢复语义按证据类分述，2026-11-02）

（append-only：裁定一至四与验收清单原文不改，本节为现行修订。依据 #1539 P 类裁定稿，用户 2026-11-02 批复（[批复评论](https://github.com/xy7365527-lang/NewChanlun/issues/1539#issuecomment-6564138279)）；草稿、考古与审查原件：首两轮在 `/Volumes/AgentStorage/decisions/1539/`，其后依次为 `/Volumes/AgentStorage/fu17-repair40r/`、`/Volumes/AgentStorage/fu17-review40y/`、`/Volumes/AgentStorage/fu17-repair41f/`、`/Volumes/AgentStorage/fu17-review41k/`、`/Volumes/AgentStorage/fu17-publish41p/`（获批版本）；本仓存档 `.chanlun/review-results/issue1539-decision-20261102/`。）

**原因**：裁定二「恢复」条（:60）含两句。前句「检测上次 checkpoint、加载六面快照并重放至当前 bar」对三种证据类都成立，ADR 0051 裁定三（:274）即据此定恢复幂等。后句「杀停等同未完成 bar 的全部外效未发生，S/B/E 回退到上一 checkpoint」的前提只在 simulation 成立。experiment/production 下 checkpoint 之后可能已有真实回报和可能已发出的调用，按后句回退与 #1340 C07.6、C09.8、C09.9、AT-13 及 ADR 0030 裁定五相反。ADR 0039 把本条写进正式 launcher 启动序列（:27、:147、:289），故在本文件修订。

1. **通用（三种证据类）。** 恢复＝从最后一个 checkpoint 加载六面快照，再按持久日志重放。幂等判据为 ADR 0051:275-276 的 (a)(b)，任一不成立报 `StorageUnavailable` 并停写。任何证据类下恢复都不删除 append-only 真相行（ADR 0051:310）。
2. **simulation。** :60 后句照旧：未完成 bar 的外效视为未发生，S/B/E 工作态回到 checkpoint 快照，由受控撮合按同一五元组重跑该 bar。重跑产出与已持久行同 `fact_key` 同内容，按 (a) 返回原 application、不新增行；不一致即报 `StorageUnavailable`。
3. **experiment / production。** 不适用 :60 后句。重放持久日志直至最后提交：checkpoint 之后已 Recorded 的 E 事实与 Pending、已 Applied 的 Bᵢ 应用、X 已持久的 OwnAttempt/MayHaveCalled 与观测记录全部保留，不回到旧状态（#1340 C09.9）。恢复后先只读对账在途订单，外部未知按最大责任保留、不伪造终态（ADR 0030:48）；MayHaveCalled 保留最大责任、不自动重发（C07.6）；X 重启或换代先确认旧发送者已退出（C09.8）。
4. **验收。** H1/H2（:80、:81、:89）判据与射程不变：照 ADR 0056:63 裁定七 (a)，三种证据类的同模式会话都在固定完整输入（输入及修订、政策/模型、时钟、种子、数值语义、故障与调度轨迹，#1340 C10.4；experiment/production 的场所侧用受控场所夹具）下验 H2，各证据类以本节第 2、3 条各自的恢复方法满足它。#1340 AT-13「回退保留所有已经发生的外效」与 C07.6 不重发是附加约束，不替代六面 H2。恢复幂等另按 ADR 0051 裁定三 (a)(b)。本修订不增删验收项。
5. seek（:62-67）、不偷未来（:69）与裁定一、三、四不变。
