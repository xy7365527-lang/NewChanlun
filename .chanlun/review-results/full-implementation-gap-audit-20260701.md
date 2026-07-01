# 缠论完整实装深度 gap 审计（Task #120，编排者核心质询"区间套等等有没有装"）

**日期**：2026-07-01
**审计对象**：econ_positive.rs 生成的信号台账（`/tmp/btc_663_ledger_sigma.csv`）所依赖的完整缠论策略实装链，逐概念对照 (A) `缠论知识库.md` 定义 / (B) theta_v0 Rust 实装 / (C) 回测实际调用链。
**方法**：3 个并行子审计（#121 parser 形态层 / #122 classifier 递归层 / #123 背驰+等价关系）+ 审计者亲自锁定区间套/递归塔两个锚点。全部读代码非记忆，行号在案，逐条 trust-but-verify。
**认识论标注**：**L0/L1 代码事实核验**（读代码确认实装状态，非 L2 实证有效性）。结论作用于"我们的 alpha 检验覆盖了完整缠论策略的哪个子集"这一元问题。
**基线**：在 #119（strategy-implementation-completeness-20260701.md）之上挖深——#119 确认"三类买卖点识别全装"，本审计挖"区间套/唯一分解/三类定律递归构成/完整力度背驰/等价关系"是否简化，尤其**装了 ≠ 回测用了**。

---

## 回测调用链锚点（所有"回测是否真调用"的答案从这条链推导）

```
econ_positive.rs::decompose_capturable_spread(data, config)
  → IncrementalClassifier::classify_at(i)              # backtest/incremental.rs:86
      → parser_incr.append(bars[i])                     # theta_v0/parser（新笔/线段/特征序列）
      → classify_with_tower_incremental(l0_i, .., cache) # 产 (cls_i, tower_i)
  → 逐 bar 遍历 cls_i.levels[].bsp                       # 识别层买卖点
  → assemble_gamma_with_tower(single, tower_i)          # interp.rs:308，拿 Candidate.dir
  → sigma_higher_at(tower_i, ..)                        # 端点价法近似上级方向
  → 退出配对（口径4：下一反向确认信号）
```

econ_positive 只消费 `Candidate.dir`（Long/Short/Flat）+ level + sigma_higher。`nest_confirmed` 是 Candidate 的旁字段，**econ 不读**。

---

## 直接回答编排者三问

### ① 区间套回测到底用没用？→ **没用（最大简化，假否证根源）**

- `classifier/nest.rs`（`NestCertificate` / `n_delta_certificate`，完整跨级方向化区间套 ⊆ 递归证书 N^δ）= **全代码库零生产调用**，只有 `#[test]` + `l3_fullwindow` 探针引用。
- `backtest/` 全部 15 个文件对 `chi_bool` / `NestCertificate` **零调用**（grep 实证）。
- 唯一触达区间套的生产路径：`interp.rs:252 nest_confirm → nest::chi_bool`，但 chain 只有 **1 级**（`lvl == level`），是 base case `N^δ_{ℓ↓e} = Conf^δ_e`（spec §6 分段函数 ℓ=e 分支）退化 = 单级 Conf^δ。注释（interp.rs:237-239）自认：完整 N^δ 跨级递归链（ℓ>e：Candidate∧⊆∧子证书）"需 `LeveledMove` 真嵌套塔，`Classification` 不导出塔"故未装。
- ⟹ **无任何"从大级别背驰段逐级收缩定位小级别转折"**（§11 区间套的定义本质：11.1 "从大级别向小级别逐级寻找背驰点"，11.3 "低级别背驰是本级别背驰的必要条件"的跨级耦合）。

**结论**：我们从没测过区间套定位的大转折 buy/sell 点。所有 alpha 否证覆盖的是 bsp 端点方向投影，不含区间套递归定位。

### ② 最大的简化点？→ 见 ③ 按假否证风险排序，区间套未调用是第一

### ③ 补齐优先级清单（假否证风险 × 补齐工作量）

| # | 简化点 | 状态 | 假否证风险 | 补齐工作量 |
|---|---|---|---|---|
| **P1** | **区间套递归定位**未进回测（classifier/nest.rs 零生产调用，interp 单级退化） | 未装（生产链） | **极高**：从没测过区间套大转折点，信号从未进样本 ⟹ 不可能被证伪 | 大（需 Classification 导出 LeveledMove 塔桥） |
| **P2** | **MACD 背驰**代替完整力度背驰（divergence.rs:228 Σ\|hist\| 面积） | 简化（§9.3 自认） | **高**：完整次级别力度累积可能有 alpha 被 MACD 平均掉；被稀释信号被 signal.rs:286 一票否决（return None）从不进样本 ⟹ 不可能被证伪 | 中（次级别序列递归塔已可取，mod.rs:249 upper_moves descend） |
| **P3** | **等价关系/新缠论第12节**全未装（比价K线/跨标的等价对/IR 不变量/四矩阵零实装） | 未装 | 中：单标的 BTC 回测定义上不覆盖新缠论维度，谈不上证伪或证实 | 大 |
| **P4** | **类型信息抹除**（#119）：台账丢 min_class + 买卖侧，无法按一/二/三类分层 | 简化 | 中：某具体类型可能有 alpha 被混合信号池稀释 | 小（interp.rs:214 已算 min_class，只需透传） |
| **P5** | **center_of 统一 c1**（signal.rs:436 `\|_m\| *c1`）：二类所有次级别走势配同一中枢 c1 | 简化 | 低：当前单中枢窗口口径无歧义（塔 Compose 每窗口只带1中枢），多中枢塔才失真 | 小（0.5-1天，需次级别走势↔中枢归属映射） |
| **P6** | **上级走势方向=端点价法**（sigma_higher，econ_positive.rs:74-79）非完整 `MoveOutcome::Trend(Direction)` 裁决 | 简化 | 低：σ_higher 条件化结论受此近似限 | 小 |
| **P7** | **出场=反向信号配对（口径4）**非缠论正规出场（背驰卖点/中枢破坏，closed_loop 有但回测不走） | 口径差异 | 中：出场时机口径不同可能翻转 spread_eaten | 中 |
| **P8** | **l_max=6 硬上界**（config.rs:74）：级别涌现被截断在 L6（BTC 300K 只涌现到 L5 未撞顶，但结构支持更高时会截） | 有限截断 | 低 | 极小（改 config） |

---

## 完整 / 无简化（不削弱结论，已逐条核验代码）

**parser 形态层（#121）**：
- **新笔**：stroke.rs，`new_stroke_min_gap` default 3 启用、旧笔禁用（第81课/《忽闻台风可休市》新笔定义），两极值 K 间隔用原始 source_index，不共用 K。**完整 L0**。
- **线段特征序列**：segment.rs + feature_seq.rs，特征序列取反向笔 + 包含处理 + 缺口判据 + 第一种（无缺口即断）/第二种（有缺口须第二特征序列分型，second_kind.rs）+ 三笔重叠起点（第77课）+ 方向一致性（第77课）。静态判据 **完整 L0**（bit-exact），第二特征序列动态确认 L1（second_kind.rs 头诚实标注：Origin canonical 无 spec，对 67课博文 + Python 交叉验证）。
- **古怪线段/顶高于底**：缠师解法 = 笔层根治（新笔已装，§5.6），古怪线段作为合法未终结 tail 态正确处理（NoFractal/SecondKindPending 留 tail 不强断，不产假段）。**唯一诚实边界**：`TopAboveBottom`（第78课）静态谓词未独立 enforce（segment.rs:14/279 仅注释锚 Origin.SegEndComplete），当前靠新笔 + 方向一致性隐式保证顶>底。与 settled/001-degenerate-segment.md 谱系一致（笔层根治）。

**classifier 递归层（#122，递归塔审计者独立复核）**：
- **递归无限向上**（§7.3）：mod.rs:217 `for level_idx in 0..=l_max`，**真涌现**——级别由中枢真派生，自然终止 = `units.len() < min_parts`(=3) break（走势分解定理二 ≥3 段的结构下界）。grep `MAX_LEVEL|truncat` 命中全是缓存 `Vec::truncate`，**无级别硬截断**。L0-L5 涌现是数据长度决定，`l_max=6` 是防护栏（BTC 未撞顶，见 P8）。**完整**。
- **唯一分解**（§8）：recursive_tower.rs:189 贪心左折叠（成立支+3/不成立支+1），确定性 = §8:330 "工程上常用同级别分解，唯一性更强便于机械化"的机械实现。结合律/多义性属人工多视角释义，工程不实装非缺口。**完整 L0**。
- **三类定律"次级别一类构成"**（§10.2 定律一）：descend.rs `second_type_via_sublevel_type1` + rmove_compose.rs `find_second_type_structure`——**真次级别下钻**（descend 取回真 subs 携坐标 LeveledMove，重跑破中枢几何 + MACD 背驰真算 sublevel_diverges），非本级别近似（rust 领先 Origin：力度真算非占位）。**唯一简化 = center_of 统一 c1（P5）**。

**背驰识别层（#123）**：
- 走势类型 τ 门控（§9.1 "没有趋势就没有背驰"）：divergence.rs:301 trend_class，完整 L0（0中枢→退化/1→盘整/≥2全同向→趋势）。
- A/B/C 段配对（§9.3）：divergence.rs:351 locate_trend_seg_a（A=相邻前一中枢离开段，跨中枢配对），完整 L0/L1。
- 趋势/盘整背驰区分完整，盘整背驰正确地不产第一类买卖点（§10.1）。
- 第一类识别：signal.rs:542 τ≠Trend 直接不产，力度真消费（一票否决 signal.rs:286）。

三类买卖点**识别**三类全装（#119），力度轴真消费。

---

## 要让"不拿任何简化测试"成立，必须补齐（编排者原话）

**最小充分集 = P1（区间套）+ P2（完整力度背驰）+ P4（类型透传）。**

- P1/P2 的信号**从未进过样本**（区间套定位点从不生成；MACD 判无背驰的一类点被 return None 丢弃）⟹ 结构上不可能被现有回测证伪。
- P4 使类型分层 alpha 无法归因（混合池稀释）。
- P3（新缠论第12节）是旧缠论完整性之外的**另一维度**——单标的回测定义上不覆盖。
- P5-P8 是口径精度，不改"检验对象是不是真缠论"的定性判断。

**假否证的精确形式**（formalization-validity-domain 231号）：现有回测对"MACD 背驰门 + bsp 端点方向投影 + 口径4 出场 + 单标的 + 无区间套定位"这个**策略实例**是可信的 L2 检验；但对"完整缠论策略（含区间套递归定位 + 完整力度背驰 + 类型分层 + 跨标的等价）"，P1/P2/P3 的信号连样本都没有，**有效域 ⊊ 定义域**。不能从子集否证外推到"完整缠论无 alpha"。

---

## 结果包六要素

1. **结论**：检验对象 = 识别层三类 bsp 方向投影 + 口径4 出场，不含区间套递归定位（P1，最大简化，生产链零调用）/ 完整力度背驰（P2，MACD 代理）/ 跨标的等价（P3，未装）/ 类型分层（P4，台账抹除）。parser 形态层 + 递归塔 + 唯一分解 + 三类识别 + 二类次级别下钻 = 完整无简化。
2. **定义依据**：知识库 §11（区间套逐级定位大转折）、§9.3（MACD 自认工程简化）、§12（新缠论等价关系）、§10.2（二类定律一）、§7.3（无限递归）、§8（唯一分解）；代码 econ_positive.rs:154-209（调用链）、classifier/nest.rs（零生产调用）、interp.rs:240-252（单级 base-case 退化）、divergence.rs:228/244、signal.rs:286/436、recursive_tower.rs:189/217、stroke.rs（新笔 min_gap=3）、config.rs:74（l_max=6）。
3. **边界条件**：若 Classification 导出 LeveledMove 塔桥 → 区间套可跨级递归（P1 翻转，从"未装"到"可测"）；若透传 min_class（interp.rs:214 已算）→ 类型分层可检验（P4）；当前单中枢窗口下 c1 统一无歧义，多中枢塔时 P5 失真；若 econ 改走 closed_loop 正规出场（P7）→ 出场口径从口径4 变背驰/中枢破坏，可能翻转部分 spread_eaten。
4. **下游推论**：所有基于 `/tmp/btc_663_ledger_sigma.csv` 的 alpha 结论（奇偶交替证伪 / σ_higher 条件化 / (ℓ,q) 三判据 / 夏普证伪）须附限定词"**识别层三类混合方向投影，MACD 背驰门，口径4 出场，单标的 BTC，无区间套定位**"。**不能从此子集否证外推到"完整缠论策略无 alpha"。**
5. **谱系引用**：[[project_interval_nesting_not_called_in_backtest]]（本审计新记忆）；[[project_oddeven_mu_identity]]（奇偶交替=beta 漂移伪结构，L2 独立否证，本审计不推翻——它是反事实置换的独立结果）；#119 `strategy-implementation-completeness-20260701.md`；`.chanlun/genealogy/settled/001-degenerate-segment.md`（古怪线段笔层根治，与实装一致）；`002-source-incompleteness.md`。
6. **影响声明**：本审计不改任何生产代码，只产 gap 矩阵 + 补齐优先级。影响 = 给所有现有 alpha 否证结论的有效域声明加"识别层子集"限定。P1/P2/P4 是新工位（不在本审计范围）。
