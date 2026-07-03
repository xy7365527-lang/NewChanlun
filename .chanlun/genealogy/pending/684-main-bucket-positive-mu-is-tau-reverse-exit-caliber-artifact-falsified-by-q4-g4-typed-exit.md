---
id: "684"
number: 684   # 当前最大占用=683。撞车让向下一空号，最终编号 /ritual 统一分配。
title: "旧口径主桶正 μ̂（L0,type3,买,σ0 +178~185/笔 p=0.005，全战役唯一候选 alpha）被 q4 证伪为 τ^reverse 出场口径伪影——G4 typed exit 重装后同桶 mean −0.37/LCB −42.6，正点估计完全消失"
status: 生成态   # 概念级发现（唯一候选 alpha 被证伪）。negation 由 codex 裁定驱动的架构重装暴露。最终结算待编排者 /ritual。
date: "2026-07-03"
type: bias-correction   # 把「主桶正 μ̂=市场结构 alpha」订正为「出场口径伪影」——否定性结果，缩小有效域边界
source: "[新缠论] goal g-full-spec-pi 收口 + q4 对照结果包 056a50be35 §1 + codex-decide #122 G4 终裁"
negation_source: heterogeneous   # codex #122 G4 终裁（致命缺口拒中间态）驱动的架构重装（#134 删 τ^reverse）暴露伪影，#135 q4 对照实测坐实
negation_model: codex-cli
negation_form: separation   # 正 μ̂ 信号从「市场结构 alpha」路径切离——被揭示为 τ^reverse 出场口径的独立伪影路径
depends_on: []
related: ["231", "663", "665", "645", "660", "685", "project_wverify_alpha_retest_pass", "project_oddeven_mu_identity", "project_iclass_delta_collinearity_perm_degeneracy"]

# 拓扑效果标注（147号下游推论3）
# negates：旧口径结论「(L0,type3,买,σ0) 主桶正 μ̂ +178~185/笔 = 可交易市场结构 alpha」。
# 实际拓扑后果（retrospective，141号结论1）：正 μ̂ 不是市场结构，是 τ^reverse 出场规则（"下一个任意
#   反向信号"）与信号方向的系统性耦合制造的会计伪影。删 τ^reverse 接生产 π fill loop 后正号消失
#   ⟹ 该结论与"市场 alpha"的连接被切断（separation），它是出场口径的独立伪影路径。
#   scope=downstream：W-VERIFY 旧 PASS 主桶 / econ-663 μ̂ 表 / 奇偶交替正数字 均建立在旧出场口径上，随之作废。
topo_effect: "sever:wverify-l0-type3-buy-sigma0-alpha:downstream"

# 矛盾（旧口径正 μ̂ vs 重装后负 μ̂）
contradiction:
  description: |
    全战役唯一候选 alpha：桶 (L0, type3, 买, σ0) 在旧出场口径下 μ̂ = +178~185/笔（p=0.005，W-VERIFY
    五窗全正，见 [[project_wverify_alpha_retest_pass]]）。q4 对照（#135，结果包 056a50be35 §1）在 G4
    typed exit 重装后重测**同一桶**：mean = −0.37/笔、LCB = −42.6——正点估计完全消失，翻负。
    差异的唯一变量是出场口径：旧口径用 τ^reverse（"持有至下一个任意反向信号"）作出场，重装后删
    τ^reverse、接生产 π fill loop（typed exit）。
    概念结构：τ^reverse 出场规则本身制造正 μ̂——"下一个任意反向信号"作为出场时点，与进场信号方向存在
    **系统性耦合**（反向信号出现处天然是有利平仓点），这是出场规则与信号方向的会计耦合，不是市场结构
    赋予的可交易优势。删除该耦合后，桶回归其真实（负）期望。
  layer: 实装   # 出场口径是 backtest 出场逻辑（τ^reverse vs typed π fill），非缠论域定义、非 Lean 层
  trigger: "goal g-full-spec-pi 收口全实装后回测 → codex #122 G4 终裁（typed exit 致命缺口，拒中间态）→ #134 四 commit 删 τ^reverse 接生产 fill loop → #135 q4 对照同桶重测 → 唯一候选 alpha 翻负。"

# 涉及的定义
definitions_involved:
  - name: "可交易性判据 μ_net(z,a)>0（663号）"
    version: ".chanlun/genealogy/pending/663-criterion-error-...md（生成态）"
    role: "判据来源。旧口径 +178/笔 满足 p<0.05 且点估计正，曾被当作 alpha；重装后 μ̂<0 ⟹ 不满足 μ_net>0 ⟹ 不可交易。本号是 663「p<0.05≠可交易」的正面反例外加一层：连点估计的正号本身都是出场口径伪影。"
  - name: "τ^reverse 出场规则（下一个任意反向信号平仓）"
    version: "旧 backtest 出场逻辑（#134 已删除）"
    role: "被否定对象。τ^reverse 出场时点与信号方向系统性耦合 → 制造正 μ̂。这是会计伪影根源。"
  - name: "G4 typed exit（生产 π fill loop）"
    version: "commit #134 四 commit（重装后 canonical 出场口径）"
    role: "canonical 出场口径。codex #122 裁 typed exit 为唯一正确出场（拒 τ^reverse 中间态）。同桶在此口径下 μ̂=−0.37。"

# 解决方式
resolution:
  type: 定义修正   # 订正「主桶正 μ̂=市场 alpha」→「τ^reverse 出场口径伪影」，非概念分离
  description: |
    q4 对照（#135，结果包 056a50be35 §1）坐实：唯一候选 alpha 是出场口径伪影。旧口径主桶正 μ̂ 作废，
    W-VERIFY 旧 PASS / econ-663 μ̂ 表 / 奇偶交替正数字（[[project_oddeven_mu_identity]]）均建立在
    τ^reverse 旧口径上，随之作废。canonical 出场口径固定为 G4 typed exit（生产 π fill loop）。
    否定性结果价值（231号/formalization-validity-domain）：本号缩小了 alpha 有效域边界——全战役唯一
    正候选被否证后，当前无经 typed-exit 口径确认的可交易正 μ̂ 桶。这比确认性结果更有价值：它排除了
    一整条被出场口径污染的伪 alpha 路径。
  decided_by: 蜂群内部（codex #122 异质终裁驱动架构重装 + #135 q4 对照 L2 实测坐实）

# 被否定的方案
negated:
  description: "维持旧口径结论：(L0,type3,买,σ0) 主桶 μ̂=+178~185/笔 p=0.005 是全战役唯一候选可交易 alpha（W-VERIFY 五窗 PASS）。"
  why_negated: |
    (1) 唯一变量是出场口径：删 τ^reverse 接 typed π fill 后同桶 mean −0.37/LCB −42.6，正号完全消失
        ——正 μ̂ 由出场规则产生，非市场结构。
    (2) τ^reverse 出场时点与信号方向系统性耦合（"下一个任意反向信号"天然是有利平仓点）= 会计伪影，
        不是可交易优势。
    (3) codex #122 G4 终裁：typed exit 是唯一正确出场口径，τ^reverse 是被拒的中间态——旧口径结论的
        出场基础本身不合法。

# 新产出
new_output:
  definitions:
    - "出场口径伪影（exit-caliber artifact）：出场规则若其平仓时点与进场信号方向存在系统性耦合（如
      τ^reverse『持有至下一个任意反向信号』），会在 μ̂ 上制造与市场结构无关的正偏——回测正 μ̂ 必须在
      canonical typed exit 口径下复现才成立，τ^reverse 口径下的正号不可作 alpha 证据。"
    - "唯一候选 alpha 否证：全战役 (L0,type3,买,σ0) 主桶在 typed exit 下 μ̂=−0.37/LCB −42.6——当前无
      经 canonical 出场口径确认的可交易正 μ̂ 桶（否定性结果，缩小有效域边界）。"
  code_changes: "无（本号是概念级证伪的谱系补记；架构重装=#134 四 commit 删 τ^reverse，已由 backtest 工位落地）。"
  orchestration_changes: "方法论：回测正 μ̂ 结论的有效域必须绑定出场口径——换出场口径须重测，不能跨口径外推正号（出场口径是 μ̂ 的隐藏自由度）。"

# 影响范围
impact:
  affected_modules:
    - "rust/src/theta_v0/backtest（τ^reverse 出场已删，canonical=typed π fill loop）"
    - ".chanlun/review-results/econ-663-full-level-mu-*（μ̂ 表基于旧 τ^reverse 口径，须重标或作废）"
  affected_definitions:
    - "[[project_wverify_alpha_retest_pass]]：唯一强桶 L0/type3/买/σ0 五窗 PASS 建立在 τ^reverse 旧口径 → 作废，须用 typed exit 重测（已由 q4 给出：翻负）。"
    - "[[project_oddeven_mu_identity]]：奇偶交替正数字同样基于旧口径 → 正号作废（本就已被 perm_p=0.69 判为 beta 漂移伪结构，本号补充第二重否证：出场口径伪影）。"
    - "663/665/645 alpha 簇：本号是簇内又一否定性证据——选择偏差消除/统计显著/结构覆盖之外，出场口径是第四条污染 μ̂ 的路径。"
  downstream_implications:
    - "下游作废清单（Lead 明列）：① W-VERIFY 旧 PASS 主桶；② econ-663 μ̂ 表；③ 奇偶交替正数字。三者均须标注『基于 τ^reverse 旧口径，typed exit 下证伪』。"
    - "任何回测 μ̂ 正结论须声明出场口径 + 在 typed exit 口径下复现，否则不作 alpha 证据。"
    - "当前无经 canonical 口径确认的可交易 alpha——后续 alpha 搜索须从 typed exit 口径起（不复用旧口径任何正号）。"

# 回溯结算（如适用）
retroactive_settlement:
  settled_by: "编排者 /ritual 确认『主桶正 μ̂=出场口径伪影』订正 + 下游三项作废后，由 genealogist 回填 settled/。"
  settlement_date: null
  settlement_description: null

# 谱系关联
related_records:
  parent: "无直接父记录（首次将回测正 μ̂ 定性为出场口径伪影）。"
  children: []
  related:
    - "231（形式化有效域/否定性结果价值）：本号是否定性结果缩小有效域的实例——出场口径是 μ̂ 的定义域自由度，旧口径正号有效域不覆盖 typed exit 口径。"
    - "663（p<0.05≠μ>0）：本号更进一层——连点估计正号本身都可能是出场口径伪影，非仅显著性问题。"
    - "645（π^cov≠π^bsp）：alpha 簇根；本号是簇内 typed-exit 口径下的否定性收口。"
    - "[[project_iclass_delta_collinearity_perm_degeneracy]]：双 goal 终局四口径全 INCONCLUSIVE——本号在出场口径维补充：唯一非 INCONCLUSIVE 的正候选也被证伪。"
    - "685（一类买卖点全历史 0=市场事实被推翻）：同期独立发现，**无直接矛盾**——684 是出场口径维度
      （出场规则与信号方向的会计耦合），685 是信号生成候选维度（趋势门累积链+缺中枢延伸）。两者
      正交，样本互不覆盖（684 桶=type3，685 死点在环1 只影响 type1 候选；type2/type3 生成路径不
      经过趋势门，外部审查报告 §0 明确"第二类、第三类正常产出"）。共享同构方法论：全战役范围内
      「核心量化结论的异常值（恒正/恒0）优先假定为实现层缺陷，而非市场结构/事实」——张力检查判定：
      同轮产出、无待处理矛盾，仅作交叉引用，不生成新谱系记录。

# 认识论等级标注（formalization-validity-domain 231号，强制）
epistemological_levels:
  - proposition: "同桶 (L0,type3,买,σ0) 旧 τ^reverse 口径 μ̂=+178~185/笔 vs typed exit 口径 mean=−0.37/LCB=−42.6"
    level: "L2/L3（真实 BTC 数据 q4 对照，#135 结果包 056a50be35 §1，可否证，翻负=否定性结果）"
    increment: "高：唯一候选 alpha 的否证 + 有效域边界收窄"
  - proposition: "τ^reverse 出场时点与信号方向系统性耦合 ⟹ 制造正 μ̂（会计伪影）"
    level: "L0（出场规则与信号方向的结构耦合，概念推导）+ L2 佐证（删耦合后正号消失）"
    increment: "高：正 μ̂ 根因从『市场结构』订正为『出场口径耦合』"
---

# bias-correction 684：主桶正 μ̂ = τ^reverse 出场口径伪影（q4 证伪，typed exit 下翻负）

## 结论

全战役唯一候选 alpha——桶 **(L0, type3, 买, σ0)** 旧口径 μ̂ = **+178~185/笔（p=0.005，W-VERIFY 五窗全正）**
——被 q4 对照（#135）正式证伪为 **τ^reverse 出场口径伪影**。G4 typed exit 重装（删 τ^reverse、接生产
π fill loop）后同桶重测：**mean = −0.37/笔、LCB = −42.6**，正点估计完全消失、翻负。

概念结构：出场规则 τ^reverse（"持有至下一个任意反向信号"）本身制造正 μ̂——出场时点与进场信号方向存在
**系统性耦合**（反向信号出现处天然是有利平仓点），这是出场规则与信号方向的会计耦合，**非市场结构**。

## 溯源链

codex #122 G4 终裁（typed exit 致命缺口，拒 τ^reverse 中间态）→ #134 四 commit 删 τ^reverse 接生产
π fill loop → #135 q4 对照（结果包 `056a50be35` §1）同桶重测 → 唯一候选 alpha 翻负。

## 下游作废清单（Lead 明列）

| 作废项 | 原因 |
|--------|------|
| W-VERIFY 旧 PASS 主桶 | 五窗 PASS 建立在 τ^reverse 旧口径，typed exit 下翻负 |
| econ-663 μ̂ 表 | μ̂ 表基于旧 τ^reverse 出场口径 |
| 奇偶交替正数字 | 同基于旧口径（本已被 perm_p=0.69 判 beta 漂移伪结构，本号补第二重否证：出场口径伪影） |

## 边界条件（翻转）

- 若后续发现 typed exit 实装本身有 bug（π fill loop 漏成交/时点错配）导致 μ̂ 被人为压负，则本证伪需
  回退——但 codex #122 已裁 typed exit 为 canonical，举证责任在"typed exit 有 bug"一方。
- 本号不否定"存在可交易 alpha 的可能"，只否定"该特定主桶在旧口径下的正 μ̂ 是市场结构 alpha"。后续
  alpha 搜索须从 typed exit 口径起，不复用旧口径任何正号。

## 影响声明

概念级谱系补记，零代码改动（架构重装=#134，已落地）。当前无经 canonical typed-exit 口径确认的可交易
正 μ̂ 桶——否定性结果，缩小 alpha 有效域边界（231号）。最终结算待编排者 /ritual。**685号交叉检查**：
与685（一类买卖点全历史0=趋势门/中枢延伸缺陷）无逻辑矛盾，两者维度正交（出场口径 vs 信号生成候选），
已互相补充 related 引用，无需生成新谱系记录。
