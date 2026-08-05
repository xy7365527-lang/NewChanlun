# issue #908 探针：NestInterval 被套两区间的父子性 + typed 真链 rung 长度分布

- **票**：[#908](https://github.com/xy7365527-lang/NewChanlun/issues/908)（wayfinder:task，父图 #787），喂 [#817](https://github.com/xy7365527-lang/NewChanlun/issues/817) N-1 裁定三有效域 / N-6 退场条款 / SPEC [#847](https://github.com/xy7365527-lang/NewChanlun/issues/847)
- **性质**：乙类探针（产读数喂决策）。**只测/只证，不裁。**
- **基线**：本地 `main` = `db5b84ecf9`，分支 `probe/908-nest-parentage`。
  - ⚠️ **归正记录**：worktree 初始 HEAD = `19b4015927`（祖传线，无 `rust/src/theta_v0/`、无 `formal/`）——本仓第十二次撞（票 #909 在案）。`git checkout -B probe/908-nest-parentage db5b84ecf9` 归正后开工。`origin/main` 未使用。
  - worktree 缺 `analysis/data_cache/btc_1m_full.json`（gitignore），symlink 到主树同名文件（只读）。
- **窗口/标的**：BTC 1min，三窗（m8 口径）
  | tag | 日期窗 |
  |---|---|
  | `p3fold` | 2023-01-01 → 2023-06-30 |
  | `wf7` | 2023-02-17 → 2023-08-16 |
  | `wf8` | 2023-08-17 → 2024-02-16 |
  - ⚠️ **`p3fold` 与 `wf7` 重叠约 4.5/6 个月** ⟹ 三窗**不独立**，实为约两段独立时段（2023-01→2023-08、2023-08→2024-02），合计 13.5 个月，占 BTC 数据全史（2017-08-17→2026-05-31）约 **13%**。**所有百分比不得外推全史。跨窗稳定性只在这 13% 内验过。**
- **门开条件声明**：**本读数在 `THETA_NEST_CERT_GATE=1` 条件下取得，非默认路径。** 默认门关
  （`backtest/admission.rs:69-72` 严格 `.ok().as_deref() == Some("1")`，空串/`0`/`true` 全判关）。
  照 `scripts/check_armR_trades_digest.py:17` 先例开门。复现命令：

  ```bash
  cd rust
  for tag in p3fold wf7 wf8; do
    M8_WIN_FILTER=$tag VOICE_EXEC=1 THETA_NEST_CERT_GATE=1 \
      M8_REPORT_PATH=/tmp/p908_m8_$tag.md \
      cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos \
      -- --ignored --nocapture
  done
  # 读数行：NEST_GATE_INDEX / NEST_GATE_RUNG908（本票新增）/ NEST_GATE_LEVEL
  ```

---

## 问二（承重）：typed 真链 rung 长度分布

`nest_index.rs:295-297` `assemble_typed_certificate` 装配、去重入索引的 typed 证书链。
`rungs = judge_at().len() - 1`（`nest_index.rs:321`）。

### 全局（逐窗）

| 窗 | indexed | rungs=0 | rungs=1 | rungs≥2 | rungs=0 占比 |
|---|---|---|---|---|---|
| `wf8` | 61 | 61 | 0 | 0 | **100.00%** |
| `wf7` | 86 | 82 | 1 | 3 | **95.35%** |
| `p3fold` | 105 | 99 | 3 | 3 | **94.29%** |
| **三窗合计** | **252** | **242** | **4** | **6** | **96.03%** |

（`assembled == indexed` 三窗全成立 ⟹ 去重零损耗。）

### 逐级别（`NEST_GATE_RUNG908 rungs_lk`，行 = exec 级别槽，列 = `[0,1,2,≥3]`）

| exec 级 | `wf8` | `wf7` | `p3fold` | 合计 [0,1,2,≥3] |
|---|---|---|---|---|
| L0 | [0,0,0,0] | [0,0,0,0] | [0,0,0,0] | [0,0,0,0]（设计性跳过，nest 不听 L0，`nest_index.rs:287`） |
| L1 | [35,0,0,0] | [44,0,3,0] | [38,2,3,0] | **[117,2,6,0]** |
| L2 | [14,0,0,0] | [8,1,0,0] | [10,1,0,0] | **[32,2,0,0]** |
| L3 | [11,0,0,0] | [29,0,0,0] | [51,0,0,0] | **[91,0,0,0]** |
| L4 | [1,0,0,0] | [1,0,0,0] | —（无 L4 槽） | **[2,0,0,0]** |

- **rungs≥1 全部出现在 L1、L2**；**L3/L4 三窗 93 张证书全部 rungs=0**（100% 单级）。
- **rungs≥3 三窗零例。**
- 全局 rung 边总数（Σ rungs）：`wf8`=0，`wf7`=7，`p3fold`=9，**合计 16**。

### 对照锚（照票面警示引用）

[#802](https://github.com/xy7365527-lang/NewChanlun/issues/802) 的**向上** rung 链 95.36% 为空。
⚠️ **那是向上机制、与本项不同源，只作量级参照，不可当预期值**（本仓 `l2-depth-distribution-20260702.md` §5
已有一次把两个方向读数当「同一件事两次测量」的实事故，该节已被删除）。
本项 `wf7` 单级占比 **0.9535** 与 95.36% 数值几乎相同——**这是巧合，不是复现，不得作为互证。**

### 样本量与聚簇自评

- **rungs≥1 的证书三窗合计只有 10 张（4 + 6），rung 边 16 条。** 这是**十位数级样本**，
  与 [#852](…/issues/852)「五样本全在同一 4K bar 区间」同一量级风险带。
- 聚簇读数（`pos_hist*`，桶宽 = 基例区间右端 source_index / 100k）：
  | 窗 | 全 indexed | 仅 rungs≥1 |
  |---|---|---|
  | `wf8` | [20,19,22] | [] |
  | `wf7` | [19,45,22] | [2,2] |
  | `p3fold` | [48,15,42] | [2,3,1] |
  - 全 indexed 分布在窗内三桶间大体均匀（无单桶 >56% 的 #851 式主簇）。
  - rungs≥1 的 10 张分散在 5 个桶（最大簇 3/10 = 30%）。**逐例与逐簇口径差别在这个 n 下无法区分**——
    「96.03% 为 0」的**逐例**读数在逐簇口径下分母会从 252 掉到十位数，占比不稳。**结论层引用请引逐例数并附本条。**
- **未测**：其余 9 个 BTC prereg 窗、其余 7 个品种、全史。

### 本项**不作**的判断

票面写明：rung 长度 95%+ 为 0 ⟹ 两条 `N^δ` 生产恒等 ⟹ N-6 处置升级为「必须收敛成一条」。
**本报告只报数（96.03%，三窗全 ≥94.29%），该升级判断归 #817 N-6，不在此裁。**

---

## 问一：`NestInterval` 那条链上被套的两个区间是不是父子？

### 步骤 1 —— 结构判定：**给不出「恒真」，N-1 的论证不外推到这条链**

**N-1 裁定三的论证依赖两件事**：(a) 被套对象在 `sub_moves` 树上是外层对象的**后代**
（`backtest/econ_positive.rs:822/:827` 只在父自己的子段里找 → `descend_type1_anchor_depth` 内递归钻进
`subs[tidx]`）；(b) `classifier/descend.rs:94/:102` 的 `lo()/hi()` 对子段递归取 min/max。
(a)+(b) ⟹ 子集取极值 ⟹ 价格包含恒真。

**这条链上 (a) 与 (b) 都不成立**，逐行点名：

1. **(a) 不成立——外层区间来自「同级全表平扫 + 只按索引包含过滤」，没有任何祖先/后代边。**
   - `classifier/nest.rs:975-977` `let Some(events) = events_by_level.get(level)`：候选父 = **该级别全部事件**；
   - `classifier/nest.rs:979-986`（`order.sort_by_key`）：按 `sel_key` 排序，纯排序，不建结构边；
   - `classifier/nest.rs:991-993` `if event.side != side { continue; }` —— 第一道过滤只判**方向**；
   - `classifier/nest.rs:994-997` `let parent = typed_interval(event, caliber); if !is_sub(child, &parent) { continue; }`
     —— 第二道过滤**只判索引包含**。
   - ⟹ 被选中的「父」= 该级别里**索引区间恰好套住子区间的同向事件**，与子事件之间**不存在任何
     所有权/子段/后代关系的检查或保证**。`d_parent_interval*` 那条 `CandDeltaEvent` 链同构：
     `classifier/nest.rs:1266`（`is_sub(child_iv, &iv)`）之前只有 `:1258` 的 `cand_delta`/`side` 过滤。
   - 反面对照：N-1 那条链**有** `find_move_by_end_index(subs, source_index)` 把搜索域限在父自己的子段里；
     本链**没有任何等价语句**。

2. **(b) 不成立——`NestInterval` 上根本没有价格。**
   - `classifier/nest.rs:43-51`：`NestInterval { end_time, start_time, idx }`，**三个字段全是索引/序号，无 lo/hi**；
   - `classifier/nest.rs:71-73` `is_sub` 只比 `start_time`/`end_time`；
   - 载体事件同样无价格区间：`classifier/level_view/confirm.rs:123-148` 的 `NestCandidateEvent` 字段
     `seg_a`/`interval_b`/`interval_a` 全是 `(usize, usize)` 源索引，**没有价格上下界字段**。
   - ⟹ 「索引 ∧ 价格 双坐标合取」这个候选在这条链上**连表达都表达不出来**——要谈它必须先**给 `NestInterval`
     补一个价格挂法**，而挂法选哪个是教义选择（归 #817），不是探针能定的。

**结论（问一走的是「非恒真」这条）**：N-1 裁定三的「价格包含恒真」**只在向下下钻链（`descend_type1_anchor_depth`
+ `descend.rs` 递归 min/max）上成立**，**不覆盖 `classifier/nest.rs` 的 `NestInterval` 链**。该裁定的有效域边界
应落纸为：**「恒真」限定在被套对象为外层对象后代、且价格区间由后代递归取极值定义的那条链**。

### 步骤 2 —— 可达性：四个构造点在生产决策路径上**全部不可达**；真正生产可达的是**第五个**

案记的四个构造点（行号已核，本基线 `db5b84ecf9`；案记 `:1074/:1084/:1095/:1111` 对应实际
`:1072/:1082/:1094/:1110`，票面已承认行号漂移）：

| # | 构造点 | 所在函数 | 消费者（逐行穷举） | 生产决策路径可达？ |
|---|---|---|---|---|
| 1 | `nest.rs:1072` | `d_parent_interval`（episode 口径） | 唯一调用 `nest.rs:1736`，位于 `#[cfg(test)] mod tests`（起 `:1363`） | **否，test-only** |
| 2 | `nest.rs:1082` | `d_parent_interval_snapshot` | `nest.rs:1104`（`d_parent_interval_full`，**零调用者**）、`nest.rs:1149-1164`（`assemble_certificate_snapshot`，实参 `&d_parent_interval_snapshot`）、`:1729/:1733`（tests） | **否**（见下 ⑤） |
| 3 | `nest.rs:1094` | `d_parent_interval_terminal` | 唯一调用 `nest.rs:1175`（`assemble_certificate_terminal`） | **否**（见下 ⑤） |
| 4 | `nest.rs:1110` | `d_child_interval` | `nest.rs:1193`、`nest.rs:1270`（皆在 `assemble_certificate_with`/`extend_upward` 内） | **否**（见下 ⑤） |

⑤ `assemble_certificate{,_snapshot,_terminal}` / `assemble_certificates*` 这条 `CandDeltaEvent` 链的
**全部消费者穷举**（`grep` 全 `src`）：

- `src/bin/p92_nest_replay_postruling.rs:302`、`p107_level_funnel_audit.rs:434`、`p116_turnpoint_anchor_existence.rs:407`、
  `p123_fast_replay.rs:1355`、`p124_shard_replay.rs:738`、`p124_merge.rs:780`、`strict_nest_check.rs:1584`
  —— 全是**探针 bin**，不在生产回测路径上；
- `src/theta_v0/backtest/runner_tests.rs:121`、`:146` —— **测试**；
- `src/theta_v0/backtest/opsem_dump.rs:224`（`summarize_strict_nest_certificates`）—— 唯一「非 bin 非 test」消费者，
  但它**由 `opsem_dump.rs:251-260` 的 `strict_nest_sidecar_enabled()` 门控**（env `THETA_STRICT_NEST_SIDECAR`，**默认关**），
  且 `opsem_dump.rs:262-266` 明写该 sidecar「**只读 env 门控…不参与订单/候选/风控/账本**」。
  ⟹ **不在生产决策路径上**（只读诊断 sidecar，门默认关）。

**⟹ 按票面自己的规则「生产不可达的判伪分歧」，这四个构造点的价格包含实测不必跑。**

**但票面漏了第五个构造点，它才是生产可达的那个：**

- **`classifier/nest.rs:487-499`（基线 `db5b84ecf9` 行号；本分支加可见性注释后为 `:490-502`）`typed_interval`** —— 唯一在生产 π 门上被消费的
  `NestInterval` 构造点。可达链逐行：
  `backtest/fill.rs:4941` → `backtest/admission.rs:1017` → `admission.rs:853`（`sync_index`）
  → `classifier/nest_index.rs:295-297`（`assemble_typed_certificate`）→ `nest.rs:912`（`base_interval`）
  与 `nest.rs:994`（rung 侧 `parent`），二者都由 `typed_interval` 构造，口径固定 `Caliber::B`（`admission.rs:856`）。
  ⚠️ 「唯一入口」这类断言按 N-2/N-6 教训穷举过：`typed_interval` 全仓调用点 = `nest.rs:912`（基例区间）/`:982`（排序键）/`:994`（rung 父区间）三处，
  全在 typed 装配内；`assemble_typed_certificate*` 生产侧消费者 = `nest_index.rs:295-297`（另有
  `classifier/turn_class.rs` 内 6 处全在 `#[cfg(test)]` 模块、`nest.rs:1035` 的批量壳）。

### 步骤 3 —— 实测：对生产可达的第五点做了什么、没做什么

**做了（结构层，n=16 rung 边，三窗）**：新增只读 sidecar 逐条 rung 边复刻生产同一过滤条件
（同向 ∧ `is_sub`，`nest_index.rs::observe_rungs_908`），数**该级别通过过滤的候选父事件个数**：

| 候选父个数 | 1 | 2 | 3 | 4 | 5 | 6 | ≥7 |
|---|---|---|---|---|---|---|---|
| `wf7`（7 边） | 7 | 0 | 0 | 0 | 0 | 0 | 0 |
| `p3fold`（9 边） | 7 | 0 | 0 | 0 | 1 | 1 | 0 |
| **合计（16 边）** | **14** | 0 | 0 | 0 | **1** | **1** | 0 |

- **14/16 条边上「父」由索引包含唯一定死**；
- **2/16 条边上有 5 个和 6 个同向事件同时通过 `is_sub`** ⟹ 这两条边的「父」是 `sel_key` 字典序
  （`nest.rs:968-971` DFS 取首个可行）**选出来的一个，而不是结构事实**。
  这是「被套两区间不必然是父子」的**直接实测证据**，不只是代码推理。
- `degenerate`（child 区间与 parent 区间完全相等）= **0/16**。
- ⚠️ n=16，其中 9 条来自与 `wf7` 重叠的 `p3fold`；**这两个数字（12.5% 多候选）绝不可外推**。

**没做（明写查不到 / 测不出来）**：

1. **价格包含率没测。** 原因不是没跑，是**碑上没有价格**：`NestInterval`（`nest.rs:43-51`）与
   `NestCandidateEvent`（`level_view/confirm.rs:123-148`）都不带价格上下界。要测必须先选一个价格挂法，
   而这是教义选择（归 #817），探针替它选就是越权。两种候选挂法的处置（**推导，非实测**）：
   - **挂法 A（同一价格序列上按索引区间取 min/max，如 `divergence.rs:888` `move_range_envelope` 的
     fold 语义）**：索引包含 ⟹ 取极值集合是子集 ⟹ 价格包含**恒真**（可证，不必测）。
     ⚠️ **但本链的 child 与 parent 处在相邻两级、`segments` 数组不同**，且 `move_range_envelope:894`
     的过滤是「段须整支落入 span」——高级别段可能被边界整支剔除、甚至 fold 出 `None`。
     **跨级 + 整支过滤这两件事使子集单调性论证不闭合**，所以挂法 A 在**同级同序列**下恒真，
     **跨级**下**未证也未测**。
   - **挂法 B（按背驰段 C 端点价定区间）**：与索引包含无单调关系，**不恒真**，且**无载体可测**。
2. `CandDeltaEvent` 链（四个构造点）的价格包含率没测——按票面规则生产不可达，免测。
3. 其余 9 个 BTC 窗、其余 7 品种、全史没跑。

---

## 问三：`assemble_typed_certificate` 的三道门是否真含「级别连续性（相邻级差恰 1）」

**查到。答案：三道门里没有这一条；但「相邻级差恰 1」在这条链上确实成立——由递归下标算术保证，不是门。**

- 文档自述三门（`nest.rs:1119-1123`，`assemble_certificate` 结果包）：「三门为**方向一致、闭包含、每级 Cand**」——**级别连续性不在其中**。`assemble_typed_certificate`（`:894`）沿用同一三门表述。
- `NestCertificate::n_delta` 的诚实边界注释（`nest.rs:306-309`）自己点名：
  「装配层前置（方向逐级来源、**级别连续性**、rung 与 `CandDeltaEvent` 的身份对应）**证书无字段可复验**，
  `n_delta=true` **不**蕴含它们成立；它们由 `assemble_certificate` 三门 DFS 在生产时保证」。
  ⟹ 教义面把级别连续性归给「装配层保证」，而**装配层并未把它做成一道判据门**。
- 装配层实际怎么保证的（逐行）：
  - `nest.rs:922`：起点 `exec_level + 1`；
  - `nest.rs:973-974`：`if level > top_level { return true }` 为递归终止；
  - `nest.rs:975`：`events_by_level.get(level)` —— 每层**只**扫该级别的事件表；
  - `nest.rs:1008`：递归实参 `level + 1`。
  ⟹ 每条 rung 恰好取自一个级别，级别从 `exec+1` 逐 1 递增到 `top_level`，**无跳级、无重级、无缺级**。
  这是**下标算术的结构后果**，链上任何一处都没有「相邻级差 == 1」这样的谓词判断。
- **顺带查到的一条（未在票面列）**：typed 路径的第三门「每级 Cand」在 **rung 级已被撤掉**——
  `nest.rs:989-990` 注释「#97 初筛只看结构（D1 裁定）：**rung 级不再要求 `divergence_confirmed`**；
  力度真值仍在事件字段上独立可查」。⟹ **rung 级运行时实判的只有两条：`side` 相等（`:991`）+ `is_sub`（`:995`）。**
  力度门只在基例（`nest.rs:905` `!base.divergence_confirmed` ⟹ `None`）。

---

## 本次改动（探针件，零生产行为变更）

1. `rust/src/theta_v0/classifier/nest.rs`：`typed_interval` 可见性 `private → pub(super)`（4 行注释 + 签名换行）。
   函数体、调用点、判定路径逐字节不动。
2. `rust/src/theta_v0/classifier/nest_index.rs`：`NestEndorsementInstrument` 追加 6 个 **只读** 字段
   （`rungs_by_level` / `rung_edges_total` / `rung_parent_cands` / `rung_edge_degenerate` /
   `indexed_pos_hist` / `indexed_pos_hist_rungs_ge1`）+ 新函数 `observe_rungs_908`（只写这 6 个字段，
   在 `cert` 移入索引**前**读完，不读/不改任何判定输入）。
3. `rust/src/theta_v0/backtest/fill.rs`：新增 `NEST_GATE_RUNG908` 打印行一行（纯增量；既有
   `NEST_GATE_STATS/CHAIN/T3/INDEX/FAIL/LEVEL` 六行 schema 逐字节不动）。
- **真编译**（不用 `cargo check`，重放缓存假绿在案）：`cargo build --release --lib` 通过；
  `cargo test --release --lib theta_v0::classifier::nest`（135 passed）、
  `theta_v0::backtest::admission`（1 passed）、`cargo test --release --test nest_isolation_guard`（1 passed）全绿。
- **未做的验证（明写）**：未跑 `scripts/check_armR_trades_digest.py` 逐位回归锁（需三窗
  `OPSEM_DUMP_DIR` 产物，本次未落 dump）⟹ **「零行为变更」是代码层论证 + 单测绿，不是逐位摘要证明。**

## 同批性自检（照实：对不上，因为窗口口径不同）

在案探针 [#846](…/846)/[#848](…/848)/[#851](…/851)/[#852](…/852) 用的是 **bar 计数窗**
（`#852`：1 200 000 bar = 2024-02-18→2026-05-31，占全量 4 613 599 的 26.0%）。
本票用的是 **m8 日期窗**（2023-01→2024-02）。**两者窗口无一相同** ⟹ `n_sig` 逐级对表**不适用**，
本次**没有做**这项对照。若 #817 需要与那四票同批，须另跑 bar 计数窗一臂（本票未跑）。

## 我认为最可能被推翻的地方（按可能性排序）

1. **「四个构造点生产不可达」这个判断。** N-2 的「唯一入口」被 N-6 推翻过一次，我这次是穷举 grep
   （`d_parent_interval*` / `d_child_interval` / `assemble_certificate*` 全 `src` 扫）+ 逐个消费者定性，
   但**没有用运行时探针反证**（例如在四个点插 panic/计数跑一遍生产回测确认零命中）。
   `opsem_dump.rs:224` 那条尤其值得复核：它是「非 bin 非 test」的，我判它不可达靠的是
   `strict_nest_sidecar_enabled()` 默认关 + 注释自述「不参与订单/候选/风控/账本」——**注释不是证明**。
2. **rungs≥1 只有 10 张 / 16 条边，「12.5% 多候选父」和「rungs≥3 零例」都可能在别的窗翻。**
   `wf8` 100% 为 0、`p3fold` 94.29%，窗间已经差 5.7 个百分点；13% 的史料占比不足以定 96%。
3. **挂法 A 在跨级下「未证也未测」这一条**。如果 #817 认定价格挂法就该跨级共用同一根价格序列
   （merged bars 高低），那子集单调性立刻闭合、价格包含恢复恒真，问一的结论就退回「与 N-1 同构、
   零额外严格度」。**这一步的胜负全在挂法怎么定，不在我的读数。**
4. **`rungs = judge_at().len() - 1` 是否真等于「rung 数」。** 我沿用了生产自己的算法
   （`nest_index.rs:321`，与 `stats.rungs_*` 同源），并用 `NEST_GATE_INDEX` 的三桶与本票
   `rungs_lk` 逐窗交叉核对一致（61/61、86=82+1+3、105=99+3+3）；但若 `judge_at` 的含义有别解，
   整张分布表要重读。
