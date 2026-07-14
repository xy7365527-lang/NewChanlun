# 验收判定：level 塔递归单调完备性（acc-classification-monotone-complete）

goal g-20260701T200047Z-8f4f50e7 / acceptance 工位。判据（可证伪）：
**level 塔递归无中间级空洞，稀疏级别满足 `level_k有信号 ⟹ level_{k-1}结构非空`。**

## 判定：**PASS（H1 坐实，H2 排除）**

判据针对的是 **结构（tower段数）非空**，不是信号非空。全历史实测 + L0 构造不变量共同判定通过。

---

## 1. 结论

- **结构单调完备**：全历史（4.6M bars, 2017-08-17→2026-05-31）tower段数 = `40028/12077/3565/992/244/61/6`（level0→6），每个中间级非空。判据 `level_k有信号 ⟹ level_{k-1}结构非空` **成立**。
- **"level5=1 但 level1-4=0" 是 sig_post（门后信号）分布，不是结构分布**：sig_post = `7744/0/0/0/0/1/0`。level5 的唯一信号骑在 level5 结构（61 段）之上，而 level5 结构由构造必然蕴含 level1-4 结构（12077/3565/992/244，全非空）——**自洽**。
- **判别实验（team-lead H1/H2 判据）结果 = H1**：level1 bsp_pre 从 300K 窗的 77 → 全历史的 1059，随窗口缩放，**非全历史归零**。team-lead 的 H2 判据是"300K≫0 但全历史全零"；实测全历史 level1 bsp_pre=1059 ≠ 0 ⟹ **H2 bug 排除，H1（N^δ 门滤空）坐实**。
- **frontier 发散 bug 已排除**：这是前期 review（level_hole_window_dependence）"frontier发散 bug 未排除"的悬置项。frontier 修复（incremental_tower `resume_from` 取代 `consumed`，task #5，已 completed）落地后，全历史 tower 结构单调完整、真封断言全通过（exit 0）——增量路径不再把 frontier 中枢当 sealed prefix 漏算。

## 2. 定义依据

- **判据条款**："level 塔递归无中间级空洞，稀疏级别满足 level_k有信号⟹level_{k-1}结构非空"。判据主体是**结构**（level_{k-1}结构非空），信号稀疏不在判据约束内。
- **L0 结构不变量**（`rust/src/theta_v0/classifier/mod.rs:218-271`）：塔自底向上构建，`levels` 是**连续前缀**——`level_idx in 0..=l_max` 每级：`units = project_to_units(&upper_moves)`（level_{k+1} 输入单元 = level_k 已 compose 的 upper_moves 投影），`if units.is_empty() { break; }`（mod.rs:269）+ `units.len() < min_parts { break; }`（mod.rs:220）。故 `levels[k]` 存在 ⟹ 每个前级 iteration 都产出非空 units ⟹ `levels[0..k]` 全存在且结构非空。**level_k 结构非空 ⟹ level_{k-1} 结构非空 是构造保证**（不是经验巧合）。
- **信号定义**：signal = bsp 提取 × N^δ 门 × 背驰。bsp 逐级独立提取（mod.rs 分级），门 = `[J_{ℓ-1}⊆J_ℓ]` 嵌套链准入。中间级有结构、有 bsp（门前），但门后信号可为 0——这是合法的稀疏，不是空洞。

## 3. 边界条件（结论翻转条件）

- **翻转为 REJECT（H2/结构 bug）当**：全历史某中间级 `tower段数 = 0` 而更高级 `tower段数 > 0`（结构空洞）。实测全部非空，未翻转。
- **翻转当**：真封断言失败——`sig_post_sum < n_signals` 或 `decomps 逐级和 ≠ n_signals`（增量/生产路径不一致）。实测 exit 0，断言通过（sig_post_sum=7745 ≥ n_signals；decomps: level0=7741 + level5=1，n_unpaired=3）。
- **翻转为 H2 当**：判别实验 level1 bsp_pre 在 300K≫0 但全历史归零（增量路径全尺度丢信号）。实测 77→1059，未翻转。
- **不翻转判据、但需 codex 确认的悬置**：H1 断言"N^δ 门**定义正确地**严格滤中间级"——门的**有效域=定义域**尚未异质确认，可能是过滤过严（over-filter）掩盖为 H1。见第 5 节 codex 任务。

## 4. 下游推论

- **对 acc-alpha / W-VERIFY 回测（task #3）**：可交易信号有效级别 ≈ level0（7744/7745 门后信号 + 唯一 level5）。中间级（level1-4）在 N^δ 门下无独立可交易信号——回测 alpha 的级别覆盖不应期待中间级贡献。
- **对区间套（N^δ 门）有效域**：P1 深度分布 depth0=96.5%（退化 base-case，单 bit confirm）、depth≥1=3.5%（真跨级）。门在有效域内**部分触达**——多数信号退化为单级确认，与"区间套已接入但 95% 退化 base-case"（interval_nesting_not_called_in_backtest）一致，非本判据翻转项。
- **对 level 塔构造**：塔结构完备性是可信的地基——上层递归（走势/背驰/买卖点）可安全假设中间级结构存在。

## 5. 谱系引用

- **level_hole_window_dependence**（前期 review）：H1 门滤空被否证转确证、窗口依赖是机制、"frontier发散 bug 未排除"——本判定**排除**该悬置（frontier 修复落地 + 全历史真封通过）。
- **frontier_resume_bt_too_late**（记忆）：`consumed` 停太晚把 frontier 中枢当 sealed——即 task #5 修复的根因，本判定消费其产出。
- **interval_nesting_not_called_in_backtest**：区间套 95% 退化 base-case——与本判定 P1 深度分布一致。
- **formalization-validity-domain 规则**：H1 断言"门严格滤中间级"是关于门**有效域=定义域**的声明，未经异质验证前不作为确证——故派 codex 确认（下）。

## 6. 影响声明

- 本判定**不改任何代码**——消费 task #5（frontier 修复，已 completed）+ `acc_classification_level_hole_dx` harness（econ_positive.rs:2094）全历史产出。
- harness 自动覆盖 `acc-classification-level-hole-20260701.md`（econ_positive.rs:2364）为全历史数据；本判定另存独立文件避免被 harness clobber。
- 派生 codex 异质确认子任务（agent_type=codex-challenger）：确认 H1 门滤空是**定义正确的严格性**而非过滤过严 bug。

---

**认识论等级**：L2（真实 BTC 全历史逐信号分级别计数 + L0 构造不变量）。判据 = 结构单调完备，PASS。
