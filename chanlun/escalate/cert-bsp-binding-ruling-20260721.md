# 正式裁定：nest 证书接入买卖点判定路径的方式（task #101 定稿）

**状态：代理裁定生效（2026-07-21 定稿）**
**Supersede 关系**：本文 = `chanlun/escalate/cert-bsp-binding-ruling-DRAFT-20260717.md`（下称 DRAFT，行号引其）的定稿正式版；DRAFT 原文按不可篡改纪律**原样保留**（一字未动），自本文落盘起以本文为准。
**工位与纪律**：纯文档定稿，零代码、零 cargo、零 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 零写入；全部核验在 worktree `/tmp/kimi-nest-mainline` 内只读完成。
**性质澄清（090）**：task #101 是**证书 → 买卖点（BSP）绑定/接入方式**裁定，不是级别标定裁定本身。「91 张证书全绑 lvl=0」的级别标定断裂是 **task-107**（2026-07-17，`f49475510e`/`d32f6d8b2b`/`d3ba02dbf3`，在 `main`/`merge-mainline-20260719`，不在本 worktree HEAD 祖先链）的后续发现，并经 V1 严格化（2026-07-20，探针 `rust/src/bin/v1_level_calib_fix.rs`）修订。两者仅在本文条款 5（层级不硬映射）处交接，修订注记见 §T5。

**依据**（引用格式 `文件:行号`）：

- DRAFT 本体：`chanlun/escalate/cert-bsp-binding-ruling-DRAFT-20260717.md`（条款 :12-31；开放条款 :33-37；物化校验 :39-45；待主人裁 :47-50）。
- #100 对账：`chanlun/review-results/nest-cert-bsp-recon-20260717.md`（全文直读）；`chanlun/review-results/p100-cert-bsp-recon-20260717.md`（BSP 总量分层 :55-59）。
- #103 双测：`chanlun/review-results/p103-truncation-replay-doubletest-20260717.md`（全文直读）。
- 缺口分析：`chanlun/review-results/p101-ruling-gap-analysis-20260717.md`（task #101 配套，逐条款可执行性核对 + 实装候选，全文直读）。
- V1 严格化：`chanlun/review-results/v1-level-calibration-attribution-20260720.md`、`chanlun/review-results/v1-level-calibration-fix-20260720.md`（全文直读）。
- 探针（直读源码）：`rust/src/bin/p100_cert_bsp_recon.rs`、`rust/src/bin/p101_cert_bsp_tag.rs`、`rust/src/bin/p92_nest_replay_postruling.rs`（:176）。
- 台账：`chanlun/review-results/nest-chain-existing-inventory-20260720.md`（:15、:79、:134）、`chanlun/review-results/e2e-existing-implementation-synthesis-20260720.md`（:69）。

## 总览

| # | 裁定 | 一句话内容 | 定稿核验 |
|---|---|---|---|
| T1 | **接入 = sidecar 打标，判据层不可见** | 证书不生成独立信号、不进判定主路径；只作已确认 BSP 的加权/过滤标注 | 已核验（探针层）；生产层未接线照实登记 |
| T2 | **绑定规则 = judge_max + 最近同侧 + forward 消歧** | cert 钟取 judge_at 逗号钟 max；绑最近同侧 BSP 确认 bar；\|dt\| 相等 forward 胜 | 已核验（探针逐字同规则） |
| T3 | **强度分档 240/1440** | \|dt\|≤240 strong；240<\|dt\|≤1440 weak；>1440 不绑定且 escalate | 已核验（探针）；探针语义缺口 a 登记（E2） |
| T4 | **时钟纪律：created_at 禁入** | 绑定只用 judge_max 与 BSP 确认 bar | 已核验 |
| T5 | **层级不做硬映射** | nest exec/top ≠ classifier lvl；near_lvl 仅归因——**经 V1 严格化修订**（口径显式分离 + T1 移位为唯一合法桥），立场维持并细化 | 已核验 + V1 修订注记 |
| T6 | **打标不回改（append-only）** | BSP 产生/确认/撤销不受影响 | 已核验（探针层）；生产层未接线照实登记 |
| T7 | **同 BSP 多证书合并：布尔 + 最强档** | 张数仅归因字段，不得线性放大权重 | 已核验（实证基础 91 口径）；合并语义未物化照实登记 |
| T8 | **稳定主键 = exec-base 判定事件** | (caliber, exec, base 事件, judge_max, side)；top 链身份仅归因 | 依据已核验；主键列未发射登记（E3） |
| G1 | 挂起：判据层不可见是否升格代码硬约束 | DRAFT 待主人裁 #1 | **未终审**（过渡口径：纪律维持，见 §开放与挂起） |
| G2 | 挂起：240 阈值固定 vs 级别自适应 | DRAFT 待主人裁 #2 | **未终审**（过渡口径：固定 240） |

---

## T1 接入方式 = sidecar 打标（DRAFT 条款 1，:14-15）

**裁决**：证书不生成独立买卖信号、不进入判定主路径；只作为已确认 BSP 的加权/过滤标注（下游仓位/风控可用，判据层不可见）。**生效**。

**核验**：p101 探针只读 sidecar（`p101_cert_bsp_tag.rs:18-19` 头注「不动判据、不写主路径状态」；`P101_TAG` 发射 :218-221）；实证基础 = #100 反向覆盖 w240 0.17% / w1440 0.97%（`nest-cert-bsp-recon-20260717.md:25-26`，直读命中）——证书密度低约 300 倍，不能作独立入场源。**照实登记**：生产侧无任何打标存储/消费点，「判据层不可见」目前靠纪律维持而非代码强制（缺口分析 :25-27）——升格问题即挂起项 G1。

**验收线**：任何实装把打标产物引入 classifier/* 判据路径 ⇒ 拒绝合入。

## T2 绑定规则（DRAFT 条款 2，:16-18）

**裁决**：cert 钟取 `judge_at` 逗号钟表的 max（judge_max）；绑最近**同侧** BSP 确认 bar；|dt| 相等时 forward（bsp_bar ≥ judge_max）胜出。**生效**。

**核验**：`p101_cert_bsp_tag.rs:76-82`（judge_max = judge_at split(',') max）、`:100-121`（`nearest_dt_tie`：partition_point 前后候选，|dt| 相等取 forward 并标 tie）——直读命中；与 p100 `nearest_dt`（`p100_cert_bsp_recon.rs:97-113`）逐字同规则（缺口分析 :30 已核，本轮抽核两探针结构一致）。同侧口径：Long↔buy bits / Short↔sell bits（p101 :166-170, 194-195）。**如实标注**：91 口径 ties=0，forward 消歧分支无实证覆盖，仅有代码路径（缺口分析 :31）。

**验收线**：绑定逻辑进生产须单一实现源（探针/生产共函数），违 644 meta-rule 即拒（缺口分析 :68）。

## T3 强度分档（DRAFT 条款 3，:19-20）

**裁决**：|dt| ≤ 240 → strong；240 < |dt| ≤ 1440 → weak；>1440 → 不绑定（一旦出现即 escalate，不得静默丢弃）。**生效**。

**核验**：`strength()`（`p101_cert_bsp_tag.rs:123-129`）逐字命中。**缺口照实登记**（缺口分析 :37）：探针把 |dt|>1440 证书计入 unbound（p101 :205-207）**但仍插入 per_bsp 绑定表**（p101 :212）——当前 unbound=0 无实害，复跑前宜修正或注明（执行项 E2）。

**验收线**：|dt|>1440 出现即 escalate；P101_TAG 行可见性兜底。

## T4 时钟纪律（DRAFT 条款 4，:21-22）

**裁决**：绑定与打标只允许用 judge_max 与 BSP 确认 bar；`created_at` 与任何前视字段禁入。**生效**。

**核验**：`p92_nest_replay_postruling.rs:176` `P92_RULE ... created_at=forbidden` 直读命中；p101 只解析 judge_at（:76-82），CERT 行本无 created_at 字段；外部一致性：#91 裁定 ④（`nest-migration-ruling-20260716.md:17`，转引自缺口分析 :42，本轮未直读该文件）。

**验收线**：任何绑定/打标路径引入 created_at 或终态回填字段 ⇒ 拒绝合入。

## T5 层级：nest exec/top 与 classifier lvl 不做硬映射（DRAFT 条款 5，:23）——**已被 V1 严格化修订，见 v1_level_calib_fix**

**裁决**：nest exec/top（塔索引/证书递归深度）与 classifier lvl（窗口合成级别）不做硬映射；`near_lvl` 仅作归因报告字段。**立场维持并细化生效**——V1 严格化后精确化为三口径显式分离：

1. **「不做硬映射」经 task-107/V1 全程考验后维持**。task-107 实测 91/91 证书在 bar 距离判据下全绑 lvl=0（转引自 `v1-level-calibration-attribution-20260720.md:20-28`，task-107 原文 `p107-level-calib-20260717.md` 不在本 worktree，**未直读**）；V1 归因裁定此为**语义定义分叉 + bar 距离判据无区分力，不是赋值 bug**（attribution §3.2-3.3，直读命中）。「全绑 lvl0」是 lvl0 密度最高 + 层级嵌套的判据必然输出，不携带级别信息。
2. **T1 移位 exec=k ↔ lvl=k−1 为两口径间唯一合法桥**（`v1_level_calib_fix.rs:524`；教义锚 037:16/024:18/043:26——转引自 attribution :73-79，该报告已直读主仓课文核对，本轮未重核原文）。
3. **跨体系级别对齐判据从 bar 距离升级为区间包含**：V1 探针实测窗口包含判据一致性 **89/89 = 100%**（exec=1 87/87；exec=2 可判定 2/2），对照旧判据同点 8.8% / 同级 bar 距离 34.1%（`v1-level-calibration-fix-20260720.md` §2、§6，直读命中）。残存两项如实落账：右缘确认延迟 2 张（idx 89/90，不可判定非反例，§3.1）；A5 存在性单调 **exist_mono=24/25 fail**（唯一残存 idx48 = 跨 src 链中间节点 d=0，检验对象推广非 p108 反例，§3.2/§4，探针 exit=1 照实登记）。
4. **绑定锚定维持现状语义**：V1 路 A (iii)「绑定一律锚 lvl0 端点」（attribution :120）与 T2 的「最近同侧 BSP」经验上汇合（lvl0 密度主导），不改变 T2 规则文本；`near_lvl` 仍仅归因（p100 :203-211 报告字段、p101 绑定键 (bsp_bar, side) 完全不带 lvl :191-221——直读命中，与缺口分析 :44-45 一致）。

**验收线**：任何实装/文档把 nest exec/top 直接读写为 classifier lvl（除 T1 移位桥）⇒ 拒绝合入；引用「91 全绑 lvl0 = 断裂」的初版强结论而不带 V1 修订注记 ⇒ 口径污染（090）。

## T6 打标不回改（DRAFT 条款 6，:24）

**裁决**：打标是 append-only 的事后标注；BSP 的产生、确认、撤销一律不受影响。**生效**。

**核验**：探针层结构性成立（只读）；生产层尚无接入，属待保持的设计不变量（缺口分析 :47-49）。**照实登记**：未物化（生产层）。

**验收线**：接入实装默认臂（writer 关 / 系数 1.0 / chi=None）逐字节不变（缺口分析 :115 铁律 2）。

## T7 同 BSP 多证书合并（DRAFT 条款 7，:25-28）

**裁决**：同一 (bsp_bar, side) 收到多张证书时，打标语义为布尔（有/无）＋强度取各证书最强档；证书张数仅作归因字段，不得按张数线性放大权重。**生效**。

**核验**：实证基础 = 末端同一顶层事件（2:4613084）与 39 个基例组合出 39 张证书、judge_max=4613104 dt=0 绑同一 BSP——`p103-truncation-replay-doubletest-20260717.md:40-42` 直读命中（「末端 `2:4613084` 簇（39 张、judge=4613104）」）；V1 修复报告 §3.1（:62-64）同簇两张 exec=2 证书交叉命中。簇可见性已物化（`P101_MULTI`，p101 :226-232）；**照实登记**：合并语义本身（布尔化 + 取最强档）在消费侧未实装（缺口分析 :51-53）；该簇属 91 口径，p92 装配 66 口径下不存在（p103 :40-42），但 66 口径下链级多证书现象仍在（缺口分析 :53）。

**验收线**：多证书簇必须 P101_MULTI 可见；任何按张数线性放大权重的实装 ⇒ 拒绝合入。

## T8 稳定主键 = exec-base 判定事件（DRAFT 条款 8，:29-31）

**裁决**：对外发布/打标的幂等主键取 (caliber, exec, base 事件, judge_max, side)；top 链身份仅作归因字段，中段快照间允许迁移。**生效**。

**核验**：依据 = #103 双测：unfold_exact=66、1 幽灵 = top 链身份迁移（3:725489→3:689893，pan→mixed，base 判定不变）——`p103-truncation-replay-doubletest-20260717.md:17-28` 直读命中；缺口分析 :57 以 dump:35 vs :807 双向直核吻合（base judge 707523 不变）。**照实登记**：p101 仍以完整 ids 串（含 top 身份）作证书键输出（p101 :49, 219），未发射条款 8 主键列（缺口分析 :58）——执行项 E3。

**验收线**：对外发布/打标一律用 T8 主键；top 链身份不得进幂等键。

---

## 开放条款与挂起项

**开放条款维持（DRAFT :33-37，原文语义不变）**：

- B 口径 Long 侧稀少：91 口径 B Long=11/45（`nest-cert-bsp-recon-20260717.md:20-21` 直读命中）；66 口径实测 B Long=12/25（转引自缺口分析 :61）。**#102 台账题名「B 口径 Long 侧 0 张」与两个口径实测均不符**，归因落地前方向不对称权重禁令（DRAFT :35-36）继续有效。
- strong/weak 具体权重数值属策略层参数，不属本裁定范围。

**挂起项（DRAFT 待主人裁 :47-50，本定稿不越权终审，登记过渡口径）**：

- **G1**（判据层不可见是否升格代码硬约束）：过渡口径 = 纪律维持（T1 验收线）。缺口分析建议升格（打标产物置于 classifier 不可达命名空间 + CI grep 门，:114）——记录在案待主人裁。
- **G2**（240 阈值固定 vs 级别自适应）：过渡口径 = 固定 240（T3 现状）。缺口分析建议先固定 240 落地、|dt| 按 near_lvl 分层出报告后再议（:38）——记录在案待主人裁。

## 定稿复核登记（逐项）

| DRAFT 引用 | 核验结果 |
|---|---|
| :3 依据 #100 对账 | 已核验：`nest-cert-bsp-recon-20260717.md` 全文直读，91 张（A46/B45）、28,417 BSP、1440 bar 100%、median\|dt\|=0、反向 0.17%/0.97%、~300 倍密度全部命中（:4, :11-12, :14, :25-26） |
| :4 物化工具 p101 | 已核验：`rust/src/bin/p101_cert_bsp_tag.rs` 存在且为只读探针（:18-19） |
| :8-9 两口径 1440 内 100%、反向覆盖 | 已核验（同上 #100 行号） |
| :16-18 条款 2 绑定规则 | 已核验：p101 :76-82, :100-121；p100 :97-113 同规则 |
| :19-20 条款 3 分档 | 已核验：p101 :123-129 |
| :21-22 条款 4 时钟纪律 | 已核验：p92 bin :176 `created_at=forbidden`；#91 裁定 ④ 为转引（未直读） |
| :23 条款 5 层级 | 已核验 + V1 修订（见 §T5；near_lvl 归因 p100 :203-211, :213 直读命中） |
| :25-28 条款 7 簇实证 | 已核验：p103 :40-42；v1 fix §3.1 交叉命中 |
| :29-31 条款 8 主键依据 | 已核验：p103 :17-28 |
| :35-36 B Long 11 张 | 已核验：#100 :20-21 |
| :41-45 物化校验数字（bound=91/unbound=0/ties=0/distinct_bsp=31/multi_bsp=22；A 39/7、B 39/6；BSP 28,417/14,762/13,655） | **多文档转引一致，本轮未重跑探针（零 cargo 纪律）**：强度分档与 #100 w240=39/39 算术自洽（39+7=46、39+6=45）；BSP 总量分解经 `p100-cert-bsp-recon-20260717.md:57-59` 直读命中（该文档自证与 DRAFT :43 逐字一致），p105/p106 复跑亦逐字一致（转引）；distinct_bsp=31/multi_bsp=22 仅 DRAFT 与缺口分析 §3 表互引，无第三来源——标「未独立复核」 |

**缺口检查结论**：未发现证据链缺口。如实登记三项非缺口事项：①ties=0 使 forward 消歧分支无实证覆盖（T2）；②p101 :212 per_bsp 插入语义（T3，E2）；③条款 8 主键列未发射（T8，E3）。

## 执行项登记

- **E1｜66 口径复跑重建实证数字**：#100/p101 以 p92 装配 66 键（A41/B25）复跑，重建绑定/强度/簇/主键列全部数字（清单见缺口分析 §3）；BSP 侧理论必须逐字复现 28,417/14,762/13,655，不同即暴露 BSP 提取口径漂移，escalate。执行关：待 #100 接口任务；本轮不跑（零 cargo 纪律）。
- **E2｜p101 :212 per_bsp 插入语义**：unbound 证书仍入绑定表，复跑前修正或注明（缺口分析 :37）。代码任务，不在本轮。
- **E3｜条款 8 主键列增列**：p101 输出增列 (caliber, exec, base 事件, judge_max, side) 幂等主键（缺口分析 :58, :106）。代码任务，不在本轮。

## 涉及文件清单

- **落盘（本论写入）**：`chanlun/escalate/cert-bsp-binding-ruling-20260721.md`（本正式版，worktree 新文件）；`chanlun/escalate/cert-bsp-binding-ruling-DRAFT-20260717.md`（DRAFT 原件，不可篡改纪律原样保留，零改动）。
- **证据只读（worktree）**：`chanlun/review-results/nest-cert-bsp-recon-20260717.md`；`p100-cert-bsp-recon-20260717.md`；`p103-truncation-replay-doubletest-20260717.md`；`p101-ruling-gap-analysis-20260717.md`；`v1-level-calibration-attribution-20260720.md`；`v1-level-calibration-fix-20260720.md`；`nest-chain-existing-inventory-20260720.md`；`e2e-existing-implementation-synthesis-20260720.md`；`rust/src/bin/p100_cert_bsp_recon.rs`；`rust/src/bin/p101_cert_bsp_tag.rs`；`rust/src/bin/p92_nest_replay_postruling.rs`。
- **未直读（转引照实标注）**：`chanlun/review-results/p107-level-calib-20260717.md`（task-107 原文，不在本 worktree HEAD 树）；`chanlun/escalate/nest-migration-ruling-20260716.md`（#91 裁定 ④）；教义课文 037:16/024:18/043:26（经 attribution :73-79 转引，该报告已直读核对）。
- **实装按裁定关将触及（非本轮）**：`rust/src/bin/p101_cert_bsp_tag.rs`（E2/E3）；生产接入点候选见缺口分析 §2（推荐序列 A 影子 → C 加权 → B 风控侧远期）。

## 代理裁定性质声明

1. **授权来源**：编排者 2026-07-21 下达纯文档定稿任务（找到 task-101 DRAFT 本体 → 逐项核验当前代码树一致性 → 落正式版；零代码、零 cargo、零 git mutation、主仓零写入；过时项照实标注不删原文）。本文在该授权域内落锤。
2. **生效范围**：T1–T8 由代理工位签署，自落盘起为工作口径生效；G1/G2 维持 DRAFT 挂起状态，本定稿**不**代主人终审（过渡口径已列明）。本文 = 证据链核验下的代理定稿，**不是人裁终审**。
3. **醒后复议通道**：编排者可对任一条款发起复议；推翻/修订以新 escalate 文档显式 SUPERSEDE 本文为准，推翻前本文有效。复议触发条件——T1–T8：实测出现与条款相悖的具体案例（带文件行号与事件流证据），或 E1 复跑重建的数字推翻 91 口径结论；T5：级别语义另裁（如 p117 S1a :184 二维一次裁定程序落地身份层移位）。
4. **090 纪律自检**：零代码改动、零 cargo（全部数字为直读文档/源码核验或多文档转引，未重跑任何探针——DRAFT :41-45 物化校验数字标「多文档转引一致，未独立复核」）；零 git mutation；主仓零写入；DRAFT 原件零改动；新增文件仅本文。未直读项（task-107 原文、#91 裁定、教义课文三处）已全部照实标注转引。

## 签字位

- [x] T1 sidecar 打标（判据层不可见）——代理签署生效（2026-07-21）
- [x] T2 绑定规则（judge_max + 最近同侧 + forward 消歧）——代理签署生效（2026-07-21）
- [x] T3 强度分档 240/1440（>1440 escalate）——代理签署生效（2026-07-21）
- [x] T4 时钟纪律（created_at 禁入）——代理签署生效（2026-07-21）
- [x] T5 层级不硬映射（V1 严格化修订注记并入：口径分离 + T1 移位桥 + 区间包含判据 89/89）——代理签署生效（2026-07-21）
- [x] T6 打标不回改（append-only）——代理签署生效（2026-07-21）
- [x] T7 同 BSP 多证书合并（布尔 + 最强档，张数不线性放大）——代理签署生效（2026-07-21）
- [x] T8 稳定主键 = exec-base 判定事件——代理签署生效（2026-07-21）
- [ ] G1 判据层不可见升格硬约束——挂起待主人裁（过渡：纪律维持）
- [ ] G2 240 阈值固定 vs 级别自适应——挂起待主人裁（过渡：固定 240）
- G1/G2 已终裁（2026-07-21）：窗口绑定全域禁用 + T1 由『只能当门』supersede，见 g1-g2-window-ban-ruling-20260721.md；本文 T1/条款2/条款3 相应失效，余款与 G1 转化后约束（窗口产物全禁 grep 门）并行有效
- [x] E1（66 口径复跑）/ E2（p101 :212 语义）/ E3（主键列）登记
- [ ] 编排者复议（空位，醒后填）
