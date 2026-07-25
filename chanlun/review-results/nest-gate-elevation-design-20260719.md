# 进场准入：nest 从贴标挂件升格为真门——现状证据链与升格路径设计

- **设计工位**：nest-gate-elevation（分支 `kimi-nest-mainline-20260717`，worktree `/tmp/kimi-nest-mainline`）
- **日期**：2026-07-19（行号锚核验于 worktree 当前快照）
- **纪律**：090 严格否定（禁简化实装/禁补丁/声明=能力）；v3 硬禁令（不引入概率推断、不回测策略、不假设 EMH）；本文只写 `.md`，未改 `rust/src`、未改 `rust/Cargo.toml`、未做 git mutation、主仓只读。
- **编排者口径②**：开仓侧 nest 是挂件不是门——518 笔 `nest_confirmed` 100% True 零拒绝率；R5-1 铁律不进 μ 桶键；`nest_confirm` 只产单级末端 Conf 基例非完整 N^δ 跨级递归链。本任务：证据链核实 + 三条升格路径对照 + 推荐路径实装卡（只设计不实装）。
- **行号漂移声明**：既往文档（l3-econ-gate-filter-rate/m8-fix-plan/e2e-fix-roadmap，均 20260719）引 `runner.rs:1309/1613/2315-2316`；本 worktree 当前快照同一锚点已漂移至 `runner.rs:1516/1847-1851/2611-2614`。本文一律用**当前快照行号**，首次出现处括注旧锚。另：任务书所给 `backtest/interp.rs:378-391` 实际位于 `strategy/interp.rs:378-391`（backtest 下无 interp.rs）。

---

## 1. 现状：三层断裂证据链

### 1.1 第一层断裂：零拒绝率——「确认」无鉴别力

- **数据锚**：m8-opsem-trades-breakdown-20260719.md:17（`nest_confirmed=True` 占比 **518/518 = 100%**）；同文档 §9 D7（:157）：「证书签发门在 entry 端从未拒绝任何候选……若设计上 nest 确认应是过滤门，则该门实际为常开」；§10 问题 4（:167）把「100% True 是否符合设计预期」列为待裁定。
- **对照证据**：`nest_depth` 分布 `{0:207, 1:146, 2:101, 3:44, 4:20}`（m8-fix-plan-20260719.md:162）——嵌套结构在塔里真实存在（60% 候选 depth≥1），但「确认」动作不消费它：depth=0 的 207 笔照样 100% True。深度有信息、确认无信息。

### 1.2 第二层断裂：贴标不是门——生产 → dump，零消费方

`nest_confirmed` 全链路（对 `rust/src/theta_v0` 全量 grep 核实，共 13 处出现）：

| 环节 | 锚点 | 性质 |
|---|---|---|
| **生产** | `strategy/interp.rs:1030`（`coverage_elements_and_gamma_with_tower` 系遍历2，`Candidate.nest_confirmed = nest_confirm(...)`） | 唯一生产点 |
| **字段声明** | `strategy/interp.rs:85`（`pub nest_confirmed: bool`，doc 于 :67） | 类型 |
| **dump 消费** | `backtest/runner.rs:1847`（`OpsemEntrySnapshot.cand_nest_confirmed`）→ `:2789/:2795`（JSONL 序列化） | **只写 JSONL** |
| **测试/fixture 默认值** | `strategy/mutex.rs:473`、`strategy/coverage.rs:3994`（测试构造器硬填 `true`）；`backtest/selector.rs:574/:635`（硬填 `false`） | 非生产 |

**没有任何决策路径读取 `nest_confirmed`**：

- χ 开仓门 `selector::chi_t`（`backtest/selector.rs:116-128`）的合取项只有 `μ>θ ∧ RiskOK ∧ ConflictOK`——无 nest 项；`filter_gamma_with_admission`（:357-）同样不读。
- π 路径 `ext_i` 装配（`runner.rs:1529-1534`，旧锚 :1322-1327）只填 `risk_mode/t_stage/eta_bucket` 三维，`cand_channel/nest_depth/origin_level` 恒 None——`runner.rs:1516-1519`（旧锚 :1309-1310）G3 注释明文：「π 路径候选不经 Nest/Xzd 准入门（cand_channel/nest_depth None 诚实口径）」。
- R5-1 铁律现场：`runner.rs:2611-2614`（旧锚 :2315-2316，字段注释「**不进 entry_z/MuClass/μ 桶键**（R5-1 铁律）」）、`:2686-2689`（模块注释：避免 MuClass derive Hash 的 nest_depth 字段破坏 μ 分桶 bit-exact）、`:2793-2795`（序列化改读 opsem_snap 不读 entry_z）。

结论：`nest_confirmed` 是**贴标挂件**——生产后立即进入 dump 旁路，决策链零消费。090 口径下这是诚实标注的「照实否定」（e2e-prototype-vs-pi-deviation-deep-study-20260719.md:219：非 N7 违反），但机制上确认门缺位。

### 1.3 第三层断裂：单级基例——nest_confirm 不是 N^δ 跨级递归链

实装全文（`strategy/interp.rs:378-391`）：

```rust
fn nest_confirm(level: u32, source_index: usize, bits: &BspBits, dir: VoiceSide) -> bool {
    let confirm_ok = match dir {
        VoiceSide::Long => bits.conf_plus(),
        VoiceSide::Short => bits.conf_minus(),
        VoiceSide::Flat => return false,
    };
    let chain = [NestLevel {
        lvl: level,
        cands: vec![Interval::new(source_index as u64, source_index as u64, 0)],
        candidate_ok: true,          // ← 硬填 true
        confirm_ok,
    }];
    nest::chi_bool(level, &chain)
}
```

- 链长恒 1 ⟹ `chi_bool`（`strategy/nest.rs:122-145`）必走 `[last]` 分支（:125：`last.lvl == ev && last.chosen().is_some() && last.confirm_ok`），递归步（:126-143，`Candidate∧子⊆父∧递归`）**永不可达**。
- 区间是零宽点 `(source_index, source_index)`（:386），`candidate_ok` 硬编码 `true`（:387）——spec §6 递归步的两个判别量（Cand^δ、⊆ 收缩）都被旁路。
- 净效果：`nest_confirmed ≡ (dir ≠ Flat) ∧ confirm_side(bits)`——单 bit 方向确认的重打包。518/518 True 的机制解释：开仓候选本身经 `candidate_dir`/`min_class` 过滤后 dir 非 Flat 且 conf bit 已置位（一类点 conf_plus/conf_minus 是 bit 定义的一部分），恒等式恒真。
- 诚实边界声明在 :369-377：「完整 N^δ 跨级递归链（ℓ>e：Candidate∧⊆∧子证书）需 LeveledMove 真嵌套塔……故此处只产证书基例（单级末端 Conf）」。声明与能力一致（090 合格），但能力=基例。

### 1.4 对照面：typed 证书能力已存在，只是没接到 π 开仓门

| 资产 | 锚点 | 能力 |
|---|---|---|
| `NestCertificate`/`NestRung` | `classifier/nest.rs:234-247`/`:155-165` | 方向化 N^δ 证书：rungs 梯级链 + 基例 terminal bits，私有 builder 收口防伪造（:212-232 compile_fail 双例） |
| `n_delta()` | `classifier/nest.rs:291-319` | 复验：基例 `Conf^δ_e` ∧ 逐级 `cand` ∧ 相邻 ⊆；诚实边界 cert F-01（装配前置由生产层保证） |
| 严格装配（三门 DFS） | `classifier/nest.rs:862-869`（`assemble_certificate`）/`:1058-`（`_terminal`） | 从 `CandDeltaEvent` 流 + cp_ownership 装配完整 N^δ_{ℓ↓0} |
| 数据载体装配（π 结构路径） | `backtest/econ_positive.rs:890-971`（`build_nest_certificate`） | 从塔构造 rungs：`partition_point` 含段查找（:934-942，partial chain 合法=break 非拒），base gate Type2/3 定律一锚定（:913-917），per-rung `cand_delta`（:952） |
| econ 二通道准入门 | `backtest/econ_positive.rs:364-374` | `build_gate_certificate`（:1572）→ `GateCertificate::Nest(cert).n_delta()` / `Xzd(ev).gate_pass()`，**false 即 continue（真拒绝）** |
| strict sidecar（π runner 内！） | `backtest/runner.rs:112-114`/`:141-189`/`:584-590` | env `THETA_STRICT_NEST_SIDECAR` 开启时逐帧装配完整证书——但「不参与订单、候选、风控、账本」（:113），且「证书流预期极稀；P2 当前 BTC 全量为 0」（:137） |

**断裂的本质**：econ 统计层有真门（会拒绝），π 生产层没有门（贴标）；两侧证书算法同源（`structural_nest_depth`，`econ_positive.rs:989-1009`，与生产门 rungs 构造同款 `partition_point`），但 π 路径只消费深度读数、不消费门判定。

---

## 2. 三条升格路径对照

### 路径 (a)：`nest_confirmed`/nest 三维进 μ 桶键

- **机制**：π 路径 `ext_i`（`runner.rs:1529-1534`）填真 `cand_channel/nest_depth/origin_level`（ZExt → `z_of_candidate` → `MuClass` 第 10-12 维，`mu_estimator.rs:126-140`，字段已存在且 `derive Hash+Ord` 进桶键）。
- **机制完整性**：✗ **不达成「门」**。进桶键 = 归因轴（μ̂ 按深度分层），不是准入门——候选照旧全开仓，只是事后分桶更细。与任务目标（nest 升格为门）错位。
- **改动面**：小（ext_i 一处 + 测试）。
- **风险**：**致命**。(i) 违反 R5-1 铁律（`runner.rs:2611-2614`）——μ 分桶 bit-exact 立即破坏，需编排者显式裁定推翻；(ii) 冻结的 Pass-1（χ≡1）μ 表按旧 z 键估出，新 z 维加上后**所有新类为空类**——χ 门对全类返回 None，`treat_empty_as_pass` 二义：true ⟹ 门失效（全覆盖退化），false ⟹ 全拒（零开仓）。两条都是机制级事故，不是调参能糊的；(iii) 需重跑 Pass-1 重建冻结表，牵涉 exit-μ-BUCKETING-FROZEN #180 边界裁定。
- **结论**：**不推荐**。既不达成门语义，又要求推翻 R5-1 并重建 μ 证据基。裁定材料备于 §4（若未来证据确需此路）。

### 路径 (b)：开仓准入门改消费 typed nest 证书（推荐）

- **机制**：π fill loop 内、χ 过滤之后、coverage step 消费之前，对 `step_gamma_trade` 加**证书门**：复用 `econ_positive::build_gate_certificate`（:1572，Nest→`n_delta` / Xzd→`gate_pass` 二通道，与 econ 门**同一函数同一判据**——不产生第二裁决源，090/no-patch 合规），证书 None 或 n_delta=false ⟹ 从当 bar 候选集剔除 ⟹ interpret 不归 open ⟹ 不开仓。拒绝 = 不开仓，不经 μ 桶键，**不违 R5-1**。
- **机制完整性**：◐ 真门（会拒绝），但有效域须照实标注——econ 侧实测 **95.36% 过门信号 rungs 为空**（`econ_positive.rs:349-352`），`n_delta` 退化为基例 `Conf^δ_e`，仅 4.64% 真跨级（max depth=1）。即本路径实装后的**期望拒绝率上限 ~4.64% + base gate（Type2/3 定律一锚定）拒绝量**——不是零拒绝率，但鉴别力主要来自 base gate 与 Xzd 通道，跨级递归分量依旧稀薄。这是「等错层级」滞后的可修复分量：先把单 bit 确认换成证书谓词（含 base gate + Xzd 硬门），跨级链稀薄属数据/塔现状，挂 (c) 长线。
- **改动面**：中。π runner 单点（`runner.rs:1560-1568` 之后）+ hist 供给 + opsem dump 新列。不改 MuClass、不改 selector、不改 interp。
- **风险**：(i) hist 依赖——`build_gate_certificate`/`build_nest_certificate` 需 MACD `hist: &[f64]`，π runner 当前仅 `center_oscillation.enabled` 时算（`runner.rs:1210-1221` pan_div_hist）；门开启时需无条件算一次 `compute_macd`（同 config.macd，O(n) 一次性，确定性——不构成 bit-exact 风险，只构成成本）；(ii) 严格装配路径（`assemble_certificate`）**不可用**作门源——runner.rs:137 实测 BTC 全量证书数=0，接它=全拒=零开仓，这是照实否定而非方案；(iii) 门默认关闭的 bit-exact 回归须由既有 R5-1 测试网守住。
- **结论**：**推荐**。唯一同时满足「真门语义 + 不违 R5-1 + 无第二裁决源 + 改动面可控」的路径。

### 路径 (c)：完整 N^δ 跨级递归链实装

- **机制**：把 `nest_confirm`（`interp.rs:378-391`）从单级基例升格为真跨级链——消费 `tower` 的 `LeveledMove` 真嵌套，逐级填 `Cand^δ_ℓ`（`classifier/cand_predicate.rs`，裁决①定义式上游）与 ⊆ 边，`chi_bool` 递归步（`strategy/nest.rs:126-143`）首次可达。
- **机制完整性**：✓ 补全 spec §6 分段函数 ℓ>e 分支，是机制层面的最终形态。
- **改动面**：**大（横切⑪长线）**。(i) `interp.rs:371-377` 诚实边界注释载明：扁平 `Classification` 不导出塔（MEMORY coverage-engine-needs-tower-export-bridge）——`assemble_gamma` 扁平路径无塔可消费；`_with_tower` 系虽持塔（`runner.rs:1544-1549`），但 nest_confirm 的签名与调用点（`interp.rs:1030`）在 candidate 组装热路径上，接口改动穿透 `assemble_gamma`/`coverage_elements_and_gamma_with_tower`/`_cached_gen` 全族 + TreeCache/CandidateCache 缓存不变量；(ii) per-rung `Cand^δ` Type1 需 MACD hist——interp 层当前无 hist 供给，接口再穿一层；(iii) 即便实装，按 econ 实测 95.36% 基例退化，**机制补全 ≠ 鉴别力补全**——鉴别力稀薄是塔/数据现状（classification level hole，acc_classification_level_hole_dx 在账），不是链实装能修的。
- **风险**：热路径接口族改动 + 缓存不变量 + hist 依赖穿透，三重叠加；且改 `nest_confirmed` 语义会改 dump 列值（历史 JSONL 不可比）。
- **结论**：**长线挂账（横切⑪）**，不作为本次升格载体。它修的是「链不完整」的机制债；本次要修的是「门缺位」，(b) 已可达。

### 对照总表

| 维度 | (a) 进 μ 桶键 | (b) 证书准入门 ★ | (c) 完整跨级链 |
|---|---|---|---|
| 达成「门」语义 | ✗（是归因轴不是门） | ✓（拒绝=不开仓） | ✗（只改标签语义） |
| R5-1 | **违反，需裁定推翻** | **不触碰** | 不触碰 |
| bit-exact（μ 分桶） | 破坏 | 不变 | 不变 |
| 第二裁决源风险 | 无 | 无（复用 build_gate_certificate） | 有（interp 侧新链 vs econ 门判据漂移） |
| 改动面 | 小 | 中（π runner 单点 + hist） | 大（接口族+缓存+hist 穿透，横切⑪） |
| 实装后期望拒绝率 | n/a（门失效或全拒） | >0（上限 ~4.64%+base gate/Xzd 拒绝） | 同 (b) 上限（数据约束） |
| 依赖裁定 | 推翻 R5-1 + #180 边界 | **无需新裁定**（默认关=bit-exact 护栏内） | 塔导出桥（MEMORY 在账） |

---

## 3. 推荐路径 (b) 实装卡（只设计不实装）

### 3.1 行号锚（worktree 当前快照）

- 挂载点：`runner.rs:1560-1568`（`step_gamma_trade` χ 过滤出口）→ `:1659-1661`（`coverage::pi_theta_step_traced` 消费 `&step_gamma_trade`）。
- 门源：`econ_positive.rs:1572`（`build_gate_certificate`，`pub(super)`，runner 同模块可直接调）；`GateCertificate` 枚举 :1177；`build_nest_certificate` :890-971；`structural_nest_depth` :989-1009（dump 深度列同源先例）。
- hist 供给先例：`runner.rs:1210-1221`（`pan_div_hist`，`compute_macd(&closes, &config.macd).hist`）。
- 候选字段：`step_gamma_trade` 元素 `Candidate`（`interp.rs:71-93`）携 `level/source_index/bits/dir`——正是 `build_gate_certificate` 形参全集。
- R5-1 不动点：`runner.rs:2611-2614`/`:2686-2689`/`:2793-2795`；`mu_estimator.rs:126-140` 零改动。
- dump 扩列先例：`runner.rs:1841-1876`（`OpsemEntrySnapshot` 构造，`gamma_count`/`chi_filter_active` 同款 dump-only 字段）。

### 3.2 前/后伪码

**前（现状，`runner.rs:1556-1568` 节录）**：

```rust
// ③' χ_t 阈值过滤：Γ_t → Γ_t^trade={γ:μ(z_γ)>θ}
let step_gamma_trade = match &chi {
    Some(ctx) => selector::filter_gamma_with_admission(
        &step_gamma, ctx.est, ctx.theta, ctx.z_alpha, ctx.shrink_tau_sq,
        ctx.treat_empty_as_pass, &tower_i, bars, &ext_i),
    None => step_gamma.clone(),
};
// …直接喂 coverage::pi_theta_step_traced(…, &step_gamma_trade, …)   (:1659-1661)
```

**后（设计；新增门为独立过滤段，默认关闭）**：

```rust
let step_gamma_trade = match &chi { /* 不变 */ };

// ★nest-gate（升格路径 b，编排者口径②）：开仓候选须持 typed N^δ/Xzd 证书。
// 门源复用 econ_positive::build_gate_certificate——与 econ 准入门同函数同判据，
// 不产生第二裁决源（090/no-patch）。证书 None（base gate 拒/无塔段）或
// n_delta=false（任一级 Cand=0 或区间不收缩）⟹ 剔除 ⟹ 不开仓。
// 拒绝不经 μ 桶键：entry_z 三维仍 None（R5-1 不动），门是准入谓词不是 z 维。
let step_gamma_trade = match &nest_gate {
    Some(ctx) => step_gamma_trade.into_iter().filter(|c| {
        if c.dir == VoiceSide::Flat { return false; } // 与 nest_confirm :382 同语义
        let (sub_centers, sub_bsp) = sub_level_views(&classification_i, c.level);
        matches!(
            econ_positive::build_gate_certificate(
                &tower_i, c.level as usize, c.source_index,
                side_of(c.dir), &c.bits, &ctx.macd_hist, i, &ls_bsp_of(c.level),
                sub_centers, sub_bsp,
            ),
            Some(econ_positive::GateCertificate::Nest(cert)) if cert.n_delta()
        ) || matches!(
            econ_positive::build_gate_certificate(/* 同参 */),
            Some(econ_positive::GateCertificate::Xzd(ev)) if ev.gate_pass()
        )
        // 注：实装时应把 build_gate_certificate 调用提取为单次求值后 match，
        // 上面双 matches! 只是伪码示意两通道判据。
    }).collect(),
    None => step_gamma_trade, // 门关闭 ⟹ 逐字节不变（bit-exact 回归锁）
};
// dump-only 观测列（同 gamma_count 先例，runner.rs:1858-1860）：
//   nest_gate_rejected_count / nest_gate_channel ∈ {Nest,Xzd,None}
```

**hist 供给**（门开启时）：`runner.rs:1210-1221` 同款，无条件 `compute_macd` 一次入 `nest_gate` ctx；门关闭不算（惰性，成本零侵入）。

### 3.3 bit-exact 影响账

| 面 | 影响 | 依据 |
|---|---|---|
| μ 分桶 / MuClass / entry_z | **零** | 门不填 z 维；`ext_i`（:1529-1534）不动；R5-1 现场（:2611-2614）不动 |
| 门默认关闭（生产冻结路径） | **零（逐字节）** | `None => step_gamma_trade` 直通；订单轨/账本/dump 全不变——由既有 R5-1 回归测试（`runner.rs:4678-4734` e1/e2/e3 dump 字段测试、:4899「π 路径不经 Nest/Xzd 准入门」断言）守住 |
| 门开启 | 候选集收缩 ⟹ opened/订单/账本**按设计变化**（这是门的目的，非回归）；μ 表因果性不破（门是结构谓词，不读未来） | 与 χ 门（:1556-1559 注释「gamma 滤掉 ⟹ 不开仓」）同一语义层 |
| opsem dump | 新增 dump-only 列，历史 JSONL 列集不同——按 m8-fix-plan:149 先例（改名/扩列不影响 μ 分桶 bit-exact）处理 | R5-1 边界同款 |

### 3.4 验证协议（实装工位执行）

1. **基线**：`cargo test --release --lib` 全绿零变红（基线 1752 passed）；新增测试镜像 R5-1 先例（`runner.rs:4678-4734`）：门关闭时 dump JSON 无新列/订单轨逐字节一致；门开启时 (i) Flat 候选必拒、(ii) 证书 None（构造无塔段候选）必拒、(iii) `n_delta=false`（人为断 ⊆ 链的 rungs）必拒、(iv) Xzd 通道 `gate_pass` 判据与 econ 侧逐信号一致（对拍，非同义反复——两路径独立构造输入）。
2. **同源对拍**：同一批候选分别过 π 新门与 econ 门（`econ_positive.rs:364-374`），判定须逐候选一致——不一致=第二裁决源实锤，实装作废。
3. **观测落账**：门开启重放后，`nest_gate` 拒绝率按**可修复分量 vs 内禀分量**分别落账（编排者口径）：base gate/Xzd 拒绝=门实装生效（可修复分量已修）；rungs 空基例占比（预期 ~95%）=跨级链稀薄（内禀于塔现状，挂路径 c）。**不承诺零滞后**——区间套把滞后压到因果下限，浮盈回吐一部分是「不预测」的必然代价，本门不加任何百分比止盈/阈值项。
4. **090 声明核对**：实装后 `Candidate.nest_confirmed`（interp.rs:85）仍是单级基例贴标——不得因门接入而改其 doc 声明为「完整 N^δ」；门证书与贴标字段是两个对象，文档须分别标注（防 m8 式「字段名语义 vs 实际能力」偏离，m8-fix-plan-20260719.md:149-169 先例）。

---

## 4. 若需推翻 R5-1 的裁定材料（路径 (a) 备用，本设计不推荐）

若编排者未来裁定路径 (a)（nest 三维进 μ 桶键），须一并裁决：

1. **铁律原文**：`runner.rs:2611-2614`（「不进 entry_z/MuClass/μ 桶键（R5-1 铁律）」）、`:2686-2689`（derive Hash 破 bit-exact 论证）、`:2793-2795`（序列化旁路）。推翻 = 三处注释 + `mu_estimator.rs:126-140` 的 None 口径全改。
2. **冻结边界**：exit-μ-BUCKETING-FROZEN #180（gap2-gamma-candidate-dump-design-20260719.md:106 援引）是否覆盖 entry z 维——若覆盖，(a) 属二次冻结违反。
3. **μ 证据基重建**：Pass-1（χ≡1）须以新 z 维重跑重建冻结 μ 表（确定性重放，新表自身可 bit-exact 复现，但与旧表不可比）；过渡期内所有新类为空类，`treat_empty_as_pass` 语义须裁定（selector.rs:108-111：二语义都合法，调用方按阶段选）——**未裁定前 (a) 实装 = 门失效或全拒**。
4. **回归网改动**：`runner.rs:4678-4734`（e3: nest_depth 非 null 断言将反向）、:4899（「π 路径不经 Nest/Xzd 准入门」断言作废）等测试需同步改——测试改动清单本身须进裁定。
5. **先例**：e2e-fix-roadmap-l0-l5-20260719.md:77/:476（D4）已备「方案 B=MuClassSidecar 旁路（R5-1 不动）vs 方案 A=进桶键（需推翻 R5-1）」对照，默认方案 B；m8-fix-plan-20260719.md:41 声明 D-1/D-2/D-3 需独立架构工位 + 显式裁定。**若目的只是 μ̂ 按深度分层归因，sidecar 旁路（方案 B）可在不推翻 R5-1 下达成——路径 (a) 的唯一增量是「深度进 χ 门查询键」，该增量须经 L2 证据支持方可立项，本设计不代裁。**

---

## 5. 路径 (c) 长线挂账（横切⑪）

- 机制债：`nest_confirm` 单级基例（`interp.rs:378-391`）vs spec §6 完整递归；`chi_bool` 递归步（`strategy/nest.rs:126-143`）生产不可达。
- 前置：塔导出桥（MEMORY coverage-engine-needs-tower-export-bridge，`interp.rs:375-377` 援引）+ interp 层 hist 供给接口 + `cand_predicate`（裁决① Cand^δ 定义式）接入。
- 数据约束：econ 实测 95.36% 基例退化（`econ_positive.rs:349-352`）——链实装后鉴别力仍受塔层覆盖率约束（acc_classification_level_hole_dx 在账）。**机制补全先行立项时须如实声明：它修「链不完整」，不修「鉴别力稀薄」。**

## 6. 与出场口径的对称性（编排者最高口径）

进出场同一条线：出场 = 反向买卖点经区间套背驰确认，止盈 = 反向买卖点，不是百分比阈值。本设计的门形状（typed 证书谓词、无阈值参数）对出场侧同构适用——反向候选的 `build_gate_certificate` + `n_delta` 即出场确认；closePred 不加止盈项的纪律在进场门设计中同样保持（门判据全部来自结构：bits/塔段/⊆/Cand，无任何 f64 阈值）。出场侧实装是独立工位，本设计只声明对称性，不代实装。

## 7. 遗留与边界

- 本文未实装任何代码；伪码仅供实装工位落笔参照，行号以实装时快照为准（本文已记录一次漂移先例）。
- `nest_gate` 的开关形态（env vs config 字段）留实装工位按 `THETA_STRICT_NEST_SIDECAR`（`runner.rs:242-246`）/ κ env（`runner.rs:1281-1287`）先例裁定——env 纯诊断惯例优先。
- Xzd 通道在 π 路径接入后，`NestTrigger::PanDivConsolidation` 通道（`econ_positive.rs:429-483`）是否同批接入，需与 #145 口径核对，本设计不展开。
- 拒绝率预期（~4.64% 上限）来自 econ 侧 BTC 300K 实测的转述（`econ_positive.rs:349-352` 注释），非本工位新测；落账时以门开启重放的实际拒绝率为准。
