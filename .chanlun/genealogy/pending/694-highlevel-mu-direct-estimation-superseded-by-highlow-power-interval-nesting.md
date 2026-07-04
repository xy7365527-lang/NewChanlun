---
id: 694
number: 694   # 候选编号（当前最大占用=693 策略对象三分冻结，同 goal 姊妹轴）。最终编号 /ritual 统一分配。
title: "acc-highlevel-mu（稀疏高级别买卖点直接估 μ̂）被 acc-highlow-power（高级别定方向 + 低级别出执行样本，区间套统计版）扬弃承接——高级别 alpha 检验节点分裂为『方向/容器角色』⊥『执行/样本源』两正交层"
status: 生成态   # goal SUPERSEDE（events.jsonl line 293）携带的方法论扬弃落谱系；转 settled 待编排者 /ritual 追认 + acc-highlow-power 实装落地回溯结算。693 覆盖同一 goal 的对象冻结轴，本号覆盖 estimand/样本源轴，二者姊妹非重复。
date: 2026-07-04
type: bias-correction   # 订正 fullhist-multilevel goal 的 acc-highlevel-mu 隐含前提「高级别买卖点可在其自身稀疏样本上直接估 μ̂ 做 alpha 检验」——被证伪为稀疏无功效（Le Cam 硬墙），估计对象须重构。
source: "[新缠论] docs/formal-chain/问题1.pdf（16页《推导完全分类》终局裁定，INDEX.md 权威链）+ 编排者令 2026-07-04（『问题1.pdf 作为 goal 严格执行』）+ goal SUPERSEDE 事件 .chanlun/goals/events.jsonl:293（old=g-20260630T2010Z-fullhist-multilevel-3ecaf6 / new=g-20260704T1756Z-strategy-object-freeze-cbc36d，reason 明文『acc-highlevel-mu 被 acc-highlow-power 扬弃承接』）；被扬弃对象出处 = old goal acc-highlevel-mu（events.jsonl:250 acc 定义）+ acc-multilevel-sample 自陈 Le Cam 硬墙分支"
negation_source: heterogeneous   # 裁定依据=问题1.pdf 外部形式化推导文档（编排者 INDEX.md 权威链）的直接应用；无外部模型 session 裁量。
negation_model: null   # 同 690 口径：编排者既有权威链裁定（问题1.pdf）的直接应用，非 codex/gemini 模型裁量。
negation_form: aufhebung   # 否定（稀疏高级别自估 μ̂ = 无功效，acc-highlevel-mu 的直接估计路径）+ 保留（高级别买卖点确是编排者关注的 alpha 方向候选，价值不否）+ 提升（分离为『高级别=方向/容器/过滤』⊥『低级别=执行/样本源』双正交层，区间套统计版）。内含 separation：高级别 alpha 检验空间分裂为方向层/样本源层。

# 拓扑效果标注（147号下游推论3：negates 非空必填）
# negates：fullhist-multilevel goal 的 acc-highlevel-mu 隐含前提——「高级别（level2+）买卖点可在其自身逐信号样本上直接估 μ̂ 并据此下 alpha 结论」。
# 实际拓扑后果（retrospective 141号结论1）：该前提被证伪——高级别买卖点结构性稀疏（几个月一次，n_eff≪n_min），
#   直接估 μ̂ 撞 Le Cam 功效硬墙（old goal acc-multilevel-sample 自陈的『level3+ 全历史 <30 ⟹ 结构性稀疏』分支坐实），
#   稀疏样本上『μ̂≤0』不构成否证、只能 INCONCLUSIVE（660/661 第三态）。
#   「高级别 alpha 检验」节点分裂：一支保留原意图（高级别 = 方向/容器/过滤角色，alpha 方向价值保留）；
#   一支携带违反记录（高级别自估 μ̂ = 无功效 Le Cam 硬墙，被否）。样本源边 re-anchor 到低级别执行层
#   （区间套统计版：高级别定方向 + 低级别提供执行与样本量 + 各桶报 n_eff/功效门对照）。
topo_effect: "split:acc-highlevel-mu-direct-sparse-estimation:downstream"
# split：高级别 alpha 检验节点分裂为『方向/容器角色（高级别）』⊥『执行/样本源（低级别）』两正交层；
#   稀疏高级别自估 μ̂ 分支携带违反记录（Le Cam 无功效）；样本源 re-anchor 低级别。
#   scope=downstream：影响 new goal acc-highlow-power 实装 + acc-tristate-rerun 复验口径（n_eff 不足桶只准 INCONCLUSIVE）。
depends_on: ["660", "661", "231"]
related: ["693", "663", "665", "645", "690", "647"]

# 矛盾（bias-correction 的被订正对象）
contradiction:
  description: |
    old goal g-...-fullhist-multilevel-3ecaf6 的验收 acc-highlevel-mu 直接要求：
    「高级别（level2+）买卖点逐信号 μ̂：全历史下 level2-4 各类 μ̂>0？train-only 挑类 +
    holdout（防选择偏差 663/665）；报高级别 holdout μ̂ 的 LCB>0 占比」——即在高级别买卖点
    **自身样本**上直接估 μ̂ 并据 LCB 下 alpha 结论。同 goal 的 acc-multilevel-sample 已自陈
    悬念：「若 level3+ 全历史仍 <30 → 高级别结构性稀疏（非窗口问题），确认 Le Cam 硬墙」。
    问题1.pdf《推导完全分类》裁定：高级别买卖点结构性稀疏（几个月一次），其自身样本 n_eff≪n_min，
    直接估 μ̂ 撞功效硬墙——稀疏样本上『μ̂≤0』既非 VALIDATED 也非 FALSIFIED，只能 INCONCLUSIVE
    （660/661 已结算：inconclusive 第三态 = 伪负 / 无功效，非真无信号）。故 acc-highlevel-mu
    的『直接估稀疏高级别 μ̂』路径无功效、不可下 alpha 结论——须重构估计对象。
  layer: 概念   # goal 验收方法论的 estimand 重构（估计对象/样本源）。非代码 bug：old goal 已 CLOSED（events.jsonl:267），本号订正的是被 SUPERSEDE 承接的验收前提，new goal acc-highlow-power 是承接实装。
  trigger: "编排者 2026-07-04 令问题1.pdf 作 goal 严格执行 → goal SUPERSEDE（old fullhist-multilevel / new strategy-object-freeze-cbc36d）reason 明文『acc-highlevel-mu 被 acc-highlow-power 扬弃承接』 → genealogist 检查扬弃是否携带概念增量 → 坐实（estimand/样本源轴，693 未覆盖）立本条。"

# 扬弃三环节（negation_form: aufhebung 必记）
aufhebung:
  negated: "acc-highlevel-mu 的『高级别买卖点在其自身稀疏样本上直接估 μ̂ + LCB 下 alpha 结论』路径——稀疏 n_eff≪n_min 撞 Le Cam 功效硬墙，稀疏样本 μ̂≤0 不构成否证（660/661 INCONCLUSIVE）。"
  preserved: "高级别买卖点确是编排者关注的真 alpha 方向候选（old goal 立意保留）——高级别的方向/容器/过滤价值不被否定，被否的只是『在其自身样本上直接估 μ̂』这一估计路径。"
  elevated: "acc-highlow-power（区间套统计版）：高级别桶只作方向/容器/过滤，低级别提供执行与样本量；输出报各桶 n_eff 与功效门对照。高级别 alpha 检验空间分离为『方向层（高级别）』⊥『样本源层（低级别执行）』两正交层——estimand 从『高级别自身 μ̂』重构为『高级别条件下低级别执行的 μ̂』。"

# 概念分离（aufhebung 内含）
separation:
  before: "『高级别 alpha 检验』被当作单一操作：在高级别买卖点自身样本上估 μ̂（acc-highlevel-mu）。方向判定与样本来源焊死在同一级别。"
  after:
    - name: "方向/容器/过滤层（高级别，level2+）"
      definition: "高级别买卖点提供交易方向、区间容器与过滤门——不承担样本量责任。高级别稀疏（几个月一次）在此层是特征非缺陷（它就该稀疏地定大方向）。"
      source: "问题1.pdf 高级别定方向裁定 + old goal 立意（高级别是编排者关注的 alpha 方向候选）。"
    - name: "执行/样本源层（低级别）"
      definition: "低级别在高级别方向条件下提供执行触发与统计样本量，μ̂ 在此层估计（n_eff 充足）。区间套统计版：μ̂(高级别方向条件下的低级别执行)。"
      source: "new goal acc-highlow-power（events.jsonl:292）：『高级别桶只作方向/容器/过滤，低级别提供执行与样本量；报各桶 n_eff 与功效门对照』。"
  pending_verification: "acc-highlow-power 实装后，低级别执行样本在高级别方向条件下的 μ̂ 是否 n_eff 达功效门且 LCB>0——若达标且方向条件真收窄有效，扬弃成立并回溯结算；若低级别执行 μ̂ 亦无稳健增量，则高级别 alpha 假设本身受质疑（回 645 wrong-object 检查）。"

# 涉及的定义
definitions_involved:
  - name: "acc-highlevel-mu（被扬弃的 old goal 验收）"
    version: "g-20260630T2010Z-fullhist-multilevel-3ecaf6 / acc-highlevel-mu（events.jsonl:250）"
    role: "被否定的估计路径。直接在稀疏高级别样本估 μ̂——撞 Le Cam 功效墙。"
  - name: "acc-highlow-power（承接的 new goal 验收）"
    version: "g-20260704T1756Z-strategy-object-freeze-cbc36d / acc-highlow-power（events.jsonl:292）"
    role: "扬弃承接。高级别定方向 + 低级别出样本，区间套统计版，报 n_eff/功效门。"
  - name: "660/661 inconclusive 第三态（功效门 / 伪负 vs 真无信号）"
    version: ".chanlun/genealogy/settled/660、661"
    role: "约束来源。稀疏高级别 μ̂≤0 = INCONCLUSIVE（无功效伪负），非 FALSIFIED——直接估计路径不可下 alpha 结论的判据依据。"
  - name: "231 形式化有效域规则"
    version: ".claude/rules/formalization-validity-domain.md（settled）"
    role: "母规则。样本 n_eff 不足 ⟹ 有效域收缩至 INCONCLUSIVE，L2 实测结论不可外推。"

# 结算状态
settlement_status: |
  概念层扬弃已辨认（estimand/样本源轴，693 未覆盖，二者同 goal 姊妹轴）。
  转 settled 待：(1) 编排者 /ritual 追认（同 693/690 口径）；
  (2) new goal acc-highlow-power 实装落地 + 低级别执行 μ̂ 的 n_eff/功效门实测（pending_verification 闭合后回溯结算）。

# 与 693 的关系（同 goal 双轴，非重复）
sibling_note: |
  693（策略对象三分冻结 + A11/A10 硬裁决）覆盖本 goal 的 acc-target-freeze 轴——
  「被检验对象在裁剪版/完整体间漂移」的对象冻结（Π_tested⊊Π_full 有效域纪律）。
  本号（694）覆盖 acc-highlow-power 轴——「高级别自估 μ̂ 无功效」的 estimand/样本源重构。
  两轴同源于问题1.pdf 但否定不同对象：693 否定「不冻结对象即下 alpha 结论」；
  694 否定「稀疏高级别自身样本可直接估 μ̂」。姊妹条目，互不蕴含。
---

# 694号：acc-highlevel-mu 被 acc-highlow-power 扬弃承接（高级别 alpha 检验分裂为方向层 ⊥ 样本源层）

## 一句话结论

编排者 2026-07-04 令问题1.pdf 作 goal 严格执行，触发 goal SUPERSEDE。除 693 记录的
**策略对象冻结轴**外，SUPERSEDE reason 明文点名第二半扬弃：old goal 的 `acc-highlevel-mu`
（在稀疏高级别买卖点自身样本上直接估 μ̂）被 `acc-highlow-power`（高级别定方向 + 低级别出
执行样本，区间套统计版）**扬弃承接**。本号锚定该方法论扬弃的出处链与三环节，693 未覆盖。

## 为何需要独立立号（而非并入 693）

693 覆盖 `acc-target-freeze`（对象三分冻结 + A11/A10 硬裁决 + 有效域定理），否定的是
「不冻结被检验对象即下 alpha 结论」。本号覆盖 `acc-highlow-power`，否定的是「高级别买卖点
可在其自身稀疏样本上直接估 μ̂」——**不同的被否对象、不同的拓扑效果**（693=对象冻结轴的
validity-domain；694=estimand/样本源轴的功效墙重构）。SUPERSEDE reason 两半分别对应两号，
姊妹非重复。

## 扬弃三环节

- **否定**：稀疏高级别自估 μ̂（n_eff≪n_min，Le Cam 功效硬墙；μ̂≤0 只能 INCONCLUSIVE，见 660/661）。
- **保留**：高级别买卖点作为 alpha 方向候选的价值（编排者立意不否）。
- **提升**：分离为「方向/容器/过滤层（高级别）」⊥「执行/样本源层（低级别）」两正交层；
  estimand 从「高级别自身 μ̂」重构为「高级别方向条件下低级别执行的 μ̂」（区间套统计版）。

## 结算路径

概念层扬弃已辨认。待编排者 /ritual 追认 + acc-highlow-power 实装落地（低级别执行 μ̂ 的
n_eff/功效门实测）后回溯结算。若低级别执行 μ̂ 亦无稳健增量 → 回 645 wrong-object 检查。
