# 工位 4g 结果（L2，CL OOS）：TreeKey-miss ceiling 前提被否证 + generation 快路真修

## 结论
任务前提（"extract 侧 TreeKey-miss ceiling 全量重建是 exp≈2.0 真因"）被 L2 数据**否证**。真因是
两个独立 O(n²) 源：(a) 每 bar `TreeKey::of` 全量重算（已修，generation 快路）；(b) candidate(gamma)
段每 bar 全量重建+排序（结构性 ceiling，bsp 中间插入，独立工位）。修了 (a)，bit-exact + sound
（codex 审 + debug 守卫抓漏修补），但全引擎 exp 仍 ≈2.1（受 (b) 同阶支配）。

## 否证证据（diag_treekey_miss_cost_16k，L2）
```
miss=26/16000（0.16%）
miss 累积重建成本=0.0001s；全 bar 若无缓存总重建=0.1272s
miss 成本占全量重建=0.1%（其余 99.9% 被缓存命中省下）
miss tree.len：avg=41 max=130（小且不随 n 增长——miss 发生在新最高级别刚涌现时该级 tree 还浅）
```
任务假设"miss 在大 n 端 tree 大 ⟹ 累积 O(Σtree_at_miss) 超线性"在数据上不成立。

## 真因隔离（diag_treekey_of_scaling_16k + diag_candidate_segment_scaling_16k，L2）
```
TreeKey::of per-bar 累积：key_exp=2.16，fp_len 20→120（随 confirmed tree 单调增）
候选段隔离（gen 快路命中后剩余）：cand_s 0.0006→0.0502s，cand_exp=2.14，cand_last 5→47
```
- `coverage_elements_and_gamma_with_tower_cached` **每 bar 无条件 `TreeKey::of(tower)`**（为判缓存命中），
  递归发射全部 tree 节点 = O(tree)/bar = O(n²)。这是源 (a)。
- candidate 段每 bar 按 `classification.levels[*].bsp` 全量重建（每个 bsp 一候选 + role 计算），
  bsp∝confirmed∝n ⟹ Σ_bar O(cand_at_bar) = O(n²)。这是源 (b)。bsp 按 source_index 中间插入+排序
  （mod.rs:923 `sort_by_key`），**非纯尾部 append**，无法像 tree 那样简单前缀缓存。

## 实装（源 (a) 真修：generation 快路）
- `TowerCache` 加 `generation: u64` + `generation()` 访问器。维护点（codex Q3 soundness 全覆盖）：
  cascade_reset ∨ 任一级 upper_moves extend 非空 tail ∨ clear() ∨ **L0-is-root**（见下）。
- `IncrementalClassifier::tower_generation()` 暴露。runner 闭包 F 返回三元组 `(cls, tower, gen)`，
  调 `coverage_elements_and_gamma_with_tower_cached_gen(.., Some(gen))`。
- `TreeCache` 加 `gen: Option<u64>` + 代次快路：`tower_gen==c.gen && c.valid` ⟹ 跳过 `TreeKey::of`
  直接 Rc::clone（O(1)）；否则 TreeKey fallback（旧路径，bit-exact）。

## soundness 漏洞修补（debug 守卫抓到——no-workaround）
`gen_fastpath_bit_exact_debug` bar 345 抓到代次假命中：`gen=111 prev_gen=111` 但 cached_len=0 vs
expect_len=3，tower_levels=1（L0-only）。根因 = codex Q3 漏洞之三：**最高非空级是 L0 时**，extract
读 `moves_tower_l0`（每 bar 从 `l0.segments` 重建，非缓存 Rc），其增长/同 len 古怪线段重划**不经**
cascade/did_extend（那两者只覆盖各级 upper_moves）。修补：L0-is-root 阶段**强制每 bar +generation**
（走 TreeKey fallback）——此阶段 tree 极小（早期 bar）不影响大 n 标度；一旦 L1+ 出现（extract 读 L1），
L0 任何变化必经 cascade 传播至 L1（codex Q1 确认）。

## bit-exact 裁判（全过）
- `gen_fastpath_bit_exact_debug`（4000 bar，debug + release 双跑）：gen 快路逐 bar == nocache extract_elements，
  含真实 CL frontier 古怪线段重划。gen 命中 2787/4000（L0-root 阶段强制 miss 降命中率，安全）。
- `bit_exact_per_bar` / `bit_exact_per_bar_cached_vs_nocache` / `bit_exact_synthetic` /
  `incremental_tower_*`：全过（generation 改动不破已有 bit-exact）。

## L2 实测（generation 快路 vs 修前）
```
            修前 full extract_s@16K   gen 快路 extract_s@16K
extract_s         0.074                    0.0499（降 ~33%，TreeKey::of O(n²) 消除）
x_exp             2.14                     2.10（候选段 O(n²) 同阶支配，exp 不降——预期）
gen_hit%          —                        92.2%@16K（与 TreeKey-miss 26/16K + L0-root 早期强制 miss 一致）
```

## codex 异质审 verdict 验证（议题一 S(N) vs R(N)，diag_candidate_s_vs_r_16k，L2）
codex（gpt-5.5）verdict："O(1)摊还可达，当前 ≈2.0 是表示层下界非数学下界"。codex 提的三个 tree
真凶候选（miss 大 n 端密集 / build_*_index 非 miss 隐式扫全树 / root-relative key 改名 Θ(n)）经我
诊断**全否**：miss 成本 0.1%、非 miss 路径 tree+index 全 Rc::clone O(1)、ElementId 非 root-relative。
真凶是 codex 框架未预设的 candidate(gamma) 段。套用 codex S/R 框架到 candidate：
```
S(N)=Σ每 bar 候选总数=353234（全量重建工作量）
R(N)=Σ每 bar 相对上 bar 新增/变化候选=49（真增量工作量，按 (level,source_index,bits) 身份 diff）
R/S=0.0001
```
**判定：R<<S ⟹ 表示层可修（candidate 前缀复用 O(R)≈O(49)），不是定义冲突。** 候选段每 bar 全量
重建 22 候选/bar 是纯冗余——前缀跨 bar 几乎全同，仅新确认买卖点 + 偶尔 frontier 重判产生 49 次真变化。
candidate 段 O(n) 增量化（codex 6 步范式：身份脱离全量重建 + 前缀冻结 + 尾部 dirty suffix）可达 O(n)。

## 认识论
- L1：generation 快路逐 bar == nocache（bit-exact 裁判过 + debug 守卫抓漏修补）。
- L2：TreeKey::of O(n²) 消除（确认性，CL 单标的，extract_s 降 33%）；全引擎 exp≈1.0 **未达**
  （否定性：源 (b) candidate 段 O(n²) 同阶支配，generation 只消源 (a)）——这是诚实有效域：
  generation 修复的有效域是"TreeKey::of 那部分 O(n²)"，不是"全引擎 exp≈1.0"。

## 工位边界判定（剩余路径 = 独立工位）
源 (b) candidate(gamma) 段 O(n²) 真修 = **独立工位**（任务标题已自动更新为 "gamma输入O(n)增量化"）：
- 需 candidate-prefix 增量缓存，处理 bsp 中间插入 + 每级 source_index 排序（非纯尾部 append）。
- high bit-exact risk（候选 role 依赖 tree hostOf + candidate-overlay 兄弟链），独立大重构。
- @16K cand_s=0.05s 绝对值小，但 BTC 1.31M bar：(1.31M/16K)²×0.05≈336s 会爆——必须修，下一工位。
