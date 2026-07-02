---
id: "660"
number: 660
type: bias-correction   # 误判降级：把 χ_t 的 L3「无 alpha 否证」降级为 inconclusive（伪否证识别）+ inconclusive 根因分离（功效/短窗 vs 真无信号）
status: 已结算   # genealogist 结构记录。delta-r-alpha L3 否证经三轮异质审(codex)+P0口径修复后 CHECK_FAIL(events 209)降级 inconclusive。645 改对对象(π^bsp)后实测仍非 falsification 非 confirmation。新 goal f65436f2(LCB) 诞生依据。最终结算待编排者(escalate-delta-r-alpha-premature-checkpass.md A/B/C 待裁)。
settled_date: "2026-07-02"
settled_by: "genealogist via /ritual（编排者明令『并行全部推进』授权；裁决来源 codex 裁决①-⑤ + staging 归档）"
date: "2026-06-30"
source: genealogist（events.jsonl 198-209 delta-r-alpha L3 链 + codex-diagnose-20260630-deltar-l3.md 异质裁定 (B) + escalate-delta-r-alpha-premature-checkpass.md）
depends_on: ["231", "645"]
related: ["656", "657", "658", "level-amplitude-gate", "g-alpha-causal-selector", "g-20260630T155255Z-f65436f2", "project_zero_lookahead_backtest"]
negation_source: "codex gpt-5.5 三轮异质审(ev-delta-r-codex-audit-underpowered / ev-three-audit-verdict-pseudo-negative / ev-degeneracy-diag-underpowered-confirmed) + P0 口径修复(commit d50ed2fcfb 干净口径仍 inconclusive) + Lead CHECK_FAIL(events 209)"
negation_form: "separation"   # 把「L3 ΔR≈0 / p=0.5」这一单一观测分离为三个不同对象：falsification(真无 alpha) / confirmation(有 alpha) / inconclusive(功效不足，无法判定)——原 CHECK_PASS 误把 inconclusive 当 falsification

topo_effect: "split:acc-delta-r-alpha:{CHECK_PASS-as-falsification[adce1d93dd-被否,events209 CHECK_FAIL] | inconclusive-underpowered[采纳,n_L3=5功效不足+16K短窗高级别样本饥饿]} | separate:L3-ΔR≈0观测:{falsification真无alpha | confirmation有alpha | inconclusive功效不足[本观测落此]} | bound:inconclusive有效域=32000bar短窗+n_L3=5+裸μ估计，非外推'χ无alpha'(231否定性结果须缩小到实装条件) | downstream:f65436f2-LCB-goal=攻击inconclusive根因(过拟合)的工位，但根因主项=功效/短窗非过拟合"

# 矛盾（type=bias-correction：误判降级 + 否证强度分离）
contradiction:
  description: |
    acc-delta-r-alpha 一度 CHECK_PASS(commit adce1d93dd)，verifier 写「8品种 walk-forward 符号检验
    p=0.5 池均值≈0 = L3 否证（鞅§11）」——把 L3 观测当作 **falsification（χ 无系统性 alpha 成立）**。

    **codex gpt-5.5 三轮异质审 + P0 口径修复后判定：这是伪否证（pseudo-negative），应降级 inconclusive。**
    Lead 据此发 CHECK_FAIL(events 209)。三层证据：

    - **codex 攻击点3（功效不足，L0/L1）**：n_L3=5 的符号检验，需 5/5 全正才 p<0.05（P(X≥5|Bin(5,.5))
      =1/32≈0.031；4/5=6/32≈0.19）。3/5 正→p=0.5 是「没有足够功效拒绝 H0」**非**「确认无效应」。
      把功效不足升级成「确认无 alpha」= 系统性偏向否定。
    - **codex 攻击点4（短窗有效域偏置，L2/L3）**：MAX_BARS=32000，split 后 train/test 各 16K(~11交易日)。
      同项目 level-amplitude-gate(ev-level-amp-L3-pass) 已证 alpha 集中高级别(recL2/recL3)。16K 短窗
      train 估不出高级别 z 类 μ → test 高级别信号落 μ=None → treat_empty_as_pass=false → 静默不交易
      → alpha 来源静默丢弃。退化诊断(ev-degeneracy-diag) 坐实：BTC/ES/QQQ 空仓根因=16K短窗 μ 估计退化
      +高级别 z 样本饥饿(单样本类 count=1 占比高)，未见类仅6%(非覆盖饥饿)。
    - **P0 口径 bug（spec-execution-gap 036 实例，已修）**：l3 注释声称 ΔR=绝对增量(E_t−E_{t−1})/nav0，
      实装 delta_r_stats 用 daily_returns=百分比口径(E_t−E_{t−1})/E_{t−1}。χ/baseline 双轨道 live NAV
      sizing 发散 ⟹ 百分比配对差被路径依赖 sizing 污染。**P0 修复后(commit d50ed2fcfb)L3 裁定不变——
      仍 3/5 正，p=0.5，池均值+3.7e-6** ⟹ 口径 bug 之前未掩盖 alpha，修复=诚实化测量非追求正结果。

    **否证强度三分离（核心增量）**：单一观测「L3 ΔR≈0 / p=0.5」对应三个不同对象，原 CHECK_PASS 混淆：
    - **falsification（真无 alpha）**：H0 被有功效地接受——需 n 足够大、覆盖足够、估计不退化。
    - **confirmation（有 alpha）**：H0 被拒绝——本观测未达。
    - **inconclusive（功效不足）**：n_L3=5 + 16K 短窗 + 退化未分解 ⟹ 无法判定。**本观测落此。**
    诚实终态：「此实装下未检测到系统性 alpha」≠「alpha 不存在」。

    **与 645 的关系（下游续章）**：645 命题A 澄清 goal 六层穿透一直否证错对象(π^cov 覆盖投影)。本号是
    645 严格下一步(构造 π^bsp 离散择时 χ_t)**已实装并 L2/L3 测试后**的结果记录——goal 改对了对象
    (χ_t=1[μ(z)>θ] 因果选择器，F_λ 可测买卖点离散下单，causal-audit ev-causal-audit-E-pass 确认无
    lookahead)，但实测**仍非 falsification 非 confirmation = inconclusive**。即：645 修正了对象错误，本号
    揭示改对对象后**第一个 L2/L3 结果是 inconclusive 而非否证**——否证有效域从「π^cov」进一步明确到
    「π^bsp 在 32K 短窗 + n_L3=5 下功效不足，未检测到 alpha」。

    这不是定义冲突（无两条定义互斥），是**否证强度分离 + 误判降级**：把被当作 falsification 的观测
    分离为 inconclusive(功效不足)。分离后各对象有效域清晰、可分层 ⟹ 不触发中断 #1。
  layer: 实装   # rust/theta_v0/backtest l3_delta_r_alpha.rs 测试设计层(32K截断/n_L3过滤/符号检验)。非缠论定义冲突(χ_t 对象本身正确,645 已确认)，是 L3 测试的统计功效有效域 ≠ 声称的否证域。
  trigger: "acc-delta-r-alpha CHECK_PASS(adce1d93dd) 后，codex gpt-5.5 三轮异质审(攻击 underpowered/退化未分解/短窗偏置) + Lead 核 P0 口径 bug(036) + 口径修复后裁定不变 ⟹ Lead CHECK_FAIL(events 209) 降级 inconclusive。SUPERSEDE 到新 goal f65436f2(LCB 置信下界，攻击过拟合根因)。"

definitions_involved:
  - name: "codex「无 alpha」否定性结论异质审（gpt-5.5 xhigh，三轮）"
    version: ".chanlun/review-results/codex-diagnose-20260630-deltar-l3.md（总判定 (B) 否证有缺陷→inconclusive）"
    role: "降级依据。4 攻击点裁定：攻击点1 前视审计 L1 稳健(无 lookahead)；攻击点2/3/4(退化未分解/n=5功效/32K短窗)均系统性偏向否定 ⟹ 否证非 clean true-negative，应降级 inconclusive。异质审≠实施授权(memory feedback)——本号记录其裁定，最终 A/B/C 待编排者。"
  - name: "231 形式化有效域规则（L0/L1/L2/L3 + 否定性结果价值）"
    version: ".claude/rules/formalization-validity-domain.md（settled）"
    role: "约束 + 印证。231 核心：有效域可严格小于定义域；否定性结果价值=缩小有效域边界。本号 inconclusive 是 231 的精确实例——L3 观测的有效域=「32K短窗+n_L3=5+裸μ」，不是「χ_t 无 alpha」全域。把 inconclusive 当 falsification=有效域膨胀(231 禁止模式1：L0/L1 后声称已验证)。注意：inconclusive **不是** 231 意义的『否定性结果』(后者缩小边界有信息增量)——inconclusive 是功效不足，信息增量低于真 falsification。"
  - name: "645 命题A（π^cov≠π^bsp，goal 否证错对象）"
    version: ".chanlun/genealogy/pending/645（生成态）"
    role: "parent。645 澄清旧 goal 否证错对象(π^cov)。本号是 645 严格下一步(构造 π^bsp χ_t)实装后的结果——改对对象后第一个 L2/L3 = inconclusive。645 修正『测错对象』，本号记录『测对对象但功效不足』。同源续章，不矛盾。"
  - name: "level-amplitude-gate（alpha 集中高级别，L3）"
    version: "events ev-level-amp-L3-pass（recL2/recL3 转正，segment 级全亏<摩擦地板）"
    role: "短窗偏置根因。alpha 在高级别(recL2/recL3)，16K 短窗 train 估不出高级别 z μ → 高级别信号 μ=None 静默丢弃。本号 inconclusive 的物质根因=短窗杀掉 alpha 所在级别的样本。印证 codex 攻击点4。"

resolution:
  type: 未解决   # 否证降级已记录(falsification→inconclusive，伪否证识别 + 根因分离)。acc-delta-r-alpha 的 A/B/C 裁决(escalate-delta-r-alpha-premature-checkpass.md)=选择类待编排者。新 goal f65436f2(LCB) 攻击过拟合根因=行动类(工位推进中 #71-74)。最终结算待编排者。
  description: |
    概念澄清（已完成）：χ_t 的 L3 ΔR≈0/p=0.5 观测是 **inconclusive（功效不足）非 falsification（真无
    alpha）**。三轮异质审 + P0 口径修复坐实：根因=n_L3=5 功效不足(需 5/5 才显著) + 16K 短窗高级别样本
    饥饿(alpha 所在级别无样本) + 退化品种 μ 估计坍缩——**非**过拟合、**非**真无信号、**非**口径 bug(修后
    裁定不变)、**非**lookahead(攻击点1 L1 稳健)。

    **否证强度分离（核心增量）**：falsification / confirmation / inconclusive 是三个不同对象。
    「ΔR≈0 + p 不显著」单凭表面可被误读为 falsification，但须先核功效(n、覆盖、估计退化)——功效不足时
    它是 inconclusive。把 inconclusive 当 falsification = 把『无功效拒绝 H0』升级为『确认无效应』。

    **新 goal f65436f2(LCB) 的诞生逻辑**：新 goal 把 χ_t 从裸 μ>θ 升级为 LCB(μ)>θ(置信下界控过拟合)，
    acc-lcb-l2-vs-naive 设可证伪二选一：(a) LCB 减少退化品种(裸μ低n类过拟合被正确拒绝) 或 (b) LCB 不改善
    (则 inconclusive 根因非过拟合而是真无信号)。**genealogist 标注张力**：新 goal 假设 inconclusive 根因
    可能含『过拟合』，但本号三轮异质审的根因主项是『功效不足 + 短窗高级别样本饥饿』——LCB 控的是过拟合
    (低 n 类的乐观偏差)，**不直接解功效不足**(n_L3=5 本身太小)。LCB 可能减少退化品种(b 路径↔过拟合),
    但若 n_L3=5 不变，符号检验功效仍不足——sg-lcb-l2 须区分『LCB 改善退化品种数』(过拟合维度) 与
    『L3 符号检验是否仍 inconclusive』(功效维度)。两者正交，不可混为『LCB 解决了 inconclusive』。

    严格下一步(行动类，工位推进中)：(1) sg-lcb-estimator(#71 已完成) LCB 实装；(2) sg-lcb-l2(#74)
    L2 对比，须分离过拟合维度(退化品种数)与功效维度(n_L3/符号检验)。acc-delta-r-alpha 的 A/B/C 裁决
    (escalate-delta-r-alpha-premature-checkpass.md)=选择类待编排者，genealogist 不裁。
  decided_by: 待裁决   # 否证降级=codex 三轮异质审 + Lead CHECK_FAIL(已落盘 events 209)；acc-delta-r-alpha A/B/C=选择类待编排者；genealogist 结构记录否证强度分离 + 根因张力标注

negated:
  description: "(1) acc-delta-r-alpha CHECK_PASS(adce1d93dd)：L3 ΔR≈0/p=0.5 = χ 无系统性 alpha 否证(falsification 成立)。(2) goal 改对对象(π^bsp χ_t)后 L3 否证 ⟹ 缠论买卖点离散择时无 alpha。(3) 新 goal LCB 升级将解决 inconclusive(LCB 控过拟合=补功效)。"
  why_negated: "(1) codex 三轮异质审：n_L3=5 功效不足(需5/5才显著)+16K短窗高级别样本饥饿+退化未分解 ⟹ inconclusive 非 falsification。P0 口径修复后裁定不变(非口径掩盖)。Lead CHECK_FAIL(events 209) 已正式否定该 CHECK_PASS。(2) inconclusive≠falsification：『此实装(32K短窗/n_L3=5/裸μ)下未检测到 alpha』≠『缠论买卖点择时无 alpha』(231 有效域:不可外推到全域)。π^bsp 是否有 alpha 仍开放。(3) LCB 控的是过拟合(低n类乐观偏差)，根因主项是功效不足+短窗——LCB 减少退化品种(过拟合维度)≠补符号检验功效(n_L3=5不变则功效仍不足)。两维度正交，不可混。"

new_output:
  definitions:
    - "★否证强度三分离：falsification(真无 alpha，H0 有功效接受) / confirmation(有 alpha，H0 拒绝) / inconclusive(功效不足，无法判定)——单一观测『ΔR≈0/p 不显著』须先核功效(n/覆盖/估计退化)才能归类，不可默认 falsification。"
    - "伪否证(pseudo-negative)识别：否定性结论同样可能因方法缺陷(功效不足/截断偏置/退化未分解)而错——否证有缺陷时应降级 inconclusive 而非『否证成立』(codex 异质审职责=对抗否定性结论)。"
    - "χ_t L3 结果=inconclusive：645 改对对象(π^bsp 离散择时)后第一个 L2/L3 非 falsification 非 confirmation，根因=n_L3=5 功效不足 + 16K 短窗高级别样本饥饿(level-amplitude-gate)，非过拟合非真无信号非口径bug非lookahead。"
    - "根因维度正交：过拟合(低n类乐观偏差，LCB 控) ⊥ 功效不足(n_L3=5 太小，LCB 不解)。LCB 减少退化品种≠补符号检验功效。"
  code_changes: "无(genealogist 纯谱系产出，624 工具有效域 Read/Grep/Glob)。LCB 实装=sg-lcb-estimator(#71 已完成)/sg-lcb-selector(#73)；L2 对比=sg-lcb-l2(#74)。P0 口径修复已落盘 commit d50ed2fcfb。"
  orchestration_changes: "方法论：①否定性结论(『无 alpha』『不改善』)落盘 CHECK_PASS 前必先核功效——n/覆盖/估计退化是否足以有功效地接受 H0；功效不足时是 inconclusive 非 falsification(否则把『无功效拒绝』升级为『确认无效应』)。②否证的有效域=实装测试条件(短窗/样本量/估计器)，不外推到该对象声称代表的全域(231)。③升级方案(LCB)攻击某根因前须核该根因是否为主项——本号根因主项=功效不足，LCB 控过拟合不直接解功效，L2 对比须分离两维度(退化品种数 ⊥ 符号检验功效)，不可混为『升级解决了 inconclusive』(声明膨胀 090)。④inconclusive 是合法的 L2/L3 终态(有信息增量:它否定了『此实装下有 alpha』的乐观结论)，不必强行追求 falsification 或 confirmation。"

impact:
  affected_modules:
    - "rust/src/theta_v0/backtest/l3_delta_r_alpha.rs → L3 测试设计(MAX_BARS=32000 / n_L3 过滤 / 符号检验)有效域=短窗功效不足。沿用此设计的结论须标 inconclusive 而非 falsification。"
    - "g-alpha-causal-selector goal(已 SUPERSEDE) → acc-delta-r-alpha CHECK_FAIL(events 209)；A/B/C 裁决待编排者(escalate-delta-r-alpha-premature-checkpass.md)。"
    - "g-20260630T155255Z-f65436f2(LCB goal，当前) → acc-lcb-l2-vs-naive 须分离过拟合维度(退化品种数)与功效维度(n_L3/符号检验)；不可把『LCB 减少退化』当作『解决了 inconclusive』。sg-lcb-l2(#74) 工位约束。"
  affected_definitions:
    - "645(生成态)：本号是其下游续章(改对对象后第一个 L2/L3=inconclusive)。645 修正『测错对象』，本号记录『测对对象但功效不足』。不否定，续章。维持生成态。"
    - "656(生成态)：656 定理2『细分类不劣 V(Z)≥V(Y) 是 L0 表达力非盈利保证』；本号印证——细分类(高级别 z 类)的盈利识别须 L2/L3 μ>0 真实数据，但 16K 短窗高级别样本饥饿 ⟹ 无法估计高级别 μ ⟹ 表达力无法兑现为可测盈利。维持生成态。"
    - "657(生成态)：657 候选2(M24 因果可测性) + 候选1(L 标签标轴)；本号是 657 候选1『L 标签标轴』在否证强度维度的扩展——falsification/inconclusive 也须标(否证强度轴)。维持生成态。"
    - "231(settled)：本号 inconclusive=有效域实例，印证。但澄清 inconclusive≠231 的『否定性结果』(后者有功效缩小边界，inconclusive 功效不足)。维持 settled。"
  downstream_implications:
    - "★χ_t(π^bsp 离散择时)是否有 alpha = 仍开放问题(inconclusive，非否证非确认)。645 修正对象错误，本号揭示改对对象后第一个结果是 inconclusive——alpha 探索须先解功效(更大品种池/更长窗/高级别样本)。"
    - "新 goal LCB 的 acc-lcb-l2-vs-naive (b) 路径(LCB 不改善)若成立 ⟹ inconclusive 根因非过拟合；但即使 (a)(LCB 减少退化)成立，n_L3=5 功效问题仍在 ⟹ LCB 单独不足以把 inconclusive 升为 falsification/confirmation，须配合更大池/更长窗(O(n²) 性能墙是约束)。"
    - "level-amplitude-gate(alpha 集中高级别) + 本号(短窗杀高级别样本) 联合 ⟹ 严格的 L3 须长窗(估出高级别 μ)，与 O(n²) 性能墙(MAX_BARS=32000)冲突——这是 alpha 检验的工程 vs 有效域张力(待 LCB goal 或后续 goal 解)。"

related_records:
  parent: "645(命题A，π^cov≠π^bsp)——本号是其下游续章(改对对象 π^bsp 后第一个 L2/L3=inconclusive)"
  children: []
  related:
    - "645(生成态)：parent，对象修正 → 本号功效不足。同源续章。"
    - "656(生成态)：细分类表达力 L0≠盈利 L2/L3；本号短窗高级别样本饥饿=表达力无法兑现可测盈利。印证。"
    - "657(生成态)：候选1 L 标签标轴；本号扩展到否证强度轴(falsification/inconclusive 须标)。"
    - "658(生成态)：reducer 单调无 revoke ⟹ 错误 CHECK_PASS(adce1d93dd) 无法撤销，Lead 改用 CHECK_FAIL(events 209) + 故意停 4/5。本号是 658 提及的『acc-delta-r-alpha 过早 CHECK_PASS』的概念层结算(否证强度分离)，658 是其 SCHEMA 层(reducer 无 revoke)。正交补充。"
    - "231(settled)：inconclusive=有效域实例，印证(但 inconclusive≠231 否定性结果)。"
    - "level-amplitude-gate(events)：alpha 集中高级别——本号短窗偏置根因。"
    - "g-20260630T155255Z-f65436f2(LCB goal)：本号是其诞生依据(攻击 inconclusive 根因) + 张力标注(LCB 控过拟合≠解功效不足)。"
    - "memory project_zero_lookahead_backtest：零前视纪律——本号攻击点1(walk-forward 无 lookahead L1 稳健)印证 χ_t 因果纯净。"

epistemological_levels:
  - proposition: "acc-delta-r-alpha CHECK_PASS(L3 ΔR≈0=falsification) 被降级 inconclusive"
    level: "L0(否证强度逻辑：功效不足≠falsification) + L1(codex 攻击点1 前视审计稳健) + L2(退化诊断/口径核实)"
    increment: "高：误判降级(falsification→inconclusive)，伪否证识别"
  - proposition: "n_L3=5 符号检验需 5/5 全正才 p<0.05，3/5→p=0.5 是功效不足非确认无效应"
    level: "L0(二项分布统计逻辑) + L1(sign_test_pvalue 实装)"
    increment: "高：功效不足的定量判定(P(X≥5|Bin(5,.5))=1/32)"
  - proposition: "16K 短窗高级别(recL2/recL3)z 样本饥饿 → 高级别 μ=None → alpha 来源静默丢弃"
    level: "L2(退化诊断 ev-degeneracy-diag 实测：单样本类 count=1 占比高，未见类仅6%)"
    increment: "高：inconclusive 物质根因(短窗杀高级别样本，非覆盖饥饿)"
  - proposition: "P0 口径 bug(daily_returns 百分比 vs 绝对增量)修复后 L3 裁定不变(仍 3/5，p=0.5)"
    level: "L2(commit d50ed2fcfb 干净口径重跑)"
    increment: "中：口径 bug 未掩盖 alpha(排除『口径污染掩盖正结果』)，但修复=诚实化非追求正结果"
  - proposition: "LCB 控过拟合 ⊥ 功效不足(n_L3=5)——两根因维度正交"
    level: "L0(过拟合=低n类乐观偏差 vs 功效=样本量，不同机制)"
    increment: "高：防『LCB 解决了 inconclusive』声明膨胀(新 goal 约束)"
---

> **[/ritual 结算段 · 2026-07-02]** 裁决来源：codex 裁决①-⑤（`.chanlun/review-results/codex-ritual-*.md`）+ 编排者明令「并行全部推进」授权。判决全文见 staging：GRAMMAR §1-B（裁决⑤升格）。判决摘要：χ_t L3 inconclusive 成立（645 下游）。 **限定语（强制随行，脱落=声明膨胀090）：** 见 660 档案（裁决⑤ codex-ritual-resubmit-20260702.md）


# 660 χ_t L3「无 alpha」否证 → inconclusive：伪否证识别 + 根因分离（645 下游续章）

## 一句话结论

acc-delta-r-alpha 一度 CHECK_PASS 把「L3 ΔR≈0 / p=0.5」当作 **falsification（χ 无系统性 alpha 成立）**。
codex gpt-5.5 **三轮异质审**（总判定 (B) 否证有缺陷）+ **P0 口径修复**（commit d50ed2fcfb 干净口径裁定
不变）+ Lead **CHECK_FAIL**（events 209）坐实：这是**伪否证（pseudo-negative）**，应降级 **inconclusive**。
根因=**n_L3=5 功效不足**（需 5/5 全正才 p<0.05）+ **16K 短窗高级别样本饥饿**（alpha 所在级别无样本），
**非**过拟合、**非**真无信号、**非**口径 bug、**非**lookahead。这是 **645 命题A 的下游续章**——645 修正
「测错对象（π^cov）」，本号记录「改对对象（π^bsp χ_t 离散择时）后**第一个 L2/L3 结果是 inconclusive**」。

## 否证强度三分离（核心增量）

| 对象 | 含义 | 条件 | 本观测 |
|------|------|------|--------|
| **falsification** | 真无 alpha（H0 有功效接受） | n 足够 + 覆盖足够 + 估计不退化 | ✗（未达） |
| **confirmation** | 有 alpha（H0 拒绝） | p<0.05 | ✗（p=0.5） |
| **inconclusive** | 功效不足，无法判定 | n_L3=5 + 16K 短窗 + 退化未分解 | **✓ 落此** |

单一观测「ΔR≈0 / p 不显著」须先核功效（n/覆盖/估计退化）才能归类——默认 falsification = 把
「无功效拒绝 H0」升级为「确认无效应」（系统性偏向否定）。

## 与新 goal f65436f2（LCB）的张力标注

新 goal 把 χ_t 升级为 LCB(μ)>θ（置信下界控**过拟合**）。**genealogist 标注**：本号根因主项=
**功效不足 + 短窗高级别样本饥饿**，LCB 控的是**过拟合**（低 n 类乐观偏差）——两维度**正交**：

- LCB 可减少退化品种数（过拟合维度，acc-lcb-l2 的 (a) 路径）；
- 但若 n_L3=5 不变，符号检验功效仍不足（功效维度）。

⟹ sg-lcb-l2（#74）须分离「LCB 改善退化品种数」（过拟合）与「L3 符号检验是否仍 inconclusive」（功效），
**不可混为「LCB 解决了 inconclusive」**（声明膨胀 090）。LCB 单独不足以把 inconclusive 升为
falsification/confirmation，须配合更大品种池/更长窗（与 O(n²) 性能墙 MAX_BARS=32000 冲突）。

## genealogist 边界

genealogist 记录否证强度分离（falsification/inconclusive）+ 伪否证识别 + 根因维度正交标注。
acc-delta-r-alpha 的 A/B/C 裁决（escalate-delta-r-alpha-premature-checkpass.md）=选择类待编排者，
genealogist 不裁、不改代码。本号是 645 下游续章的结构记录。

## 张力检查（019d/020）

### 检查范围（同源 delta-r-alpha 链 ∪ 1-hop ∪ Hub）
- 同源链：events 198-209（delta-r-alpha L3 + codex 三轮审 + P0 口径 + martingale-guard + CHECK_FAIL）。
- 1-hop：645（parent）/231/level-amplitude-gate/658。
- Hub：231（有效域）/645（命题A）。

### 张力1：vs 645（parent，对象修正）——续章非冲突
645 澄清旧 goal 否证错对象（π^cov 覆盖投影）。本号是 645 严格下一步（构造 π^bsp χ_t）实装后的结果——
改对对象后第一个 L2/L3 = inconclusive。645 修正「测错对象」，本号记录「测对对象但功效不足」。同源续章，
两者一致（否证有效域逐步收窄：π^cov 无 alpha → π^bsp 在 32K 短窗下功效不足未检测到）。**无矛盾。**

### 张力2：vs 231（settled，有效域）——印证 + 澄清
本号 inconclusive 是 231 有效域实例（L3 观测有效域=32K短窗+n_L3=5，非「χ_t 无 alpha」全域）。
**澄清**：inconclusive **不是** 231 意义的「否定性结果」——后者有功效地缩小边界（信息增量正），
inconclusive 是功效不足（信息增量低于真 falsification）。印证 231 不冲突。

### 张力3：vs 656（生成态，细分类表达力）——印证
656：V(Z)≥V(Y) 是 L0 表达力非盈利保证，盈利需 L2/L3 μ>0。本号：16K 短窗高级别样本饥饿 ⟹ 无法估计
高级别 μ ⟹ 表达力（细分类的高级别 z）无法兑现为可测盈利。印证 656（表达力 ⊥ 盈利）。

### 张力4：vs 658（生成态，reducer SCHEMA）——正交补充
658 是 SCHEMA 层（reducer 单调无 revoke ⟹ 错误 CHECK_PASS adce1d93dd 无法撤销，Lead 改用 CHECK_FAIL）。
本号是概念层（否证强度分离：为何该 CHECK_PASS 是伪否证）。658 记「无法撤销的机制」，本号记「为何须撤销」。
可分层（SCHEMA 机制 ⊥ 否证强度概念），正交补充。**无矛盾。**

### 概念分离信号检测（中断 #1）
检查：是否同一定义在不同上下文产出矛盾结论且不能分层？
- falsification vs inconclusive：不是同一定义的矛盾，是把被当作 falsification 的观测**分离**为
  inconclusive（功效不足）。分离后各对象有效域清晰、可分层。
- **无不可分层定义矛盾 ⟹ 不触发中断 #1。** 本号是 bias-correction（误判降级 + 否证强度分离），
  acc-delta-r-alpha A/B/C 走 escalate（已有 escalate-delta-r-alpha-premature-checkpass.md 待编排者）。

### 递归运动结构完成检测（020）
- 第0层：本号写入（否证强度三分离 + 伪否证识别 + 根因维度正交）。
- 第1层：本号 × 645 碰撞 → 改对对象后第一个 L2/L3=inconclusive（净新发现高：否证有效域从 π^cov 收窄到 π^bsp 功效不足）。
- 第2层：本号 × 231 碰撞 → inconclusive=有效域实例但≠否定性结果（净新发现中：231 实例已知，inconclusive 与否定性结果的区分是新）。
- 第3层：本号 × 656/658 碰撞 → 表达力无法兑现 / SCHEMA 正交（净新发现降：印证已立定义）。
- 涉及范围：scope₁(否证有效域收窄 645 续章) > scope₂(231 inconclusive 区分) > scope₃(656/658 印证)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** acc-delta-r-alpha A/B/C=选择类待编排者（escalate 已存在）；
  LCB goal 工位推进=行动类（#71-74）；本号是 645 下游续章的结构记录，**不触发新 /escalate**（无不可分层
  定义矛盾，A/B/C 已有 escalate 文档）。

## 回溯扫描（职责3）

- **645（生成态）**：本号是其下游续章（改对对象后 inconclusive），不否定，续章。维持生成态。
- **656/657（生成态）**：本号印证 656（表达力⊥盈利）+ 扩展 657 候选1（L 标签标轴→否证强度轴）。维持生成态。
- **658（生成态）**：本号是其概念层补充（为何须撤销 vs reducer 无 revoke 机制），正交。维持生成态。
- **231（settled）**：本号 inconclusive=有效域实例，印证（澄清≠否定性结果）。维持 settled。
- **无 settled 被本号回溯破坏。** 本号是 bias-correction（否证降级 falsification→inconclusive），
  acc-delta-r-alpha A/B/C=选择类待编排者（escalate-delta-r-alpha-premature-checkpass.md），LCB goal
  工位推进=行动类（#71-74，须分离过拟合维度与功效维度），最终结算待编排者。
