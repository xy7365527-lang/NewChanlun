# 待裁材料：215 窗 D1 InvalidSeed 收复路线（CarriedOnly 版本化迁移 vs 维持 fail-closed）

- 日期：2026-07-16　状态：**已裁**（Q1=选项 1，用户"按原文裁决"；定稿记录见文末 #96 节）　记录：Claude
- 命名口径：本文 "D1" = #83/#84 诊断计数名（InvalidSeed `165/43/7/0/0`，共 215 窗），
  与 `d1-seed-anchor-d2-episode-fallback-ruling-20260715.md` 的 D1（36 窗种子锚）及
  `c2-pending-rulings-material-20260714.md` 的 D1–D7 是三个命名空间，不可混用。
- 上游材料：
  1. **#84 审计**：`chanlun/review-results/failclosed-audit-20260715.md`（215 窗全部 A3 定格分型、
     `ZD>ZG` 核空；179 窗无其他合法三段，36 窗有滑窗候选）
  2. **#93 探针**：`chanlun/review-results/p93-invalidseed-probe-20260716.md`
     （基线 `e168eb90b6`，守恒锚 `P93_CONSERVATION status=PASS bars=4613599 as_of=4613598
     d1_invalid_seed=215 per_level=[165, 43, 7, 0, 0] events_total=3807`）
  3. 既有裁定链：`d1-seed-anchor-d2-episode-fallback-ruling-20260715.md`（D1 选项 0 + 三条款：
     作废须有解释）、`silent-dual-core-c1-seam-ruling-20260715.md`（#90 条款 3：任何 seed 语义
     变更须独立立项 + 版本化迁移 + A/B 重放对账）、`EXTENDED_TO_EXACT_THREE_V2` +
     `SeedCoreProvenance` 迁移先例（`60a48bcc00`）。

---

## #93 三项测量（已复核，锚见探针 stdout）

1. **携带核 215/215 全覆盖且全部合法**（`carried_exists=215/215 carried_valid=215/215`，
   `centers_len=1`）。"携带核作 seed"的几何前提无死角；#84 的 36 个滑窗候选窗全在内，
   滑窗改锚不再是必需路线。
2. **证书产量域交集 `overlap_any=214/215`**（同级 181/215，跨级 214/215；唯一零交集窗
   `L1 id=1:1182 span=553754:553954`）。"域外封存 + 有效域 complete 收口"只能覆盖 1/215，**作废**。
3. **相邻 run 拓扑**：`adjacent_runs_both=194`（收复 ⟹ 两 run 合并）、`one=20`（延长）、
   `none=1`、`clustered_invalid=21`。任何收复都改变 provider run 分区 ⟹ 语义变更，
   落入 #90 条款 3 管辖，不存在"零影响补洞"。

## 待裁问题

**Q1**：215 窗是否收复？
- **选项 0（维持 fail-closed）**：无实现变更。R7(a) 以显式能力边界声明收口——215 窗为
  "A3 定格分型、三段重叠核空"的教义外几何，`InvalidSeed` 即其"作废解释"
  （满足 2026-07-15 D1 裁定条款 1 的解释义务）。代价：provider 完整性永久缺 215 窗，
  且其中 194 窗两侧 run 被永久隔断。
- **选项 1（CarriedOnly 版本化迁移）**：#93 判定的唯一存活路线。新 provenance 类
  `CarriedOnly`（区别于 `SelfConsistent`/`InheritedRecut`），seed 取塔 compose 携带核，
  version tuple 升版，走 V2 先例式静默缝合。**教义前置**：携带核来自上级塔延伸语义，
  以它充当"被至少三个连续次级别走势类型所重叠的部分"（`0010:5` 存在性定义）是否成立，
  原文层面未回查——按 2026-07-15 裁定先例，结裁前须原文回查（语料 `docs/chanlun/text/chan99/`）。

**Q2**（仅选项 1 需裁）：验收门槛核准——建议条款：
1. 独立任务立项（#90 条款 3），不并入现有任务；
2. 全量重放 A/B 对账：4,374 个未涉窗 bit-exact 零 diff；215 窗逐窗显式 provenance 清单；
3. 下游证书差异清单（因结论 3 的 run 合并，`provide_nest_candidate_events` 输出差异须逐条列出
   交裁决，不得默认接受）；
4. #93 边界补齐：单快照交集口径对逐窗差异不足，迁移内须补 prefix 重放逐时点对账；
5. 原文回查为结裁前置（同 D1/D2 裁定惯例），无翻案方可定稿。

## 边界声明

推荐倾向不在本文给出。选项 0 与选项 1 均满足既有裁定链的一致性约束，取舍属教义裁决
（携带核的存在性定义地位）+ 工程代价权衡（194 窗 run 隔断 vs 全量迁移验证成本），裁决人：用户。

---

## 执行记录（#95，2026-07-16，Claude）

Q2 建议条款 1–4 的验收测量已完成（独立任务 #95；探针 `rust/src/bin/p95_carriedonly_ab.rs`，
报告 `chanlun/review-results/p95-carriedonly-ab-20260716.md`，事件差异全量 815 行
`chanlun/review-results/p95-carriedonly-ab-eventdiff-20260716.md`）：

| 条款 | 结果 |
|---|---|
| 1 独立立项 | 任务 #95，未并入现有任务 |
| 2 全量 A/B | 11,682/11,682 未涉窗 bit-exact，`seed_diff=0`；215/215 收复，`provenance=CarriedOnly` 逐窗打印 |
| 3 run 合并差异 | L1 160→1、L2 40→1、L3 7→1；completed +185/−123、events +580/−232，逐条归档交裁决 |
| 4 prefix 逐时点对账 | 3,137 采样点：`pre_violation=0`、`offdomain_diffs=0`、`bonly_flips=0`（邻域采样口径，演进见报告） |
| 5 原文回查 | p93 补测 0010:29 延伸判据 215/215 触核；0010:5 存在性定义地位待定稿 |

探针终态 `P95_STATUS status=PASS`。本记录不改变本文"待裁"状态：Q1（选项 0/1 取舍）
与条款 3 差异清单的接受与否，仍属裁决人。

---

## 裁决定稿（#96，2026-07-16，裁决人：用户）

**裁决指令**：用户原话"按原文裁决"（本会话，2026-07-16）。据此三项待裁按下述原文回查
记录的读法结裁；解释权行使方式为"以原文行文为准"，回查证据链由 Claude 记录、用户核可。

### 原文回查记录（结裁前置，Q2 条款 5 完结）

语料 `docs/chanlun/text/chan99/0010-第八节 走势中枢.md`，行号锚：

1. **核的诞生（:23）**：ZG=min(g1,g2)、ZD=max(d1,d2)——公式只含头两个 Z 段，g3 以后
   不参与定核；[ZD,ZG] 即中枢区间。⟹ 核形成即定格，原文层面无"逐窗重算核"的要求。
   （代码"定理一核心冻结在父窗"与此逐字对应。）
2. **核的存续（:15 中心定理一、:29 延伸）**：延伸判据 = 任意新段 [dn,gn] 与 **同一个
   固定 [ZD,ZG]** 重叠；原文在整个延伸期反复引用该核为参照物，新段只需触核，无需重新叠出核。
3. **核的死亡（:13 定理三）**：当且仅当次级别离开后回抽不回，中枢终结。无终结事件则中枢存在。
4. **:5 定义的满足时刻**：携带核诞生时即为"被至少三个连续次级别走势类型所重叠的部分"，
   完全在定义内；"携带"仅指跨窗传递，不引入任何定义外区间。

### 三项结裁

| 项 | 裁决 | 依据 |
|---|---|---|
| `0010:5` 携带核存在性定义地位 | **成立**（读法 A） | 上述 1–4：核定格于诞生（:23）、延伸期持续存在（:15/:29）、未经定理三事件不消亡（:13）；215 窗全部处延伸期且价格触核（p93 补测 215/215），无终结事件 |
| Q1 收复路线 | **选项 1：CarriedOnly 版本化迁移定稿** | 教义前置解除；工程验收 #95 全项 PASS（条款 1–4 见上节） |
| 条款 3 差异清单（815 行） | **接受** | 中枢存续 ⟹ run 连续是 :15/:29 的直接推论；run 合并（L1 160→1、L2 40→1、L3 7→1）与事件差异（completed +185/−123、events +580/−232）是对旧漏报的纠正，非副作用 |

### 执行（#96）

- 生产 version tuple 钉版切换：`C2VersionTuple::auto_pairing()` / `pairing_disabled()` 的
  `projection_provider_version` 由 `EXTENDED_TO_EXACT_THREE_V2` → `EXTENDED_TO_EXACT_THREE_V3`；
  `carried_only()` 保留为等值显式命名入口。V2 保留为历史基准锚与 A/B 参照，`validate()` 仍接受。
- 全量测试与提交见 #96 提交记录（hash 见 git log）。
