# Codex 异质审计：经济正条件 ΣAb<0 = 对象错配（非实装 bug）

**模式**：diagnose（read-only，不改代码）
**日期**：2026-06-30
**调用**：`codex exec --skip-git-repo-check --sandbox read-only -c 'service_tier="fast"'`
**喂入上下文**：econ_positive.rs 全文 + economic-positive-condition-chain.md (L0) + econ-l2-btc-diagnosis-20260630.md (L2 结果) + bsp.rs:90-115 + signal.rs:90-105
**完整交互**：prompt=`/tmp/ab_audit_prompt.txt`，response=`/tmp/ab_audit_out.txt`（codex 爬全树，实际回答在文件末尾）

---

## codex 判定（四问逐答）

### Q1: bug 还是对象错配？
**对象错配（概念发现），局部实装不一致是症状。**

- 表层不一致：econ_positive.rs:200 `let eps = delta`（交易方向 δ），但其注释（line 8-9, 44）把 Ab 定义为笔自身方向 εb。
- 但「把 eps 改回 sign(Pρ−Pλ)」**只会恢复 Ab=|Pρ−Pλ|≥0 的同义反复**（PDF §1 的恒真语法），不会测出反转交易的理想价差。
- 根本问题：代码测的是**信号前的触发段**（pre-signal trigger segment），而缠论买卖点执行是**反转交易**，其经济机会从触发段端点**之后**才开始。

### Q2: locate_lambda_bar 定位的是什么段？
**背驰/触发/挂靠段（divergence/trigger/hang segment），不是离开段。**

- `BspPoint.source_index` = 段终止端点（signal.rs:99-105 seg_end）；`end_index==source_index` 的 LeveledMove = 终止于该端点的段。
- 对反转信号，其几何方向**内在地**与交易方向相反：底买出现在下跌段末端、顶卖出现在上涨段末端。
- 这个反平行关系**不是缺陷，正是反转信号之所以为反转信号**。

### Q3: 正确的 Ab 测量对象是什么？
**信号后的交易腿（post-signal trade leg），不是信号前触发段。**

精确 re-spec 形态：
```
g = 入场买卖点, δ(g) ∈ {+1,−1}
λ_rev(g) = source_index(g)           信号挂靠的反转 pivot 端点
ρ_rev(g) = source_index(h)           h = π^bsp 出场策略采用的首个后续反向 BSP
                                     （或策略声明的其他 post-signal 结构目标）
Ab_rev(g) = δ(g)·(P[ρ_rev(g)] − P[λ_rev(g)])
```
当前「出场=下一反向信号」配对下：买点 pivot → 下一卖点 pivot（卖镜像）。
若要中枢-目标规则，ρ_rev 必须是该声明的目标边界端点。
**不变量：ρ_rev 是 post-signal 且由策略 owned，不是触发段的起点。**

### Q4: 与 645/663 同级？
**是，同级 peer 概念分离，不是低层实装缺陷。**

干净的概念名：**触发段价差 ≠ 反转交易价差**，且 **笔方向 ε ≠ 交易方向 δ**。
ΣAb<0 是有价值的否定性结果——它暴露判据对反转信号测错了结构对象。
literal eps fix 会通过恢复同义反复来**掩盖**错配，不解决经济问题。

---

## 工位简化质询（判定否定是否成立）

1. **问题真实存在？** 成立。双交叉证据：(a) PDF §1 自定义 εb=sign(Pρ−Pλ) ⟹ Ab≥0 恒成立（chain.md:51-53 确认同义反复）；(b) econ_positive.rs:200 用 δ 而非笔方向 + locate_lambda_bar 定位 pre-signal 挂靠段（signal.rs/bsp.rs 确认）。两者叠加 ⟹ ΣAb<0 系统性。codex 诊断与代码/文档逐字吻合。
2. **已被其他机制覆盖？** 否。这是 econ_positive.rs 唯一 Ab 路径，无保护。
3. **严重性合理？** 合理。645=π^cov≠π^bsp（覆盖对象≠择时对象）；本发现=触发段对象≠反转腿对象——同构（都是「判据测错结构对象」），同级成立。

**工位补强（codex 未说透）**：codex 指出 literal eps fix「用恢复同义反复掩盖错配」——这正是 no-patch-mentality 禁止的补丁思维。正确解是改测量对象（ρ_rev=post-signal 腿），不是改 eps 符号。改符号会让 ΣAb 翻正但产出零信息（同义反复），是声明膨胀（090号）。

---

## 结果包（简化版）

**结论**：ΣAb<0 是真实的**对象错配概念发现**（peer 645/663），不是实装 bug。判据 Ab=εb(Pρb−Pλb) 隐含**顺笔交易**假设（εb=笔方向），但缠论买卖点是**反转交易**——把信号前「挂靠/触发段」端点价差当「结构理想价差」是对象错配。正确对象是 **post-signal 交易腿** Ab_rev=δ·(P[ρ_rev]−P[λ_rev])，ρ_rev 由 π^bsp 出场策略 owned。**需谱系记录**（concept separation，建议张力检查与 645/663 咬合）。

**边界条件（否定翻转条件）**：
- 若策略实为**顺笔**而非反转交易（εb=δ 同号），则触发段=交易腿，对象错配消失，ΣAb≥0 恢复——但缠论买卖点定义为反转（chain.md:47-48 顶/底分型端点），此条不成立。
- 若把 eps 改回 sign(Pρ−Pλ) 在同一段上：ΣAb 翻正但退化为同义反复（零信息）——形式上「修复」实质掩盖错配，不接受。

**影响声明**：不改任何代码（read-only 审计）。涉及模块：`rust/src/theta_v0/backtest/econ_positive.rs`（Ab 测量对象，line 98-104/200-201）。涉及定义：`.chanlun/proofs/economic-positive-condition-chain.md`（命题 S 的 Ab 对象需补充「顺笔 vs 反转」适用域限定）。L2 报告 `econ-l2-btc-diagnosis-20260630.md` 的 ΣAb<0 结论从「执行吃光价差」需重新归因为「判据测错对象」——失血三源对比的「结构无价差」一行解读失效。
