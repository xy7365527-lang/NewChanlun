# #336 R3：中枢事件机改消费塔链（单一真相源）——实装 + wf8 实测

**票面**：#336（parent #330 / #274 / #278，blocked-by #331，阻塞 #292）
**日期**：2026-07-26
**裁定来源**：#336 票面（用户裁定 2026-07-26）+ #330 §5 R3 + `.chanlun/review-results/center-death-identity-rootcause-20260726.md` + `.chanlun/review-results/center-death-fix-r8-r0prime-r2-20260726.md`
**基线**：main @ `df64f034ed`（#331 R8+R0′+R2 落地，miskill 零收敛 888→888）

---

## 0. 一句话结论

**miskill 888 → 0（结构性归零，非收敛）**；born 717 与塔 `new_center` **逐身份对账一致**（差额恰为每级开机时静默采纳的首条，L0/L2/L3 各 1、L1 链空 0）；生产轨迹 `trades.jsonl`/`tower_events.jsonl` 与三份基线**逐字节一致**。

**残余不是消失了，是换了形态**：683 条死亡请求命中「链上已退场实例」（新分类 `stale`），其中 **680 条（99.6%）的 Δidx 恰为 −1**——即载体指的是**紧邻上一格**的链实例。对比 #331 的 Δidx ∈ [2, 549]（散布 350 余个取值、全为正、机器系统性落在上游），残余从「无界上游漂移」坍缩为「一格滞后」。三类死亡请求的破坏放行率 = **120/780 = 15.4%**。这一格滞后是否应当算「杀对」（即「场」是否该落在链尾的**前一格**），是教义裁决，本票不裁，如实登记为未判定项（§6.1）。

---

## 1. 事件源切换摘要（符号锚）

### 1.1 唯一真相源

事件机 level ℓ 的在场中枢（「场」）= `Classification::levels[ℓ].centers` 的游标处实例。**这恰是死亡请求载体的源表**（`BspPoint.center` 的 `OwnerRef::Center`，由 `mod.rs` BSP 提取喂给 level ℓ 的同一张 centers）⟹ 在场与载体同表同层，级别对齐**按定义**成立。

| 项 | 改前（#331 交付态） | 改后（#336 R3） |
|---|---|---|
| 出生源 | 自建尾 3 段滑窗复算（`push_segment` + `center_from_segments`/`center_from_window`） | 塔链前缀推进（`consume_chain`） |
| 喂数 | L0 `tower[0][..confirmed_lens[0]]`；ℓ≥1 `project_to_units_resume(tower[ℓ][..w], levels[ℓ-1].moves)` | `&classification.levels[ℓ].centers`（当前全量） |
| 水线 | 方向冻结界推导 `w = min(confirmed_lens[ℓ], tower[ℓ].len()-1)` | **无**（塔链即真相；前缀分叉走重基） |
| 「一中枢一场」 | 抑制规则（在场时 `push_segment` 早退） | **链游标单点**（按构造成立） |
| 出生序号 | `born_seg_ordinal`（段号） | `chain_index`（链下标） |
| 载体不符 | 一律 `miskill` | 二分：链上已退场 ⟹ `stale`；不在链上 ⟹ `miskill` |

### 1.2 塔链消费点（符号锚）

| 位置 | 符号 | 作用 |
|---|---|---|
| `rust/src/theta_v0/backtest/opsem_dump.rs:501` | `OpsemDump::feed_center_lifecycle` | 旁路入口（签名去掉 `tower` / `confirmed_lens` 两参） |
| `rust/src/theta_v0/backtest/opsem_dump.rs:535` | `let chain = &classification.levels[lvl].centers` | **塔链消费点**（唯一真相源读取；借用切片，零 `Rc` 流量） |
| `rust/src/theta_v0/classifier/center_lifecycle.rs:317` | `CenterEventMachine::consume_chain` | 出生的唯一来源；三分支 `Advanced`/`Adopted`/`Rebased` |
| `rust/src/theta_v0/classifier/center_lifecycle.rs:367` | `CenterEventMachine::adopt` | 静默采纳（首次消费 / 重基共用），不产事件 |
| `rust/src/theta_v0/classifier/center_lifecycle.rs:376` | `CenterEventMachine::push_point` | 死亡事件（broken/reset）对链解析 |
| `rust/src/theta_v0/classifier/center_lifecycle.rs:442` | `CenterEventMachine::resolve_kill_target` | #329 校验的 R3 口径（ArenaEmpty / Alive / Stale / Err） |
| `rust/src/theta_v0/backtest/fill.rs:1072` | 调用点 | 参数收缩到 `(i, &classification_i, &classification_step)` |

### 1.3 born/broken/reset 的实际推出方式（如实区别于票面措辞）

票面写「born/broken/reset 由塔结构事件直接推出」。**broken/reset 不可能由塔结构事件推出**——塔**无破坏概念**（`opsem_dump.rs` 模块头原文：「前缀因果塔单调增长（prefix classification 不删结构），**无破坏概念** ⟹ tower_events.jsonl 不输出 destroy 事件（诚实缺席，不伪造）」）。R3 改的是**「在场是谁」的真相源**，不是死亡的触发源：

- **born** = 塔结构事件（链尾新增中枢）⟹ 票面措辞成立。
- **broken** = 本级三类点，且载体恰为在场实例（三类点仍是死亡的唯一教义触发）。
- **reset** = 本级一类点 ⟹ 在场实例同死。
- **链推进时前一实例未收到死亡事件** ⟹ 记 `superseded` 诊断行，**不伪造 broken**。

这条区别写进了 `center_lifecycle.rs` 模块头，不藏在报告里。

---

## 2. 独立复算移除/冻结清单（谱系注记，tombstone 原位）

**全部选择「移除」而非「冻结」**——留着一条能跑的第二判据源，就是留着下一次「两条链互校」。删除物的口径与作废理由逐条写进 `center_lifecycle.rs` 模块头「已作废/已移除」节（学 #282 先例：tombstone 留在原位而非只写报告）。

| 删除物 | 位置（改前） | 谱系注记 |
|---|---|---|
| `CenterEventMachine::push_segment(UnitRange)` | `center_lifecycle.rs` | 独立复算入口 |
| `build: fn(&UnitRange,&UnitRange,&UnitRange) -> Option<Center>` 构造算子指针 | 同上 | 第二套判据源（L0 `center_from_segments` / ℓ≥1 `center_from_window` 的第二个调用者） |
| `segs: Vec<UnitRange>` + 尾 3 段滑窗 + `segments_since_reset()` | 同上 | 段序列口径整节作废 |
| `Reset::cleared_segments` 字段 + JSONL `cleared_segs` 键 | 同上 + `opsem_dump.rs` | 段序列清零证据，无对应物（写 0 会是编造） |
| `born_seg_ordinal` / `died_born_seg_ordinal` 字段 + JSONL `born_seg`/`died_born_seg` 键 | 同上 | 改为 `chain_index` / `died_chain_index` + `chain_idx`/`died_chain_idx` |
| `alive_miskills` 字段 + `alive_mis_kills()`（#331 R2） | 同上 | 逃逸阀读数（对象=吸收态锁死，已被链游标消灭） |
| `resync()` | 同上 | 两个调用者（`watermark_shrink` / `miskill_escape_valve`）都退役 |
| `cl_fed_units: Vec<Vec<UnitRange>>` | `opsem_dump.rs` | units 投影缓存 |
| `MISKILL_ESCAPE_N` + 逃逸阀触发块（#331 R2） | 同上 | 同上 |
| ℓ≥1 `project_to_units_resume` 调用 + `debug_assert_eq!` 配对守卫 + 方向冻结水线推导（#331 R8） | 同上 | R8 口径吸收进 R3（见 §4.2） |
| 本模块 **14 条**单测 | `center_lifecycle.rs` `mod tests` | 全部写在已删路径上；其固化读数在 R3 下**无对应物**（口径消失，非回归）。测试模块头留注记 |

**「建造错误 = 重造没接生产，同型事故」**（票面要求的谱系注记，已写进模块头）：#291 把塔既有的中枢构造重实现了一遍，产出一条**塔链外的第二条中枢链**——#330 §3 例 1 实测身份 `si=624` 在塔 720 条 `new_center` 中**零命中**。同型先例：`.chanlun/review-results/built-but-unwired-pattern-20260723.md`。

### 2.1 #291 对账基线作废登记（票面第 3 项）

**#291 全部 wf8 对账读数作废**，不得再作任何比较基准。已在两处登记：`center_lifecycle.rs` 模块头「已作废」节 + 本节。

作废清单（建立在已删除的第二条链上）：255/711、83/751 身份一致率、`h1_base_dump` 的 763 born / 733 broken / 59 reset、「si+core 同而 ei/dd/gg 异 33 例」、#331 的 65 born / 1 broken / 888 miskill / 62 resync、#331 §3 的 Δidx ∈ [2,549] 分布与「target 本机从未生出 841 条」分层。**新基线 = 本报告 §3。**

特别地：`h1_base_dump` 的 `broken=733` **不是**「R3 让 broken 从 733 掉到 120」的比较基准——#329 步骤一已在同窗坐实那 751 次杀里 **668 次（89%）载体身份与在场中枢不一致**，其中 300 次源区间完全不相交。733 里绝大多数是杀错。

---

## 3. wf8 实测读数

命令：`M8_WIN_FILTER=wf8 VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/r3_dump cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture`
窗口：BTC anchored i=8，test 段 2023-08-17..2024-02-16，264,960 bar，全 5 级。

### 3.1 总表（★miskill 结构性归零）

| 态 | born | broken | reset | **miskill** | stale | superseded | chain_sync | resync |
|---|---|---|---|---|---|---|---|---|
| #291 原态（`h1_base_dump`，**已作废**） | 763 | 733 | 59 | — | — | — | — | 7 |
| #329 锁死态（`h1_after_dump`） | 13 | 1 | 0 | **888** | — | — | — | 7 |
| #331 交付态（`fix331_dump2`） | 65 | 1 | 0 | **888** | — | — | — | 62 |
| **#336 R3 交付态（`/tmp/r3_dump`）** | **717** | **120** | **36** | **0** | **683** | **586**（502 行） | 81 | 0 |

`center_lifecycle.jsonl` 行数 909 → **2139**（350,009 B）。

### 3.2 分级（交付态）

| lvl | born | broken | reset | miskill | stale | superseded | chain_sync |
|---|---|---|---|---|---|---|---|
| 0 | 553 | 113 | 7 | 0 | 655 | 372 行 | 18 |
| 1 | 133 | 4 | 18 | 0 | 17 | 106 行 | 44 |
| 2 | 27 | 2 | 0 | 0 | 1 | 22 行 | 16 |
| 3 | 4 | 1 | 11 | 0 | 10 | 2 行 | 2 |
| 4 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |

**锁死彻底解除**：改前 `alive_si` 全窗只有 2–3 个取值；改后 L0 出生覆盖链下标 1..550（550 个不同实例）。#331 「51 次 born 只 2 个身份」的复锁现象消失（逃逸阀已退役，不再从前缀头回放）。

### 3.3 ★塔对账（票面「born/broken 与塔对账一致」）

对账口径：机器 level ℓ ⟺ 塔 `new_center level ℓ+1`（塔 L(k) ≡ `levels[k-1].centers`，#330 §2.3）。**逐身份 `(si, zd, zg)` 比对**，不只比计数：

| 机器 lvl | born | 塔 `new_center` L(lvl+1) | 塔有机器无 | **机器有塔无** |
|---|---|---|---|---|
| 0 | 553 | 554 | 1 | **0** |
| 1 | 133 | 133 | 0 | **0** |
| 2 | 27 | 28 | 1 | **0** |
| 3 | 4 | 5 | 1 | **0** |

**「机器有塔无」全级为 0**——机器不再生出任何塔链外身份（这正是 #330 §3 例 1 的病：`si=624` 在塔 720 条里零命中）。

**3 条「塔有机器无」全部归因坐实**，不是缺口：每级唯一未 born 的链下标恰为 **0**，而每级开机（首个交易活跃 bar / 该级涌现 bar）的 `chain_sync reason:"adopt"` 恰好 `chain_len=1`：

| 机器 lvl | adopt bar | adopt chain_len | 未 born 的链下标 | 塔有机器无的身份 |
|---|---|---|---|---|
| 0 | 597 | 1 | {0} | `(si=5, zd=2856641e6, zg=2863593e6)` |
| 1 | 2502 | **0** | {} | —（开机时链空 ⟹ 零差额） |
| 2 | 4766 | 1 | {0} | `(si=1254, zd=2602696e6, zg=2621000e6)` |
| 3 | 14011 | 1 | {0} | `(si=1254, zd=2581200e6, zg=2629900e6)` |

即：**开机时链上已有的那条中枢走静默采纳，不伪造出生 bar**（它的出生在开机之前）。这与 `write_tower_event` 同款纪律（`prev_tower` 在非活跃 bar 也持续更新 ⟹ 首个活跃 bar 不重播既有 Compose）。L1 开机时链空，差额为 0，是该口径的对照见证。

⟹ **born ≡ 塔 new_center，模去每级开机采纳的首条。对账一致。**

**L0 born 553 条覆盖 550 个不同链下标**（3 个下标二次出生：392 / 396 / 489）。成因坐实：这 3 个恰是 L0 的 3 次**回缩型**重基（`at == chain_len`：bar 189379 at=392、bar 192010 at=396、bar 236523 at=489）——链回缩后同一下标由**不同实例**占据 ⟹ 该下标二次出生。逐条核对两次出生的 `(si,zd,zg)` 确实不同（如 idx 489：`si=236024 zd=4210003e6` vs `si=236024 zd=4208000e6`）。

### 3.4 ★残余：683 条 stale，Δidx 坍缩为一格

| 指标 | #331 交付态（miskill 888） | **#336 R3（stale 683）** |
|---|---|---|
| Δidx = target_idx − alive_idx | ∈ **[2, 549]**，全为**正**，350 余个取值 | ∈ **[−2, −1]**，全为**负** |
| Δidx 众数 | 最大频次仅 21 | **−1 占 680/683 = 99.6%**（−2 占 3 条） |
| 语义 | 机器系统性落在塔链**上游**（无界漂移） | 载体指**紧邻上一格**（一格滞后） |
| 分类 | miskill（当作错位） | stale（时序滞后，不计误杀） |

stale 按 trigger：`third` 660 / `first` 23。按级：L0 655 / L1 17 / L2 1 / L3 10。

**三类死亡请求的破坏放行率 = 120 / (120 + 660) = 15.4%**，如实登记，不做措辞美化。一类点：reset 36 条中 died 非空仅 **10** 条（26 条到达时场为空），另 23 条被判 stale。

**superseded 586**（502 行）：绝大多数链实例是被链推进换下的，而非被死亡事件杀掉。这是塔的结构真相（塔按结构推进），本机**不伪造** broken 掩盖它。

**归因（诚实）**：三类点的载体是「所离开回抽的中枢」。点确认时，链往往已经推进一格——因为「离开旧中枢的那段走势完成」既是三类点可确认的条件，也是塔新中枢成交的条件，二者在结构上几乎同时。故 Δidx = −1 是**结构性一格滞后**，不是接线错误、不是身份空间错位（miskill 已归零坐实）。是否应把「场」定义为链尾的**前一格**（从而让这 680 条变成 broken），是教义裁决 ⟹ §6.1。

### 3.5 miskill 归零的含义（不夸大）

**miskill = 0** 的语义是：**wf8 全窗没有一个死亡请求的载体落在本级塔链之外**。R3 前该数为 888（含 96 条 ℓ≥1 级别错配 + 792 条 L0 塔链外身份）。这是「同源」的产物级证明。

但 0 **不是**「校验恒真」的证明——它是「本窗没有回归」的证明。校验的存废裁决见 §4.1。

---

## 4. 校验存废与 R8/R0′/R2 处置

### 4.1 #329 校验（CenterMisKill 拒杀）：**保留为廉价看门狗**（票面第 4 项二选一）

**裁决：保留，但口径收窄。** 理由逐条：

**为什么不退役**：R3 让在场与载体同表 ⟹ 「两条链互校」这层语义确实消失，`target ∈ 链` 变成**近恒真**（本窗 0 反例）。但**近恒真不是恒真**，剩下的反例正是真回归信号，且只有它们：
- **载体不在本级链上**：跨级错取（#330 §3 例 3/4 那一面，96 条）、载体表被重切后旧快照失效、#148 升级重切。这类错位在 R3 下**不可能由链滞后造成**（滞后一律落进 `stale`）⟹ 一旦出现就是接线/口径回归。
- **载体缺席**（`target=None`）：点无中枢载体却要杀中枢 ⟹ 无从校验，一律拒（不猜）。

**代价**：每个已确认点一次三元组比对 + 链上线性查找（`rposition`，陈旧请求多命中游标附近）。wf8 全窗 839 次死亡请求，可忽略。

**收益/代价比**：上述两类回归立即显影，且 miskill=0 本身就是「同源不变量」的持续机检——退役等于把这条不变量的哨兵拆了。

**口径收窄如实登记**：载体命中链上**已退场**实例的情形**不再计入 miskill**（改判 `stale`）。⟹ **miskill 读数与 R3 前不可比**（888 vs 0 不是同一口径下的下降，是「888 条里 0 条属于新口径的 miskill、683 条属于新增的 stale 类、其余因机器不再锁死而根本没发生」）。这一点写进了 `CenterMisKill` 文档。

### 4.2 #331 R8（塔层索引对齐）：**退役（口径吸收）**

R8 修的是「事件机吃第几层塔单元」。R3 下本机**不再投影任何塔单元**——`project_to_units_resume` / `confirmed_lens` 水线 / `blocks` 配对全部从本路径消失。级别对齐**按定义**成立：机器 level ℓ ⟺ `levels[ℓ].centers`。

- R8 的**结论**（事件机 level ℓ 应与 `levels[ℓ].centers` 同层）被 R3 吸收为构造性事实，并由 §3.3 逐身份对账实证（层错配 0，「机器有塔无」0）。
- R8 的**实现**（换层 + 水线重推 + `debug_assert_eq!` 配对守卫）随投影路径一并删除。
- 顺带：#330 未能判定项 2（`center_own_dir_at` 索引错位潜伏缺陷）在本路径上**彻底消失**（不再调 `project_to_units_resume`）。但该缺陷在**分类器内部**是否还有别的暴露面，本票未查（`mod.rs:2158-2171` 是唯一正确配对，未动）。

### 4.3 #331 R0′（破坏后段游标推进）：**退役**

其对象（段序列）已不存在。「已消费段不倒回」的执行者本就是塔窗口扫描游标 `i=j*`（`recursive_tower.rs` 窗口扫描），R3 直接消费其结果 ⟹ 无需在机器侧复制该语义。#331 已实测其在本窗爆炸半径上界 = 1 事件，退役无读数影响。

### 4.4 #331 R2（拒杀逃逸阀）：**退役**

其对象——「在场实例只能靠死亡事件换人 ⟹ 吸收态锁死」——已被链游标推进消灭（无吸收态则无需逃逸）。实证：交付态 `resync=0`（#331 交付态 62 次），L0 出生覆盖 550 个不同实例（#331 为 2 个）。留着反而重演「resync 后从前缀头回放 ⟹ 确定性复生同一身份」的噪声。#331 未能判定项 3（resync 从前缀头回放）**随之关闭**——回放路径已删除。

---

## 5. 双轨与锁读数

| 项 | 结果 |
|---|---|
| `wf8 trades.jsonl` vs `fix331_dump2` / `h1_after_dump` / `vocab_align_dump` | **三份全部逐字节一致**（948,340 B，`cmp` 无差异） |
| `wf8 tower_events.jsonl` vs 同三份 | **三份全部逐字节一致**（354,946 B） |
| wf8 报告行 | `execR=+4040483 MaxDD=0.0917 stage=I R=+4417092 LCB(R)=-1921382 → INCONCLUSIVE`（与 #331/#329 基线逐字相同） |
| `center_lifecycle.jsonl` | **变化**（预期）：909 → 2139 行，schema 换口径，登记见 §3 |
| `cargo test --lib` | **1910 passed / 0 failed / 138 ignored**（基线 1909 − 删 14 + 新 15 = 1910，逐条对上） |
| 四把锁（`--ignored`，BTC 数据） | **4/4 绿**：`btc_type2_open_short_channel_witness` / `btc_prune_leg_exit_type_matches_account_identity` / `btc_type2_residual_correction_witness` / `typed_ledger_btc_smoke` |
| #291 wf8 自证（`center_lifecycle_wf8_events_replay`，新 R3 断言） | **绿**，读数与独立分析脚本逐项一致（born=717 broken=120 reset=36 miskill=0 stale=683 superseded=586 chain_sync=81 resync=0） |

**四把锁「零翻动」的诚实标注（沿用 #323 Critical-2 / #331 先例）**：本票全部改动位于 `feed_center_lifecycle` 及其调用的 `CenterEventMachine`，二者由 `OPSEM_DUMP_DIR` env 门控（`fill.rs:1071-1073`：`if let Some(dump) = opsem.as_mut()`）。四把锁**不设该 env**，因此对本票改动路径的覆盖为 **0**。**零翻动是零覆盖的结果，不构成安全证据。** 真正覆盖本次改动的两条硬证据是：(1) 开着 dump 跑 wf8 后 `trades.jsonl`/`tower_events.jsonl` 与三份独立基线**逐字节不变**——证明诊断旁路即便打开也不回馈生产链；(2) 15 条新单测（`consume_chain` / `push_point` 全分支）+ #291 wf8 自证测试。

**慢锁不跑标注**：本票未跑重型多窗回归（`m8_e2e_all_systems_oos` 全窗臂、`l3_*` 多窗族、`p1xx` 校准 bin）。理由 = 生产轨迹逐字节不变 + 改动全在 env-gated 旁路内，重型窗口的期望翻动为 0；**留重型窗口**，如需硬证据请在重型窗口批次里补跑。

**090 照实**：`cargo test --lib` 与四把锁跑的是当前工作区（含既有脏文件 `rust/src/bin/p107_level_calib.rs`）。`p107_level_calib.rs` 是 `[[bin]]`，不进 `--lib` 编译单元 ⟹ 对本次读数无影响；其余脏文件全为 `.md`/`.json`/`.claude` 配置，不入 Rust 编译。

---

## 6. 未能判定项

1. **★「场」是否该落在链尾的前一格（Δidx=−1 的 680 条该不该算杀对）**——这是本票残余的全部内容，且**是教义裁决不是实现选择**：三类点的载体是「所离开回抽的中枢」，而点确认时链已推进一格（离开完成 ⟺ 新中枢成交，结构上几乎同时）。若裁定「场 = 链尾前一格」，broken 放行率会从 15.4% 大幅上升；若裁定「场 = 链尾」，则须承认「三类点破坏的多数是刚被链换下的那个」并让 `stale` 成为常态读数。本票按后者实装（场 = 链游标 = 链尾），因为「链游标单点」是「一中枢一场按构造成立」的直接读法。**须用户裁决。**
2. **`superseded` 586 的教义地位未裁**：链推进取代 ≠ 教义破坏（塔无破坏概念），本票只如实计数不产事件。但若 #292 的减补动作要覆盖「中枢下场」这件事，586 次取代 vs 130 次教义死亡的差额是个待裁的动作缺口。
3. **前缀分叉守卫的有效域（已写进代码注释）**：只查「长度回缩」与「已消费末条身份」，**不做全前缀比对**（全比对 O(链长)/bar/级 ⟹ wf8 量级 1e9 次比较）。依据 = `LevelCache.centers` 文档「前缀不可变，尾部追加」+ 唯一前缀变异源是该级缓存全量重置（frontier 变异触发，几乎必然改写末条）。**比这更深的前缀改写本守卫检不出。**
4. **重基把场重置到链尾实例**——若该实例此前已被死亡事件杀掉，重基会让它重新在场。本窗 81 条 `chain_sync` 中 76 条是 rebase，未逐条核查是否有此复活。重基是工程再同步（非教义生死），条数已如实计数。
5. **一类点同死的「禁横跨走势类型生死边界拼中枢」约束移交塔后是否仍被执行——未核验**。R3 删掉了机器侧的段序列清零，该约束的执行者变成塔的中枢构造。塔是否真的不跨走势类型边界拼中枢，本票未查（是 `recursive_tower` 窗口扫描的性质，属另一票）。
6. **跨窗普适性**：全部读数只来自 wf8 单窗。塔对账（born ≡ new_center 模去开机采纳）与层对齐是结构性的（不依赖窗口），但 Δidx 分布、放行率 15.4%、superseded/born 比例均为单窗观测。
7. **L4 面仍未覆盖**（#330 未能判定项 5 / #331 未能判定项 6 仍未闭合）：交付态 L4 只有 1 条 adopt（chain_len=0）、零 born/broken/stale。
8. **`h1_after_dump` / `fix331_dump2` 作对拍基线**依据其 mtime 与提交时序推断（沿用 #330 §1 / #331 的判断），未做二进制级对应验证。三份基线的 `trades`/`tower_events` 互相逐字节相同，间接提高了可信度。

---

## 7. 纪律

- 只 `git add` 本票自己改的 4 个源文件 + 本报告：`rust/src/theta_v0/classifier/center_lifecycle.rs`、`rust/src/theta_v0/backtest/opsem_dump.rs`、`rust/src/theta_v0/backtest/fill.rs`、`rust/src/theta_v0/backtest/wverify_run.rs`。既有脏文件（`CLAUDE.md`、`AGENTS.md`、`.claude/`、`.agents/`、`rust/src/bin/p107_level_calib.rs`、`skills-lock.json`、`tmp/fold-r3-experiment/*`、既有 `.chanlun/**` M/?? 文件）**未动不 add**。
- 未回关 issue、未发 GitHub 评论。
- 分析脚本为一次性 `python3 -c`（未落盘）；产物 `/tmp/r3_dump`。
- **TDD 红→绿四切片**（每片先写测试跑红、再实装跑绿）：
  ① 塔链消费（3 条测试）→ 红（桩体返回空 `Advanced`）→ 实装 `consume_chain`/`adopt` → 绿；
  ② 死亡事件对链解析（6 条）→ 红（`push_point` 桩体返回 `Silent`）→ 实装 → 绿；
  ③ stale/miskill 分野（4 条）→ **未取得独立红**（如实登记的 TDD 偏差：`resolve_kill_target` 的 Stale/Err 分支在切片②实装时一并写入，其测试后补 ⟹ 一写即绿）；
  ④ 前缀分叉重基（2 条）→ 红（守卫缺失）→ 实装守卫 → 绿。
- **接缝（seam）声明**：测试只打在 `CenterEventMachine` 公开面（`consume_chain` / `push_point` / `alive_center` / `chain_len` / `counts` / `mis_kills` / `stale_requests` / `superseded`）。JSONL 写入器（`write_cl_*`）不单测——其正确性由 #291 wf8 自证测试（消费真实产物、断言 schema 与 stale 判据 `target_idx < alive_idx`）覆盖。
- 末尾 code-review 按编排者指示**未做**（改由独立影子评审）。
