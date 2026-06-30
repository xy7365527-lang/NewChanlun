# Codex 异质审计：Ab_rev 反转交易腿对象正确性验证（664 号修复）

**模式**：review（read-only，不改代码）
**日期**：2026-06-30
**调用**：`codex exec --skip-git-repo-check --sandbox read-only -c 'service_tier="fast"' -c 'mcp_servers={}'`
**喂入上下文**：econ_positive.rs re-spec 后核心实装（信号收集 + 退出配对 + Ab_rev 计算）+ candidate_dir/source_index 结构事实 + L2 BTC 重测结果（ΣAb_rev=+7.66e4, Σcaptured=−2.33e5）+ per-class 分桶
**完整交互**：prompt=`/tmp/abrev_audit_prompt.txt`，response=`/tmp/abrev_audit_out.txt`

---

## codex 四问逐答

### Q1: ΣAb_rev=+7.66e4 翻正是真测对象还是隐藏同义反复？
**判定：Ab_rev 本体对象正确，非隐藏同义反复。**

- `Ab_rev = δ(Pρ−Pλ)` 不是 `|Pρ−Pλ|`——δ 来自 BSP buy/sell bits → VoiceSide（candidate_dir），不是从 Pρ−Pλ 回推。source_index 是买卖点端点位置，不是价差符号函数。
- **强证据**：`level=0, δ=+1` 桶 ΣAb_rev=−2.65e4 为负——若任何隐藏路径等价于 eps=sign(Pρ−Pλ)，则每项 Ab≥0，该桶总和不可能为负。足以排除「改符号成绝对值」的伪装修复。
- **补强（codex 提出的待验不变量）**：`exit_bar > entry_bar` 只保证「反向 BSP 事件 post-signal」，还应统计 `rho_rev_bar > lambda_rev_bar`、`lambda_rev_bar ≤ entry_bar`、`rho_rev_bar ≤ exit_bar`。若存在确认晚但 pivot 早于入场 pivot 的反向 BSP，则事件 post-signal 而价格腿不是正向时间腿。

### Q2: λ_rev/ρ_rev 配对是否正确对应反转交易腿？
**判定：配对大体对应反转信号腿；不是旧触发段，但不是完整实际账户持仓。**

- λ_rev=入场 BSP source_index，ρ_rev=首个后续反向 BSP source_index——已从「单个信号前触发段」换成「两次相反买卖点 pivot 之间的反转腿」。
- **关键排除**：用出场信号 pivot 是**理想出场端点**，不是「出场信号触发段另一端」——没有重新引入对象错配（这是审计最担心的重错配点，codex 明确排除）。
- 「pivot 端点价差（理想 Ab_rev）vs 确认 bar 执行价（η）」分离建模正确。
- **边界1（信号腿 ≠ 真实持仓腿）**：代码配对信号流首个后续反向信号，没有模拟账户状态/冲突排序/重复开仓拒绝/止损/RiskClose。= 「π^bsp-only 信号腿」，不严格等于完整 runner 真实成交持仓腿。
- **边界2（same-bar 反向）**：`eb > entry_bar` 严格大于排除了同确认 bar 反向信号。若 post-signal 按确认时间严格后续定义则合理；若策略允许同 bar 冲突序退出则漏掉。建议报告 `same_bar_opposite` 数量。

### Q3: Σcaptured=−2.33e5（执行滞后吃光）结论可信吗？
**判定：η 参照系正确；Σcaptured 是保守偏负口径，不等于实际 PnL。**

- ηin/ηout 参照点正确（入场相对入场 pivot，出场相对出场 pivot）。
- **核心发现（captured ≠ 实际 PnL，工位独立代数验证成立）**：
  ```
  x = δ(Pτin − Pλ), y = δ(Pρ − Pτout)
  实际无成本成交价差 = δ(Pτout − Pτin) = Ab_rev − x − y
  实装 captured = Ab_rev − max(x,0) − max(y,0) − Ce
  captured_impl − actual_pnl = min(x,0) + min(y,0) ≤ 0
  ```
  有利滑移被 max(0,·) 丢弃、不利滑移全计 ⟹ **captured 系统性低估实际 PnL**。
- 它是「理想反转空间被不利确认滞后吃掉多少」的**保守压力测试**，不能直接声明为真实执行 PnL。
- 若要支撑「执行滞后吃光」，需同时输出 signed `Σx/Σy` 与 adverse-only `Σx+/Σy+`。

### Q4: 664 号修复是否完整解决对象错配？
**判定：解决了 Ab 测量对象错配；完整性仍有边界。**

- 无配对出场信号时 `continue` 正确——没有 ρ_rev 就不应兜底末 bar，否则回到旧对象污染。
- **边界（右删失）**：丢掉窗口末端未平仓腿。对符号是否有系统性影响须报告 `n_unpaired` 按 δ/level 分布 + 末端 mark-to-market 敏感性。

**总判定**：Ab_rev 修复方向正确，已从「信号前触发段端点价差」切换为「post-signal 反向 BSP pivot 配对的反转信号腿」，per-class 负值排除了绝对值同义反复。但「完整正确解决」只限于 paired BSP signal-leg 的 Ab_rev；captured 当前是 adverse-only 保守分解，不是实际成交 PnL；需补三项审计统计（rho>lambda、same-bar 反向数、未配对删失分布）。

---

## 工位简化质询（判定 codex 否定/确认是否成立）

1. **Q1 对象正确判定成立？** 成立，工位独立确认。candidate_dir 把买卖点 bits 映射 δ（interp.rs:205-213），与价差符号无函数依赖；per-class ΣAb_rev<0（level0 δ=+1）是充分排除证据。codex 的时序不变量补强（rho>lambda）是真实边界但不动摇对象正确性——source_index=seg_end（signal.rs:101），确认 bar 因果晚于 pivot，多级别 pivot 交错时罕见可逆序，需统计而非假设。

2. **Q3 captured≠实际PnL 发现成立？** 成立，工位独立代数验证 `captured_impl − actual_pnl = min(x,0)+min(y,0) ≤ 0` 完全正确。这是 codex 本次审计**最有价值的产出**：L2 报告措辞「执行损耗吃光反转交易腿价差」偏强——准确说法是「在 adverse-only 最坏口径（丢弃全部有利滑移）下被吃光」。这不是 Ab_rev 对象问题（对象正确），是 **captured 分解的口径声明膨胀**（声明「执行吃光」实际只证明「最坏情况下吃光」，090 声明膨胀的弱版本）。

3. **Q2/Q4 边界是否已被其他机制覆盖？** 否。same-bar 反向数、未配对删失分布在当前实装无统计输出，L2 报告未报告这些边界——codex 要求的三项审计统计确为真实缺口。

4. **codex 是否误读上下文？** 否。codex 声明「没在 /private/tmp 树找到 econ_positive.rs，基于贴入实装审计」——这是 read-only sandbox 工作目录限制（cd /tmp），审计基于工位贴入的核心实装全文 + 工位核实的结构事实（candidate_dir/source_index），逐字吻合代码。无误判。

---

## 结果包（简化版）

**结论**：664 号对象错配修复**方向正确且对象正确**——Ab_rev=δ(P[ρ_rev]−P[λ_rev]) 真测了 post-signal 反转交易腿（非触发段），per-class ΣAb_rev<0 排除了同义反复伪装，ρ_rev 取出场信号 pivot 是理想出场端点（未重新引入错配）。ΣAb_rev=+7.66e4 翻正是真测对象的结果，不是改符号恢复 |·| 的伪装。**但发现一个口径问题（非对象错配）**：captured = Ab_rev − max(x,0) − max(y,0) − Ce 是 adverse-only 保守分解，工位代数验证 captured ≤ 实际 PnL，差值=被丢弃的有利滑移。「执行滞后吃光价差 Σcaptured=−2.33e5」的措辞偏强——严格说法是「最坏口径下吃光」。

**边界条件（结论翻转条件）**：
- 若实装存在 rho_rev_bar < lambda_rev_bar（事件 post-signal 但价格腿逆向时间），则部分 Ab_rev 测的不是正向时间腿——需统计验证此情形占比。占比非零则对象正确性局部受损。
- 若 same-bar 反向信号占比显著，则 `eb > entry_bar` 严格大于漏掉同 bar 出场，持仓区间被系统性拉长——需统计验证。
- 若改 captured 为 signed（不丢弃有利滑移）后 Σcaptured 翻正，则「执行吃光」结论被推翻——当前 adverse-only 口径下结论成立但偏保守。

**影响声明**：不改任何代码（read-only 审计）。
- 涉及模块：`rust/src/theta_v0/backtest/econ_positive.rs`（Ab_rev 对象正确，captured 口径为 adverse-only 保守版）。
- 涉及定义：664 号谱系（对象错配修复经异质源确认对象正确）。
- 需补强（行动类，待 Lead）：(1) 三项审计统计输出（rho>lambda 验证、same_bar_opposite 数、n_unpaired 按 δ/level 分布）；(2) L2 报告「执行吃光」措辞改为「adverse-only 口径下吃光」或补 signed Σx/Σy 对照；(3) captured 口径决策（保守 adverse-only vs 实际 PnL）属选择类，需谱系/编排者裁定。
