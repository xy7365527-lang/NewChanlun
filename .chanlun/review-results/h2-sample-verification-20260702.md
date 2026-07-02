# H2 样本级验证：1473 信号的 s_prev==m1 互斥链实证（结果包）

- task: #8（codex H2 边界条件 L2 收口）
- negation_source: 本地 L2 插桩（`h2_sample_exclusion_dx`，`econ_positive.rs` tests，`#[ignore]`）
- **认识论等级：L2**（真实 BTC 全历史 4,613,599 bar 逐信号分解，可产否定性结果——本轮即产出否定性结果：codex 命名的具体机制只解释少数）
- 裁决对象：`.chanlun/review-results/codex-h1-ndelta-gate-20260702.md` 的边界条件1（codex 自留缺口）
- 原始数据：`.chanlun/review-results/h2-sample-raw-20260702.md`
- 复现命令：`ECON_L2_MAX_BARS=5000000 cargo test --release h2_sample_exclusion_dx -- --ignored --nocapture`（796s）

---

## 1. 结论

对全历史 BTC（2017-08-17→2026-05-31，4.6M bar）**level1-4 第二类(buy2/sell2) bsp_pre 信号逐信号插桩**，
在 N^δ 门 rung k=lvl+1 的 `div_cand` 分解拒绝阶段。信号计数 **1473**（level1=1059 / level2=324 /
level3=69 / level4=21）**逐级 bit-exact 复现** `acc-classification-level-hole-20260701.md`，
**gate_pass=0（真 100% 归零坐实）**。

**三 deliverable：**

**(a) Extreme 必假占比（codex 等价「s_prev 使互斥成立」）= 100%（257/257）。**
到达 cond3 的信号（cond1∧cond2 通过）共 **257**，其中 Extreme 必假（`m2` 未创新极值 vs 最近同向
`s_prev`）**257 个（100.00%）**，无一例外。

**(b) s_prev≠m1（Extreme 可满足 gap）样本门分布：样本数 = 0。**
codex 边界条件1 担心的情形（rfind 命中比 m1 更近的 q≠m1 使 `m2.lo<q.lo` 与 `m2.lo≥m1.lo` 同真、
互斥链不成立）在 1473 个真实信号中**零实例**——到达 cond3 的 257 个信号**全部** Extreme 必假，
无「m2 创新极值」的 gap 样本。**codex 自留缺口经实证不成立（缺口未实现）。**

**(c) level1-4 100%归零 ❌ 不完全由 cond3 互斥链解释——是「混合机制，互斥链是少数子集」。**

| 拒绝阶段（rung k=lvl+1，互斥优先序） | 信号数 | 占拒绝 |
|---|---|---|
| cond1_dir（`dir(m2)≠−δ`，m2 非背驰段要求方向） | **865** | **58.72%** |
| cond2_noprev（无前序同向段 s_prev） | **351** | **23.83%** |
| **cond3_extreme（Extreme 必假 = codex 互斥链）** | **257** | **17.45%** |
| base_none / no_upper / no_target / cond4_weak / reject_elsewhere | 0 | 0% |

codex 命名的互斥链（cond3 Extreme）**只解释 17.45%（257/1473）**。归零的**主因**是更基础的
**范围误用**：`div_cand` 作为**背驰段谓词**，条件1 要求终端段 `s=m2` 方向 = 背驰段方向（买侧=下跌），
条件2 要求存在前序同向段——但第二类买卖点的结构段 `m2`（回拉走势）**在 82.55% 的样本上要么方向
不是背驰段要求方向（58.72%），要么在父 `sub_moves` 内无同向前驱（23.83%）**，根本到不了 Extreme 检验。

**互斥链方向修正**：codex 假设「B1 与 B2 间只隔一条反向离开腿」⟹ `s_prev` 跨反向腿命中 `m1`
（leg_gap=2）。实测 **leg_gap 全为 1**（257/257）——`s_prev` 是 `m2` 在父 `sub_moves` 内**紧邻的
同向前段**（外缘 `rmove_dir` 下相邻两段可同向，非严格交替），不是 codex 设想的「跨一条反向腿的 m1」。
outcome（Extreme 必假）一致，但**结构路径与 codex 描述不同**：不是「跨反向腿命中第一类离开段」，
而是「紧邻同向段未被 m2 突破」。

## 2. 定义依据

- **codex 互斥链 L0 推导**（裁决对象 §1）：`build_nest_certificate`（`econ_positive.rs`）在 rung
  k=lvl+1 把 `Cand^δ` 操作化为 `div_cand`（`cand_predicate.rs`）的四条件合取，其条件3 Extreme
  `Side::Long => s.lo < s_prev.lo`（cand_predicate.rs:168）。第二类分类前提 `retrace_no_break`
  `Side::Long => no_new_low(m1,m2) = m1.lo <= m2.lo`（rmove_compose.rs:80/97）。当 `s=m2, s_prev=m1`
  时 `m2.lo<m1.lo`（Extreme）与 `m1.lo<=m2.lo`（分类前提）逐字互斥。
- **级别对齐坐实**（本测试消除 codex 未验证的隐含假设）：level-lvl 第二类信号由 `extract_second_for_level`
  （mod.rs:1169/253）从 **level-(lvl+1)** 的 `parent`（`RMove::Compose`）产出，`m1`/`m2` 是
  `descend(parent)` 的 level-lvl 次级别走势，`source_index=index_of(m2)=m2.end_index`。而
  `build_nest_certificate` 的 rung k=lvl+1 的 `knode`（tower[lvl+1] 含 src 段）**正是**该 `parent`，
  `knode.sub_moves=[m1,m2,…]`、`target_idx=i2`（m2 位置）、`s=m2`——codex 的「s_prev vs m1」是
  **同级别对象**比较，无级别偏移（本测试的前提验证，非假设）。
- **codex 等价可测判据**（裁决对象 §3 边界条件1 原文）："或等价地，s_prev 的低点仍高于/接近 m1 使
  Extreme 必假"——本测试直接测 rung k=lvl+1 的 Extreme 真假，即 codex 授权的「s_prev 使互斥成立」
  可测等价物。
- **背驰段谓词范围**（`cand_predicate.rs` docstring + codex C1 裁决）：`DivCand^δ` = 背驰段候选谓词
  （条件1 方向=背驰段方向、条件2 Comparable 前序同向段、条件3 Extreme 价格创新极值、条件4 Weak
  MACD 力度衰减）。第二类买卖点的安全性由走势完备性（第17课，非背驰）保证——套用背驰段谓词 = 范围误用。
- **bit-exact**：门判定用 `build_nest_certificate`+`n_delta`（与生产 `build_multilevel_nest_cert`
  共用构造）；`div_cand`/`ContextMove::from_rmove` 与生产同源；`macd_hist` 全 bar 域 close（与
  `decompose_capturable_spread` 同口径）；cond4 用「cond1∧2∧3 过但 `div_cand`=false」推断（不触碰
  私有 `segment_macd_area`）。真封断言：`Σ阶段=1473=n_total`；`cond3_reached=257=必假257+真通过0+真拒0`。

## 3. 边界条件（本结论在何种情况下翻转）

1. **s_prev==m1 的字面身份**：本测试用 codex 授权的等价物「Extreme 真假」，**未**逐信号重建
   `find_second_type_structure` 的 `i1` 做字面 `s_prev==m1` 比对（因分类的 `divergence_of` 用 merged
   域 hist+close_src，与门的 bar 域 hist 坐标系不同，重建不能 bit-exact）。若后续用分类内插桩取回
   bit-exact `i1`，且发现 cond3 的 257 例中 `s_prev` 字面≠`m1`（尽管 Extreme 仍必假），则「互斥链」
   的字面机制描述需改为「紧邻同向段未突破」——但 deliverable (a)/(b)/(c) 的**数值结论不变**（Extreme
   必假 100%、gap=0、互斥链占 17.45%），因为它们建立在 Extreme 可测判据上，不依赖字面 m1 身份。
   实测 leg_gap 全=1 已强烈提示字面机制与 codex 描述（leg_gap=2）不同。
2. **窗口依赖**：本结论基于全历史 4.6M bar（与 acc 同窗，1473 逐级复现）。150K 近端窗口曾见
   gate_pass=2/50（level1 有信号通过门）——中间级归零是**全窗**性质，非任意子窗恒成立。跨窗差异见
   `h2-sample-raw` 对照。
3. **配置依赖**：`ThetaConfig::default()`（含 MACD/level/tick 默认）。改中枢口径/level 参数/MACD
   参数可能改变 sub_moves 方向标注与 Extreme 数值 ⟹ 阶段分布可变。
4. **cond1/cond2 主因的定性**：cond1_dir(58.72%)+cond2_noprev(23.83%) 是「背驰段谓词 vs 第二类结构」
   不兼容的更基础表现。若修复方案给第二类独立的 `Cand^δ` witness（非背驰段谓词），这 82.55% 的拒绝
   机制消失——但那是**定义扩展决策**（codex §4），不在本 L2 实证范围。

## 4. 下游推论

- **codex H2 核心裁决（范围误用）强化**：本 L2 不仅未推翻 H2，反而**加强**——`div_cand` 背驰段谓词
  与第二类结构的不兼容比 codex 识别的更**深**：不止 cond3 Extreme 互斥（17.45%），主因是 cond1 方向
  （第二类结构段 m2 多非背驰段方向）+ cond2 无同向前驱（82.55%）。codex 的「Type1 走背驰段谓词、
  Type2 需独立 candidate witness」定义扩展方向由此获额外 L2 支撑。
- **codex 边界条件1（自留缺口）经实证关闭**：gap 样本=0 ⟹「rfind 命中 q≠m1 使 Extreme 可满足」
  从未发生 ⟹ 无需为该 undecidable 分支保留 codex 建议的「部分转向 undecidable」。cond3 到达域内
  互斥（Extreme 必假）无例外。
- **对 acc 报告的修正**：`acc-classification-level-hole-20260701.md` 把 level1-4 100%归零解释为
  H1（N^δ 嵌套链 [J⊆J] 滤空）。本 L2 定位到**更精确的机制**：归零发生在 rung k=lvl+1 的 `div_cand`
  条件层（cond1/cond2/cond3），**不是** [J⊆J] 区间套嵌套（`is_sub`）滤除——reject_elsewhere（含
  is_sub 失败）= 0。即「中间级空洞」的真因是 Cand^δ 谓词与第二类的谓词层不兼容，非区间套嵌套结构。
- **修复范围不由本任务决定**（codex §5）：是否给 `Cand^δ` 按 bsp 类型分叉、分叉依据（第二类走
  次级别第一类构成 witness vs 背驰段 witness）是**选择类**决断，需编排者/异质裁决。本任务只补 L2 证据。

## 5. 谱系引用

- `.chanlun/review-results/codex-h1-ndelta-gate-20260702.md`（H2 裁决，本任务是其边界条件1 的 L2 收口）
- `.chanlun/review-results/codex-cand-predicate-C1-20260701.md`（C1：`Cand^δ`=背驰段候选谓词，Type1
  正确、无差别套 Type2 是其未覆盖角落）
- `.chanlun/review-results/acc-classification-level-hole-20260701.md`（1473 信号来源；本任务修正其
  H1 机制归因为更精确的 cond1/cond2/cond3 谓词层，非 [J⊆J] 嵌套）
- 本任务**未发现**需新开谱系条目的「定义间不可弥合矛盾」——这是操作化范围误用（背驰段谓词误套第
  二类），codex 已归此类；是否 `/escalate` 取决于修复方案选择（`Cand^δ` 是否分叉），属选择类决断。

## 6. 影响声明

- **不改动任何生产逻辑/定义**。仅新增一个 `#[ignore]` L2 诊断测试 `h2_sample_exclusion_dx`
  （`rust/src/theta_v0/backtest/econ_positive.rs` tests 模块，插桩复用 `build_nest_certificate` +
  `div_cand` + `ContextMove`，无新公开 API）。
- 产出两份文件：本结果包 `h2-sample-verification-20260702.md` + 原始数据 `h2-sample-raw-20260702.md`。
- 涉及（只读引用，未改）模块：`econ_positive.rs`（build_nest_certificate）、`cand_predicate.rs`
  （div_cand/ContextMove）、`rmove_compose.rs`（retrace_no_break）、`mod.rs`（extract_second_for_level）、
  `recursive_tower.rs`（LeveledMove/sub_moves）、`nest.rs`（n_delta）。
- 对下游的影响：为 H2 修复方案的**范围决策**提供 L2 依据（背驰段谓词的不兼容主因是 cond1/cond2，
  非 cond3 互斥；互斥链只占 17.45%）。修复方案本身（是否分叉 `Cand^δ`）待编排者/异质裁决。
