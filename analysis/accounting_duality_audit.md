# 会计双重性审计——nested_fugue 会计层 vs §8 双重性（538/540 谱系）

> 任务（2026-06-13 编排者）："先做会计审计（双重性）。"
> 审计对象：`rust/src/trading/nested_fugue.rs` 会计原语（URS bit-exact 复用）vs
> 规格 `docs/nested_fugue_accounting.md` §1-§8 的同一-差异双重性（538号）。
> **认识论等级：L0**（纯代码 + 定义追踪 + cargo test 数值断言；6 维度各经独立对抗性核验）。
> 方法：workflow `wf_272c0bc9-6dd`——6 维度并行审计 → 各维度怀疑论 agent 独立重追踪反驳。
> 全部 6 发现经对抗核验成立（high confidence）。

---

## 0. 判决（先行）

**会计双重性的代码实装正确；规格文档有系统性声明膨胀（090号）。**

| 维度 | 不变量 | verdict | 性质 | 核验 |
|---|---|---|---|:---:|
| **D1** | §8.2 同一笔存一次 + 视图按需导出 | **CONFORMS** | L0-canonical-correct | holds（5 轴反驳全失败） |
| **D2** | §8.3 child.P&L ≡ father.cost_reduction | **GAP** | spec-incomplete | holds |
| **D3** | §8.1 Σunits=N + §1 N 恒定 | **DIVERGES** | spec-code-tension | holds |
| **D4** | §7 earning 多空对称 | **GAP** | 声明膨胀 | holds |
| **D5** | §8.4 NAV 全方向逐市 | **GAP** | spec-incomplete | holds（+4 次级修正） |
| **D6** | §5 递归嵌套三层守恒 | **CONFORMS** | L0-canonical-correct | holds |

**一句话**：双重性的**核心**（D1 存一次/视图导出 + D6 递归守恒）**代码忠实且正确**——
Phase 3（逐仓嵌套/双层记账）地基稳固。但规格 §8 的**四条无条件不变量**（§8.3 恒等 /
§7 对称 / §8.4 逐市 / §1 N 恒定）只在**理想化分支**（盈利 ∧ cost_pool 足 ∧ 满额回补）
成立；代码正确处理了规格未言明的分支（亏损 / 池竭 / 空头），其行为由代码自身 doc 注释
（line 12-13/32-34/185-189）已显式声明。**声明膨胀在规格文档，不在代码。**

---

## 1. 统一根因：双重性 = 单一 leftover 的多去向分裂

规格把 child.P&L ≡ father.cost_reduction 当作「同一数字两身份」（538号双重性心脏）。
代码追踪揭示：存在**唯一物理量** `leftover = capital − u_back×c`（pop_tail line 217），
它**分裂为四个去向**（非两身份）：

```
                  leftover = capital − u_back×c   （唯一物理量）
盈利侧（c < basis, shortfall=0）：
   ├─ cost_pool 减（降成本）    reduce = leftover.min(parent.cost_pool)   line 222
   ├─ earning（N 增, 池竭后）   excess→dq, parent.units += dq, n_base += dq  line 229-236
   └─ free（现金沉淀）          *free += reduce/leftover                   line 236/241/244
亏损侧（c > basis, shortfall>0）：
   └─ shortfall 单位缩水（N 减）  *n_base −= shortfall                      line 220
      leftover = 0 ⇒ reduce = 0 ⇒ cost_pool 不增（亏损不是负 cost_reduction）
```

**§8.3 恒等式 child.P&L ≡ father.cost_reduction 仅在「盈利 ∧ 池足 ∧ shortfall=0」成立**
（D2 情形 a）。三情形数值演算（basis=104, u=250, capital=26000, parent Long）：

| 情形 | 条件 | child.P&L | father.cost_reduction(=reduce) | 恒等？ |
|---|---|---:|---:|:---:|
| (a) 盈利+池足 | c=96, pool=100000 | +2000 | 2000 | **✓ 等** |
| (b) 盈利+池竭 | c=96, pool=500 | +2000 | 500（+1500 走 earning） | **✗ 分裂** |
| (c) 亏损 | c=110 | −1500 | 0（亏损→13.64 单位缩水） | **✗ 破** |

（cargo test 印证：a=`recovery_refills_parent_and_reduces_cost_pool`、
c=`negation_kills_child_with_shrink_rebase` line 917-936）

**读法歧义澄清**（D2 核验精化）：情形 (b) 在「cost_reduction := reduce（cost_pool 减量）」
窄读法下破；在「cost_reduction := leftover（sunk-against-position 现金总额）」宽读法下
总额仍 ≡ P&L（只是 1500 路由到 earning/N 重定基）。**情形 (c) 亏损在两读法下皆破**
（reduce=0 ∧ leftover=0 ≠ P&L=−1500）——这是恒等式破裂的无条件硬证据。

**严格形式**：§8.3 应写作 `child.P&L ≡ cost_pool_reduce + earning_excess − shortfall_loss`
（四去向守恒），或显式标注有效域 = {盈利 ∧ shortfall=0 ∧ u_back==u}。

---

## 2. 多空不对称是 L0 硬墙（D3 + D4，对 Phase 2/5 决定性）

### 2.1 N_base 双向重定基（D3）⇒ §1"N 恒定"失效

规格三方冲突：
- **§1**：N 建仓后恒定，只有 earning 可增（单调非降，26课"一开始买够不加仓"）。
- **§8.1**：Σunits = N「根 voice 恒仓」（把 N 当不变锚）。
- **代码 line 220**：亏损回补 `n_base −= shortfall`（N **减少**）；line 233 earning `n_base += dq`（N **增加**）。

数值（test line 917-936）：根 1000 股 → spawn 子空 250@104 → 价涨破否定线@111 →
u_back=234.234, shortfall=15.766 → **n_base 1000 → 984.234**（15.766 股永久消失）。
守恒守卫（line 657 `|Σunits−n_base|<eps`）**PASS——但语义被掏空**：line 219+220 把同一
shortfall 加到 units、减自 n_base，差恒为 0，守卫退化为「记账两侧一致」（防单边记账 bug），
**不验证 §8.1 字面的「N=建仓常数恒仓」**。

**调和读法**：N_base ≢「建仓股数」（§1），N_base ≡「当前链上在手单位聚合」（line 656 运行
定义）。在此重定义下代码自洽，但与 §1 的 26课「恒仓」存在论冲突——规格内部 §1（建仓常数）
vs §8.1 实装（运行态聚合量）的**语义裂缝**，非实现 bug。代码 doc line 12-13 已自发命名此
镜像（earning N+Δ ↔ 亏损 N−δ）。

### 2.2 earning 多空不对称（D4）⇒ §7"对称"是声明膨胀

规格 §7："earning 多空会计完全对称…机制与方向无关"，"空头降成本…cost≤0→增加空头仓位"。

代码两分支：
- **多头父 earning**（line 226-236）：excess→dq，`parent.units += dq`（多头**增股**），N 重定基。
- **空头父 earning**（line 247-259）：line 249 注释"空头 earning（挣负股数）L0 不可构造——
  现金留 capital"；line 256 仅 `nrf_short_earning_hits += 1`（**计数**），**不增 units**。

**为何 GAP 而非 DIVERGES**：DIVERGES 要求两选项都可实现、代码选了冲突的那个。此处 §7 声明
的"增空头股数"在线性现货/期货载体下 = "挣负股数" = **L0 构造性不可表示**（空头均价 ≤0 后
继续盈利需持有负数量股）。代码诚实实装不可构造性（计数而非伪造增仓）。§7 把一个 **L0 代数
对称（极性翻转）误当作 L0 构造可实现性**，未察载体非线性约束 = 声明膨胀。
谱系：`project_put_option_short_earning`（空头 earning 只在凸性载体=期权消解）、
`project_bidirectional_accounting`（空头单相单律非对称定理"earning 相 L0 不可构造"）。

---

## 3. NAV 逐市的方向不对称递延（D5）

规格 §8.4："total_NAV = initial + Σ已关闭 P&L + 未实现 P&L"（每 bar，无方向限定）。
代码 nav()（line 103-112）：多头 `units×c`（逐市）+ 空头 `capital`（**开空时点冻结，非逐市**）。

空头未实现 P&L = (basis−c)×units **递延到回补**才物化（line 101-102 注释）。
**误差双向**（D5 核验修正了原 finding 的"仅低估"片面表述）：
- 空头**浮盈**（c<basis）：nav 漏计正项 → equity 峰值**低估**。
- 空头**浮亏**（c>basis）：nav 漏计负项 → equity **高估**（capital 冻结 > 真逐市）。
- 递延界 = `(−Σunits×basis, Σunits×basis]`，受 1x 逐仓强平守卫单边上封（line 472
  `capital+units×(basis−c)≤0` 即 c≥208 触发），回补时经 leftover(盈)/shrink(亏) 双通道归零。

**口径一致性**：nav() 全仓 5 处引用（nested_fugue:689、unified_recursive:466、
recursive_nested_fugue:100/160/443——D5 核验修正原 finding 的"仅3处"计数），三引擎 bit-exact
复用同一函数 ⇒ **R4(MDD) 跨变体同口径，比较公平**；但 MDD 绝对值在空头相位**双向扭曲**
（非一律低估）。终态 final_nav（全回补后）正确（line 699）——GAP 是**逐 bar 口径**，非终态错误。

---

## 4. 结果包六要素

1. **结论**：会计双重性核心实装正确（D1 存一次/视图导出 + D6 递归三层守恒 = CONFORMS,
   L0-canonical）；规格 §8 四条无条件不变量（§8.3/§7/§8.4/§1）系统性声明膨胀，仅理想化分支
   成立。统一根因 = 双重性"同一数字两身份"实为**单一 leftover 的四去向分裂 + 多空构造不对称**。

2. **定义依据**：538号（会计双重性 child.P&L≡father.cost_reduction）/ 540号（同一-差异双重性
   根谱系，压缩↔展开）/ §8 五不变量（`docs/nested_fugue_accounting.md`）。审计输入 = nested_fugue.rs
   会计原语逐行追踪 + cargo test 数值断言 + 对抗核验独立重追踪。每个 verdict 锚定具体行号 + 数值轨迹。

3. **边界条件**：结论翻转条件——(a) 若守恒律本身改写则 D1/D6 翻转；(b) 若 §8.3 改为四去向守恒式
   或标注有效域={盈利∧shortfall=0}，则 D2 从 GAP 降为 CONFORMS（规格补全）；(c) 若空头 earning
   换凸性载体（期权）则 D4 的 L0 不可构造解除；(d) 若 nav 改为全方向逐市则 D5 解除（但会引入不存在
   的第二份持仓，D1 核验已判定 capital 形式才是物理单真值——故 D5 的"修复"方向是改规格 §8.4 而非改 nav）。

4. **下游推论（对 Phase 2-5 分阶段推进决定性）**：
   - **Phase 3（逐仓嵌套/双层记账）地基稳固**：D1+D6 CONFORMS ⇒ 双层记账 + 递归守恒已正确实装，可建。
   - **Phase 5（earning）撞 L0 墙**：D4 ⇒ 空头 earning 线性载体不可构造。任务 Phase 5"earning 多空
     对称"若按 §7 字面实装会撞墙——必须按代码现实（仅多头父增仓 / 空头仅计数）或换期权载体。
   - **Phase 2（多空对称）前提部分 L0-假**：D3+D4 ⇒ 多空在 earning + N 重定基维度**结构不对称**。
     任务"voice 方向是参数，一套逻辑多空镜像"成立于**金额守恒/极性翻转**（D1/D6 对称），但**不**成立于
     **earning 构造/N 重定基**（D3/D4 不对称）。"一套逻辑"须分层：守恒层对称 ∧ earning 层不对称。

5. **谱系引用**：538号（会计双重性，本审计证其 §8.3 恒等仅 partial）/ 540号（同一-差异双重性根，
   cost_reduction 与 P&L"同一中的差异"的会计实例）/ `project_bidirectional_accounting`（空头单相单律
   非对称定理）/ `project_put_option_short_earning`（空头 earning 凸性载体消解）/ `project_nrf_v4_strict_accounting`
   （守恒零违反——本审计揭示守恒守卫验"记账一致"非"N 恒仓"）/ 002号（源不完备：§1 未覆盖亏损情形）。

6. **影响声明**：不改动**代码**（代码正确，其 doc 注释已自洽声明四去向 + 多空不对称）。产出 = 本审计报告
   + 规格修正（§5）。规格文档 `docs/nested_fugue_accounting.md` 的声明膨胀须修正（no-patch 090号：
   proven-false 声明不可留）——见 §5 修正方案。不改任何已结算谱系定义；审计为 538/540 补充"双重性的
   时间结构（恒等式仅回补时刻成立）"+ "四去向分裂"+ "多空构造不对称"三个未结晶细面。

---

## 5. 规格修正方案（no-patch 合规，保留原文 + 标注有效域）

规格 `docs/nested_fugue_accounting.md` 的四处声明膨胀须标注有效域（不删原文，保留谱系；
append 审计修正段，交叉引用本报告 + 谱系）。代码侧零改动（代码已正确）：

| § | 原声明 | 有效域修正 |
|---|---|---|
| §8.3 | child.P&L ≡ father.cost_reduction（无条件） | 仅 {盈利∧shortfall=0∧池足}；一般式 = P&L ≡ cost_pool_reduce + earning_excess − shortfall_loss（四去向守恒） |
| §1 | N 建仓后恒定，只 earning 增 | N_base = 运行态在手聚合（双向重定基：earning +Δ / 亏损回补 −δ）；§1"恒定"= 不主动加仓口径，非数量不变 |
| §7 | earning 多空完全对称 | 金额守恒对称 ∧ **资本化构造不对称**：多头父 earning 增 units（可构造）/ 空头父 L0 不可构造（线性载体），需凸性载体 |
| §8.4 | NAV 每 bar 含未实现 P&L（全方向） | 分裂：§8.4a 已实现逐 bar 精确；§8.4b 未实现 = 多头逐市 ∧ 空头摊余有界递延（界 (−Σu×basis, Σu×basis]，回补归零） |

**为何修规格不修代码**：D1 核验坐实——nav 的 capital 形式才是物理单真值（空头持现金，无独立
MtM 负债；MtM 形式反而引入第二份不存在的持仓）。代码的四去向 + 多空不对称是**物理真实**
（亏损股数不可逆消失、空头挣负股数不存在）；规格的无条件声明是 26课理想化口径未覆盖逆境分支。
修复方向 = 规格向代码看齐（090号声明膨胀禁止），非代码向规格看齐。
