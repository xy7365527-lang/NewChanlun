# 主线合并路线图：区间套递归确认 → 完全互斥分类 + 多空双开（2026-07-17）

**状态**：生效（编排者批准框架；逐关验收，关关落盘）
**主线一句话**：买卖点由次级别走势经「区间套+背驰」递归确认，喂给完全互斥分类的全定义策略，多空双开消费，三阶段 treasury 兜底验收。
**目标结构**：主目标 = M8 Π_max-full 四层验收；退路 = M7 treasury 可达性（退而求其次）。两目标按 693 边界互斥定理分判，任一边负结论不外推。
**已收官前置**：nest 谓词口径 R1–R3 实装+验证（1647/0 测试、重放 A=354/B=319 vs 基线 41/25、bit-exact 五项=0、裁定书 289 行）。

## 脊柱与依赖

```
阶段 1 信号地基（Π_signal-full / M1–M4）
   ├─ 1a 存在性探针：真转折点×锚证书覆盖率 → 沉默两本账 → 有效域定量
   ├─ 1b BSP 侧口径修复（754 清单）→ 生产 BSP 路径教义保真
   ├─ 1c 877 池终端语义裁定 + U1–U3 遗留裁定
   └─ 1d 小转大显式分支
   依赖：1a ∥ 1b 可并行；1c/1d 跟在 1b 口径稳定后
阶段 2 执行层（Π_exec-full / M5–M6）
   ├─ 2a 嵌套子声部 + fill 双账本负仓（G1/G2）
   ├─ 2b recover 触发区间套语义裁决 + 衰减边界（G5/G6）
   └─ 2c A10 成本模型生产口径（M6，费率标定）
   依赖：全部吃阶段 1 的 BSP 质量（假 BSP = 噪声择时）
阶段 3 资金层验收（Π_treasury-full → Π_max-full / M7–M8）
   ├─ 3a M7 treasury 重验（带 A10 成本的可行集内）= 退路裁定
   └─ 3b M8 Π_max-full 四层报告 = 主目标验收
横切  纪律线：090 修复两项；Lean 残余；塔内原生化（独立长线终局）
```

排序原则：不靠主观优先级，靠物质依赖（信号→执行→资金）。边界互斥定理保证各关可并行而结论不互相污染。

## 阶段 1 信号地基

| 关 | 内容 | 验收 |
|---|---|---|
| 1a | 存在性探针（doc-necessity-boundary §6.2 判定式）：真 L 级转折点全集 × 任一级别 ℓ≤L 锚证书存在性 | `chanlun/review-results/p116-turnpoint-anchor-existence-20260717.md`：转折点定义+分级别数量、存在率（新旧 dump 对照）、沉默逐案归因（教义课号 / 代码文件:行号）、有效域定量（可捕获/延迟/不可达）、数字可复现 |
| 1b | BSP 侧口径修复：S0 取段坐标桥（113 案=73.4%）、S1a τ 门上下文（37 案=24%）、S2/S4 anchor 门（2.6%）、BSP 补 037:20 破极值项（p112 §6-2..6 清单） | 实装+单测；cargo test --lib 全绿零变红；p92 全量重放对照基线（产量、塔/笔/线段/中枢 bit-exact=0） |
| 1c | 877 pan 池终端确认语义裁定；U1（L5 零覆盖）/U2（c 确立时点 84/70）/U3（rung 147126）遗留裁定 | `chanlun/escalate/` 裁定文书，逐项教义锚+实证行锚 |
| 1d | 小转大显式分类分支：044:22-26 必要条件（c′ 三类买卖点）过滤 + 053:28 二类点补充 + 039:34 defer 语义；断链不得落入无类状态 | 实装+单测；分类输出含显式小转大标记 |

## 阶段 2 执行层

| 关 | 内容 | 验收 |
|---|---|---|
| 2a | 嵌套子声部（recognize 产 depth>0 子声部）+ fill 层双账本负仓（G1/G2；voice_side/parent_cap/signed_notional 已有预留） | 实装+单测；纸面定义与运行时产出一致（grammar-audit 闭环） |
| 2b | recover 触发级别裁决（父级别 BSP vs 子级别 BSP 的区间套语义，bsp_consumption 开放分叉）+ recover 率级别衰减边界（G6） | 裁定落盘 + 实装按裁定 |
| 2c | A10 成本模型：保证金/强平/资金费生产口径（693 硬裁决 MUST；M6 六项分解 R = ΣN_tΔP_t − Commission − Slippage − Funding − Borrow − LiquidationLoss） | 实装+守恒断言+费率标定来源落档 |

## 阶段 3 资金层验收

| 关 | 内容 | 验收 |
|---|---|---|
| 3a | M7 treasury 重验：带 A10 成本的可行集 K 内，三阶段可达性（Reach(StageIII)>0、Q_T>Q_0、W_T≥I_0、η_T≥η_*） | witness 报告；照实否定也是合格结果（161 号） |
| 3b | M8 Π_max-full 端到端：四层报告（signal / execution / treasury / total） | 各层合格判据按 693/MAXFULL 冻结列 |

## 横切纪律线

1. **090 修复**（随阶段 1 一并落）：`docs/recursive_fugue_necessity_proof.md:79`「本级别只有三类买点，其他都是次级别买卖点」从「原文」降级为「用户洞察（教义一致）」，锚改挂 021:62/66、053:32；`docs/formal-chain/有效域定理-20260704.md` 悬空引用——补档或改引 693 pending 谱系 + PDF §2（注意：主仓禁写期间此两项只能落 worktree 镜像补丁或待主仓解禁）。
2. **Lean 残余**：Θ_parse 固定（谱系 617 pending ①）；E5 杠杆完全分类缺失。
3. **塔内原生化**（独立长线终局）：区间套背驰作为买卖点形成必要条件长进递归塔内部；nest-tower-native-gap-audit 已盘清对象映射。

**塔内原生化端到端定义（条目 3 展开）**：本段与 `chanlun/review-results/doc-divergence-endtoend-prototype-20260718.md` 共用且只使用以下编号：`E2E-D1` 定义域与种类、`E2E-D2` 比较对象、`E2E-D3` 结构前提、`E2E-D4` 力度与代理、`E2E-D5` 状态与因果钟、`E2E-D6` 递降/谱系/地板（合称六要素），以及 `E2E-O`（算子）、`E2E-F`（本地地板）、`E2E-L`（一等谱系）、`E2E-N0`–`E2E-N7`（迁移映射）、`E2E-S1`–`E2E-S8`（缝合线实证）。编号语义以该原型文档 §1–§6 为准，不得在实装中重命名后偷换口径。

- **`E2E-O` 状态算子定义**：`E2E-O(as_of, tower_prefix, prior_state, ManagedBspPolicy) → (Delta, next_state=advance(prior_state,as_of,CallHashes,Delta)) | fail-closed error`，其中 `prior_state` 含 state/prev-state ID、state_as_of、输入/配置/政策摘要及 Event/Lineage/Bsp/Link 规范 key 头映射；`Delta` 只含本次追加的 EventRevision、LineageRevision、ManagedBspCreation 与 BspLink，不是全量快照。state_id 必须按原型 WireV1 重算一致；推进时从本次输入截到旧 state_as_of 重算 prior_slice 摘要，再把截至新 as_of 的摘要写入 next state；同一 state_as_of 重跑强制零 Delta。Genesis 的 state_as_of/prev 均为 null、映射为空；每一步都读取同一个 `as_of` 前缀及对应旧头：`ObserveCandidate_at(E2E-D1–E2E-D3) → ReviseEvent_at(E2E-D4–E2E-D5) → BuildLineage_at(E2E-D6) → CloseFloor_at(E2E-F) → Consume_at(policy)`。新 key 从 revision 1 起；同 key 仅在 WireV1 业务载荷投影变化时 `n→n+1` 并指回 `supersedes_revision=n`，载荷相同零输出；一次调用的中间态先合并、每个 key 至多追加一个最终 head；`Confirmed/Invalidated` 与 `Closed/Invalidated` 对同 key 为终态，路径延伸必须生成带 `extends_lineage_key` 的新 key。结构候选先获得稳定身份；普通空分支为 `NoCandidate / UnresolvedFloor / NoConsumption`，非法政策、旧状态不一致或晚到授权分别 fail closed 为 `InvalidPolicy / InconsistentState / LateAuthorization`，不得删除、复活旧事件或靠重复运行增生修订。教义边界是：趋势背驰比较最后中枢两侧的 `c` 与 `b`（031:883、033:26），且合取趋势、`c` 含 B 的第三类买卖点、`c` 创新极值、`b` 级别不大于 `c`（037:16、037:18、037:20）；盘背比较同一中枢前后的第一、三段（027:20）；二者的后一比较段都可成为背驰段（027:22）。向下确认的对象始终是背驰段内的背驰段（027:38、027:46）。`Consume_at` 是**授权 BSP 在同一生产事务形成并写入谱系反向边**，不是对既有 BSP 的事后贴标；这是工程冻结，不冒充原文 Rust 数据流。
- **`E2E-F` 地板本地化**：地板属于每条谱系分支，不是一个无类型的全局截断。`CloseFloor_at(as_of)` 仅在当前节点已经位于塔声明的最低形式概念层时，因果地产出 `FormalFloor`；若同一前缀还在线段内部继续缩点，则产出独立 `QuasiFloor(QuasiDivLocalization)`；分支尚在更高层却没有当前可证的低级事件、或数据/规则不足时，只能产出不可消费的 `UnresolvedFloor`，不得靠“未来也不会再来事件”的判断提前闭合。本塔当前对象映射为 `L1 = FormalFloor`、`L0 = QuasiFloor`。Formal 闭合不预言未来没有 L0 定位；后到的类背驰以 `localization_tail_key` 生成一条新谱系并指回 `extends_lineage_key`，不回写原形成钟，消费政策须显式选择接受 Formal 还是要求 Quasi。线段以下只有类背驰且「和背驰是两回事情」（065:94），原文也有最后一重落在线段内部笔间的定位实例（088:196）；因此 L0 不得反注为形式背驰证书或同级 BSP，三分流账由 `E2E-S6` 单独验收。
- **`E2E-L` 谱系一等公民**：塔内必须持久化带规则版本的 `EventKey/LineageKey/BspKey` 及其确定性 ID、父子边、`event_level`、实际父/子级别、`side`、种类、比较对象、`divergence_interval`（旧名 `interval_b`，指后一比较段而非小写 `b`）、力度证据、状态与因果钟、本地地板，以及一等 `ProjectionKey/BspLink` 双向索引；`consumed_bsp_ids[]` 只由 LinkKey 排序派生，不回写终态谱系。不得再靠 `source_index` 等值临时拼接或由 sidecar 事后贴证书。WireV1 冻结固定长度 JSON 数组、原子类型/枚举/空位/排序及 `"v1:" + lowerhex(SHA-256(RFC8785-JCS(x)))`；`LineageKey` 只含谱系规则及选择政策 ID/版本、有序 EventKey 路径、EdgeKey 路径与可选 localization tail，不含 floor/status/clocks/revision；同政策路径延伸才指向唯一 proper-prefix，政策更换是 `extends=null` 的新 key。`BspKey` 是 `(formation_level,point_class,side,source_index)` 全局点身份，多个谱系以各自 ProjectionKey 在同一形成事务共享一点。`event_level` 固定为运行该背驰谓词的塔桶/中枢级别；比较段自身级别与 BSP 形成级别另存，不得混称。`side=Long` 对应底背驰/买侧，`side=Short` 对应顶背驰/卖侧，边的方向一致即 `side` 相等。原文不保证背驰级别与走势级别逐级一一对应（032:227），且存在 30 分钟走势由 1 分钟背驰触发的跨级形态（044:16）；所以谱系可记录显式 skip edge，但不得为缺失的中间级别伪造证书。

迁移映射以 `nest-tower-native-gap-audit-20260717.md` 的 N0–N7 为源；下表定义完成态，不是简化补丁清单：

| 统一编号 | 审计缺口 | 现状对象/缝合线 | 塔内完成态 | 对应实证门 |
|---|---|---|---|---|
| `E2E-N0` | N0 Cand 语义二分 | typed 结构宽候选 vs 旧 `cand_delta` 力度确认 | `ObserveCandidate_at` 只管 `E2E-D1`–`E2E-D3`，`ReviseEvent_at` 只管 `E2E-D4`–`E2E-D5` 与状态转移；全级别一套语义 | `E2E-S1` |
| `E2E-N1` | N1 背驰段区间无塔内载体 | `seg_c/interval_b` 只活在 C2 seam/sidecar | 每级塔原生产出带版本化 `EventKey`、完整 `E2E-D1`–`E2E-D5` 证据与 `divergence_interval` 的事件修订 | `E2E-S2` |
| `E2E-N2` | N2 跨级包含谓词缺 | `is_sub` 只在 nest 管线 | 塔原生 `LineageEdge` 保存 `child.divergence_interval ⊆ parent.divergence_interval` 的逐边见证、同 `side` 及 skip 级别 | `E2E-S3` |
| `E2E-N3` | N3 跨级链/证书缺 | `NestCertificate` 为塔外装配物 | `E2E-L` 成为 `Classification` 的原生、可复验、可持久化输出 | `E2E-S4` |
| `E2E-N4` | N4 事件↔BSP 身份边缺 | `source_index == turn_source` 临时绑定 | WireV1+CanonicalV1 确定 Event/Lineage/全局 Bsp ID；`ProjectionKey` 含 Lineage/policy/rule/slot，唯一 `LinkKey=(BspKey,ProjectionKey)` 写双向边 | `E2E-S7` |
| `E2E-N5` | N5 首证钟缺 | prefix 首见钟与终态几何拼合 | 事件五钟 `observed/first_provable/structure_end/confirmed/invalidated` + 谱系两钟 `opened/closed` 全由同一前缀产生；`postcondition_at?` 仅诊断 | `E2E-S5` |
| `E2E-N6` | N6 选择器缺/DFS 序分叉 | `sel_order`、typed DFS、旧 DFS 三序不一 | 同一 Pan 候选组按比较对逐一发事件并全部保留；若消费者要求唯一见证，谱系保存 `SelectionPolicy` 与未选 EventKey | `E2E-S4`、`E2E-S7` |
| `E2E-N7` | N7 生产消费点缺 | BSP 先产、证书事后标注 | 合法且 hash 固定的 policy 每条唯一 rule 精确投影一个 ProjectionKey（多点拆多规则）；按全局 BspKey 分组，一点一次创建、组内全部链接同事务写入；晚到缺失授权 fail closed，validator 严格联结 s2 事件 head + s4 谱系 head 重算 eligible，零值只能 NotExercised | `E2E-S8` |

依赖序固定为 `E2E-N0 → E2E-N1 → E2E-N2 → E2E-N3`，`E2E-N4/E2E-N5` 依赖 `E2E-N1`，`E2E-N6` 依赖 `E2E-N3`，`E2E-N7` 依赖 `E2E-N3`–`E2E-N6`；八道 `E2E-S*` 未全部给出逐字段实证前，只能称「塔内近似」，不得称端到端原生。此长线不改写阶段 3 的 M7/M8 验收列。

## 全路线纪律

- 写入范围：只写 /tmp/kimi-nest-mainline（分支 kimi-nest-mainline-20260717）；主仓绝对禁写；禁 git mutation；bin 自动发现、禁改 rust/Cargo.toml；生产源码除 1b/1d/2 系列明确授权的文件外只读。
- v3 硬禁令：不引入概率/统计推断作决策基础；不用回测验证策略（历史数据只验证代码正确性与不变量）；不假设有效市场假说。
- 090：禁简化实装、禁补丁方案、声明必须与实际能力一致；教义权威链 博文 > 编纂版 > 思维导图。
- 停止规则（任一触发即停并如实报告，不伪造通过）：既有测试变红且无法在界内修复 / 塔路径 bit-exact 被破坏 / 深研后仍有无法定夺的教义分歧 / 数据源或外部依赖缺失。

## 剩余（不假装完成）

- 本图是依赖框架不是工期承诺；1a 探针的有效域数字可能改写后续关的范围。
- treasury 在杠杆/期货/加密环境是三阶段原文域外延伸，「负成本免强平」禁止编码进强平判据。
- 教义内禀边界（A 类 10 条）永不列为工作项——它们是声明的环境，不是待修的缺陷。
