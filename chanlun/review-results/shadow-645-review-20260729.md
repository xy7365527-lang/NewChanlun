# 影子评审 #645：#614 合并中两处跨线手工重放（TowerCache 三访问器 / γ dump 生产钩子）

- **日期**：2026-07-29
- **评审器**：独立影子工位（新上下文，非交付方；禁子代理、前台单线程）
- **工位**：`/private/tmp/wt-645`（分支 `shadow/645-review`，尖端 = main `23a1869849`）
- **评审对象**
  1. ⚠HIGH-4 轴 — 收口提交 `2080d2bcad`「补齐 kimi 侧 TowerCache 三访问器 + freeze_boundary 只写缓存」
  2. ⚠MED-5 轴 — merge 提交 `79a2715070` 内手工移植进 main `backtest/fill.rs` 的三处 γ dump 生产钩子
- **参照面**：`/private/tmp/kimi-nest-mainline`（封存只读，仅 `git show` / `grep`，未在其上跑任何构建）
- **评审基准**：`chanlun/review-results/issue614-merge-log-20260729.md` §3 ⚠HIGH-4 / ⚠MED-5

---

## 判词（两轴）

| 轴 | 判词 | 一句话理由 |
|---|---|---|
| ⚠HIGH-4（TowerCache 三访问器 + freeze_boundary） | **PASS WITH CONDITIONS** | 登记位点/公式同源/「只写缓存零生产读点」三条论证经复核**全部成立**；但同批新增的 `level_scan_units(1)` 把一条 **main 侧并不成立**的不变量（「与 `tower[0]` 同序同长」）升格为公开契约，段账本回缩 bar 上实测破裂（`0` vs `4`），且 kimi 侧对应的断言与回归测试未随入活代码。 |
| ⚠MED-5（γ dump 三处生产钩子） | **PASS** | 三处位点在 main `fill.rs` 上唯一确定、与 kimi 侧**逐字相同**、相对次序（χ 真值 → nest gate；step_trace → PanDiv 最终选址）同形；env 门控「零写入」实测坐实，模块本体与 kimi 逐字无差。 |

**发现分级计数：HIGH 1 / MED 2 / LOW 3。**

---

## 轴一：⚠HIGH-4 复核

### A. 登记位点与实参同源（复核结论：成立）

| 复核项 | 结论 | 证据锚 |
|---|---|---|
| 登记点相对次序与 kimi 同位 | **成立**。main `classifier/mod.rs:2495` 紧随 `decompose_resume`（:2486）、在 `extract_first_third_resume`（:2535/:2551）之前；kimi tip `797c9ad35c` 的 `classifier/incremental.rs:597` 逐字同形（报告引用的行号锚**精确成立**，同一行即登记行）。参照面 HEAD（`632ae9eb58`）该代码已迁至 `incremental/tower.rs:291`（`assemble_level_state` 内），相对次序不变。 | `rust/src/theta_v0/classifier/mod.rs:2486-2495`；kimi `797c9ad35c:…/incremental.rs:590-597` |
| 覆盖级别集一致（每级无条件，非挂 memo-miss 分支） | **成立**。main 在 `for level_idx in 0..=l_max` 主体内无条件执行；kimi 由 `process_level → assemble_level_state` 每级各调一次。两侧注释同款钉住「不得挂在 BSP memo miss 分支」。 | `mod.rs:2047`（循环头）、`mod.rs:2495`；kimi `incremental/tower.rs:169-291` |
| 实参 `(prefix_count, dirty_e)` 与 `extract_first_third_resume` 单一同源 | **成立**。`prefix_count = lc.upper_moves.len()`（main `mod.rs:2329`；kimi `tower.rs:228` 同定义）；`dirty_e` 在函数内**唯一赋值点** `mod.rs:2111`，位于登记点之前，登记点与 extract 之间无第二次写入（`awk` 全区间扫描 2320–2560 仅见读取）。 | `mod.rs:2111 / 2329 / 2495 / 2539 / 2554` |
| 第三参数 `&lc.centers` 在两读点之间未被改写 | **成立**。2495→2554 之间 `centers` 仅出现于只读位置（`Rc::clone`、`len()`、传引用），无 `make_mut` 写入。 | `mod.rs:2495-2603` |
| `signal::freeze_boundary_src` 未改一字 | **成立**。公式单点持有（`signal.rs:1629`），生产 resume 路径自己在 `signal.rs:1704` 独立调用同一函数，**不读**新缓存字段。 | `signal.rs:1629 / 1704` |
| 新字段不进任何相等比较 | **成立**（附加发现，加强论证）。`LevelCache` 仅 `#[derive(Debug, Clone, Default)]`，**无 `PartialEq`** ⟹ 新字段不可能参与任何 bit-exact 对拍的结构相等；构造点全走 `LevelCache::default()`（`mod.rs:2063`），commit message 的「构造点无需改」成立。 | `mod.rs:1021-1022 / 2063` |

### B.「只写缓存、生产路径零读点」全树读点枚举（复核结论：成立）

`last_freeze_boundary` 全树出现 6 处，逐条判定：

| 位置 | 性质 | 是否活代码 |
|---|---|---|
| `classifier/mod.rs:1067` | 字段声明 | 活 |
| `classifier/mod.rs:2495` | **唯一写点** | 活 |
| `classifier/mod.rs:1287` | **唯一读点**（`TowerCache::freeze_boundary`） | 活 |
| `classifier/tower_cache.rs:117 / :353` | kimi 拆分模块的同名字段与访问器 | **死代码**（`classifier/mod.rs` 无 `mod tower_cache;`） |
| `classifier/incremental/tower.rs:291` | kimi 拆分模块的同一登记行 | **死代码**（无 `mod incremental;`，`mod.rs:6001` 的 `incremental_profile` 是另一模块） |

`TowerCache::freeze_boundary` 的调用点全树 **2 处**，均在诊断 bin `src/bin/p123_fast_replay.rs:2057 / :3098`（`freeze_boundary(level - 1)`）。分类器、交易、订单、风控、`backtest/`、`strategy/` 侧 **零调用**。
另两个访问器 `level_scan_cursor` / `level_scan_units` 的读点同样只在 `p123_fast_replay`（:1737/:1741/:1771-1777/:1818-1824）；`classifier/nest_lifecycle.rs:2124 / :2191 / :5210` 三处引用**全部是 doc 注释**（`///`），非生产读点。

⟹ 报告 ⚠HIGH-4 的「只写缓存 + 唯一读点是新访问器 + 唯一消费方是诊断 bin」**成立**。补充口径：该论证之所以成立，前提之一是 kimi 拆分文件留树但**未挂载**；同一公式因此在树上存在两份源（见 MED-1）。

### C. 三访问器与 kimi 侧的逐字对照（复核结论：等价）

- `level_scan_cursor`：与 kimi `tower_cache.rs:284-289` **逐字相同**。
- `level_scan_units`：main `mod.rs:1274-1279` 用 `&self.l0_units_cache[..]`，kimi 用 `self.l0_units()`（其体 = `&self.l0_units_cache`）⟹ **语义等价**。
- `freeze_boundary`：与 kimi `tower_cache.rs:350-354` **逐字相同**（含 `levels.get(level)` 的差一口径）。
- `TowerCache.levels` 字段文档两侧**逐字相同**（「下标 = 级别 idx，与 `tower_snapshots` 同构」）⟹ 下标口径一致，访问器的 `level-1` / `level-2` 换算在 main 上同义。
- `p123_fast_replay.rs` 在 merge 提交 `79a2715070` 时点与 kimi tip **逐字相同**（`diff` 空），报告「取 kimi 版、main 一行未动」的登记准确。

### D. 编译面复核

`cargo check --all-targets` 于本工位实测 **exit=0，0 error**（仅既有 warning）⟹ 收口确实消掉了 12 错。

### ⚠HIGH-1（本轴唯一 HIGH）— `level_scan_units(1)` 的同长契约在 main 侧不成立，段账本回缩 bar 实测破裂

**文件锚**：`rust/src/theta_v0/classifier/mod.rs:1272-1279`（新增访问器与其契约文档）、`:1813-1826`（`00_l0_units_build`）、`:1889-1905`（回缩检测 + `cache.clear()`）、`:1327`（`clear()` 清 `l0_units_cache`）。

**机理**：main 内联版把「段账本回缩 ⟹ `cache.clear()`」放在 `00_l0_units_build` **之后**；本 bar 的塔用的是 build 后 `Rc::clone` 出的值（因此**生产结果正确、bit-exact 不受影响**），但 `clear()` 把 `cache.l0_units_cache` 清空，而全函数内该字段**只有一个写点**（`:1817`）位于 clear 之前 ⟹ 函数返回时 `l0_units_cache.len()==0` 而 `tower[0]` 满载。kimi 线已在 **#613（收 #609 F2）** 把回缩检测前移到构建之前（kimi tip `797c9ad35c:…/incremental.rs:66-91`，注释原文：「本检测**必须**在 `l0_units_cache` 构建之前」，并给出 #609 F2 实测 BTC 100k @ as_of=71040 的 `0 vs 547`）。本次移植只搬了**读**这个字段的访问器，没有搬修复。

**实测复现（本工位）**：把 kimi 死文件里的现成夹具（`classifier/tests/cache_and_units.rs:33` 的 `segment_ledger_shrink_fixture`，段账本 6→4）搬为临时集成测试，断言改用**本次新增的公开访问器**：

```
[bar1 非回缩]        level_scan_units(1)=Some(6)  tower[0]=Some(6)
[bar2 段账本回缩 6→4] level_scan_units(1)=Some(0)  tower[0]=Some(4)   ← 契约破裂
```

（临时文件 `rust/tests/tmp_shadow645_l0units.rs` 跑完即删，工作区已复原为 clean；复现方式见文末「复现」一节。）
缺陷在**被评 commit 时点即存在**：`git show 2080d2bcad:…/classifier/mod.rs` 中访问器在 `:1140`，而回缩 `clear()` 在 `:1692`、`00_l0_units_build` 在 `:1655`（顺序未变）。

**影响域（照实收窄）**：
- 生产分类/交易/订单/风控：**零影响**（本 bar 消费的是 clone 值，非缓存字段）。
- 诊断 bin `p123_fast_replay` 的 L2/L3 活窗派生：回缩 bar 上落原因码 `scan_units_out_of_sync` 并 `continue`（`p123_fast_replay.rs:2851-2860` 的 #613 消费点守卫接住）⟹ **不静默、可观测**，但该 bar 该级的 PanLive 诊断被跳过；`outcome_tally` 会多出这一桶计数。
- 若无该守卫（kimi 注释点名的 #609 F2 静默通道），后果会是伪装成 `no_window_formed`。守卫在合并树里是唯一兜底。

**为何仍判 HIGH**：它直接否证本轴的核心验收命题——「与 kimi 同位重放**语义等价**」。同一契约在 kimi 侧由「修复 + `debug_assert` + 回归测试」三重保证（守卫恒 0），在 main 侧三者全无（守卫会命中）。这属于「验收句悬空」而非纯性能项。

**收口建议（一行改动 + 两处随入）**：把 `mod.rs:1889-1905` 的回缩检测（`if l0.segments.len() < cache.last_l0_segments_len { … cache.clear(); }`，含 `#543 D1a` 的 `txn_cleared` 快照块整体）前移到 `:1813` 的 `00_l0_units_build` **之前**（kimi `797c9ad35c:…/incremental.rs:80-91` 即成品次序）；随入 kimi 的 `debug_assert_l0_units_in_sync`（kimi `incremental/mod.rs:350-364`）与回归测试（`classifier/tests/cache_and_units.rs:33`）。

### ⚠MED-1 — 新公开契约在 main 活代码里零断言、零回归测试（护栏未随入）

**文件锚**：`rust/src/theta_v0/classifier/mod.rs:1272`（契约文档「与 `tower[0]` 同序同长同源」）；未随入的护栏在死文件 `classifier/incremental/mod.rs:357-364`（调用点 `:290`）（`debug_assert_l0_units_in_sync`）与 `classifier/tests/cache_and_units.rs:33`（`l0_units_stays_in_sync_with_tower0_on_segment_ledger_shrink`）。

移植把「契约声明」搬进了活代码，却把「证明契约的断言与测试」留在**未挂载的死文件**里。后果不止 HIGH-1 这一处：树上因此存在**同一公式的两份源**（`last_freeze_boundary` 登记行同时在 `mod.rs:2495` 活代码与 `incremental/tower.rs:291` 死代码；三访问器同时在 `mod.rs:1262-1288` 与 `tower_cache.rs:284-354`）。合并报告 ⚠LOW-12 登记了「未声明死文件留树」，但未点明**本次移植的源文件本身就是死文件、其配套护栏随之失效**——这是把死文件从「留树无害」变成「留树有害」的实质变化，故独立列 MED。

### ⚠MED-2 — `p123_fast_replay` 守卫注释在合并树里是假声明

**文件锚**：`rust/src/bin/p123_fast_replay.rs:2845-2850`。

守卫注释原文：「根因已在 classifier 侧修死（段账本回缩的 `cache.clear()` 前移到 units 构建之前…），本守卫是**契约面的独立可观测**…**不变式成立时本码恒 0**」。这句在 kimi 线为真，在合并树为**假**（根因未修，本码会命中，见 HIGH-1 实测）。该文件在 merge 时点与 kimi 逐字相同，说明这是「取 kimi 版 bin + 取 main 版 classifier」的组合结果，非移植者笔误；但结果是树上留了一条**与实物矛盾的验收句**。修 HIGH-1 即自动消解；若决定暂不修根因，则须就地订正该注释（说明不变式当前不成立、命中原因码即预期）。

---

## 轴二：⚠MED-5 复核

### A. 位点唯一性（复核结论：三处均唯一确定）

| 钩子 | main 位点 | 唯一性论证 | kimi 对照 |
|---|---|---|---|
| ① `GammaDump::from_env()` | `backtest/fill.rs:3420` | 全文件 `OpsemDump::from_env()` **唯一一处**（`:3412`），钩子紧随其后，与 kimi 相对位置同 | kimi `fill.rs:982`（紧随同一符号） |
| ② `gamma_chi_admitted` | `fill.rs:3723` | `let step_gamma_trade` 在主循环内出现 3 次（`:3710` χ 过滤 → `:3738` nest gate → `:3796` #647 入场复检门），**第一次绑定即 χ 过滤产出**，钩子紧随其后、在所有下游收窄之前 ⟹ 位点唯一 | kimi `fill.rs:1295`（χ:1277 → nest:1310 之间）；两侧该段代码**逐字相同** |
| ③ `write_step` | `fill.rs:3981` | `pi_theta_step_traced_with_risk_seeds` 在文件内**唯一调用**（`:3963`）；PanDiv 候选准备在其**前**（`:3876-3903`），PanDiv 最终选址在其**后**（`:4018+`）⟹ kimi 注释钉死的「三者首次同时在手、且在 PanDiv 最终选址消费前」在 main 结构上唯一满足 | kimi `fill.rs:1453`（`pi_theta_step_traced`:1436 之后、`select_for_bar`:1504 之前）；钩子体 22 行**逐字相同**，含「#71 钩子符号重锚」整段注释 |

补充：`step_gamma`（raw）在 `:3694` 绑定后至 `:3991` 只被只读借用（`:3714` 传引用、`:3717` `.clone()`），dump 读到的确是 χ 前原始候选集；`gamma_dump` 全文件出现点 main 5 处 / kimi 5 处，一一对应，**无漏移植的第四处钩子**。
`backtest/gamma_dump.rs` 与 kimi 侧 `diff` **空**（逐字相同），模块以 `mod gamma_dump;`（`backtest/mod.rs:86`）正常挂载，非死文件。

### B. env 门控「零分配零写入」实测（复核结论：成立）

- 生产门控**唯一**：`GammaDump::from_env()` 读 `OPSEM_GAMMA_DUMP_DIR`，未设/空串 ⟹ `None`（`gamma_dump.rs:64-79`）；`GAMMA_DUMP_DIR_OVERRIDE` 旁路是 `#[cfg(test)]` 限定（`:54-57`, `:65-70`）⟹ release 无第二开启路径。
- **实测（不设 env）**：`env -u OPSEM_GAMMA_DUMP_DIR cargo test --lib backtest::fill` → **34 passed / 0 failed**；随后在 `/private/tmp/wt-645`、`/tmp`、`/var/folders`（maxdepth 5）扫描 `gamma_candidates.jsonl` → **零命中**。这批测试逐 bar 走真实 fill 主循环（含三处钩子所在路径），故「不设 env ⟹ 无文件产出」有可观测证据。
- **实测（模块自测）**：`cargo test --lib gamma_dump` → **4 passed / 0 failed**（含 `gamma_dump_env_gated_bit_exact`：未设/空串均 `None`，且 `typed_ledger` / `n_orders` / `trade_pnls_with_forced` 三输出开关前后 bit-exact）。
- 「零分配」口径：关闭时 `:3723` 分支走 `Vec::new()`（不分配），仅多一次 `is_some()` 布尔求值——与代码注释里 #563 L5 的订正措辞（「零额外**分配/写入**」，非「零额外指令」）一致。**合并报告 §3 ⚠MED-5 正文的「零分配零写入」应以此订正读法为准**（报告已在括号内注明该订正，故不另列发现）。

### ⚠LOW-1 — 三访问器参数口径差一，移植时未加交叉对照说明

`freeze_boundary(level)` 读 `levels.get(level)`，而 `level_scan_cursor(level)` 读 `levels.get(level - 1)`、`level_scan_units(level)` 读 `levels.get(level - 2)`——**同一个 `LevelCache` 需用相差 1 的实参**（`mod.rs:1263 / 1274 / 1286`）。这是 kimi 原样口径（逐字移植，非本次引入），main 侧文档也照搬了「p123 的 target level L 读 `freeze_boundary(L-1)`」这句用法提示；但三者放进**同一个 impl 块相邻位置**后，缺一句互指的下标口径对照（kimi 侧靠 `tower_cache.rs:314-330` 的长文档承担这一角色，该文档未随入）。属易错面，不是错。

### ⚠LOW-2 — 移植的 #563 L5 注释删去了指向设计稿的两句，#600 MED-4 的「两处同步」退化为一处

kimi `fill.rs:1288-1294` 的注释含 #600 Std MED-4 的成果句（「现已回写设计稿正文——`chanlun/review-results/gap2-gamma-candidate-dump-design-20260719.md` §4① …两处措辞自此同步」）；移植进 main 的版本（`fill.rs:3720-3722`）删掉了这两句。删除本身合理（该设计稿随 main #504 归档整删，留着即指向不存在的文件），但结果是 #600 MED-4 要求的「设计稿 + 代码注释两处措辞同步」在合并树里**只剩代码一处**，且注释未留归档指针（合并报告 ⚠LOW-10 已给出 blob 恢复指针 `797c9ad35c` 下 `d290516b34`，代码侧未引用）。
附带：合并报告 §2 A 组称「kimi 该勘误的代码侧**同款**注释随 fill.rs 钩子移植入树」——严格说是**删减版**，建议报告措辞订正或在注释里补一句「设计稿已随 #504 出仓，恢复指针见 #614 报告 ⚠LOW-10」。

### ⚠LOW-3 — `write_step` 文档的下游收窄清单已被 #647 超出

`gamma_dump.rs:200-201`：「`admitted_indices` 来自生产 `filter_gamma_with_admission` …即使下游 **Nest/Xzd 门**继续收窄，也不会污染 `chi_admit`」。合并之后 main 侧在 nest gate 之后又多了 #647 入场结构复检门（`fill.rs:3796`，第三次收窄同名 Vec）。字段语义未破（`chi_admit` 仍是 χ 真值，声明依旧成立），但 `admitted − opened` 差的归因解读从「nest/xzd + 开仓判据」变成「nest/xzd + **入场复检** + 开仓判据」，文档清单应补一项。非本次移植引入（#647 落在 merge 之后，`828809a089` / `8070d8424c`），登记于此以免后续误读 dump。

---

## 复现

```bash
# 工位
cd /private/tmp/wt-645/rust

# 轴一 D：编译面
cargo check --all-targets                     # 实测 exit=0, 0 error

# 轴二 B：env 门控零写入
env -u OPSEM_GAMMA_DUMP_DIR cargo test --lib backtest::fill   # 34 passed
find /private/tmp/wt-645 /tmp /var/folders -maxdepth 5 -name gamma_candidates.jsonl  # 零命中
cargo test --lib gamma_dump                   # 4 passed

# HIGH-1 实证：把 kimi 死文件夹具搬成临时集成测试（跑完即删）
#   源：rust/src/theta_v0/classifier/tests/cache_and_units.rs:33 的 segment_ledger_shrink_fixture（6 段 → 4 段回缩）
#   断言：cache.level_scan_units(1).len() == tower[0].len()
#   结果：非回缩 bar 6==6；回缩 bar 0 != 4  ⟹ 契约破裂
```

## 本次未做（照实登记）

- 未跑全库 `cargo test --lib`（本轴改动面已由 `--all-targets` check + 目标测试子集覆盖；合并报告声明的 `2441/0/138` 因 HEAD 已推进至 #647/#679 而不再可逐项对齐，现库为 2711 项）。
- 未在 kimi 参照面上跑任何构建（封存只读，仅 `git show` / `grep` / `diff`），kimi 侧「守卫恒 0」是据其源码与 #613/#609 F2 注释所载实测数据判定，非本工位复跑。
- 未评审 ⚠HIGH-4 / ⚠MED-5 之外的其余 10 条 ⚠ 条目（不在本次任务范围）。
