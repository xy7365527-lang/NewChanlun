# 缠论完整策略实装可判定判据清单（2026-07-01）

## 元数据

- **任务**：Lead 委托 codex diagnose——"什么才是完整的缠论策略实装"
- **negation_source**: 同质代理质询（Claude Sonnet 4.6），**非异质**
- **codex CLI 状态**：二进制丢失（`/vendor/aarch64-apple-darwin/codex/` 目录为空，exit=-9 SIGKILL）
- **降级依据**：memory `project_heterosource_openai_quota_exhausted.md` 降级策略——不冒充异质否定，标注同质，写 pending 等待异质源恢复
- **认识论等级**：L0/L1（代码事实读取，trust-but-verify 实证，非 L2 实盘有效性）
- **trust-but-verify 范围**：读遍 gap-audit / impl-roadmap / C1-cand-predicate / gap3-pdf-plan / econ_positive.rs / nest.rs / interp.rs / divergence.rs / sell.rs + 回测调用链全路径

---

## 一、完整实装可判定判据清单（三贯通：定义→实装→回测调用）

"完整"的可判定条件 = 以下每条必须**三者全部贯通**：知识库有可编码定义 / Rust 有实装 / 回测实际调用链消费。任意一环缺失 = 不完整。

| # | 机制 | 知识库定义 | Rust 实装 | 回测调用链消费 | 完整性 |
|---|------|----------|----------|-------------|------|
| M1 | 新笔（最小间隔/极值K/不共K） | §2-§5，第81课 | stroke.rs min_gap=3，source_index 不共K | IncrementalClassifier.append(bars[i]) | **完整 L0** |
| M2 | 线段特征序列（第一/第二种 + 包含处理 + 缺口） | §5，第67/71/77/78课 | segment.rs + feature_seq.rs + second_kind.rs | 同 M1 路径 | **完整 L0/L1** |
| M3 | 中枢（三段重叠区，ZS起止） | §6 | recursive_tower.rs ZoneState | classify_at(i)→塔 | **完整 L0** |
| M4 | 走势唯一分解（贪心左折叠） | §8 | recursive_tower.rs:189 | classify_at → 塔 | **完整 L0** |
| M5 | 级别递归无限涌现 | §7.3 | mod.rs:217 0..=l_max 真涌现 | classify_at 每 bar | **完整 L0** |
| M6 | 走势类型τ门控（趋势/盘整/退化三态） | §9.1 | divergence.rs:301 TrendClass | signal 层消费 | **完整 L0** |
| M7 | A/B/C 段配对（跨中枢定位） | §9.3 | divergence.rs:351 locate_trend_seg_a | signal 层消费 | **完整 L0/L1** |
| M8 | 趋势/盘整背驰区分（盘整背驰不产一类） | §9.1/§10.1 | divergence.rs TrendClass 分支 | signal.rs:542 τ≠Trend 直接不产 | **完整 L0** |
| M9 | MACD 背驰判据（|hist| 面积 C<A） | §9.3 工程简化 | divergence.rs:228 segment_macd_area+is_divergence | signal.rs:286 一票否决门 | **完整（含工程选择的边界，见 P2 分析）** |
| M10 | 三类买卖点识别（全三类） | §10 | signal.rs 全三类路径 | assemble_gamma_with_tower → econ 消费 | **完整 L0** |
| M11 | 二类定律一次级别下钻（真次级别 MACD） | §10.2 定律一 | descend.rs sub_level_type1，真 subs 携坐标 LeveledMove | classify 路径 | **完整 L0/L1** |
| **P1** | **区间套递归定位（跨级逐收缩）** | §11（11.1大转折/11.3跨级必要条件） | nest.rs NestCertificate::n_delta **已实装** | **零调用（接线缺口，最大简化）** |
| **P2** | **力度背驰完整版（次级走势累积 Σ Power）** | §9.3 自认工程简化，无超越 MACD 的可编码权威定义 | divergence.rs MACD 面积 = §9.3 自认简化版 | 已调（但一票否决=选择偏差，见下） |
| P3 | 等价关系/新缠论§12 | §12 | 零实装 | 零调用（spec 缺口，另一维度） |
| P4 | 类型信息透传（bsp_class 一/二/三类+买卖侧） | 三类买卖点完全分类的附属 | interp.rs:214 已算 min_class | econ signals tuple 已含 bsp_class（W4 已装） |
| P7 | 正规出场（背驰卖/中枢破坏） | §10 出场规则 | sell.rs CloseRoot/ReduceCore:50 已实装 | **接线缺口：econ 走口径4（反向信号配对），sell.rs 不走** |

---

## 二、P1/P2 缺口性质精确判定

### P1 区间套——接线缺口，非 spec 缺口

trust-but-verify 代码核实：

- `classifier/nest.rs` 中 `NestCertificate::n_delta()` 完整实装（NestLevel 结构、chi_bool 跨级递归、is_sub 区间包含、select_best Sel_Θ 选择器）
- `interp.rs:240-252 nest_confirm()` 构造的 chain 只有**单元素**：`[NestLevel{lvl: level, ...}]`，`nest::chi_bool(level, &chain)` 命中 spec §6 分段函数 `ℓ=e` 分支（末级 Conf^δ），**不是多级递归**
- `interp.rs:237-239` 注释自认：完整 N^δ 跨级递归链需 LeveledMove 真嵌套塔，Classification 不导出塔，故未装
- `backtest/` 对 `NestCertificate` / `chi_bool`（多级版）/ `nest_confirmed` 字段 **零调用**（grep 证实）

**结论：接线缺口（实装完整，桥未接）。** 信号从未因区间套条件过滤过，现有 alpha 否证对区间套定位的大转折点无检验效力。

### P2 力度背驰——§9.3 已结算工程选择，但含选择偏差风险

- `divergence.rs:228` 用 MACD `|hist|` 面积 = §9.3 明确自认的"实践中用 MACD"工程简化
- 知识库对"完整力度背驰"无超越 MACD 面积的可编码权威定义（§9.3 是已结算的工程选择，非补丁）
- **但**：`signal.rs:286` 背驰门一票否决（`return None`）= 结构性选择偏差：MACD 判无背驰的候选从不进样本，**在样本外存在的可能**永不可检验

**结论：P2 是工程简化选择（§9.3 认可的），不是新 spec 缺口。但一票否决门 = 选择偏差，不满足背驰的信号永不进样本 = 有效域限定。**

---

## 三、力度背驰完整性裁定

**裁定：MACD 面积代理 = §9.3 授权的最完整可编码判据，无更权威替代。**

- §9.3 明确写"实践中用 MACD 指标的快速线和慢速线的面积"——这不是缺省工程简化，是缠师认可的操作化
- "次级别走势力度累积 Σ Power"——知识库中无独立于 MACD 面积的可编码"力度"定义
- 因此 P2 不是"完整性"项，而是**选择偏差项**：一票否决门是关键问题，而非换算法

**一票否决门 = 选择偏差的精确形式**：
- 未过 MACD 背驰门的候选 → `signal.rs:286 return None` → 从不进 signals
- 这些候选的实际表现（是否有 alpha）无法从现有回测得知
- 这不是"完整力度背驰"的缺口，而是"背驰筛选导致样本缩小"的有效域限定

---

## 四、最小充分集——使"缠论完整策略无 alpha"否证有效的必要条件

**最小充分集 = P1 + P4 + P7**（严格递减优先级）

| 项 | 为何进充分集 | 缺此项的后果 |
|---|---|---|
| **P1 区间套** | 信号集完全不含区间套定位的大转折点，有效域对"完整缠论"的关键声明根本无覆盖 | 当前所有 alpha 否证的有效域仍限定在"无区间套定位"子集 |
| **P4 类型透传** | 一/二/三类混合池可能稀释特定类型 alpha，无法归因 | 目前 W4 工位声明已装（interp signals tuple 含 bsp_class），需确认 CSV 导出已含此字段 |
| **P7 正规出场** | 口径4（反向信号配对）vs 正规出场（背驰卖/中枢破坏）影响持仓周期和 spread_eaten 方向 | 出场口径不同可能翻转部分 spread_eaten 结论 |

**可后置的精度项**（不改"是否测的是完整缠论"这一定性判断）：
- P2（MACD 面积 = §9.3 认可，精度项而非完整性项）
- P5（c1 统一，单中枢窗口无歧义）
- P6（sigma_higher 端点价近似）
- P8（l_max=6，BTC 未撞顶，无害）

**P3（新缠论§12 等价关系）**：单标的 BTC 回测范围内定义上不覆盖，超出旧缠论完整性问题。

---

## 五、结果包六要素

1. **结论**：当前回测对象 = "识别层三类 bsp 方向投影 + MACD 背驰门 + 口径4 出场 + 单级 Conf 区间套 + 无区间套跨级递归定位"。完整缠论策略的可判定判据三贯通矩阵见第一节；M1-M11 完整，P1/P7 是接线缺口，P4 已部分装。最小充分集 = P1+P4+P7。

2. **定义依据**：
   - §11.1 "从大级别向小级别逐级寻找背驰点" + §11.3 "低级别背驰是本级别背驰的必要条件" → P1 缺口根因
   - §9.3 "实践中用 MACD 面积" → P2 已结算工程选择（非缺口）
   - `interp.rs:237-239` 自认注释 → P1 接线缺口的诚实标注
   - `sell.rs:35` still-MISSING 标注 → P7 接线缺口的诚实标注

3. **边界条件**：若后续 authoritative spec 给出 Cand^δ 的独立定义式（C1 矛盾结算），P1 实装方案须跟随更新；若 P7 接入 closed_loop 正规出场，出场口径变化可能翻转部分 spread_eaten 结论；若 W4 类型透传到 CSV 已完成，P4 完成。

4. **下游推论**：所有基于 `btc_663_ledger_sigma.csv` 的 alpha 结论（奇偶交替/σ_higher 条件化/(ℓ,q)三判据/夏普）须附限定词"无区间套定位、口径4 出场"。补齐 P1+P4+P7 后可声明"单标的旧缠论（§1-§11）完整实装"，有效域从子集扩到完整单标的旧缠论，但仍单标的 L2，P3 缺。

5. **谱系引用**：`full-implementation-gap-audit-20260701.md`（#120 审计，P1-P8 原始矩阵）；`full-impl-roadmap-20260701.md`（#124 impl-architect，塔桥纠正 + W1-W8 DAG）；`codex-cand-predicate-C1-20260701.md`（C1 矛盾结算）；`gap3-pdf-landing-plan-20260701.md`（PDF 靶子纠正）；`.chanlun/genealogy/settled/001-degenerate-segment.md`；`002-source-incompleteness.md`。

6. **影响声明**：本产出为诊断文档（L0/L1 代码事实），未改任何生产代码。影响 = 给编排者提供完整性判据矩阵 + 接线缺口/spec缺口精确分类 + 最小充分集。codex CLI 不可用（二进制丢失），本产出非异质否定，需 codex 恢复后补真异质验证。

---

## 附：codex CLI 故障记录

- **故障**：`codex exec` 失败，exit=-9（SIGKILL）
- **根因**：`/Users/silencehan/node-v22.14.0-darwin-arm64/lib/node_modules/@openai/codex/node_modules/@openai/codex-darwin-arm64/vendor/aarch64-apple-darwin/codex/` 目录为空（二进制丢失）
- **处置**：按 memory `project_heterosource_openai_quota_exhausted.md` 降级策略，同质代理质询代替，明确标注非异质
- **恢复建议**：`npm install -g @openai/codex@latest` 或重新安装 codex npm 包补回二进制
