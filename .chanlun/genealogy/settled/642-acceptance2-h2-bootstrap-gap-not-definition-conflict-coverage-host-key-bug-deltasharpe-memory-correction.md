---
id: "642"
number: 642
status: 已结算   # genealogist 结构记录：误判降级（定义冲突/选择类 → 工程缺口/定理类）。codex异质 + codex-challenger 双向收敛坐实。【2026-07-02 回溯结算更新：ΔSharpe L2 重测已交付(commit 792305023b)，机制层 ΔN≠0(b2→b1)坐实、NAV 层 ΔSharpe=0.000(8/8)照实，开放子句关闭；裁决⑤已升格「成立」并入 staging §1-B。memory 修正已由 Lead 落盘。最终结算待编排者 /ritual。】
settled_date: "2026-07-02"
settled_by: "genealogist via /ritual（编排者明令『并行全部推进』授权；裁决来源 codex 裁决①-⑤ + staging 归档）"
date: "2026-06-29"
type: bias-correction   # 误判纠正：把误升级的"定义冲突/选择类"降级为"工程bug/定理类"
depends_on: ["639"]
related: ["638", "640", "641", "newchanlun-deltasharpe-zero-stale-rooting-perbar-reclass", "l2-falsify-dual-barrier-not-just-perf", "newchanlun-sigma-p-is-parent-container-not-held-leg"]
title: "acceptance[2] escalate『I5父结构live vs §13/639(c)父held仓位定义冲突』被codex异质+codex-challenger双向收敛降级为H2引擎自举缺口（定理类）；真因=coverage.rs:1253 host注入key错（注入候选同级host(c.level,source_index)而AncOK需候选parent_id高一级容器）——可在coverage.rs内修不改任何定义；连带修正 deltasharpe-zero-stale-rooting memory 把同一工程bug记为『真定义层冲突/编排者方向裁定（选择类）』的误判"
negation_source: "codex异质审查 + codex-challenger 双向收敛（H2 引擎自举缺口，非定义冲突）+ 源码事实（coverage.rs:1253 host注入用 (c.level, c.source_index) 同级 key，line 1262 ancestor_close_by_id 需候选 parent_id 高一级在场）+ testing-override.md 判据（不改定义即可修实现=实现错误）"
negation_form: "demotion"   # 把误升级的"定义冲突/选择类"降级为"工程bug/定理类"（非否定某条 settled，而是纠正一次误判 + 一条 memory）

# negation：两处误判被同一机器事实降级——
#   误判A（acceptance[2] escalate）：「I5（父结构live）vs §13/639(c)（父held仓位）定义冲突」。
#     →codex 双向收敛：639 已把 σ 来源（结构容器，§7.2）与持仓准入（§13 AncOK）分离为正交机制，
#       高级别走势可同时 structure-live + held，不冲突。真因是引擎只开 BSP 叶子腿、高级别容器从不持仓→死循环。
#   误判B（memory deltasharpe line 24）：「真定义层冲突：因果性 vs 身份连续性不可兼得…编排者方向裁定（选择类），非补丁可解」。
#     →codex 决定性裁决：held_registry_alive=100% 但 held_op_parent_alive=0=腿活父不活；pid 已 parent-path-independent（I3），
#       跨bar身份本应匹配→Y（substrate跨bar身份断裂/anc.pdf§16增量parser大架构）被否。
#     →真bug：coverage.rs:1253 host 注入 key 错——注入候选同级 host（c.level, c.source_index），
#       但 AncOK（line 1262 ancestor_close_by_id）需候选 parent_id（高一级 c.level+1 容器）在 raw。
#       是 (X) 工程最后一公里，可在 coverage.rs 内修，不改任何定义。

topo_effect: "demotes:acceptance2-escalate-as-definition-conflict; demotes:deltasharpe-memory-as-selection-class"

# 矛盾（type=bias-correction 必填）
contradiction:
  description: "两次产出把一个引擎自举工程缺口误判为概念层冲突，从而误归为『选择类』需编排者方向裁定（no-unnecessary-escalation 四分法）。误判A：工位C escalate 把『高级别走势 structure-live（I5/§7.2 σ 来源）vs 该走势未被持有（§13 AncOK）』判为定义冲突。误判B：deltasharpe-zero-stale-rooting memory（ab5f5a29d 修订后 line 24）把 depth>0 对冲腿零贡献的根因判为『因果性 vs 身份连续性在 held_leg_tree_index 值比较下不可兼得 = 真定义层冲突 = 编排者方向裁定（选择类）非补丁可解』。两者实为同一机器事实：高级别容器从不入 raw（open 集 100% 是 BSP 叶子点 lambda==rho==source_index）⟹ depth>0 子腿的真 Compose 父永不在 raw ⟹ AncOK 永剪 ⟹ active depth>0 腿=0 ⟹ ΔSharpe=0。codex 核实 held_registry_alive=100% 但 held_op_parent_alive=0（腿活父不活），pid 已 parent-path-independent（I3）跨bar身份本应匹配 → 否定『substrate 跨bar身份断裂』(Y)。真 bug 是 coverage.rs:1253 host 注入用错 key：`tree_endpoint_idx.get(&(c.level, c.source_index))` 取候选**同级**（c.level, c.rho）host，而 ancestor_close_by_id 要求候选的 parent_id（c.level+1 高一级 Compose 父容器）在 raw——注入同级 host 不满足高一级父在场，AncOK 仍剪 depth>0 腿。修复 = 注入候选 parent_id 容器（非同级 host），可在 coverage.rs 内完成，不改 §7.2/§13/639 任何定义。按 testing-override.md 判据（不改定义即可修实现=实现错误），这是定理类（实现错误，正常修复），非选择类（定义冲突）。"
  layer: 实装   # rust/theta_v0/strategy/coverage.rs 引擎自举层；非缠论域、非 Lean 形式化层、非定义层
  trigger: "本轮 goal g-sigma-complete-l2-nautilus 推进：工位G 实装容器入场后 L2 仍 active_depth>0=0，归因 Y（substrate 跨bar身份断裂，需 anc.pdf §16 增量parser大架构）。codex 核实推翻：held_op_parent_alive=0=腿活父不活，pid 已 I3 路径无关→Y 否；真 bug 在 coverage.rs:1253 host 注入 key 错（X 工程最后一公里）。codex-challenger 明确建议 genealogist 标注修正 deltasharpe memory 的『选择类/定义冲突』结论。"

# 涉及的定义
definitions_involved:
  - name: "639 σ_{p(g)} 来源 = 父容器方向（非持仓父腿）"
    version: ".chanlun/genealogy/settled/639-sigma-p-source-is-parent-container-direction-not-held-leg.md（status: 已结算）"
    role: "正交机制依据。639 已把 σ 来源（§7.2 结构容器方向，给买卖点分类角色）与持仓准入（§13 AncOK，决定 ShortDiff 腿能否被持有）分离为两个正交机制。本号印证 639：误判A（结构live vs 持仓冲突）正是把 639 已分离的两个正交机制误当冲突。639 line 79-81 已预言：『(a) iii-bridge 须 per-bar 因果递归塔=真工程缺口（非定义冲突）；(b) ShortDiff 持仓准入仍走 §13 AncOK，与 σ 来源分离』——本号是该预言的兑现实例。"
  - name: "spec §7.2 σ 来源 / §13 AncOK 持仓准入"
    version: "rust/src/theta_v0 spec（639 引用 line 367-381 / 659）"
    role: "约束来源。§7.2 给候选分类角色（结构对象，不依赖持仓）；§13 决定腿能否被持有（依赖父腿在 A_t）。两者正交。误判A 把两者当冲突 = 概念混淆（与 639 触发的混淆同构：用 §13 持仓判定算 §7.2 σ 来源）。"
  - name: "testing-override.md 判据（定义冲突 vs 实现错误）"
    version: ".claude/rules/testing-override.md"
    role: "分类判据。『如果你能在不改变任何定义的前提下修复实现 → 实现错误，正常修复；如果修复需要改变某条定义的含义/边界/适用范围 → 定义冲突，上浮』。本号两误判的修复（注入 parent_id 容器 / open 集补容器入场）均不改 §7.2/§13/639 任何定义 → 实现错误（定理类），非定义冲突（选择类）。"
  - name: "memory newchanlun-deltasharpe-zero-stale-rooting-perbar-reclass"
    version: "~/.claude/projects/-Users-silencehan/memory/newchanlun-deltasharpe-zero-stale-rooting-perbar-reclass.md"
    role: "被纠正对象。该 memory（ab5f5a29d 修订后）line 24 把 depth>0 对冲腿零贡献判为『真定义层冲突…编排者方向裁定（选择类）非补丁可解』，line 28 修复方向定为『父腿追踪/CoordDrift 语义』。codex 决定性裁决否定 substrate 身份断裂（Y），真因是 coverage.rs:1253 host key 错（X 工程 bug）。memory 的『选择类/定义冲突』结论是误判，须降级为『定理类/工程 bug』。**memory 由 Lead 维护（641 先例），本号产出修正标注供 Lead 写入，genealogist 不直接改 memory。【2026-07-02 更新：Lead 已落盘 memory 修正（Lead 消息确认「memory 侧我已更新」）。】**"

# 解决方式
resolution:
  type: 部分解决   # 【2026-07-02 更新】概念误判已诊断澄清（定义冲突→工程bug，选择类→定理类）；机制层修复已落地并经 ΔSharpe L2 重测坐实（commit 792305023b，ΔN≠0/b2→b1）；memory 修正已由 Lead 落盘。剩余：NAV 层零贡献照实（ΔSharpe=0.000 8/8，仅 NAV 层成立、净头寸层已证伪）；裁决⑤已升格「成立」入 staging §1-B；最终结算待编排者 /ritual。
  description: "概念澄清（已完成）：acceptance[2] 与 deltasharpe 零贡献均非定义冲突，是 H2 引擎自举工程缺口（定理类）。机器事实坐实：coverage.rs:1253 `tree_endpoint_idx.get(&(c.level, c.source_index))` 注入候选**同级** host，而 line 1262 ancestor_close_by_id 经 ancestors_by_id 要求候选的高一级 parent_id 容器在 raw——同级 host 不满足高一级父在场。严格修复（行动类，Lead 已派工位执行）：把 host 注入改为注入候选的 parent_id 容器（c.level+1 Compose 父），使 depth>0 子腿的真 Compose 父进入 raw，AncOK 不再剪。修复在 coverage.rs 内完成，不改 §7.2/§13/639 任何定义。memory 修正（Lead 已落盘）：deltasharpe memory line 24『真定义层冲突…编排者方向裁定（选择类）』改为『工程 bug：coverage.rs:1253 host 注入 key 错（注入同级 host 而 AncOK 需高一级 parent_id 容器）=定理类，可在 coverage.rs 内修不改定义』；line 28『父腿追踪/CoordDrift 语义』修复方向降级（CoordDrift/Stale 不是真根因，held_op_parent_alive=0 才是，pid 已 I3 路径无关身份本应匹配）。"
  decided_by: 蜂群内部   # codex异质+codex-challenger 双向收敛诊断 H2；genealogist 结构记录降级 + memory 修正标注；行动类修复已由 Lead 派工位执行并 L2 重测坐实；最终结算待编排者 /ritual

# 被否定的方案
negated:
  description: "(1) 保留 acceptance[2] escalate 为定义冲突（I5 父结构live vs §13/639(c) 父held仓位），走编排者方向裁定（选择类）。(2) 保留 deltasharpe memory『真定义层冲突…编排者方向裁定（选择类）非补丁可解』结论 + 『父腿追踪/CoordDrift 语义』修复方向。(3) 归因 Y（substrate 跨bar身份断裂，需 anc.pdf §16 增量parser大架构）。"
  why_negated: "(1) 639 已把 σ 来源（§7.2 结构）与持仓准入（§13）分离为正交机制——高级别走势可同时 structure-live + held，不冲突。escalate 把已分离的正交机制误当冲突 = 概念混淆（与 639 触发的混淆同构）。修复不改任何定义 → 定理类非选择类，escalate 为误升级（no-unnecessary-escalation：可自决问题不上浮，对偶面——误判定理为选择=反向违规）。(2)(3) codex 核实 held_registry_alive=100% 但 held_op_parent_alive=0=腿活父不活；pid 已 parent-path-independent（I3）跨bar身份本应匹配 → 否定 substrate 身份断裂（Y）。真因是 coverage.rs:1253 host 注入用错 key（X 工程最后一公里）。CoordDrift/Stale 不是真根因。把工程 bug 记为『定义冲突/选择类』= 误判分类（把定理类误升为选择类），违 testing-override.md 判据（不改定义即可修=实现错误）。memory line 24 的『非补丁可解』论断本身被否——本就可在 coverage.rs 内修。"

# 新产出
new_output:
  definitions:
    - "误判降级：acceptance[2]『I5 父结构live vs §13/639(c) 父held仓位定义冲突』= H2 引擎自举工程缺口（定理类），非定义冲突（选择类）。639 正交机制分离印证。"
    - "真 bug 定位：coverage.rs:1253 host 注入 key 错——注入候选同级 host（c.level, c.source_index），而 AncOK（line 1262 ancestor_close_by_id）需候选 parent_id（c.level+1 高一级 Compose 父容器）在 raw。"
    - "memory 修正：deltasharpe-zero-stale-rooting line 24『真定义层冲突/编排者方向裁定（选择类）』+ line 28『父腿追踪/CoordDrift 语义』修复方向被 codex 决定性裁决否定（held_op_parent_alive=0=腿活父不活，pid 已 I3 路径无关，Y 被否；真因 X=coverage.rs:1253 host key）。"
    - "误判模式：把工程缺口误判为定义冲突 = 反向越级上浮（把定理类误升为选择类）——no-unnecessary-escalation 四分法的对偶面违规。"
    - "【2026-07-02 L2 坐实增量】机制级已解决≠盈利（231 铁律）：ΔSharpe L2 重测（commit 792305023b）机制层 depth>0 流动 ΔN≠0（b2→b1，降级证据成立）、NAV 层 ΔSharpe=0.000（8/8 照实）。**口径限定：零贡献仅 NAV 净值层成立、净头寸层已证伪。**"
  code_changes: "genealogist 侧无（本号是概念误判降级诊断 + memory 修正标注，纯谱系产出）。coverage.rs:1253 host 注入改 parent_id 容器 = 行动类修复，已由 Lead 派有 Write 工位执行（超出 genealogist 工具有效域 Read/Grep/Glob，624 硬墙）；L2 重测坐实机制层 ΔN≠0（commit 792305023b）。"
  orchestration_changes: "方法论：①escalate『定义冲突』前必查正交机制是否已被既有谱系分离（639 已分离 σ 来源/持仓准入）——把已分离的正交机制当冲突=概念混淆。②归因『需大架构（Y）』前必先排除工程最后一公里（X）——codex 核实 held_op_parent_alive=0 直指 X，Y 是被否的过度归因。③testing-override.md 判据双向适用：不仅『定义冲突勿当实现错误硬修』，也『实现错误勿当定义冲突上浮』（反向越级）。④escalate-requires-l2-evidence 同理双向：L0 推测既不能据以判定冲突，也不能据以判定『需大架构』——本号 X 由源码事实（coverage.rs:1253/1262）+ codex 机器证据（held_op_parent_alive=0）坐实。⑤机制级修复坐实（trades/ΔN 变动）≠盈利坐实（NAV/ΔSharpe）——231 铁律的操作化：L2 重测须分层报口径（净头寸层 vs NAV 层），零贡献结论的有效域仅覆盖被证伪的那一层。"

# 影响范围
impact:
  affected_modules:
    - "rust/src/theta_v0/strategy/coverage.rs:1253 → host 注入 key 已改：注入候选 parent_id 容器（c.level+1 Compose 父），非同级 host（c.level, c.source_index）。修复后 depth>0 子腿真 Compose 父入 raw，AncOK（line 1262）不再剪。L2 重测坐实 ΔN≠0（b2→b1）。"
    - "rust/src/theta_v0/strategy/coverage.rs:1233-1257 → §8 引擎自举入口（开启候选时把 hostOf 容器加 raw）的有效域注释须核：当前注释（line 1243-1245）声称 host 命中其右端点即兑现 §8 持仓，但注入的是同级 host 非高一级 parent_id，未真正使 depth>0 子腿父在场。"
  affected_definitions:
    - "639（已结算）：本号印证其正交机制分离（σ 来源 §7.2 vs 持仓准入 §13），并兑现其 line 79-81 预言（per-bar 因果塔=工程缺口非定义冲突）。维持 settled。"
    - "memory newchanlun-deltasharpe-zero-stale-rooting-perbar-reclass：line 24『定义冲突/选择类』+ line 28 修复方向已降级为『工程 bug/定理类（coverage.rs:1253 host key）』。memory 由 Lead 维护，Lead 已落盘（2026-07-02）。"
  downstream_implications:
    - "acceptance[2] escalate 撤销（误升级）——按定理类（实现错误）正常修复，不需编排者方向裁定。"
    - "deltasharpe 8/8 否证的有效域不变（仍是保守欠对冲版，不否定完整 #5 alpha）——根因从『CoordDrift 身份判据/定义冲突』修正为『coverage.rs:1253 host key 工程 bug』。**修复后 L2 重测坐实（commit 792305023b）：机制层 depth>0 腿准入 ΔN≠0（b2→b1），NAV 层 ΔSharpe=0.000（8/8 照实）——零贡献仅 NAV 层成立、净头寸层已证伪。231 铁律兑现：修复≠盈利，ΔN 变动≠ΔSharpe 非零。**"
    - "未来 escalate『定义冲突』前查正交机制分离 + 排除工程最后一公里（X）后才归因大架构（Y）。"

# 谱系关联
related_records:
  parent: "639号（σ_p 来源 = 父容器方向，正交机制分离）——本号印证并兑现其工程缺口预言"
  children: []
  related:
    - "638号（hostOf 附着，端点命中非区间包含）：本号触及 coverage.rs host 注入 key（(level, ρ) 端点命中），638 是其附着规则来源"
    - "640号（同轮，函数性单值 vs 结构性单值）：不同轴（Lean 形式化层 vs rust 自举层），无矛盾"
    - "641号（同轮，增量塔 exp≈1 声明膨胀）：相关——641 的 centers.clone() 性能问题与本号 host key 正确性问题均在 per-bar substrate/coverage 路径，但 641=性能声明膨胀（exp）、本号=自举正确性（depth>0 腿准入），不同轴。两者均印证 deltasharpe 零贡献是多重独立缺口（性能 O(n) 已解 ≠ 自举 host key 正确，正交）。"
    - "memory l2-falsify-dual-barrier-not-just-perf：双重障碍（性能 + 身份）——本号细化『身份』障碍的真因是 coverage.rs:1253 host key（工程），非 CoordDrift 语义（定义）。"
    - "memory newchanlun-sigma-p-is-parent-container-not-held-leg（=639）：σ_p 口径本身对（本号印证），问题在自举 host 注入未让父容器入 raw。"

# 认识论等级标注（formalization-validity-domain 231号，强制）
epistemological_levels:
  - proposition: "coverage.rs:1253 host 注入用 (c.level, c.source_index) 同级 key；line 1262 ancestor_close_by_id 经 ancestors_by_id 要求候选高一级 parent_id 在 raw"
    level: "L0（源码事实：coverage.rs:1246-1257 + 470-482 + build_tree_endpoint_index keying (e.level, e.rho)）"
    increment: "高：host 注入 key 错配的结构判定"
  - proposition: "高级别容器从不入 raw（open 集 100% BSP 叶子点 lambda==rho==source_index）⟹ depth>0 子腿真 Compose 父永不在 raw ⟹ AncOK 永剪 ⟹ active depth>0 腿=0"
    level: "L2（codex 机器证据：held_registry_alive=100% 但 held_op_parent_alive=0；CL/BTC 32K 真实数据 depth>0 腿=0）"
    increment: "高：误判 Y（substrate 身份断裂）的否证 + 真因 X 的定位"
  - proposition: "639 已把 σ 来源（§7.2 结构）与持仓准入（§13）分离为正交机制 ⟹ 高级别走势可同时 structure-live + held，不冲突"
    level: "L0（谱系事实：639-sigma-p.md 矛盾精确形式表 + 正交机制双列表）"
    increment: "高：误判A（定义冲突）的否证判定"
  - proposition: "修复（注入 parent_id 容器）不改 §7.2/§13/639 任何定义 ⟹ 实现错误（定理类），非定义冲突（选择类）"
    level: "L0（testing-override.md 判据 + 源码可达性：coverage.rs 内可改 host 注入 key，无需触定义文件）"
    increment: "高：误判分类（选择类→定理类）的降级判定"
  - proposition: "修复落地后 depth>0 腿准入 ΔN≠0（b2→b1），但 NAV 层 ΔSharpe=0.000（8/8）——机制级已解决≠盈利，零贡献仅 NAV 层成立、净头寸层已证伪"
    level: "L2（ΔSharpe L2 重测机器证据：commit 792305023b / 642-delta-sharpe-retest-20260702.md，CL/BTC 真实数据 8/8）"
    increment: "高：口径限定的分层坐实——修复≠盈利（231 铁律），零贡献结论有效域收窄至 NAV 层"
---

> **[/ritual 结算段 · 2026-07-02]** 裁决来源：codex 裁决①-⑤（`.chanlun/review-results/codex-ritual-*.md`）+ 编排者明令「并行全部推进」授权。判决全文见 staging：GRAMMAR §1-B（裁决⑤升格，L2 证据齐备 commit 792305023b）。判决摘要：误判降级（定义冲突→工程缺口）成立；机制层 depth>0 流动 ΔN≠0（b2→b1）L2 坐实。 **限定语（强制随行，脱落=声明膨胀090）：** 限机制级已解决（≠盈利，231 铁律）；口径限定：零贡献仅 NAV 净值层成立（ΔSharpe=0.000 8/8）、净头寸层 ΔN≠0 已证伪


# 642 acceptance[2] H2 引擎自举缺口（非定义冲突）+ deltasharpe memory 误判修正

## 一句话结论

acceptance[2] 的 escalate「I5 父结构 live vs §13/639(c) 父 held 仓位定义冲突」与 deltasharpe-zero-stale-rooting memory 的「真定义层冲突 / 编排者方向裁定（选择类）非补丁可解」结论，被 codex 异质 + codex-challenger 双向收敛**降级为同一个 H2 引擎自举工程缺口（定理类）**。真因 = `coverage.rs:1253` host 注入用错 key——注入候选**同级** host `(c.level, c.source_index)`，而 AncOK（`line 1262` `ancestor_close_by_id`）需候选的**高一级** `parent_id`（c.level+1 Compose 父容器）在 raw；同级 host 不满足高一级父在场 ⟹ depth>0 子腿仍被 AncOK 剪 ⟹ active depth>0 腿=0 ⟹ ΔSharpe=0。这是 (X) 工程最后一公里，可在 coverage.rs 内修，**不改 §7.2/§13/639 任何定义** → 按 testing-override.md 判据是定理类（实现错误），非选择类（定义冲突）。

## 两处误判被同一机器事实降级

| 误判 | 原结论 | 分类 | codex 降级 |
|------|--------|------|-----------|
| A：acceptance[2] escalate | I5 父结构 live vs §13/639(c) 父 held 仓位「定义冲突」 | 选择类（需编排者方向裁定） | 639 已分离 σ 来源/持仓准入为正交机制→不冲突；真因引擎只开 BSP 叶子腿→定理类 |
| B：deltasharpe memory line 24/28 | 「因果性 vs 身份连续性不可兼得 = 真定义层冲突…编排者方向裁定（选择类）非补丁可解」；修复方向『父腿追踪/CoordDrift 语义』 | 选择类 | held_op_parent_alive=0=腿活父不活，pid 已 I3 路径无关→Y（substrate 身份断裂）被否；真因 X=coverage.rs:1253 host key→定理类 |

## 真 bug 的精确机器形式

```
coverage.rs:1246  let tree_endpoint_idx = build_tree_endpoint_index(&work[..tree_end]);
                  //                       ↑ keying = (e.level, e.rho)
coverage.rs:1253  if let Some(&host_idx) = tree_endpoint_idx.get(&(c.level, c.source_index)) {
                  //                                              ↑ 候选**同级** key（c.level）
coverage.rs:1262  let next_idx = ancestor_close_by_id(&work, &raw);
                  //  → ancestors_by_id 要求候选**高一级** parent_id（c.level+1）∈ raw
```

注入的 host 在候选**同级**（c.level），AncOK 要求的父在**高一级**（c.level+1）。同级 host 入 raw 不能满足高一级父在场要求 ⟹ 候选 depth>0 腿的真 Compose 父仍缺席 ⟹ AncOK 剪。修复 = 注入候选 `parent_id` 容器（c.level+1），非同级 host。

`build_tree_endpoint_index`（line 272-282）按 `(e.level, e.rho)` 建索引——同级唯一。`ancestor_close_by_id`（line 470-482）按 id 结构闭包要求全部祖先在 raw。两者口径一致（都是结构身份），bug 纯在 host 注入选错了层级。

## 为何是定理类（testing-override.md 判据）

- 修复 = 在 coverage.rs:1253 把 host 注入 key 从同级改为候选 parent_id 容器。
- 不改 §7.2（σ 来源）、§13（AncOK）、639（σ_p 口径）任何定义的含义/边界/适用范围。
- ∴ 实现错误，正常修复（定理类），不需编排者方向裁定。

误判 A/B 把它当『定义冲突/选择类』= **反向越级上浮**（把定理类误升为选择类）——no-unnecessary-escalation 四分法的对偶面违规。

## 回溯结算 addendum（2026-07-02 · L2 证据齐备）

**触发**：ΔSharpe L2 重测交付（commit `792305023b` / `642-delta-sharpe-retest-20260702.md`，task #6/#47）——本记录原开放子句「修复后 depth>0 腿可准入，ΔSharpe 重测有望非零（待 L2 坐实，不预设结果）」的坐实。

**分层坐实结果**：

| 层 | 度量 | 结果 | 结论 |
|----|------|------|------|
| 机制层（净头寸） | depth>0 流动 ΔN | **ΔN≠0（裁定 b2→b1）** | host key 修复生效，depth>0 腿真准入——误判降级证据成立，「零贡献」被证伪 |
| NAV 层（净值） | ΔSharpe | **0.000（8/8 照实）** | 净值层零贡献照实——修复≠盈利（231 铁律） |

**口径限定（强制随行入 settled，脱落=声明膨胀 090）**：**零贡献仅 NAV 净值层成立、净头寸层已证伪**。即：
- 「depth>0 腿零贡献」这一原结论的有效域**收窄至 NAV 层**；
- 净头寸层 ΔN≠0 已**证伪**「零贡献」——机制确实产生了持仓流动；
- 但机制级已解决 **≠** 盈利（ΔSharpe 仍 0.000）——231 铁律：修复≠alpha，ΔN 变动≠ΔSharpe 非零。

**状态更新**：resolution.type 未解决→**部分解决**（概念误判已澄清 + 机制修复已 L2 坐实 + memory 已由 Lead 落盘；剩余仅编排者 /ritual 最终结算）。裁决⑤已将 642 从原「证据不足待重裁」**升格为「成立」**并入 staging §1-B（限定语随行）。**开放子句关闭。**

## 张力检查（019d/020）

### 检查范围（同轮蜂群 ∪ 1-hop ∪ Hub）
- 同轮蜂群（639-643）：639（σ_p 来源，缠论域，settled）/640（单值性分层，Lean 域，settled）/641（exp≈1 声明膨胀，性能，生成态）/643（本轮 acceptance[1] guard，实装，生成态）/本号（acceptance[2] 自举缺口，实装，生成态）。
- 1-hop：639/638/testing-override.md/no-unnecessary-escalation/231。
- Hub：639（σ_p 正交机制，本轮高度节点）。

### ★张力1（任务点4）：vs 639——应一致，印证而非冲突
639 矛盾精确形式表把 σ 来源（§7.2，结构对象，不依赖持仓）与持仓准入（§13 AncOK，依赖父腿在 A_t）分离为两个**正交机制**。误判A（结构 live vs 持仓冲突）正是把这两个已分离的正交机制误当冲突。**codex 裁决印证 639**：高级别走势可同时 structure-live（§7.2 σ 来源给角色）+ held（§13 AncOK 准入），两机制正交不冲突。639 line 79-81 已预言「per-bar 因果塔=真工程缺口（非定义冲突）」「ShortDiff 持仓准入仍走 §13 AncOK，与 σ 来源分离」——本号 H2 缺口正是该预言兑现。**∴ 本号与 639 完全一致**（同向印证），无矛盾，不可分层冲突不存在。

### 张力2：vs 641——不同轴，正交缺口
641 = per-bar substrate 性能声明膨胀（centers.clone() 残留 exp 1.65）。本号 = 自举 host key 正确性（depth>0 腿准入）。两者均在 coverage/substrate 路径但不同轴：641 是性能（O(n) 已解 ≠ 真线性），本号是正确性（host key 选错层级）。共同印证 deltasharpe 零贡献是多重独立缺口。可分层，无矛盾。

### 张力3：vs 643（本轮 acceptance[1]）——同源不同点
643 = parser frontier 变异守卫的有效域膨胀（L1 合成 PASS 掩盖 L2 真实发散 + 投影有损守卫漏 reset）。本号 = coverage 自举 host key。两者都是「L1/合成 GREEN 掩盖真实缺口」的实例（与 625 同构），但 643 在 parser 守卫层、本号在 coverage 自举层，不同模块。可分层，无矛盾。

### 张力4：vs testing-override.md 判据——本号是其反向应用
testing-override 通常用于『定义冲突勿当实现错误硬修』。本号反向应用：『实现错误勿当定义冲突上浮』。同一判据双向，无矛盾——本号扩充其对偶面（反向越级也是违规）。

### 递归运动结构完成检测（020）
- 第0层：本号写入（acceptance[2] 降级 + coverage.rs:1253 host key + deltasharpe memory 修正）。
- 第1层：本号 × 639 碰撞 → 正交机制印证（净新发现：host key 选错层级是新的具体定位）。
- 第2层：本号 × testing-override 碰撞 → 反向越级模式（净新发现降：四分法对偶面已知）。
- 第3层：本号 × 641 碰撞 → 多重独立缺口同模式（净新发现骤降=背驰：性能/正确性正交已知）。
- 涉及范围：scope₁(639 印证+host key 定位) > scope₂(反向越级) > scope₃(多重缺口)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 修复=改 host 注入 key=行动类（定理类，超 genealogist 工具有效域，Lead 派工位）。memory 修正标注=待 Lead。本号是误判降级诊断，**不触发新 /escalate**（恰相反：撤销一次误升级）。最终结算待编排者 /ritual。

## 回溯扫描（职责3）

- **639（settled）**：本号印证其正交机制分离 + 兑现 line 79-81 工程缺口预言，不否定，维持 settled。
- **638（settled）**：本号触及 coverage host 注入（(level, ρ) 端点命中，638 来源），不否定，维持 settled。
- **640（settled，同轮）**：不同轴，维持 settled。
- **641（生成态，同轮）**：相关但不同轴（性能 vs 正确性），不破坏，维持生成态。
- **memory deltasharpe-zero-stale-rooting**：line 24『定义冲突/选择类』+ line 28 修复方向已被本号降级为『工程 bug/定理类（coverage.rs:1253 host key）』。**memory 由 Lead 维护（641 先例），Lead 已落盘（2026-07-02，「memory 侧我已更新」）。**
- **memory l2-falsify-dual-barrier / sigma-p-is-parent-container**：本号细化『身份』障碍真因（coverage.rs:1253 host key 工程，非 CoordDrift 定义）；σ_p 口径本身对（印证）。供 Lead 同步。
- **无 settled 被本号回溯破坏。** 本号是误判降级（bias-correction），修复=改 host key（行动类，Lead 已派工位并 L2 坐实）+ memory 修正（Lead 已落盘），最终结算待编排者 /ritual。
