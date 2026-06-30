---
id: "665"
number: 665
status: 生成态   # genealogist 结构记录：概念分离——「选择偏差消除」≠「可交易 alpha」，两命题逻辑独立。#101 walk-forward 决定性检验（train-only 挑类除偏差）产出双结论分裂：①Q2 BIAS-FATAL 翻案（选择偏差确被消除）②§11 稳健性否证（不达可交易 alpha）。除掉选择偏差 ⊬ 证明 alpha——只排除了一个伪因。peer 663（判据错误）/664（对象错配），同簇「判据/对象/方法论错配」。**不自结算**（概念层重大分离属编排者 /ritual）。
date: "2026-06-30"
type: concept-separation   # 概念分离：「除偏差」与「证 alpha」此前被隐式当作同一命题（"翻案=可交易"），本号分离为逻辑独立两命题。
negation_source: "cc（蜂群内部·经济正条件主线终点 #101 walk-forward 决定性检验，train-only 挑类除选择偏差）+ codex Q2 BIAS-FATAL 翻案 + §11 稳健性否证"
negation_form: "separation"   # separation：把「level0 卖 +3.47e4 in-sample 结果」当作单一命题（"是否可交易 alpha"），分离为两个逻辑独立对象——(a)选择偏差消除（train[0,180000) 独立挑出，非含 holdout 全样本挑赢家产物）vs (b)可交易 alpha（holdout LCB>0 + 多窗稳健 + 抗事件聚集）。前者翻案、后者否证，互不蕴含。

# 拓扑效果标注（147号下游推论3：negates 非空必填）
# 本号 negates 两个对象：codex Q2 BIAS-FATAL 的「全样本挑赢家产物」定性 / econpositive-OOS 主线的「OOS 稳健」隐含声称
# 以否定事件的实际拓扑后果（retrospective，141号结论1）判断，非 negation_form 预分类
topo_effect: "split:codex-q2-bias-fatal:local | freeze:econpositive-oos-robust-claim:downstream"
# split:codex-q2-bias-fatal:local — codex Q2 BIAS-FATAL 判定分裂：一个保留「全样本挑类是真实选择偏差风险（方法论警示成立）」，一个携带「level0 卖此例被 #101 train-only 检验翻案——train+2.19e4/n=219 独立挑出，非含 holdout 全样本产物」违反记录。仅 local（此例翻案），不否定 BIAS-FATAL 作为通用方法论警示。
# freeze:econpositive-oos-robust-claim:downstream — 冻结 econpositive-OOS 主线「OOS 稳健→可交易」的隐含声称及其下游（任何"除偏差即翻案即可交易"的叙事），等待 ChatGPT 反馈定 econpositive-OOS 方向后回溯解冻。冻结而非删除——选择偏差消除（①）是真实成立的结果，只是不蕴含 alpha；解冻条件=独立的 μ>0 长窗稳健证据（与 663 判据对齐）。

# 概念分离（type=concept-separation 必填）
separation:
  before: "「level0 卖 in-sample +3.47e4」被隐式当作单一命题：codex 判 Q2 BIAS-FATAL（全样本挑类选择偏差）⟹ 若除掉偏差（#101 train-only 挑类）翻案，则等价于证明可交易 alpha。即「除偏差」与「证 alpha」被当作同一件事的两面。"
  after:
    - name: "选择偏差消除（命题①，翻案成立）"
      definition: "level0 卖确是 train[0,180000) 区间独立挑出的赢家（train +2.19e4 / n=219），非「含 holdout 的全样本挑赢家」产物。codex BIAS-FATAL 在「是否为挑赢家产物」这一点上被翻案——选择确实发生在 train-only，holdout 未参与挑类。"
      source: "#101 walk-forward 决定性检验（train[0,180000) 挑类，holdout 独立验证）；codex Q2 BIAS-FATAL 原判（全样本挑类）"
    - name: "可交易 alpha（命题②，否证）"
      definition: "level0 卖不达可交易 alpha 阈值。§11 稳健性逐项否证：holdout LCB=−333≤0 / 多窗 LCB>0 占比 0% / neff/nraw=0.21（事件聚集 Σρk=1.88）/ 剔最大赢家 5 转负 / block bootstrap p=0.371 / holdout 仅 +7.87e3 不达阈值。"
      source: "§11 稳健性检验（holdout LCB / 多窗 / block bootstrap / 剔赢家 / 事件聚集 neff）；alpha.pdf μ(z,a)>0 + 抗稳健性判据"
  pending_verification: "ChatGPT 反馈定 econpositive-OOS 方向后：①是否存在独立的 μ>0 长窗稳健证据（与 663 判据对齐，非短窗符号检验）支撑高级别 level0 卖累积正期望？②方向不对称免疫（见下）能否在更多 (level,δ) 桶键上复现？"

# 涉及的定义
definitions_involved:
  - name: "选择偏差（selection bias / winner's curse）"
    version: "codex Q2 BIAS-FATAL 判定 + wc-proof-overfitting-winners-curse-lecam（663 related）"
    role: "命题①的对象。#101 用 train-only 挑类排除「含 holdout 全样本挑赢家」这一伪因。排除偏差 = 排除一个伪因，不 = 证明真信号。"
  - name: "可交易 alpha（μ(z,a)>0 + 稳健性）"
    version: "alpha.pdf μ(z,a)>0 判据（同 663 真判据）"
    role: "命题②的对象。§11 稳健性逐项否证。注：与 663 一致——判据是 μ>0，但此处 holdout LCB≤0/多窗 0%/block bootstrap p=0.371 表明即使按 μ>0 判据，此例 OOS 也未稳健成立。"
  - name: "方向不对称免疫（level,δ 桶键先验给定）"
    version: "econ-oos-level0sell 补充观察"
    role: "命题①的强化论据。(level,δ)=(级别×方向) 是缠论先验给定的桶键，非从收益挑出——卖优于买的方向不对称对选择偏差部分免疫，比泛泛的「OOS 稳健」更抗偏差。"

# 解决方式
resolution:
  type: 概念分离   # 分离「除偏差」与「证 alpha」两逻辑独立命题
  description: "把 level0 卖 in-sample 结果分离为两个逻辑独立命题：①选择偏差消除（翻案，train-only 独立挑出）②可交易 alpha（否证，§11 稳健性不达阈）。核心：除掉选择偏差只排除了一个伪因，不蕴含 alpha 存在——两命题逻辑独立（¬伪因 ⊬ 真信号）。方向不对称（卖优于买，桶键先验给定）对偏差部分免疫，是命题①的强化但不补救命题②。"
  decided_by: 蜂群内部   # genealogist 结构记录；概念层重大分离的最终结算待编排者 /ritual

# 被否定的方案
negated:
  description: "「codex Q2 BIAS-FATAL 翻案（#101 train-only 除偏差）⟹ level0 卖是可交易 alpha」——把「除偏差成功」等同于「证明 alpha」。"
  why_negated: "逻辑独立性：选择偏差是一个伪因（让样本内结果虚高），消除它使样本内结果回到无偏估计；但无偏估计仍可能是「无偏的零/负 alpha」。§11 正是如此——除偏差后 holdout LCB=−333≤0、多窗 LCB>0 占比 0%、block bootstrap p=0.371。¬(伪因) 不蕴含 (真信号)，因为还存在「无伪因但也无信号」的第三种情形。把翻案当证明 alpha = 排中律谬误（排除伪因 H_bias 不等于证明 H_alpha，因 H_null 同样满足 ¬H_bias）。"

# 新产出
new_output:
  definitions:
    - "选择偏差消除 ≠ 可交易 alpha：两逻辑独立命题（¬伪因 ⊬ 真信号）"
    - "命题①翻案：level0 卖 train[0,180000) 独立挑出（+2.19e4/n=219），非全样本挑赢家产物"
    - "命题②否证：holdout LCB=−333≤0 / 多窗 0% / neff/nraw=0.21 / 剔赢家5转负 / block bootstrap p=0.371 / +7.87e3 不达阈"
    - "方向不对称免疫：(level,δ) 是缠论先验给定桶键（级别×方向），非从收益挑→卖优于买对选择偏差部分免疫，比「OOS 稳健」更抗偏差"
  code_changes: "无（本号纯谱系产出。#101 walk-forward 检验代码已存在并产出结果）。"
  orchestration_changes: "方法论：除偏差检验（walk-forward train-only 挑类）证明的是「无选择偏差」，不证明「有 alpha」——两者须独立验证。任何「除偏差即翻案即可交易」的叙事是排中律谬误，须分离。"

# 影响范围
impact:
  affected_modules:
    - "econpositive-OOS 主线 → 「OOS 稳健→可交易」隐含声称被冻结（topo_effect freeze），待 ChatGPT 反馈定方向"
    - "#101 walk-forward 检验 → 结果重定性：证明命题①（除偏差），不证明命题②（alpha）"
  affected_definitions:
    - "663（生成态）：判据错误（统计显著性≠μ>0）。本号是其在「除偏差」语境的姊妹——663 说判据错，665 说即使判据对（μ>0），除偏差也不蕴含 μ>0 成立。"
    - "664（生成态）：对象错配（触发段≠交易腿）。本号与之同簇但不同维：664 测错对象，665 是方法论错配（把方法论检验结论误推为信号结论）。"
  downstream_implications:
    - "ChatGPT 反馈定 econpositive-OOS 方向时，须区分「方法论已干净（除偏差）」与「信号是否存在（alpha）」两问，不可合并。"
    - "方向不对称免疫可一并记入 econ-oos-level0sell：(level,δ) 桶键先验给定使方向不对称比泛 OOS 更抗偏差，是命题①的额外支撑。"
    - "若后续在更多 (level,δ) 桶键上 μ>0 长窗稳健成立（663 判据），则命题② 可能在特定桶键翻正——但 level0 卖此桶键当前 §11 否证。"

# 谱系关联
related_records:
  parent: "663号（判据错误——统计显著性≠可交易性）。665 与 663 姊妹：663 修判据（用 μ>0），665 指出即使用 μ>0，除偏差也 ⊬ μ>0 成立（OOS 稳健性是独立验证）。"
  children: []
  related:
    - "664号：对象错配（触发段≠反转交易腿）——同簇「判据/对象/方法论错配」，不同维（664 对象层，665 方法论层）"
    - "645号：簇根（π^cov≠π^bsp，测错对象）——665 是簇的方法论维成员"
    - "231号：形式化有效域规则——除偏差检验的有效域=「证无伪因」，不延伸到「证有信号」（有效域≠定义域）"
depends_on: ["231", "645", "663", "664"]

# 认识论等级标注（formalization-validity-domain 强制）
epistemological_levels:
  - proposition: "level0 卖是 train[0,180000) 独立挑出（非全样本挑赢家产物）→选择偏差消除"
    level: "L2（train-only 挑类 + holdout 独立 walk-forward，真实 BTC 数据，可否证；codex Q2 翻案）"
    increment: "正：否证了「全样本挑赢家产物」伪因，缩小偏差来源有效域"
  - proposition: "level0 卖不达可交易 alpha 阈值（§11 稳健性否证）"
    level: "L2/L3（holdout LCB / 多窗 LCB / block bootstrap p / 剔赢家 / 事件聚集 neff，真实数据多检验，否定性结果）"
    increment: "高：否定性结果——缩小 alpha 有效域边界（此 (level,δ) 桶键 OOS 不稳健）"
  - proposition: "「除偏差」与「证 alpha」逻辑独立（¬伪因 ⊬ 真信号）"
    level: "L0（逻辑必然：排中律——¬H_bias 不蕴含 H_alpha，因 H_null 同满足 ¬H_bias）"
    increment: "中：方法论分离的逻辑判定（同义反复但澄清此前隐式合并的谬误）"
  - proposition: "方向不对称（卖优于买）对选择偏差部分免疫——(level,δ) 桶键先验给定"
    level: "L0/L2（L0：桶键由缠论先验给定非从收益挑=结构上不引入选择偏差；L2：卖优于买的不对称在 BTC 数据观察到）"
    increment: "中：命题①的额外结构支撑（先验桶键比经验 OOS 更抗偏差）"

---

# concept-separation 665：「选择偏差消除」≠「可交易 alpha」（逻辑独立两命题）

## 一句话结论

经济正条件主线终点（#101 walk-forward 决定性检验）产出**双结论分裂**：①选择偏差**消除**（翻案）②可交易 alpha **否证**。核心概念分离——**除掉选择偏差只排除了一个伪因，不蕴含 alpha 存在**。两命题逻辑独立（¬伪因 ⊬ 真信号，因 H_null 同样满足 ¬H_bias）。peer 663（判据错误）/664（对象错配），同簇「判据/对象/方法论错配」，本号居**方法论维**。

## 双结论分裂

| 命题 | 结论 | 证据（L2/L3） |
|------|------|--------------|
| ① 选择偏差消除 | **翻案成立** | level0 卖确是 train[0,180000) 独立挑出（train +2.19e4 / n=219），非含 holdout 全样本挑赢家。codex Q2 BIAS-FATAL 在「是否挑赢家产物」被翻案。 |
| ② 可交易 alpha | **否证** | holdout LCB=−333≤0 / 多窗 LCB>0 占比 0% / neff/nraw=0.21（Σρk=1.88 事件聚集）/ 剔最大赢家 5 转负 / block bootstrap p=0.371 / holdout +7.87e3 不达阈。 |

## 逻辑独立性（为何①不蕴含②）

除偏差检验排除的是伪因 H_bias（"样本内虚高来自全样本挑赢家"）。但 ¬H_bias 有两种情形：
- (a) 真信号 H_alpha（无偏估计为正且稳健）
- (b) 无信号 H_null（无偏估计为零/负）

§11 落在 (b)：除偏差后 holdout LCB≤0、block bootstrap p=0.371。**把①当作②的证明 = 排中律谬误**——排除 H_bias 不等于证明 H_alpha，因 H_null 同样满足 ¬H_bias。

## 方向不对称免疫（命题①的额外支撑）

(level,δ)=（级别×方向）是缠论**先验给定**的桶键，**非从收益挑出**——这点比泛泛的「OOS 稳健」更抗选择偏差：从收益挑桶键会引入偏差，按缠论结构先验定桶键不会。观察到**卖优于买**的方向不对称，是命题①的结构支撑（偏差源被先验桶键结构性堵住），但**不补救命题②**（先验桶键 ⊬ 该桶键有 alpha）。

## 与 663/664 的簇关系（张力检查咬合）

| 号 | 错在哪 | 维 |
|----|--------|----|
| 645 | 测错对象（π^cov≠π^bsp） | 对象层（簇根） |
| 663 | 用错判据（统计显著性≠μ>0） | 判据层 |
| 664 | 测错对象（触发段≠反转交易腿） | 对象层 |
| **665** | **方法论错配（除偏差结论误推为 alpha 结论）** | **方法论层** |

- **vs 663**：姊妹关系。663 说判据错（该用 μ>0）；665 说即使判据对（用 μ>0），除偏差也 ⊬ μ>0 成立——OOS 稳健性是独立验证。663 修「用什么判据」，665 修「检验证明了什么」。可分层，无中断 #1。
- **vs 664**：同簇不同维。664 测错对象（触发段几何价差）；665 是方法论层（把方法论检验结论误推为信号结论）。664 的对象错配是「测什么」，665 是「测出来证明了什么」。可分层，无矛盾。
- **vs 645（簇根）**：665 是「判据/对象/方法论错配」簇的方法论维新成员。簇从「对象（645/664）+ 判据（663）」扩展到「+ 方法论（665）」。

## 边界条件（结论翻转）

- 若后续证明 #101 的 train/holdout 划分本身泄漏（如 holdout 信息进入挑类）→ 命题① 翻案被撤销（退回 BIAS-FATAL）。当前 train[0,180000) 与 holdout 时序隔离，无泄漏证据。
- 若在更多 (level,δ) 桶键上 μ>0 长窗稳健成立（663 判据，非短窗符号检验）→ 命题② 在那些桶键翻正，但 level0 卖此桶键 §11 已否证（六项独立检验一致）。
- 若 ChatGPT 反馈给出 level0 卖 OOS 稳健的新口径（如更长窗/不同 neff 校正）→ 命题② 重评，本号 freeze 解冻。当前 §11 六项检验一致否证，单一新口径难翻案。
- 若编排者裁定「除偏差即可交易」在缠论先验桶键语境下成立（先验桶键足以保证 alpha）→ 命题①②合并，本号降级。当前 §11 否证表明先验桶键不足以保证此桶键 alpha。

## 下游推论

1. ChatGPT 反馈定 econpositive-OOS 方向时，须分两问：方法论是否干净（除偏差，①已答是）+ 信号是否存在（alpha，②当前答否）。不可合并。
2. 方向不对称免疫记入 econ-oos-level0sell：(level,δ) 先验桶键比经验 OOS 更抗偏差。
3. 未来除偏差类检验（walk-forward / train-only 挑类）的结论须标注「证无伪因」而非「证有信号」——有效域=排除偏差，不延伸到证明 alpha（231）。

## 谱系引用

- 父：663（判据错误）——665 与之姊妹（663 修判据，665 修「检验证明了什么」）。
- 同簇：664（对象错配）/645（簇根，π^cov≠π^bsp）——本号居方法论维。
- 母规则：231（有效域≠定义域）——除偏差检验有效域=「证无伪因」，不延伸到「证有信号」。

## 影响声明

写入「选择偏差消除≠可交易 alpha」概念分离（生成态，不结算）。分离 #101 walk-forward 双结论：①除偏差翻案（L2 成立）②alpha 否证（L2/L3 否定性结果）。指出二者逻辑独立（排中律：¬伪因 ⊬ 真信号）。记入方向不对称免疫（先验桶键比 OOS 更抗偏差）。冻结 econpositive-OOS「稳健→可交易」隐含声称待 ChatGPT 反馈解冻。split codex Q2 BIAS-FATAL（仅 local 翻案，保留方法论警示）。不修改任何 settled 谱系（231 维持 settled）、不运行脚本、不改代码。最终结算待编排者 /ritual。

## 张力检查（019d/020）

### 检查范围（同轮蜂群 ∪ 1-hop ∪ Hub）
- 同轮蜂群：663（判据错误）/664（对象错配）/645（簇根）——经济正条件主线同簇。
- 1-hop：231/645/663/664/660（χ_t inconclusive，663 父）/656（细分类不劣）。
- Hub：231（有效域规则，度高）/645（判据/对象错配簇根，度高）。

### 张力1：vs 663——姊妹非冗余
663 修「用什么判据」（统计显著性→μ>0）。665 修「检验证明了什么」（除偏差≠证 alpha）。即使采纳 663 的 μ>0 判据，665 仍成立——OOS 稳健性是独立于判据选择的验证。两者咬合但维度不同（判据 vs 方法论结论），可分层，无矛盾。

### 张力2：vs 664——同簇不同维
664 测错对象（触发段几何价差非交易腿）。665 方法论层（除偏差结论误推为信号）。664 是「测什么」错，665 是「测出来证明了什么」错。两者均 231 有效域膨胀实例的不同投影，可分层，无中断 #1。

### 张力3：vs 645 簇根——扩展非冲突
645 是「测错对象」（π^cov≠π^bsp）。665 扩展簇维度从「对象+判据」到「+方法论」。665 与 645 一致深化（同为有效域≠定义域的实例），无矛盾。

### 递归运动结构完成检测（020）
- 第0层：本号写入（除偏差≠alpha 分离 + 方向不对称免疫）。
- 第1层：本号 × #101 双结论碰撞 → 逻辑独立性揭示（净新发现高：排中律谬误的精确定位）。
- 第2层：本号 × 663/664 碰撞 → 簇方法论维确认（净新发现中：簇从对象+判据扩到+方法论）。
- 第3层：本号 × 645/231 碰撞 → 有效域≠定义域同模式确认（净新发现骤降=背驰：231 已知母规则）。
- 涉及范围：scope₁(#101 双结论分离) > scope₂(663/664 簇维) > scope₃(645/231 母规则)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 概念分离的最终结算属编排者 /ritual，**不触发新 /escalate**（无定义冲突，是 concept-separation；与 663/664 同簇统一裁决）。

## 回溯扫描（职责3）

- **231（settled）**：本号实例化其论断（除偏差检验有效域=证无伪因，不延伸到证信号），不否定，扩展其适用域到方法论检验。维持 settled。
- **645/663/664（生成态）**：本号是其簇的方法论维新成员（peer），不修改其内容，不影响其待 /ritual 状态。与 663 姊妹咬合（663 判据维，665 方法论维）。
- **无 settled 被本号回溯破坏。** 本号是概念分离（concept-separation），辨认待编排者 /ritual，与 663/664 同簇统一裁决。
