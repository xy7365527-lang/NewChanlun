# 全仓端到端模块化总勘察（第三轮：面③④ + 收拢）

- 日期：2026-07-29；执行：Fable 5（架构勘察，纯调研零改动）
- 前两轮在案事实直接采信（面①②：孤儿 `pipeline.rs` 第二份 `classify_impl`；身份键 6 族 14 型、`LifecycleKey` 一键三判同、`band_eq_start_neq_pts` 实测不一致；N1/N2/N3 已内联主路径但出口零生产消费者【ADR-0007 明文阶段态 + NestEventIdentity 纯序号身份与 #206 相顶的双面表述】；收敛落点初判 = 点身份值桥两元锚 + 段/事件结构身份合流、ledger_kernel 簿记宿主）。
- 收拢基准：ADR-0005 nest 三件 GUARD-ROLE 豁免禁回灌、键不可并入；金标准锚常例 = 动它必碰锚 ⟹ 每票带 counterfactual 反证（`THETA_*_SKIP` 式旧行为恢复、逐字节 cmp），不是不能动。

---

## 一、面③：消费链逐段接口与泄漏

### 1.1 端到端主链（π 现役，ADR-0004：`pi_theta_fill_loop_overlay` 家族根）

```
data.rs（Dataset；契约「source_index == 数组下标」data.rs:140,161）
→ parser（ParseLayer：L0 线段账本）
→ classifier::classify_impl（classifier/mod.rs:489；出口 Classification{ levels: Vec<LevelState> }，
   索引=级别 ℓ，levels[ℓ].bsp: Vec<BspPoint>——「双序号坐标系」：级别下标 × L0 K 序）
→ strategy::interp::assemble_gamma（Candidate{level, source_index, bits, dir, bsp_class, role,
   nest_confirmed, gamma_index, force}，interp.rs:72）
→ strategy::recognize（mod.rs:506：按 source_index 分组时刻 x → interp::interpret 三桶 𝒟/ℬ/𝒦
   → build_decision → VoiceDecision）；持仓侧 𝒟 由 runner per-moment 传 HeldVoice 驱动
→ backtest::admission（NestChainGate：χ/nest/k_Θ 三门 + κ 解析；#439 溯源=净新增 900+ 行）
→ backtest::fill::pi_theta_fill_loop(_overlay)（fill.rs:718/742；apply_voice_fill + account_mirror_*
   + drive_campaign_wiring（fill.rs:1250，短差 campaign 接线）+ 8 组静态探针计数器）
→ backtest::ledger（TypedTrade / LedgerOpen / TwLedgerThread / OpsemEntrySnapshot）
→ metrics / opsem_dump（观测面）
```

出场侧：`strategy/exit.rs`（HeldVoice 台账 + `exit_decision_for` §9 closePred）自述逻辑单源、双消费路径（回测 runner 从模拟 units、生产 ThetaCore 从 Nautilus portfolio 读持仓真相）——差异仅「真相从哪读」，判定零分叉（exit.rs:1-17）。风控：`strategy/risk.rs`（结构止损 + sizing 三路 min；自标「Θ_risk 设计选择非缠论可导」，与 #659 名分订正一致）。

### 1.2 「source_index 等值临时拼」接合点全枚举

**生产侧（theta_v0 非 bin）8 处**：

| # | 位置 | 形态 | 定性 |
|---|---|---|---|
| 1 | `classifier/nest.rs:681` | `point.source_index == turn_source && bits.confirm_side(side)` | **生产绑定规则原型**（BSP↔证书事件绑定） |
| 2 | `backtest/admission.rs:883` | `.find(\|e\| e.source_index == c.source_index)` | nest gate 物化锚（a* 解析；`:394,:862` 注释「禁第二查法」；测试侧语义 pin 在 runner.rs:5019） |
| 3 | `backtest/signal.rs:114` | `lvl.bsp.iter().find(\|p\| p.source_index == c.source_index)` | 入场结构止损回查（(level, source_index) 查 BspPoint，:95 自述「bsp 确认点坐标，稳定不漂移」） |
| 4 | `backtest/fill.rs:3134` | 同上形态 | #647 逆侧归因 dump 回查（观测面） |
| 5 | `backtest/econ_positive.rs:1344` | `q.source_index == source_index` | L2 旧臂证书查询 |
| 6 | `strategy/mod.rs:543` / `:759` | `gamma.iter().filter(\|c\| c.source_index == x)` | **时刻分组键**（spec §12 语义，合法契约非泄漏，但与身份 join 同形无类型区分） |
| 7 | `strategy/interp.rs:254` | `projection.child_source_index == child.source_index` | 投影层联动 |
| 8 | `parser/fractal.rs:80,140` | 组内序号契约 | parser 内部（合并组语义，自洽） |

**bin 侧 12+ 处**（复制生产绑定规则；`p107_level_funnel_audit.rs:232` 注释明示「绑定规则（生产）：BSP.source_index == event.turn_source 且 confirm_side」）：p102:356、p107_level_funnel_audit:550/566、p108:836、p109:171/557、p111:156、p112:544/656、p113:736、p117:1053/1282、pi_bsp_timing:102。

**泄漏定性**：绑定规则「(level, source_index[, confirm_side]) → BspPoint/证书」**无单源函数**——生产 5 处手写 + 诊断 bin 12+ 处逐字复制。改绑定口径（如未来按 #206 值桥锚重写）需同步 17+ 处；诊断读数与生产判定可静默分叉（p107 类 bin 的历史读数已因 #607 大闸不可比，正是这种分叉的实例）。K 序号在包含合并下漂移不作身份依据是 CONTEXT.md 不变量条明文；现状是「确认点坐标稳定」的口头契约撑着，无类型防护。

**第二个同族泄漏——gamma_index 平行数组对齐**：`recognize` 用 `points[cand.gamma_index]` 索引回平行 BspPoint 列表（mod.rs:512-519,549），依赖 `assemble_gamma` 与 `flat_map` 遍历序 1:1 的隐式契约；`Classification.levels` 索引=级别同型。序号当接口、无 newtype 防护，是全链默认背景。

### 1.3 配置面泄漏：45 个 env 键直读

`theta_v0` 内 `env::var` 直读 **45 个不同键**。行为门与观测门混散、绕过 `ThetaConfig` 单源：

- **行为改变类**：`THETA_NEST_CERT_GATE`、`VOICE_EXEC`、`THETA_T3INC_SKIP`、`THETA_ENTRY_STOP_RECHECK_SKIP`、`THETA_CENTER_OSCILLATION`、`KAPPA_BARRIER_NUM/DEN`、`ENFORCE_GROSS_CAP`、`THETA_DIR_PRESET`、`ECON_L2_*`（10 处重复直读）。
- **观测/dump 类**：`OPSEM_DUMP_DIR`(4处)、`ENTRY_STOP_REVERSE_DUMP`、`DELTAFREE_DUMP`、`THETA_STRICT_NEST_SIDECAR`、`THETA_OTHERWISE_DOMAIN_SIDECAR`、`T5A_CHAIN_DUMP_*`、探针/剖析类 20+。

要害：counterfactual 反证常例（金标准锚流程的核心工具）就实装在这些 SKIP 门上，但门**无注册表可枚举**——审计「当前有哪些行为开关、默认臂是什么」只能全仓 grep。`OPSEM_DUMP_DIR` 并行测试竞态已咬过一次（admission.rs:24 注释在案），thread_local 注入惯例只覆盖了 VOICE_EXEC/NEST_CERT_GATE 两个。另有层次纪律散在注释里（「classifier 不可读 backtest 门 env」admission.rs:72-73），无机制看守。

### 1.4 行数 top10 与 god module 初判

原始行数 top10（`find rust/src -name "*.rs" | xargs wc -l | sort -rn`）：

| 文件 | 总行 | 生产行(≈至 mod tests) | 注 |
|---|---|---|---|
| theta_v0/backtest/runner.rs | 7493 | **771** | 90% 是测试；**非 god module，是测试宿主** |
| theta_v0/classifier/mod.rs | 6192 | **3049** | **god module 首选**（见下） |
| theta_v0/classifier/nest_lifecycle.rs | 5889 | 2506 | V3 活假设状态机（#231 重建） |
| theta_v0/backtest/econ_positive.rs | 5507 | 1672 | L2 旧臂 + build_nest_certificate |
| theta_v0/backtest/fill.rs | 5109 | ≈1333 | 三职责同居（见 C3） |
| theta_v0/classifier/signal.rs | 4711 | 2032 | 结构门 + judge_first_cached |
| trading/level_operating_unit.rs | 4586 | 2385 | 前代族 |
| bin/p123_fast_replay.rs | 4435 | — | 诊断 bin |
| theta_v0/classifier/recursive_tower.rs | 4039 | 2384 | 塔构件 |
| trading/positional_fusion.rs | 3985 | 1580 | 前代族 |

生产行数视角另有两个大件：`recursive_t/rec_engine.rs` 3277（前代 T 引擎，非现役线）、`backtest/wverify_run.rs` 3008（W-VERIFY `#[ignore]` 实验跑批工位，非生产路径；其旧 PASS 口径已作废在案）。

**god module 初判 = `classifier/mod.rs`**：classify 五入口变体（classify / classify_with_tower / +events / +incremental / +events_incremental，:698-2805）+ `TowerCache`(:1127) + cand_delta 三驱动器(:766-836) + `cp_recall_upper_bound_audit`(:930) + 三个内嵌诊断子模块（cp_replay_diagnostics:138、stage_profile:1536、oracle_probe:1668）同居一文件；且 #529 N 系列还在向 `classify_impl` 内联扩建（cand_event 产出已内联），只会继续长。**runner.rs 出榜**：其 7493 行系测试挂载所致，生产本体 771 行已被历轮 seam（admission/fill/ledger/opsem_dump 拆出）瘦到合理——但**头注释整块过期**：仍描述「阻塞点 A/B」「recognize 未实装」并引用已不存在的 `RECOGNIZE_NOT_IMPLEMENTED` 常量（runner.rs:1-46），而 `recognize` 实装已在 strategy/mod.rs:506；同一注释自述 simulate_fills 与 strategy::exec 双 fill 语义「引擎稳定后对齐」——至今未对齐，双语义并存。

### 1.5 浅模块（deletion test 初判，6 个）

| 模块 | 行数 | deletion test 结果 | 定性 |
|---|---|---|---|
| `classifier/pipeline.rs` | 416 | **无 mod 声明，编译不进**；删除对构建零影响 | **真孤儿**（含第二份 classify_impl，面①在案；判据双源风险的唯一实体） |
| `classifier/voice_eat.rs` | 382 | 全仓零调用（仅 mod 声明） | Lean C06 Eat 谓词镜像 + 内部自测 |
| `theta_v0/complete/` | 655 | 零外部消费者 | 17 分量 CompleteState/8 元组事件 schema 的 Lean 镜像 |
| `theta_v0/ledger/separate.rs` | 490 | 零外部消费者 | P^sep 分账本 Lean 镜像（生产分腿记账实体在 strategy/ledger+account） |
| `classifier/six_state.rs` | 264 | 零调用；econ_positive.rs:841 注释称「上游 six_state.rs 置 bit」**与事实不符**（注释级漂移，实际置位在 signal/bsp 路径） | canonical 对照层（自述「不重复实装」） |
| `classifier/ref_v1.rs` | 311 | 仅 `tests/theta_v0_center_parity.rs` 消费 | 旧版中枢区间参照件（parity 专用） |

定性纪律：除 pipeline.rs 外，其余五件均有 Origin/Lean 契约锚名分，是**对照/镜像件而非垃圾**——deletion test 说的是「模块深度≈0（接口≈实现、零生产深度）」，处置须走 ADR-0004 名分程序，本报告不预拍删除。

---

## 二、面④：机件盘点

### 2.1 区间套/嵌套机件谱系（现役 / 阶段态 / 对照 / 死代码）

**A. 现役生产判据（当下放行的真源）**：
- `strategy/nest.rs`——`n_delta` 递归核（候选过滤门唯一判据源，CONTEXT.md「候选过滤门」条）；
- `backtest/admission.rs` `NestChainGate`——π 开仓准入三门（χ/nest/k_Θ），`THETA_NEST_CERT_GATE` 门控，严格链判定唯一 nest 判定源（T3 #172），typed 无证回退旧臂 Xzd 通道（econ_positive.rs:1606 单一来源）；
- 证书供应链：`econ_positive::build_nest_certificate`(:898, pub(super)) → `classifier/nest.rs::assemble_typed_certificate` → `nest_index.rs:262` 索引 → admission 消费（ADR-0005 核验 §1 方向锚定）。

**B. nest 管线三件（GUARD-ROLE 豁免，ADR-0005）**：`classifier/nest.rs`(3151) / `nest_index.rs` / `turn_class.rs`——独立对照实现身份，产物禁回灌判据 crate；三套 Cand^δ 实装互不等值是其独立性的反证成立面。`interval_necessity.rs` 声明 GUARD-ROLE: judge 被拦，只读观测器定性（#416 已解）。

**C. N 系列新族（塔内原生化，map #529；产出已内联、消费未接 = ADR-0007 明文阶段态）**：
- `cand_event.rs`（N1 #550 载体：自述「只产出、存储候选生命史，不接 BSP/admission/订单消费点」）；
- `cand_sub.rs`（N2 #552：C⊆C 闭区间含端点、相切算包含，source_index 坐标域）;
- `chain_cert/`（N3 #641 级别链证书：自述「**纯产出零消费**：调用点 = 单测 + 诊断 bin + p123 侧信道 dump 只写不判」）；
- `cand_predicate.rs`（W1 DivCand^δ 谓词）；`cand_delta.rs`（P1/P52/P53 只读驱动器——**旧力度确认语义**，ADR-0006 已排出原生定义，留 ℓ=0 对拍面）。

**双本体并存关系一句话**：旧 nest 三件是当下放行判据的真源（admission 经 typed 证书消费），N 系列是塔内原生化的接班本体——每 bar 已付产出成本、出口 API 生产调用者为零，接线归 N7 另裁；这是设计中的阶段态，同时「NestEventIdentity 纯序号身份与 #206 禁序号判同相顶」的事实并立在案（前两轮双面表述沿用，不改写）。

**D. 账本/活假设机件（第三条线，勿与区间套证书混层——ADR-0003 两层裁定）**：
- `nest_lifecycle.rs`（V3 活假设状态机 NestLifecycleBook，#231 重建；生产 2506 行）；
- `ledger_kernel/`（T1 #573：自 nest_lifecycle 抽出的泛型账本内核，双消费方定形）；
- `retrace_ledger/`（S1 #621：买卖点身份账本，ADR-0008 三态三钟日志真相；adapter/audit/book/log/portal 五件）。

**E. 死代码**：`pipeline.rs` 第二份 classify_impl（唯一编译不进的实体）。相邻退役面：v1/dual 出场形态已 #499 退役（decision-injection gate 词条 deprecated）；`descend.rs` 与 bottom-up 区间套 bit-exact 等价在案（parity 测试固化）。

### 2.2 spiral/ vs theta_v0/ 与外围族

`spiral/` 是 D∞ 群第一性原理的 v2 螺旋引擎（自述保留 `trading::unified_necessity` 为 bit-exact 对照基线），与 `theta_v0/`（π 世代）是**前后世代关系而非分工关系**——ADR-0004 裁定 π 唯一现役后，`spiral/`、`fugue_v3/`（D∞ word 处理器）、`recursive_t/`（standalone 统一算子 T——注意与现役 memory「T 递归自相似架构 v2」概念同名不同实体）、`trading/`（有机赋格 v2 交易层）、顶层散件（`lib.rs`/bi_engine/stroke/segment/zhongshu/buysellpoint 等 2467+ 行）全属前代族，合计约 3.5 万+ 生产行，名分逐件核定归 ADR-0004 程序（多数将落 deprecated/对照件，非本轮裁）。

### 2.3 nautilus 与 Python 一句话

`theta_v0/nautilus/` = canonical S_Θ 接 NautilusTrader 的**真生产适配层**（#524 订正已过骨架期：nautilus 0.60.0 依赖入 Cargo、`theta_strategy.rs`/`backtest_engine.rs` 真实 use，feature 门控），对应 CONTEXT.md「生产拆三义」的第三义；`src/newchan/`（Python a_*.py 五十余件）= Rust 全量重写前的研究原型遗产层，生产零角色（引擎 Rust 化在案）。`bin/theta_backtest.rs` = 第二义「最小生产路径 CLI」（run_theta_v0_pi + 内部 fill）。

---

## 三、收拢：目标模块图

### 3.1 端到端深模块划分（目标态）

```
┌─ 数据层 data（契约：source_index==数组下标）
├─ 结构引擎（塔）＝深模块①：parser + classifier
│    接口收窄目标：classify 入口族（全量/增量两口）+ Classification + 事件流出口
│    深度论证：吃 bars 吐全部结构事实（分型/段/中枢/走势/BSP/N 系列事件），
│    实现 2.4 万+ 行、接口应≤5 个类型——典型深模块；当下的浅化力量是
│    诊断驱动器与入口变体都挂在同一 mod.rs 门面（C4）
├─ 判据/证书层＝制度性双本体（ADR-0005 禁并）：
│    nest 三件（GUARD-ROLE 对照）∥ N 系列原生链（接班，N7 接线前纯产出）
├─ 策略解释器＝深模块②：interp + recognize + coverage + voice
│    接口：Classification → VoiceDecision[]；深度=spec §11-§13 全部唯一化逻辑藏在 interpret fold
├─ 准入门＝深模块③：admission（χ/nest/k_Θ + κ；「禁第二查法」已是深模块纪律）
├─ 执行/账本＝三个应分立的深模块：fill loop（撮合推进）/ account+ledger（分腿账本）
│    / oscillation campaign（短差旁路，自包含账本教义断言=开臂关臂逐位相同）
├─ 观测面：opsem_dump + witness + env 门注册表（目标新增，C2）
└─ 生产出口：nautilus 适配（持仓真相源切换点，exit.rs 单源纪律已立）
```

**递归本体**：`classify_impl`（classifier/mod.rs:489）的逐级 decompose 循环——L0 段账本 → Lk 中枢/走势/BSP → L(k+1) 输入单元（`Origin.RecursiveLevelSystem` 锚），`recursive_tower.rs` 供塔构件、`incremental/` 供增量外壳、`TowerCache` 供缓存；全部下游消费其 `Classification{levels[ℓ].bsp}` 双序号坐标系。

### 3.2 Deepening candidates（6 个）

**C1. BSP 绑定回查单源化（source_index join 收敛）** ⭐Top
- 文件：nest.rs:681 / admission.rs:883 / signal.rs:114 / fill.rs:3134 / econ_positive.rs:1344 + bin 12 处
- 问题：§1.2 全枚举——绑定规则无单源、生产/诊断 17+ 处手写复制、序号 join 无类型防护与 #206 值桥教义并存。
- 方案：classifier 出口新增绑定查询 API（`bsp_at(level, source_index)` + `bind_turn(event)` 一族），生产五处与 bin 全部改调；「时刻分组键」与「身份 join」用 newtype 在类型层分开。
- 收益：改绑定口径 17+ 处 → 1 处；诊断与生产不可再分叉；为未来换值桥锚留单点。
- 强度：中（机械收敛）。ADR 冲突：无（#206 方向背书）。**碰锚：否**（纯等值重构，counterfactual = wf8 双锚逐字节不变）。

**C2. env 门注册表（配置面收口）**
- 文件：theta_v0 全域 45 键（§1.3）。
- 方案：单模块注册表（键名/语义/行为 vs 观测/默认臂/所属层），全部经它读；thread_local 测试注入惯例统一；「classifier 不可读 backtest env」层次纪律进注册表分层。
- 收益：行为开关可枚举可审计（counterfactual 常例的基础设施）；竞态防线统一。
- 强度：低-中。ADR 冲突：无。**碰锚：否**（默认臂语义不变）。

**C3. fill.rs 三职责拆分**
- 文件：fill.rs（fill loop + account_mirror + drive_campaign_wiring + 8 组探针 + 3800 行测试同居）。
- 方案：campaign wiring 及其 3 千行测试分家至 strategy/oscillation 侧；探针组收敛为单 witness 结构。
- 收益：fill loop 本体可读；短差旁路自包含性在文件边界上成立（与「开臂=关臂」教义同构）。
- 强度：中。ADR 冲突：无。**碰锚：是**（wf8 四层 + trades.jsonl/tower_events.jsonl 双锚）——纯移动票也须带逐字节反证。

**C4. classifier/mod.rs 入口族瘦身（god module）**
- 方案：cand_delta 三驱动器 + cp_recall_upper_bound_audit + 三内嵌诊断子模块迁 `classifier/diag/`；TowerCache 合流 tower_cache.rs；mod.rs 收窄为入口门面。
- 收益：主路径与观测面分层；#529 后续 N 票合并冲突面缩小。
- 强度：中。ADR 冲突：增量/全量对拍锁（E2E-S1）必须原样随迁。**碰锚：是**（classify 主路径宿主）——反证同 C3。

**C5. runner.rs 文档订正 + 测试宿主治理**
- 问题：头注释描述已解除的阻塞点 A/B、引用已删除常量（090 违反面）；6721 行测试挂生产文件；双 fill 语义未对齐自述。
- 方案：注释重写为现状；测试迁出；双 fill 口径对齐单独立案。
- 强度：低。ADR 冲突：无。**碰锚：否**（纯注释/测试位置）。

**C6. 浅模块名分核定批**
- 文件：§1.5 六件。pipeline.rs 按孤儿待删走 ADR-0004；五个 Lean 镜像件判「对照件」名分并统一自述标记（GUARD-ROLE 式）或集中 `formal_mirror/` 目录；econ_positive.rs:841 注释漂移随手订正。
- 收益：第二份 classify_impl 消灭 = 判据单源闭合。
- 强度：低。ADR 冲突：无（正是 ADR-0004 执行）。**碰锚：否**。

### 3.3 实施序列（按依赖排序）

1. **C5**（零行为，纯文档/测试搬家）→ 2. **C6**（pipeline.rs 孤儿程序 + 名分表）→ 3. **C2**（env 注册表——先立，为后续每张 counterfactual 票提供门清单）→ 4. **C1**（绑定单源化，依赖 C2 的门清单核对 SKIP 臂）→ 5. **C4**（classifier 瘦身，依赖 C1 先把出口收敛）→ 6. **C3**（fill 拆分——碰锚最重，最后做，独立 worktree + 双锚 cmp）。

碰金标准锚标记：C3、C4 必碰（票内带 counterfactual 反证）；C1 理论不碰但涉 admission「禁第二查法」注释语义，验收加双锚 cmp 一道；C2/C5/C6 不碰。

### 3.4 Top recommendation

**C1（BSP 绑定回查单源化）**：同时命中泄漏 top1（17+ 处复制的绑定规则）、#206 身份教义方向（序号 join 的唯一收敛出口）、诊断/生产分叉的现实风险（p107 族读数已发生过跨版本不可比），且验证成本最低（逐字节不变即 PASS）——是深化收益/风险比最高的一张票。前置只有 C2 的门清单（半依赖，可并行）。

---

## 附：本轮证据索引

- 行数榜：`find rust/src -name "*.rs" | xargs wc -l | sort -rn | head`（2026-07-29 实测，总 245,313 行）
- 测试边界：runner.rs:772 `mod tests`；fill.rs:1333/1926/4949 三测试块；classifier/mod.rs:3049
- env 键清单：`grep -rho 'env::var("[A-Z_0-9]*")' rust/src/theta_v0 | sort -u`＝45 键
- 零消费验证：ref_v1/six_state/voice_eat/complete/separate 五件符号级 grep（含 tests/ benches/）
- 名分单源：docs/agents/generation-constitution.md §2（π 唯一现役、main 唯一线）
- ADR 底座：0001-0008 全读；CONTEXT.md 词汇表对照
