---
id: "691"
number: 691
status: 生成态   # 概念分离落谱系；转 settled 待编排者 /ritual 最终辨认。本条目锚定「单实例 vs 时序多 campaign」两判断的消解出处链——账本层实装（LedgerOpen/TypedTrade +position_node_id + gen_hiwater）已在 commit 6af9b5fd1a 落地，本条目记录的是「同一 position 生命周期对象的两个判断被 codex 裁定 C + 端到端实证消解」。
date: "2026-07-03"
type: bias-correction   # 前任偏差订正：codex-f2「单 carrier 单 active instance」YAGNI 裁定的「时序再入场不会发生」前提被端到端实证（合成 fill loop gen=1）推翻。同时纠正前任把 generation/entry_certificate 死字段塞 ActiveLeg 的归属层错误（090 声明膨胀实例）。
source: "[新缠论] docs/formal-chain/级别容器.pdf p13-15（§2/§3/§12/§13 四元组伪码 + p15『同容器多次开仓损失 entry-level 区分』）+ 编排者裁定 docs/formal-chain/INDEX.md（唯一权威）；异质裁定 codex a9-posnode decide（裁定 C，`codex exec --skip-git-repo-check --sandbox read-only`）；端到端实证 rust/src/theta_v0/backtest/runner.rs `same_carrier_reentry_distinguished_by_generation`（合成 fill loop 同一 carrier ElementId(0,0) 两次 campaign：buy1@3 gen=0 / buy1@16 gen=1）；核验=ws-a9pos2 逐页 Read + 独立读码（.chanlun/review-results/a9-posnode-20260704.md，Task #166）"
negation_source: heterogeneous   # 主裁定=codex a9-posnode decide（异质）；出处映射+代码核验=ws-a9pos2 逐页 Read PDF + 独立读码（homogeneous）；实证=合成 fill loop 端到端跑数。
negation_model: "codex (a9-posnode decide, sandbox read-only)"
negation_form: separation   # 同一 position 生命周期对象被两判断混淆：codex-f2「单 carrier 单 active instance」（时序单实例）vs gap2rulings 条目1「实装 ceiling 四元组」（时序多 campaign）。分离后：单实例=某一瞬时 ActiveLeg 唯一（结构层，真）vs 时序多 campaign=同 carrier close→reopen 跨时序多次开仓（账本层，实证 gen=1 可达）。两者不冲突——是同一对象在两个层的不同投影，前任把它们当作互斥的单一判断。

# 拓扑效果标注（147号下游推论3：negates 非空必填）
# negates：(1) codex-f2「单 carrier 单 active instance」YAGNI 裁定的「时序再入场不会发生」前提；(2) 前任把 generation(恒0)/entry_certificate(恒None) 死字段塞 ActiveLeg 的归属层错误。
topo_effect: |
  (1) split — codex-f2 的「position 生命周期 = 单实例」节点分裂为两个：一个保留原判断（**瞬时**投影——某一 bar 的 ActiveLeg
      由树重建唯一，结构层，真）；一个携带违反记录（**时序**投影——同 carrier close→reopen 跨时序多次开仓，账本层，实证 gen=1
      可达，YAGNI「不会发生」前提被推翻）。两投影不互斥，前任把 position 生命周期当作单一「瞬时=时序」判断是混淆。
      target=codex-f2「单实例」判断；scope=local（仅四元组归属层判定，账本层实装已落地）。
  (2) sever — 前任把四元组身份（generation/entry_certificate）误归 ActiveLeg 结构层的错误归属边切断。四元组身份 re-anchor 到
      账本生命周期层（LedgerOpen/TypedTrade）——依据 codex 裁定 C：ActiveLeg 每 bar 由 element_as_leg(&CoverageElement)
      从树重建，CoverageElement 不携 campaign 身份，拿不到入场候选/代次；真正同时持有 Candidate+ActiveLeg 的点是
      StepTrace.opened（runner 据此登记 LedgerOpen）。target=ActiveLeg 死字段归属；scope=local。

# 概念分离（type 主线——同一 position 生命周期对象的两判断）
separation:
  before: "同一 position 生命周期对象的身份判定被当作单一判断：codex-f2（coverage.rs:544-550，较早）裁『单 carrier 单 active instance，四元组是无消费者的死机制，YAGNI 不实装』——隐含前提『同 carrier 时序再入场不会发生』。"
  after:
    - name: "瞬时单实例（结构层，ActiveLeg）"
      definition: "在任一 bar，某 carrier 的 ActiveLeg 由 element_as_leg(&CoverageElement) 从走势树重建，同一瞬时唯一。这是 codex-f2 判断成立的域——瞬时投影上确实单实例。"
      source: "级别容器.pdf p14 §14 简化（carrier ActiveLeg::id 作生产持仓身份）；ActiveLeg 每 bar 树重建=瞬时快照。"
    - name: "时序多 campaign（账本层，LedgerOpen/TypedTrade）"
      definition: "同一 carrier 可在时序上 close→reopen 多次开仓（interpret 无历史 tombstone，close 后同级同向候选可重进 open，open_trades.insert 二次 insert 同 ElementId）。每次开仓是独立 campaign，须由 generation 单调区分——四元组 posId=hash(carrier, entry_certificate, side, generation) 补足 entry-level 区分。"
      source: "级别容器.pdf p14 §13 四元组伪码 + p15『同一容器多次开仓时损失 entry-level 区分；更严格的是 position instance』；端到端实证 runner.rs same_carrier_reentry_distinguished_by_generation（buy1@3 gen=0 / buy1@16 gen=1）。"
  pending_verification: "生产 close/risk/silent close 是否真调用 registry.close/invalidate 并拒绝该 carrier 再 open？若拒绝（时序再入场不可达），四元组退化 carrier-id，本分离降级为身份完备性冗余（回退 codex 选项 A）。当前生产 interpret 无 tombstone ⟹ 再入场可达（测试实证 gen=1）——分离成立。"

# 涉及的定义
definitions_involved:
  - name: "codex-f2「单 carrier 单 active instance」YAGNI 裁定"
    version: "coverage.rs:544-550（前任落痕，引 codex-f2 选项a），codex-f2-design-ruling-20260703"
    role: "被推翻的较早裁定。隐含前提『同 carrier 时序再入场不会发生』⟹『四元组是无消费者的死机制，YAGNI 不实装』。端到端实证 gen=1 推翻其前提。"
  - name: "gap2rulings 条目1（A9 范围=账本层 ceiling）"
    version: "codex-gap2rulings-20260703 条目1（#157，较晚）"
    role: "与 codex-f2 张力的另一端：『实装 ceiling 四元组』。codex a9-posnode 裁定 C 消解张力——四元组归账本层（非 ActiveLeg 结构层）。"
  - name: "级别容器.pdf p14 §13 position_node 四元组"
    version: "docs/formal-chain/级别容器.pdf p14 §13（p15 补 entry-level 区分论证），INDEX.md 唯一权威"
    role: "四元组权威定义：position_node_id = hash(carrier_id, entry_signal_id, side, generation)。p15『同一容器多次开仓时损失 entry-level 区分；更严格的是 position instance』=时序多 campaign 的原文锚。"
  - name: "四元组账本层实装（LedgerOpen/TypedTrade + gen_hiwater）"
    version: "HEAD（gap3-rework-codex9-fix），commit 6af9b5fd1a"
    role: "codex 裁定 C 的实装：LedgerOpen/TypedTrade +position_node_id 字段；runner gen_hiwater: HashMap<carrier,u32> 高水位表（open 时 carrier 首见=0、close→reopen=+1 单调）。删 ActiveLeg 死字段。"

# 解决方式
resolution:
  type: 概念分离   # 同一 position 生命周期对象的瞬时单实例（结构层）vs 时序多 campaign（账本层）分离 + codex 裁定 C 归属层裁定 + 端到端实证。
  description: |
    codex a9-posnode decide 裁定 C（三问逐一）：
    (1) **时序再入场可达**：interpret 无历史 tombstone（close 后同级同向候选可重进 open），open_trades.insert
        close→reopen 二次 insert 同 ElementId。codex-f2「不可达」前提**不成立**。
    (2) **四元组归属层 = TypedTrade/LedgerOpen，非 ActiveLeg**：ActiveLeg 每 bar 由 element_as_leg 从树重建，
        CoverageElement 不携 campaign 身份，拿不到入场候选/代次；真正同时持有 Candidate+ActiveLeg 的点是
        StepTrace.opened（runner 据此登记 LedgerOpen）。
    (3) **ceiling 严格形式 = C（账本层）**：Nautilus account 层只 net_position 单净仓，无 carrier 级仓位复用；
        现有消费者已按交易行分离（typed_ledger: Vec<TypedTrade>）。
    **端到端实证消解**：合成 fill loop 同一 carrier ElementId(0,0) 两次 campaign（buy1@3 bar7 开 / bar14 反向关，
    buy1@16 bar17 再开）——voice_id 碰撞（同 hostOf）但 posId 不碰撞（gen=0 vs gen=1）。时序再入场**在合成数据就已发生**。
  decided_by: "异质裁定（codex a9-posnode decide 裁定 C）+ 端到端实证（ws-a9pos2 合成 fill loop，Task #166）+ team-lead 派 #178 立条落盘"

# 被否定的方案
negated:
  description: "codex-f2「单 carrier 单 active instance」YAGNI 裁定（前任据此在 interp.rs 加 generation(恒0)/entry_certificate(恒None) 死字段塞 ActiveLeg + 无消费者的 position_node_id()/PositionNodeId/hash64()）。"
  why_negated: |
    (1) **前提被事实推翻**：codex-f2 的 YAGNI 依据是「同 carrier 时序再入场不会发生 ⟹ 四元组无消费者」。
        端到端实证（合成 fill loop）显示 interpret 无 tombstone，同一 carrier ElementId(0,0) close→reopen 二次开仓
        gen=1 已发生——「不会发生」是错前提。YAGNI 只在需求确实不存在时成立；需求在合成数据就出现 ⟹ YAGNI 不适用。
    (2) **前任实装同时违反两裁定**：把 generation(恒0)/entry_certificate(恒None) 死字段塞 ActiveLeg——既没实装
        ceiling（值恒退化，四元组无区分力，违 gap2rulings 条目1），又声明膨胀（字段存在但恒 None/恒 0，声明了
        ActiveLeg 不具备的 campaign 区分能力，违 090 号）。
    (3) **归属层错误的一般教训**：ActiveLeg 每 bar 从树重建（element_as_leg），CoverageElement 不携 campaign 身份——
        把携时序代次的四元组塞进纯瞬时重建的结构层，字段必然恒退化。**字段放错层则恒退化**——身份四元组须归
        账本生命周期层（LedgerOpen/TypedTrade），那里才真正同时持有 Candidate+ActiveLeg（StepTrace.opened）。

# 新产出
new_output:
  definitions:
    - "**position 生命周期两投影分离**：瞬时单实例（结构层 ActiveLeg，某 bar 树重建唯一，codex-f2 判断的真域）vs
      时序多 campaign（账本层 LedgerOpen/TypedTrade，同 carrier close→reopen 多次开仓，gen 单调区分）。两者非互斥——
      同一对象在两层的不同投影。codex-f2 把瞬时投影的真判断错误外推到时序投影。"
    - "**四元组归属层裁定（codex 裁定 C）**：posId=hash(carrier, entry_certificate, side, generation) 归**账本生命周期层**
      （LedgerOpen/TypedTrade），非 ActiveLeg 结构层。判据：ActiveLeg 每 bar 从树重建拿不到 campaign 态；真正同时持有
      Candidate+ActiveLeg 的点是 StepTrace.opened。"
    - "**「字段放错层则恒退化」一般教训**：把携时序代次/campaign 态的身份字段塞进纯瞬时重建的结构层，字段必然恒退化
      （generation 恒0、entry_certificate 恒None）= 声明膨胀（090 实例）。身份字段的层归属由「哪一层真正持有该态」决定。"
  code_changes: |
    commit 6af9b5fd1a（ws-a9pos2 #166 实装，本条目为其谱系落痕）：
    - interp.rs：删 ActiveLeg 2 死字段（entry_certificate/generation）+ position_node_id() 方法（消归属层错误+声明膨胀）；
      保留 EntryCertificate/PositionNodeId/hash64() 类型，语义迁账本层。
    - runner.rs：LedgerOpen/TypedTrade +position_node_id 字段；runner +gen_hiwater: HashMap<carrier,u32> 高水位表
      （open carrier 首见=0、close→reopen=+1 单调）+ 5 处 close 写入 posId + 窗口末 censored 写入。
    - 端到端测试 same_carrier_reentry_distinguished_by_generation（gen=1 实证）。全库 1478 测试通过，bit-exact 保持。
  orchestration_changes: "无编排层变更。codex-f2-design-ruling 被本号以 codex a9-posnode 新裁定 C 覆盖其「不可达」前提（实证优先于设计裁量——异质裁决非实施授权，本处以端到端实证为消解锚）。"

# 影响范围
impact:
  affected_modules:
    - "rust/src/theta_v0/strategy/interp.rs：删 ActiveLeg entry_certificate/generation 死字段 + position_node_id() 方法；
      EntryCertificate/PositionNodeId/hash64() 类型语义迁账本层。本条目锚定归属层订正正确性。"
    - "rust/src/theta_v0/backtest/runner.rs：LedgerOpen/TypedTrade +position_node_id + gen_hiwater 高水位表 + 5 处 close 写入。
      本条目锚定四元组归账本生命周期层的正确性。"
    - "coverage.rs:544-550：前任 codex-f2 YAGNI 落痕——错源须订正（隐含『时序再入场不会发生』前提被实证推翻）。
      本条目为订正载体，不改写原落痕（谱系落痕订正，非篡改历史）。"
  affected_definitions:
    - "PositionNodeId 四元组：归属层从 ActiveLeg（结构层，前任错误）订正为 LedgerOpen/TypedTrade（账本生命周期层）。"
    - "ActiveLeg：删 campaign 身份字段——回归纯瞬时结构层持仓身份（carrier ActiveLeg::id，§14 简化）。"
    - "generation：从恒 0 死字段升级为 runner gen_hiwater 高水位表单调赋值的真值（carrier 首见=0，close→reopen=+1）。"
  downstream_implications:
    - "A11（#167 多空对冲 overlay 账本）：TypedTrade 现携完整 position instance 身份，为 overlay 头寸提供 carrier 级
      campaign 区分基座（overlay 须按 position instance 而非 carrier 归属）。"
    - "μ 层口径不变：仍按 entry_z 逐笔观测，posId 不入统计——本实装零 μ 口径变更、零 alpha 声明（L1 管线正确性，非 L2）。
      若未来 μ 层引入按 posId 分桶/去重的消费者，posId 从『身份完备载体』升级为『统计输入』，须 L2 重跑（新 prereg）。"
    - "090 又一 locus：『字段放错层则恒退化』——凡把携时序/campaign 态的身份字段塞进纯瞬时重建的结构层者，字段必恒退化
      = 声明膨胀。身份字段层归属由『哪一层真正持有该态』决定，非由『字段代数可加于哪个 struct』决定。"

# 回溯结算（待编排者 /ritual + 错源核销后补记）
retroactive_settlement:
  settled_by: "编排者 /ritual 最终辨认 + coverage.rs:544-550 codex-f2 YAGNI 落痕 / codex-f2-design-ruling 错源核销后，由 genealogist 回填 settled/。"
  settlement_date: null
  settlement_description: null

# 谱系关联
related_records:
  parent: "090（声明膨胀禁止）——前任死字段（generation 恒0/entry_certificate 恒None）塞 ActiveLeg = 声明膨胀，本号否定之；638（bsp host attachment=carrier 身份来源）——carrier=hostOf(g) 本级右端点命中，四元组 carrier 分量的身份来源。"
  children: []   # coverage.rs codex-f2 落痕全库核销 + A11 overlay 按 posId 归属落地后可派生实证子记录。
depends_on: ["090", "638"]
related: ["690", "231"]
---

# bias-correction 691：codex-f2「单 carrier 单 active instance」YAGNI 被端到端实证推翻（gen=1 时序再入场）

## 结论

codex-f2（`coverage.rs:544-550` 前任落痕）裁「单 carrier 单 active instance，四元组是无消费者的死机制，YAGNI
不实装」，隐含前提「同 carrier 时序再入场**不会发生**」。经 ws-a9pos2 端到端实证（Task #166），此前提被推翻——
**时序再入场在合成数据就已发生**：合成 fill loop 同一 carrier `ElementId(0,0)` 两次 campaign（`buy1@3` bar7 开 /
bar14 反向关，`buy1@16` bar17 再开），`voice_id` 碰撞（同 `hostOf`）但 `posId` 不碰撞（gen=0 vs gen=1）。

**同一 position 生命周期对象的两判断分离**：
1. **瞬时单实例（结构层 `ActiveLeg`）**：某 bar 的 ActiveLeg 由 `element_as_leg(&CoverageElement)` 从树重建，
   同一瞬时唯一——codex-f2 判断成立的**真域**（瞬时投影）。
2. **时序多 campaign（账本层 `LedgerOpen`/`TypedTrade`）**：同 carrier close→reopen 多次开仓（interpret 无
   历史 tombstone），每次是独立 campaign，须 generation 单调区分——codex-f2 错误外推到的**时序投影**。

两者不互斥，是同一对象在两层的不同投影。codex-f2 把瞬时真判断外推到时序，前提在合成数据即被证伪。

## 定义依据

- **级别容器.pdf p14 §13**（唯一权威）：`position_node_id = hash(carrier_id, entry_signal_id, side, generation)`
  + `position_node.carrier/entry_signal/side/level` 伪码。**p15**：「如果不想引入 position instance，也可以简化为
  `host.entry_signal=g/host.side/host.active=true`。但这会在**同一容器多次开仓时损失 entry-level 区分**；更严格的是
  position instance」——时序多 campaign 的原文锚。
- **级别容器.pdf p14 §12**：「持仓节点必须是容器 carrier 或容器-入场证书 position instance。买卖点叶子只能作为
  开仓证书，不能作为父声部持仓节点」——四元组归属的权威依据。
- **codex a9-posnode decide 裁定 C**（异质）：三问确认（时序再入场可达 / 四元组归 TypedTrade|LedgerOpen 非
  ActiveLeg / ceiling 严格形式=账本层 C）。
- **端到端实证**：`runner.rs` `same_carrier_reentry_distinguished_by_generation`——buy1@3 gen=0 / buy1@16 gen=1，
  posId 不碰撞。ws-a9pos2 Task #166 逐页 Read PDF + 独立读码（`.chanlun/review-results/a9-posnode-20260704.md`）。

## 边界条件（结论翻转）

- **(a) 生产 close/risk/silent close 真调用 `registry.close/invalidate` 并拒绝该 carrier 再 open**（时序再入场
  不可达）⟹ generation 恒 0、四元组退化 carrier-id，本分离降级为身份完备性冗余（无害但无增量），应回退 codex
  选项 A（删）。**当前生产 interpret 无此拒绝（无 tombstone）**，故再入场可达（测试实证 gen=1）——分离成立。
- **(b) 未来 `element_as_leg` 改为持久 campaign 对象**（ActiveLeg 不再纯树重建、可携账本态，结构层与账本层合一）
  ⟹ 四元组归属层可回迁 ActiveLeg，本号归属层判定须重审。
- **(c) μ 层引入按 posId（而非 entry_z）分桶/去重的消费者** ⟹ posId 从「身份完备载体」升级为「统计输入」，须
  L2 重跑（新 prereg）。当前 μ 口径不变（posId 不入统计）。

## 下游推论

- **A11（#167 多空对冲 overlay 账本）**：TypedTrade 现携完整 position instance 身份，为 overlay 头寸提供 carrier 级
  campaign 区分基座（overlay 须按 position instance 而非 carrier 归属）。
- **μ 层零口径变更**：仍按 entry_z 逐笔观测，posId 不入统计——本实装零 μ 口径变更、零 alpha 声明（L1 管线正确性，
  非 L2）。全库 1478 测试通过，bit-exact 保持（ActiveLeg 删字段不改 `==`/held 对位逻辑，账本层加字段不改 P&L/equity）。
- **090 又一 locus——「字段放错层则恒退化」**：前任把 generation(恒0)/entry_certificate(恒None) 死字段塞
  ActiveLeg（每 bar 树重建的结构层拿不到 campaign 态）⟹ 字段必然恒退化 = 声明膨胀。身份字段层归属由「哪一层
  真正持有该态」决定（`StepTrace.opened` 同时持 Candidate+ActiveLeg ⟹ 账本层），非由「字段代数可加于哪个 struct」决定。

## 谱系引用

- 母规则：`090`（声明膨胀禁止——前任死字段塞 ActiveLeg = 声明膨胀，本号否定之并订正归属层）、`638`（bsp host
  attachment=carrier 身份来源——carrier=hostOf(g) 本级右端点命中，四元组 carrier 分量的身份来源）。
- 相邻：`690`（两 PDF z 形态 + d 层归属裁定——同为「字段/维的层归属」订正族：690 裁 d 归风险层非 μ 分类 z，
  本号裁四元组归账本层非 ActiveLeg 结构层）、`231`（形式化有效域——codex-f2 单实例判断的「定义域」= position 生命周期
  全体，但其「有效域」仅瞬时投影；时序投影上有效域为空 = 前提被证伪）。
- 溯源链：codex-f2（coverage.rs:544-550，较早，YAGNI「不可达」前提）→ 前任死字段塞 ActiveLeg（同违两裁定）→
  gap2rulings 条目1（#157，较晚，实装 ceiling）张力 → codex a9-posnode decide 裁定 C（异质，归属层裁定）→
  ws-a9pos2 端到端实证（gen=1 可达，Task #166）→ team-lead 派 #178 立条。

## 影响声明

纯谱系记录，**零 git 代码改动**（账本层实装已在 commit 6af9b5fd1a 落地，本条目为其谱系落痕）。本条目锚定：
(1) 同一 position 生命周期对象的**瞬时单实例（结构层 ActiveLeg）vs 时序多 campaign（账本层 LedgerOpen/TypedTrade）
两投影分离**——codex-f2「单实例」判断的真域仅瞬时投影，外推到时序投影的「不会发生」前提被端到端实证（gen=1）推翻；
(2) **四元组归属层裁定（codex 裁定 C）**：posId 归账本生命周期层，非 ActiveLeg 结构层（每 bar 树重建拿不到 campaign 态）；
(3) **「字段放错层则恒退化」一般教训**——前任 generation(恒0)/entry_certificate(恒None) 塞 ActiveLeg = 090 声明膨胀
实例。边界条件(a)（生产真拒绝再 open）成立时四元组退化 carrier-id，应回退 codex 选项 A——已显式记录，不预判。
