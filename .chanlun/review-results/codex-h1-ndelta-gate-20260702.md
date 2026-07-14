# Codex 异质诊断：N^δ 门滤空中间级(level1-4)第二类信号——H1 vs H2 裁决

- task: #7（codex 异质确认 H1）
- negation_source: heterogeneous（OpenAI Codex CLI，diagnose 模式）
- epistemic_level: L0（代数/逻辑推导，从代码定义直接推出矛盾）+ 边界条件标注需 L2（真实样本切片）补全
- 原始 Codex 交互持久化：`.chanlun/review-results/codex-diagnose-20260702-0150.md`
- 裁决对象：`.chanlun/review-results/acc-classification-level-hole-20260701.md` 中 H1 候选判定

## 1. 结论

**verdict: H2-overfiltering-bug（范围误用，非实现 typo，也非"正确严格性"）**

Codex 独立裁决（在其无法访问真实仓库、仅凭我提供的逐字代码摘录 + 缠论原文进行推理的条件下）与我
本轮在仓库中直接追踪代码得出的机制假设**收敛一致**：

`build_nest_certificate`（`rust/src/theta_v0/backtest/econ_positive.rs`）在 `k = lvl+1` 这一级 rung 上，
把 `Cand^δ_ℓ` 操作化为 `div_cand`（`cand_predicate.rs`）四条件之一的 **Extreme**：

```rust
Side::Long  => s.lo < s_prev.lo   // 要求候选段创造更低低点
Side::Short => s.hi > s_prev.hi   // 要求候选段创造更高高点
```

`target_idx` 用 `end_index == source_index` 精确定位——被检验的 `s` 正是终端信号自身所在的段。
在最常见结构（B1 与 B2 回拉段之间只隔一条反向离开腿）下，`div_cand` 条件2的 `rfind` 命中的
`s_prev` 就是 `find_second_type_structure` 里的 `m1`（第一类离开走势）本身。

而该终端信号能进入 `bsp_pre` 统计（即被分类为第二类）的前提，是 `rmove_compose.rs::retrace_no_break`：

```rust
Side::Long  => no_new_low(m1, m2)   // 回拉段 m2 不能比 m1 更低
Side::Short => no_new_high(m1, m2)  // 回拉段 m2 不能比 m1 更高
```

即 `m2.lo >= m1.lo`（Long侧）。当 `s=m2, s_prev=m1` 时，DivCand 要求 `m2.lo < m1.lo`，与分类前提
`m2.lo >= m1.lo` **在同一对象、同一比较对上不可同真**——不是"经验上很少满足"，是结构性必假。

这解释了 level1-4（100% B2/S2）100% 归零：只要这些级别的信号在上级 tower 中已形成包含它的
Compose（rung 链未断），DivCand 条件3 必然拒绝。而 level0 96.5% depth=0（rung 链在第一步就断，
根本没有触达 Extreme 检验）门通过率因此高得多——这不是"门有效域窄"的良性退化，其中至少
level1-4 这一部分是范围误用导致的系统性假阴性。

## 2. 定义依据

- **区间套原文**（`docs/chanlun/text/chan99/0027-第六节 区间套.md`）：区间套是"根据背驰段"从大级别
  向小级别逐级寻找背驰点的方法，理论依据是"低级别背驰是本级别背驰的必要条件"——对象明确是
  **背驰段**。
- **第二类买点原文**（`docs/chanlun/text/blog/017-第17课.md` line 60，一级权威，逐字）：
  > "……这样的买点是绝对安全的，其安全性由走势的'不患'而保证……"
  第二类买点的安全性由**走势完备性定理**（图形必须包含≥3段次级别运动才算完成）保证，**不是**由
  背驰/MACD 力度对比保证。第17课全文的背驰判断（均线/MACD）服务于走势-级别-趋势的比较逻辑，
  对象是"趋势"，不是任意端点。
- **代码实现**：`retrace_no_break`（第二类分类前提，"不创新低/新高"）与 `div_cand::Extreme`
  （背驰候选谓词，"必须创新低/新高"）在 `s=m2, s_prev=m1` 时逐字互斥——这是我从
  `rmove_compose.rs`、`cand_predicate.rs`、`econ_positive.rs`、`signal.rs` 四个文件直接读取
  确认的，非转述。
- **先前裁决**（`codex-cand-predicate-C1-20260701.md`）：C1 裁决把 `Cand^δ_ℓ` 定为"背驰段候选区间
  存在性谓词"，并明确留了边界条件——"若后续 authoritative spec/PDF 明确给出 Cand^δ_ℓ 独立定义式
  → 推翻本裁决"。本次诊断正是发现：C1 的定义对 Type1 成立，但被无差别套用到 Type2 时与 Type2
  自身的分类前提冲突——这是 C1 裁决范围的一个未覆盖角落，不是对 C1 结论本身的推翻。

## 3. 边界条件（本裁决在何种情况下翻转）

1. **s_prev ≠ m1 的情形**（Codex 独立指出，我认为是本裁决唯一实质性缺口）：若 B1 与 B2 之间存在
   多于一条同向子腿，`div_cand` 条件2的 `rfind` 会命中比 `m1`更近的同向段 `q`（而非 `m1`本身）。
   此时 `m2.lo >= m1.lo`（分类前提）与 `m2.lo < q.lo`（Extreme条件）可以同时成立，互斥链不必然
   成立。**要把"level1-4 100%归零"完全归因于本机制，需要对这1473个实际信号做样本级验证**——
   检验其中 `s_prev` 实际等于 `m1`（或等价地，`s_prev` 的低点仍高于/接近 `m1` 使 Extreme 必假）
   的比例。本次诊断未做这一步实证核验（超出 diagnose 模式的范围，需要额外的数据切片/日志插桩）。
   若实证发现多数样本 `s_prev ≠ m1` 且 Extreme 独立地经验性为假（非结构必然），则应部分转向
   undecidable，需要具体的信号级归因数据。
2. **区间套权威定义的进一步澄清**：若后续能在缠师原文中找到明确表述——"任何类型买卖点的上级
   候选段都必须展现价格新极值，无论该买卖点自身的分类前提是什么"——则 H1 成立，本裁决翻转。
   目前三级权威链中未发现此类表述；第17课原文明确把第二类的安全性归因于走势完备性而非背驰强度，
   这是本裁决判 H2 而非 H1 的直接依据。
3. **Codex 自身认识论限制**：Codex 本轮无法访问真实仓库（其执行环境仅可见 `.serena/`），其 H2
   判定基于"我提供的代码摘录与仓库一致"这一前提。该前提由我在本轮会话中直接用 Read/Bash 从
   真实文件逐字提取核实（非转述/臆造）——前提成立，因此 Codex 的条件性裁决可以采信为
   "repo-accurate 前提下的 H2"。

## 4. 下游推论

- 若 H2 成立，`build_nest_certificate` 需要按 bsp 类型分叉 `Cand^δ_ℓ` 的操作化：Type1 走现有
  `DivCand`（背驰段候选，含 Extreme）；Type2（及需核实的 Type3）需要独立的 candidate witness
  定义（如"次级别相应第一类买卖点构成"，回应 §10.1 买卖点定律一），而非统一套用背驰段谓词。
  这是一个**定义扩展**决策（Cand^δ_ℓ 是否应该有多个变体、变体的选择依据是什么），不是纯代码修复。
- 这直接影响 `acc-classification-level-hole-20260701.md` 的结论解释：该报告把"95.36%通过门信号
  depth=0"归为"门有效域本就窄，这是背景不是结论"——本次诊断表明，至少 level1-4 的 100% 归零
  这一部分**不属于"有效域窄"的良性背景**，而是范围误用导致的系统性假阴性，需要在该验收报告中
  补充这一区分（有效域窄 ≠ 范围误用假阴性，两者都会导致低"真跨级"比例，但性质不同）。
- P1 acc 验收整体结论（"门机制工作正常，只是有效域窄"）需要重新评估——不能笼统地把所有
  depth=0/归零现象都归为良性退化。

## 5. 谱系引用

- `.chanlun/review-results/codex-cand-predicate-C1-20260701.md`（C1裁决，本次是其留下的
  follow-up边界条件的具体触发）
- `.chanlun/review-results/acc-classification-level-hole-20260701.md`（H1候选的原始提出者，本次
  是对该文档判定的异质诊断）
- 本任务未发现需要新开谱系条目的"定义间不可弥合矛盾"——这是操作化范围的误用（C1定义对Type1
  正确，误套到Type2），不是缠论原文内部或原文与本项目定义之间的矛盾。是否需要走 `/escalate`
  取决于修复方案的选择（Cand^δ_ℓ 是否要分叉、分叉依据是什么）——这是"选择"类决断，建议
  team-lead 走 `/escalate` 或至少经编排者/Gemini确认后再改动 `econ_positive.rs`/`nest.rs`。

## 6. 影响声明

本产出**不改动任何代码或定义文件**，只产出诊断裁决。涉及模块：
`rust/src/theta_v0/backtest/econ_positive.rs`（build_nest_certificate/build_multilevel_nest_cert）、
`rust/src/theta_v0/classifier/cand_predicate.rs`（div_cand）、
`rust/src/theta_v0/classifier/nest.rs`（NestCertificate::n_delta）、
`rust/src/theta_v0/classifier/rmove_compose.rs`（retrace_no_break）、
`rust/src/theta_v0/classifier/signal.rs`（extract_second_signals）。
若采纳本裁决进行修复，需要新的实现决策（Cand^δ_ℓ 按类型分叉的具体形式），且需要先做边界条件1
中提到的样本级实证核验（level1-4 实际1473个信号中 s_prev==m1 的比例），再决定修复范围。
