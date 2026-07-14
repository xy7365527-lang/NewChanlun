# Downloads PDF 全读 D：结构概念组（task #73）

**认识论等级**：本报告为文档全读 + 代码对照审计（无数据实验）。PDF 推导本身标 L0（定义/形式化）；实装对照为代码事实核查。
**范围**：区间套.pdf(15p)、关于背驰.pdf(14p)、k的条件.pdf(13p)、anc.pdf(27p)——四份逐页全读（含占位图标页）。纯只读，零 git。
**源权威**：`docs/formal-chain/`（编排者 2026-07-02 裁定=最严格起点唯一权威）。

---

## 覆盖声明（四份全读完成）

| PDF | 实际主题（≠文件名预期时标注） | 页数 | 核心裁决 |
|---|---|---|---|
| 区间套.pdf | 区间套证书两个 bit-exact 问题 | 13正文+2占位 | ①rung 端点相等→区间包含 ②frontier 未确认必须重算 |
| 关于背驰.pdf | 力度=递归签名支配序 + 背驰四条件 | 12正文+2占位 | 无唯一全序力度；三值判定 {Yes,No,Undetermined} |
| k的条件.pdf | **不是 K 线预处理——是 GAP3 κ(kappa) 严格定位** | 11正文+2占位 | κ 不可从价格识别；barrier η*=L^wc+κQ |
| anc.pdf | 跨 bar 持久身份 vs 确定性 ID | 25正文+2占位 | depth>0 归零=身份层剪枝非交易逻辑；引入 persistent registry |

**文件名预期偏差（team-lead 注意）**：`k的条件.pdf` team-lead 预期为「K 线预处理条件」，实际全文是 **GAP3 treasury 层 κ 的严格推导**（EnterEarning barrier / BuyCore 预算约束），与 GAP3 EarningShares 可达性直接相关，与 K 线包含/分型预处理无关。

---

## 一、矛盾/张力即报（任务点名的最高优先级检查点）

### 1. 区间套.pdf 问题① 端点相等 rung —— 未修复的实装缺口（真缺口）

**区间套.pdf 裁决**（页1-8、13）：跨级 rung 定位当前用 `target_idx = position{m : m.end_index = source_index}`（端点相等 ρ(m)=s），这比区间套语义 `J_{k-1} ⊆ I(m)` **强得多**（端点相等是区间包含的真子条件）。端点相等产生系统性 **false negative**，"depth≥2 恒为 0 正是这种 false negative 的系统性表现"。正确语义应为 `J_child ⊆ J_parent`（`start(parent)≤start(child) ∧ end(child)≤end(parent)`），唯一性由良式分解或 Sel_Θ 保证。

**代码对照（缺口成立）**：`rust/src/theta_v0/backtest/econ_positive.rs` 仍用端点相等：
- L560 `find_move_by_end_index(subs, source_index)`（次级别 Type1 rung）
- L632 per-rung `Cand^δ_k` 在 `rung_subs` 找 `end_index==source_index`
- L736/L1163 执行级候选段 `find_move_by_end_index(exec_moves, source_index)`
- L527/L725 注释明确：`None ⟺ 无 end_index==source_index 段 ⟹ 门直接拒`（= false negative 来源）

`nest.rs` 层面**有** `is_sub` 区间包含检查（cand_predicate.rs:479），但那是 rung 选定**之后**的事后校验；rung 的**候选定位**仍用端点相等——正是区间套.pdf §2 批评的"先端点选段再检查包含"模式，非其裁定的"bottom-up 先定 J_{k-1} 再找包含它的 J_k"。**问题①未修复。**

### 2. 与 C3 level==1 终局关闭裁定的关系（澄清：非同一判据，但同根张力）

team-lead 要求核对「区间套.pdf 是否与 C3 level==1 关闭裁定相反」。结论：**不是直接相反推导，但存在方法论张力，仍即报**。

- C3（小转大门第三判据，codex #44/#55）用的是 `start_index >= source_index`（新中枢突破，见 c3-breakout-impl），**不用端点相等 rung**。两者是不同判据，区间套.pdf 问题① 不直接推翻 C3 关闭。
- **但同根张力真实存在**：C3 关闭裁定基于 level==1 实测 **0/84**；区间套.pdf(depth≥2=0)、anc.pdf(#5α=0) 与之共享同一「跨级/depth>0 结构实测归零」现象。区间套.pdf + anc.pdf 系统性论证：**跨级/depth>0 实测 0 高度可疑，是实现层伪影（端点相等 false negative / 身份层剪枝），非结构真相**。
- codex 自己在 xzd2-impl-codex-audit 已承认「当前 0/114 不能作为严格门真结果采信，至少部分是实现伪影」。区间套.pdf 为「这个 0 是伪影」提供了**独立的形式化根因**，加强了「C3 关闭的经验依据(0/84)不可全信」的判断。C3 关闭作为 level==1 门是保守合法的，但**不能据此声称 level==1 结构上无小转大 alpha**——区间套问题① 未修前，该 0 值的结构解释被污染。

### 3. 三条独立的「depth>0 归零」链（综合缺口地图）

| 链 | 出处 | 机制 | 实装状态 |
|---|---|---|---|
| 身份层链 | anc.pdf | snapshot-only 全量重建→held pid 找不到→Stale→AncOK 剪 depth>0 | **已修**（persistent.rs 最小 overlay）；彻底修(增量 extract) = ceiling 未做 |
| 端点相等定位链 | 区间套.pdf 问题① | rung 用 end==source 端点相等→depth≥2 false negative | **未修**（econ_positive.rs 仍端点相等） |
| frontier 链 | 区间套.pdf 问题② | 未确认最后窗口误纳 confirmed prefix→L0 多 1 中枢 | 部分（incremental.rs 有 resume；confirmed-prefix-immutable = ceiling 未做） |

**即报要点**：身份层链已修，但端点相等定位链未修——即使 persistent registry 修好身份，rung 仍会因端点相等归零 depth≥2。区间套.pdf 明言：「不修这两个问题，就不能声称已经回测了缠师 100% 区间套多级判据」，而问题① 仍未修。

---

## 二、各 PDF 定义要点（供下游对照）

### 区间套.pdf
- rung 递归：`N^δ_{k↓e} = Cand^δ_k(J_{k-1},t) ∧ N^δ_{k-1↓e}`，本体 `J^δ_e ⊆ ... ⊆ J^δ_ℓ`（区间包含链非端点相等链）。
- soundness/completeness 定理 1-2 已给；定理2：`Γ_old ⊊ Γ_new`（端点相等旧实现是区间包含新实现的真子集）——解释旧回测有效嵌套深度几全为 0。
- 问题②：`T^inc = Prefix(T^inc, b_t) ⊕ Detect(h_{b:t+1})`，只 sealed prefix 可复用，mutable frontier 必须重算。

### 关于背驰.pdf
- **背驰四条件**（div_cand 对照锚点）：4.1 同级别同向 `ℓ(s)=ℓ(s'), dir=-δ`；4.2 同上级趋势语境 `Comparable_ℓ(s',s)=1`；4.3 价格推进 `min P(s)<min P(s')`（买）；4.4 力度弱化 `𝔉_ℓ(s)≺𝔉_ℓ(s')`。
- 力度=完整递归签名 `𝔉_ℓ(s)`，MACD 面积只是一个投影坐标（非充分统计量）。
- 力度比较=支配序 `s≺_A s' ⟺ ∀m∈A_ℓ, m(s)≤m(s') ∧ ∃m, m(s)<m(s')`；无唯一全序（定理2）；三值 {Weak,NotWeak,Incomparable}。
- 推荐 `Cand^δ = StructEligible^δ`（宽结构候选），非 `Cand = MACDdiv`——不让候选因 MACD 提前删除。对照 #62 StructBreak 第四类纤维方向一致。

### k的条件.pdf
- κ 严格含义：`c^adj_t ≤ -κ ⟺ η_t ≥ κQ_t`（每单位核心持仓保留的负成本缓冲）。
- barrier：`η*(x_t) = L^wc_{t+1}(x_t) + κQ_t`；EnterReady/BuyCore 合法性 `a_n + L^wc_{n+1} + κΔQ_n ≤ η_n + g_n - κQ_n`。
- 定理2：κ 对价格数据**不可识别**（是风险偏好参数，非价格能推出）。κ↑⟹ΔQ_max↓；**κ 太大会导致系统永不进增股阶段**（GAP3 可达性对照）。
- canonical：`η* = L^wc + κ_policy·Q`，`κ_policy ∈ Θ_risk` 须显式声明来源。

### anc.pdf
- 核心链（§17）：`per-bar 重建 E_i ⇒ 旧 held ID 不在 E_{i+1} ⇒ Stale ⇒ prune ⇒ 父链缺失 ⇒ AncOK 剪 depth>0`（snapshot-only 下数学必然）。
- held_exact_CL=0.8%, BTC=27.8%（72% held 腿跨 bar 丢身份）。
- 四态分类 `Ω(u)∈{LivePresent, LiveDetached, Closed, Invalidated}`；Q4 把 LiveDetached 误并入 Stale = 系统性剪枝根源。
- 修复：`pid(e)=H(ℓ,start_anchor,end_anchor,δ,kind,tie_break)`（不含 parent）；held validity `ε(L)∈P_i ∧ ¬Invalidated`。
- §16 confirmed prefix immutable / frontier only mutable = 07b 引用锚点（引用准确）。

---

## 三、A3/07b anc 引用完整性核对

- 07b 引 anc §16「confirmed prefix immutable」：**准确**——§16 完整修复即"增量 extract，confirmed prefix immutable，frontier only mutable"，与区间套.pdf 问题② 同构。
- anc.pdf 最小修复（persistent overlay）**已实装** `strategy/persistent.rs`（头注直引 anc §1-§16，不变量 I1-I5、四态、LiveDetached 全落地）；彻底修复（增量 extract）诚实标为 ponytail ceiling 未做。引用与实装一致。

---

## 结果包六要素

1. **结论**：D 组四份全读完成。核心缺口=区间套.pdf 问题①（端点相等 rung）**未修复**，是 depth≥2=0 的独立伪影根因，与已修的 anc 身份层链并列。C3 level==1 关闭与区间套非同一判据，但共享「跨级实测 0=伪影」同根张力，即报。
2. **定义依据**：区间套.pdf §2「端点相等是区间包含真子条件」+ 定理2「Γ_old⊊Γ_new」；代码 econ_positive.rs L560/632/736/1163 端点相等定位。
3. **边界条件**：若把 rung 定位改为 `J_child⊆J_parent` 区间包含后 depth≥2 仍恒为 0，则区间套.pdf 归因被否，depth 归零转为结构真相（当前证据不支持——是三条伪影链叠加）。
4. **下游推论**：所有基于 depth>0/跨级/level==1 实测 0 的结论（C3 关闭、#5α=0/ΔSharpe=0、wverify 高级别无 alpha）在问题① 修复前，其"结构无 alpha"解释被污染，须标 frontier/端点相等伪影候选。
5. **谱系引用**：606（区间套有效域=Type1）、534（nested-recursion-accounting）、project_interval_nesting_not_called_in_backtest（95%退化 depth=1）、project_gap3_l2_unreachable_architecture（#5α=0，anc 给出更精确根因=身份层剪枝，订正"纯架构"归因）、project_level_hole_window_dependence（C3 W-VERIFY 有效域=level0）。不确定是否已有「端点相等 rung=区间包含退化」独立谱系条目，建议 genealogist 评估。
6. **影响声明**：本报告纯只读审计，无代码/git 改动。指出一个未修复实装缺口（区间套问题①）+ 一处 PDF 主题标注偏差（k的条件=κ 非 K 线预处理）+ 加强 C3/GAP3 实测 0 的伪影质疑。
