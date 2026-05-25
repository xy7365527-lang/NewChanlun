---
id: pending-005-orchestrator-proxy-decide-reverse-gap
timestamp: 2026-04-27
status: 已结算
settlement: 修正
settled_by: '090'
settled_date: 2026-05-24
settled_classification: 定理
settlement_scope: "SKILL.md 三处'待激活/未实现'声明同步到已实装代码；区分 decide可调用(已闭合) vs 自动事件路由(独立正向缺口,016号)"
type: bias-correction
negation_source: homogeneous
negation_form: expansion
topo_effect: "revise:.claude/skills/orchestrator-proxy/SKILL.md:local（原标 split，修正为 revise）"
---

## 结算（2026-05-24，定理类自动结算）

**决断四分法分类：定理**（090号 声明-能力一致性的反向实例，逻辑必然，无价值判断）。

**结晶四分法：修正**（SKILL.md 文档同步到已实装代码，pending 自身影响声明已指定此动作）。

**双侧验证（verification-before-completion，结算前实查）：**
- 代码侧确认 decide 完整实装：`newchan.gemini.__main__` choices=`{challenge,verify,decide,derive}`（实测 `--help` 确认）；`modes.py:decide()/decide_with_tools()`（L112/197）；`registry.py` decide ModeConfig（L295）。
- 声明侧确认萎缩：SKILL.md L102"CLI choices 尚未包含 decide...待激活"、L198"依赖 Task #2 完成后才能调用"、L202-209"decide 子命令路由 未实现"。

**执行的动作**：修正 SKILL.md 三处——(1) L98 调用命令 `newchan.gemini_challenger`→`newchan.gemini`（实测正确入口）；(2) L102/L198 移除"待激活/未实现"，标注 decide 已激活；(3) L202-209 限制表更新 decide 子命令为"已实现"。

**精确化（避免过度结算）**：反向缺口（decide CLI 能力 > 声明）已闭合。但**诚实区分**——decide *可调用* 已成立，escalate_choice → decide 的**自动事件路由**仍是独立的**正向缺口**（Claude Code 无语义事件总线，平台限制，与 016号"规则无代码强制就不执行"同模式）。本号只结算反向缺口（声明萎缩），不声称自动路由已解决。

**保留张力**：W-8 自标的同类清单（145/049/133 正向缺口）未一并结晶——若后续累积为"声明层↔代码层不对称"统一模式可升格。本号仅结算 decide 一例。

**影响声明**：改 `.claude/skills/orchestrator-proxy/SKILL.md`；不改 src/（代码本已正确）。本节点由生成态转已结算（修正）。

---

# orchestrator-proxy decide 反向缺口——代码已实装，skill 声明落后

## 矛盾

W-8 自标：041号编排者代理的 decide 子命令在代码层完整实装：
- `src/newchan/gemini/__main__.py:21` choices 已含 decide
- `modes.py` 全谱实装
- `registry.py` decide ModeConfig 完备

但 `.claude/skills/orchestrator-proxy/SKILL.md` L100-103 / L198 / L202-209（标注"353号消费断裂"）声明 decide "待激活"。

## 否定了什么

否定的是"声明—能力一致性"在反向（代码 > 声明）这一侧的对称性。
- 正向缺口（声明 > 能力）= 声明膨胀（090号已结晶）
- **反向缺口（能力 > 声明）= 声明萎缩**（W-8 暴露的新模式）

反向缺口的危害：041号路由路径在 runtime 可能读取 skill 声明误判 decide 不可用 → 回退到手动 challenge → 浪费 Gemini 调用 → 编排者代理协议事实失效。

## 推导链

- 041号：编排者代理（四分法路由扩展，选择/语法记录走 Gemini decide()）
- 090号：声明—能力一致性（严格性语法规则）
- 137号：声明层无效，需要正面格式——本号的反向（能力存在但声明否定）同样违反 137 精神
- W-8 实测：modes.py + registry.py + __main__.py 三处证据 vs SKILL.md L100-209 三处误声明
- 反向缺口与 016号"规则没有代码强制就不会执行"对偶——"代码没有声明覆盖就不会被路由"

## 谱系链接

- 041号（编排者代理）
- 090号（严格性 / 声明—能力一致性）
- 137号（声明层无效）
- 016号（规则代码强制）
- 049号（5 项待实现，包含 spec/theorems/ 目录不存在——同类反向缺口模式）
- 145号（Gemini 回复 ≤ 8KB 约束——文档层声明，代码无 enforcement，正向缺口）

## 影响声明

- 影响：`.claude/skills/orchestrator-proxy/SKILL.md`（L100-209 段需重写）+ 041 路由的 runtime 行为
- 改动：直接修改 SKILL.md，移除"待激活"声明，更新 353号消费断裂段——不是补丁，是事实更新
- 优先级：高——直接影响编排者代理协议的可用性

## 同类待澄清（W-8 自标 spec-execution gap 清单）

下列同样为反向/正向缺口，但暂不进入本号 pending（避免一次结晶过多）：
- 145 Gemini 回复 ≤ 8KB 约束（正向缺口）
- 049 spec/theorems/ 目录不存在（正向缺口）
- 133 双向收敛协议无执行器（正向缺口）
- 158 第三步撤回的边界漂移（语义模糊）

如果 145/049/133 后续被结晶为同一模式（"声明层与代码层不对称"），可考虑统一升格。

## 异质审计降级

本 session 全程 gemini-challenger 不可用。
