# tranche 递归建仓重验（C 段修复后）

> 任务：在 C 段边界修复（commit 5739ec4f A+B2 / 0359f0dc Python oracle 同步 /
> 36c6a033 三标的重跑）之后，重验旧结论"tranche 空定义域是定理性的"是否翻转。
> 主标的：OKLO 447,739 bar（1min, databento）。
> 方法：纯 Rust 闭环（`rust/src/c_segment_verify.rs` 新增 side-split 捕获测试，
> 无 Python→PyO3→bash）。认识论等级 **L2**（单标的真实数据终态对照）。

---

## 0. 结论（TL;DR）

**旧定理被证伪，tranche 定义域从空集变非空（1 层）。**

- 旧结论（`_rev_tranche_path2_section.md`）：buy1 ladder 集合恒为 {2,3} 是
  buy1≤3 的定理 → entry≤3 → tranche 区间 (2, entry−1]=(2,2]=∅。
- 重验（OKLO 447K，C 段修复后）：**buy1_ladders = {2, 3, 4}**，
  entry 上限 **3 → 4**，tranche 区间 **(2, 3] 非空 = 1 层**。
- 直接证据（side-split，旧捕获只有 (kind,confirmed) 折叠了 side）：
  ladder4 **type1_buy_confirmed = 5**（旧恒 0）。

产物：`analysis/data_cache/tranche_recheck_side_split.json`。

---

## 1. 旧结论的精确表述（被重验的对象）

旧诊断（`_rev_tranche_path2_section.md` / `rev_tranche_path2_coarse.py`）的"空定
义域是定理性的"精确含义：

> tranche 目标区间 = `(rev home k, entry_ladder−1]`，
> 其中 `entry_ladder` = 进场 bar 上**最高 confirmed type1 buy（buy1）的 ladder**
> （`runner.rs` FLAT→ARMED 升序扫描 `sig.buy1.get(k)` 取 hi）。
> 三标的 × 三周期（1/5/30min）的 buy1 ladder 集合**恒为 {2,3}**——
> recursive 层（ladder≥4）有 bsp/div 事件，但其中**无一以 confirmed type1 buy
> 形态进入 buy1 掩码**。entry≤3 → cap=entry−1≤2 → 区间 (2,2]=∅。

关键：旧结论不是"没观测到"，而是把它升级为定理——"tranche 空性是 buy1≤3 的
定理，与 base 周期无关"，且明确给出**翻转条件**：

> "某 TF/某历史段出现 ladder≥4 的 confirmed type1 buy 进入 buy1 掩码（届时
> entry≥5 才有 ≥2 级 tranche 空间，entry=4 给 1 级）。"

C 段修复正是命中这个翻转条件的根因：修复前递归层 C 段恒空 → 趋势背驰永不触发
→ 高级别 type1 = 0。

---

## 2. 重验：buy1 ladder 集合 + entry 上限 + tranche 区间

捕获方法：`c_segment_verify.rs::c_segment_oklo_447k_tranche_side_split`
（新增测试，复用既有私有 helper `level_bsps`/`bi_zhongshu_bsps`，与生产信号层
`organic_signals.compute_organic_signals` 同链——buy1[ladder] 由
`buysellpoints_from_level` 产出的 **confirmed type1 buy** BSP 置位）。
纯 Rust，无 Python/PyO3。

### OKLO 447K 终态（side-split，confirmed）

| ladder | type1 **buy** | type1 sell | type2 buy | 进 buy1 掩码？ |
|--------|--------------|-----------|-----------|---------------|
| 2（笔中枢） | 187 | 187 | 182 | ✅ |
| 3（走势级） | 31 | 29 | 29 | ✅ |
| **4（递归 L2）** | **5** | 3 | 5 | ✅（旧恒 0）|
| 5（递归 L3） | **0** | 1 | 0 | ❌ |

- `buy1_ladders = [2, 3, 4]`，`entry_cap = max = 4`。
- tranche 区间 `(home=2, entry_cap−1=3]` → **{3}，1 层，非空**。
- ladder5 type1_buy_confirmed = 0（只有 1 个 sell）→ entry 上限停在 4 不到 5
  → 恰好 1 层（与旧报告翻转条件预言"entry=4 给 1 级"逐字吻合）。

旧报告同一捕获量（`buy1_ladders`）此前在 1/5/30min 三点恒为 {2,3}；本次 1min
C 段修复后变 {2,3,4}——**把"1/5/30min 三点不变事实"证伪于 1min 修复态。**

---

## 3. tranche 递归建仓的最小可行验证（定义域非空 + ladder 结构）

任务只要求确认**定义域非空 + ladder 结构**，不要求完整回测。结论与边界：

### 3.1 定义域非空（已证）

`t4b_addons`（`level_operating_unit.rs:660-735`）的 tranche 级别上限
`cap = entry_ladder.saturating_sub(1)`，加码目标层取自
`(rev.max_level()+1 ..= cap)`（home=rev 腿 ladder=2 起）。
entry_cap=4 → cap=3 → 目标层域 = {3} 非空。**旧 cap≤2、域 (2,2]=∅ 的算术坍缩
不再成立。** voices 域 `[floor=2, entry=4)` = {2,3}，tranche 占据 {3}
（master 占 entry=4，不越音域）。

### 3.2 ladder 结构（已证）

buy1 在 ladder4 真实存在 confirmed type1 buy（5 个），即"次级别买点进位到
递归 L2"在结构上发生。区间套（27 课）的级别坐标完整：master 锚 ladder4，
tranche 锚 ladder3，home rev 锚 ladder2——三级齐备。

### 3.3 边界（什么条件下"非空"不等于"实际触发"）

**诚实声明**：本验证证明的是**结构定义域非空**（必要条件 + 旧定理证伪），
不等于某次回测里 `n_rev_tranche_adds > 0`。实际触发还需在**同一进场 bar**：
(a) `entry_ladder=4`（该 bar buy1 最高位=4，而非全程聚合 5 个分散在不同 bar）；
(b) 一条 rev 腿以 home<4 开腿；(c) level-3 层出现反向 run（D3 方向行
`dir_row[3]==Down` 且 `run_anchor[3]≥open_bar−margin`）。
报告 `engine_c_segment_fix.md §4.2` 已记录 "v2 Rust 交易层 C4 tranche 递归建仓
仍待 M2 信号层 D3 行"——本次解锁的是其**前提**（递归层 confirmed type1 非空），
触发率的全量回测是下一步（不在本任务范围）。

---

## 4. 结果包六要素

1. **结论**：旧"tranche 空定义域是 buy1≤3 定理"被证伪。OKLO 447K C 段修复后
   buy1_ladders={2,3,4}，entry 上限 3→4，tranche 区间 (2,3] 从空集变 1 层
   非空。ladder4 confirmed type1 **buy**=5（旧恒 0），ladder5 buy=0（entry 停 4）。
2. **定义依据**：tranche 区间 = `(home, entry−1]`，entry = 进场 bar 最高
   confirmed type1 buy ladder（`rust/src/trading/runner.rs:337-345` 升序扫
   `sig.buy1`；`level_operating_unit.rs:679` `cap=entry_ladder−1`，
   tranche 目标层 `level_operating_unit.rs:708-714`）。buy1 置位源 =
   `buysellpoints_from_level` 的 confirmed type1 buy（第17课趋势背驰买点）；
   递归层 type1 由 C 段修复（B2 第24课 C 段终于转折点）解锁。捕获代码
   `rust/src/c_segment_verify.rs::c_segment_oklo_447k_tranche_side_split`。
3. **边界条件**：①结论翻转回"空"的条件 = 任一标的/历史段上 ladder4 不再出现
   confirmed type1 buy（buy1 退回 {2,3}）；本次仅 OKLO 1min 单点，未验证 QQQ/BRN
   修复态 buy1 是否也达 ladder4（旧三标的均 {2,3}，修复后未重测该量）。
   ②"非空"≠"触发"：定义域非空是必要条件，实际 `n_rev_tranche_adds>0` 还需进场
   bar 上 entry=4 + rev 腿 home<4 + level-3 反向 run D3 行三者同 bar 命中（§3.3）。
   ③entry 停在 4（非 5）依赖 ladder5 type1_buy=0；若更长历史/其他标的 ladder5
   出现 confirmed type1 buy，cap→4，区间 (2,4]=2 层。④数字依赖 0.9 type1 确认
   阈值与振幅力度路径（enable_macd=False）。
4. **下游推论**：①tranche 递归建仓首次拥有非空结构定义域（1 层 @level3）→
   v2 交易层 C4 路径不再恒空转（`engine_c_segment_fix.md §4.2` 预言兑现）；
   ②所有引用"tranche 恒空/V1r≡V1f 因 tranche 从不激活"的在册结论需在修复态重评
   （`_rev_tranche_path2_section.md` 的 tranche 判决节、`fugue_version_i` REV 矩阵）；
   ③下一步明确：全量回测测 `n_rev_tranche_adds`（本任务未做，超范围）。
5. **谱系引用**：532（seg-end-trigger 轴分裂）= C 段修复直接母结算，本重验是其
   下游兑现（dag.yaml/settled/532 已随 C 段修复 commit 修改）。005b（对象否定对象）/
   090（同概念两处定义统一）见 `engine_c_segment_fix.md §5`。**不确定**是否已有
   编号谱系专记"tranche 定义域重开"——经查 `.chanlun/genealogy/` 未见 tranche 专属
   结算条目；建议谱系工位以 532 子结算补录本重验。
6. **影响声明**：①新增报告 `analysis/tranche_recheck_post_csegfix.md`（本文件）；
   ②新增数据 `analysis/data_cache/tranche_recheck_side_split.json`；
   ③在测试文件 `rust/src/c_segment_verify.rs`（`#![cfg(test)]`，非引擎源码）
   追加 1 个 `#[ignore]` 测试函数 `c_segment_oklo_447k_tranche_side_split`，
   复用既有私有 helper，**未改任何引擎逻辑**（rust/src 引擎模块、src/newchan 零改动）；
   cargo test 通过（76 测试，新增 1 个 ignored 显式跑通）。未 commit。
