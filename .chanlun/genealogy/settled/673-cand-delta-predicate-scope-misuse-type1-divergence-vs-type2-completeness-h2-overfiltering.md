---
id: "673"
number: 673   # 671 后续号（672 已被占用=dag.yaml 存根节点 title:'' + edge 672→623；按 668「撞车让向下一空号」用 673）。最终编号待 /ritual 由编排者在统一编号空间裁定。
type: bias-correction   # 结构分类被力度/极值谓词污染致过度过滤——615/671 同族。核心发现有缠论域根基（Type1/Type2 判据互斥），但签名=过滤偏差矫正对象，故 bias-correction。
status: 已结算   # 矛盾报告=区间套候选谓词 Cand^δ_ℓ 范围误用。修复方案（Cand^δ_ℓ 按 bsp 类型分叉）=选择类，待编排者 /ritual（Lead 2026-07-02 指名）。genealogist 不自裁修复分叉方案。
settled_date: "2026-07-02"
settled_by: "genealogist via /ritual（编排者明令『并行全部推进』授权；裁决来源 codex 裁决①-⑤ + staging 归档）"
date: "2026-07-02"
source: codex #7 异质裁决（H2-overfiltering-bug；裁决文件 .chanlun/review-results/codex-h1-ndelta-gate-20260702.md；对应 task #7「codex 异质确认 H1：N^δ 门滤中间级信号是定义正确的严格性还是过滤过严 bug」→ 裁定=过滤过严 bug）+ H2 样本级实证（.chanlun/review-results/h2-sample-verification-20260702.md，task #8 完成，见 §addendum）
negation_source: heterogeneous
negation_model: "codex-cli（异质源 OpenAI Codex），#7 裁决：H1 N^δ 门系统性假阴性的根因=Cand^δ_ℓ 候选谓词范围误用（非门阈值本身）"
negation_form: separation   # 统一范畴「Cand^δ_ℓ 候选谓词」内部暴露不兼容异质性——Type1 判据（背驰段/Extreme 创新极值）与 Type2 判据（走势完备性/「不患」，第17课）在 (m1,m2) 上结构性互斥，被单一谓词无差别套用。

topo_effect: "sever:cand-delta-unified-predicate:type2-branch"
# separation→sever（147号映射）：切断 Type2 与 Type1-Extreme 谓词路径的绑定，Type2 分支应走走势完备性判据独立路径。
# ⚠️ 本 sever 的 target=实装/spec 层的统一 Cand^δ_ℓ 谓词，非 settled 谱系节点——不否定 606（反而 606 已把有效域限定为 Type1，本 bug 是违反 606 scoping）。故 topo_effect 为语义标注，非对 settled 边的否定（negates settled-node=空）。

depends_on:
  - "606"   # ★上游权威——606 背驰/区间套完全分类明文「本号只覆盖趋势背驰⟹第一类 BSP」，有效域=Type1。本 bug=H1 N^δ 门无视 606 scoping，把 Type1 区间套候选谓词套到 Type2 上=违反 606 有效域标注。
  - "615"   # 同族：μ̂-f Layer1 ⊊ 缠论严格分类 Layer2。本号=该 subset 关系在「区间套候选谓词」层的实例（Cand^δ 力度/极值谓词 ⊊ 完整 Type1+Type2 结构判据）。
  - "671"   # 同族：P2 R2 MACD C≥A preveto 选择偏差。三号共模式=用力度/极值谓词污染结构分类致过度过滤（假阴性/预删）。
related: "[[2026-07-02-source-tracing-type2-panzhengbeichi-vs-maimai-def4]]（定义层上游root：maimai#4过度泛化为本bug提供定义正当化，genealogist裁定/ritual同批处理）"
  - "670"   # 区间套 location 条件性决定性（已裁并入 606）。Cand^δ_ℓ 是区间套候选谓词的实装形态；670 的「区间套对象=背驰段」在本号被证实装层未按 bsp 类型分叉。
  - "231"   # 形式化有效域：Cand^δ_ℓ 有效域=Type1（背驰段）< 定义域=all bsp（含 Type2）。有效域<定义域的又一实例（231 已 settled 为规则 formalization-validity-domain.md，本号=实例累积非新结晶）。
  - "017"   # 第17课 line 60（一级权威）：Type2 买卖点判据=走势完备性/「不患」，非创新极值。Type1/Type2 判据互斥的原文依据（溯源核实=source-auditor 职责，本号仅引 Lead 转述的 codex 裁决）。

# 涉及的定义（版本核实：606 已结算 Lean machine-checked；615 按 Lead 转述，路径待 source-auditor/编排者核）
definitions_involved:
  - name: "606号（背驰/区间套完全分类，已结算，Lean machine-checked）"
    version: ".chanlun/genealogy/settled/606-divergence-nesting-complete-classification-lconfirm-constructor-field"
    role: "★区间套=Type1（趋势背驰→第一类 BSP）的有效域权威。606 明文『本号只覆盖趋势背驰⟹第一类』『几何骨架/单支，有效域<定义域』。本 bug=实装把此 Type1 谓词无差别套到 Type2，违反 606 已标的有效域边界。"
  - name: "615号（μ̂-f Layer1 ⊊ 缠论严格分类 Layer2）"
    version: "（Lead 转述；settled/pending 路径待编排者/source-auditor 核实——settled/ 下无 615-*.md 文件名，谱系链接按编号）"
    role: "力度代理层 μ̂-f ⊊ 缠论结构分类的 subset 关系上游。本号 Cand^δ_ℓ 范围误用=该 subset 在区间套候选谓词层的具体致病实例。"

# 矛盾精确形式
proposition:
  claim: "H1 N^δ 门 level1-4 系统性假阴性的根因=Cand^δ_ℓ 候选谓词范围误用（非门阈值调参问题）。"
  mechanism: "区间套的对象=背驰段（Type1，判据=Extreme 创新极值）。Cand^δ_ℓ 把此 Extreme 判据无差别套用到 Type2 候选（Type2 判据应=走势完备性/『不患』，第17课 line 60 一级权威）。两判据在 (m1,m2) 上结构性互斥⟹凡真 Type2 候选（满足走势完备性但不创新极值）均被 Extreme 谓词预删⟹level1-4 系统性假阴性（over-filtering）。【§addendum 修正：H2 实证显示互斥（Extreme）仅占归零 17.45%，主因更深=Type2 结构段根本不具背驰段形状（方向/前驱），互斥链只是子集。】"
  epistemic_level: "L1（codex 异质裁决 + H2 样本级验证完成，task #8=.chanlun/review-results/h2-sample-verification-20260702.md，见 §addendum）；根因归属=异质确认（codex #7）+ 样本实证（H2）；修复后是否产生可交易 alpha=L2 未做（W-VERIFY #3/#4，须修复分叉后重跑）。"

# 四分法分类（Lead 指名）
classification:
  contradiction: "定理性发现——Cand^δ_ℓ 违反 606 已标 Type1 有效域，且与第17课 Type2 判据结构互斥。矛盾本身（范围误用致假阴性）由 codex 异质裁决 + H2 样本实证双重坐实。"
  fix: "【选择类】修复方案『Cand^δ_ℓ 按 bsp 类型分叉』（Type1 走 Extreme 判据 / Type2 走走势完备性判据）涉及候选谓词的结构性重构，多种分叉粒度/接口选择，需价值判断⟹待编排者 /ritual。genealogist 不自裁分叉方案。"

# 张力检查（genealogist 2026-07-02）
tension_check:
  family: "615（μ_f⊊缠论，Layer 层）→ 671（MACD C≥A preveto，MACD 面积层）→ 673（Cand^δ_ℓ 区间套候选谓词层）三号同族=用力度/极值谓词污染结构分类致过度过滤。净新维度（本号 vs 615/671）=Type1/Type2 判据在 (m1,m2) 上的具体结构互斥（Extreme 创新极值 ⊥ 走势完备性/不患），615 是一般 subset、671 是 MACD 单点、本号是区间套候选谓词的类型分叉缺失。带新维度⟹未背驰⟹family 计数累积，不触发结晶（模式已由 231 settled 规则覆盖，实例累积非新结晶）。【§addendum 加强净新维度：H2 证 Type2 结构段主因是方向/前驱不符（82.55%）而非 Extreme 互斥（17.45%），不兼容比 codex 识别的更深——净新维度巩固，仍未背驰。】"
  vs_606: "无矛盾——606 已把有效域限定 Type1，本 bug 是实装违反 606 scoping，606 是被违反的上游权威而非冲突对象。606 的 Lean 分类不受影响。"
  new_pending_genealogy: "无新矛盾涌现于本号与 606/615/670/671 之外；本号自身即该族的新实例记录。"

# 影响声明
impact:
  - "改动模块（待编排者裁定分叉方案后）：H1 N^δ 门的 Cand^δ_ℓ 候选谓词实装（level1-4 假阴性来源）。裁决文件 codex-h1-ndelta-gate-20260702.md。"
  - "不改任何 settled 定理/谱系/守恒律（606 Lean 分类不受影响——本号确认 606 有效域标注被实装违反，非否定 606）。"
  - "谱系效力：新增本条（生成态）。修复=选择类待编排者 /ritual；修复落地后本号可升级/结算。"
  - "下游：H2 样本级验证（task #8）已完成=本 L1 裁决的样本级证据链（gate_pass=0 坐实，见 §addendum）；W-VERIFY（task #3/#4）须在修复分叉后重跑（假阴性消除改变信号集与 μ 分布）。"
  - "memory 关联：project_level_hole_window_dependence（H1 门滤空、归因『未定』）——codex #7 + H2 实证共同将归因收敛为『div_cand 谓词层范围误用』（非 [J⊆J] 嵌套，reject_elsewhere=0）。归因已定，可更新该 memory（MEMORY.md 已由 Lead/linter 更新为『归因已定=H2范围误用』）。"

# ============================================================
# §addendum（机制修正——task #8 H2 样本级实证闭合，Lead 2026-07-02 交办）
#   源：.chanlun/review-results/h2-sample-verification-20260702.md（1473 真实信号，全窗）
# ============================================================
addendum_h2_mechanism_correction:
  # (1) codex 核心判定加强，但机制分解修正：互斥链只是子集
  finding_strengthened_mechanism_refined:
    verdict: "codex 核心判定（Cand^δ 范围误用致中间级假阴性）加强——1473 中间级信号 gate_pass=0（真 100% 归零坐实）。但拒绝按 codex 命名的『s_prev==m1 互斥链』单一机制解释。"
    decomposition: "归零三分（1473 全覆盖，Σ=100%，真封断言 Σ阶段=1473=n_total）——cond1_dir（m2 非背驰段要求方向 dir(m2)≠−δ）865=58.72% + cond2_noprev（父 sub_moves 内无同向前驱 s_prev）351=23.83% + cond3_extreme（Extreme 必假=codex 互斥链）257=17.45%。"
    interpretation: "Type2 结构段根本不具背驰段形状（方向/前驱）——82.55%（cond1+cond2）的信号根本到不了 Extreme 检验就被背驰段谓词的方向/前驱前件删除。codex 的互斥链（cond3）只解释 17.45%，是少数子集。⟹Cand^δ 与 Type2 的不兼容比 codex 识别的更深（不止 Extreme 互斥，主因是背驰段方向/前驱前件与第二类结构不兼容）。到达 cond3 的 257 例中 Extreme 必假 257/257=100%（互斥链在其子集内 100% 成立，但该子集仅 17.45%）。"
  # (2) 归零层定位：div_cand 谓词层，非 [J⊆J] 嵌套
  zeroing_layer:
    verdict: "归零发生在 div_cand 候选谓词层，非 [J⊆J] 区间套嵌套层——reject_elsewhere=0（base_none/no_upper/no_target/cond4_weak/嵌套拒绝全 0）。"
    acc_report_refinement: "对 acc 报告『门滤』表述的精化——不是区间套嵌套 [J⊆J] 把中间级滤掉，而是更上游的 div_cand 候选谓词层（cond1/cond2/cond3）在信号进入嵌套前就归零。修复靶点=div_cand 谓词层的 bsp 类型分叉，非嵌套逻辑。"
  # (3) codex 自留边界关闭
  codex_boundary_closed:
    verdict: "codex 自留边界（s_prev≠m1 gap，即『Extreme 可满足』样本）实证零实例——1473 中真实分布样本数=0，到达 cond3 的 257 个信号全部 Extreme 必假。可关闭该边界。"
    note: "实测 leg_gap 全为 1（257/257，s_prev 是 m2 在父 sub_moves 内紧邻同向前驱），codex 假设的 leg_gap=2（跨反向腿命中 m1）不出现。字面 s_prev==m1 身份未逐信号重建（用 codex 授权的等价物『Extreme 真假』，见 h2 报告 §诚实边界1）——若后续 bit-exact 重建 i1 发现字面≠m1 但 Extreme 仍必假，结论不变（建立在 Extreme 可测判据上，不依赖字面 m1 身份）。"
  # 有效域声明（诚实边界，231/formalization-validity-domain）
  validity_domain: "L1 全窗（1473 信号）实证；gate_pass=0 是全窗性质（h2 报告 §诚实边界3：任意 50-bar 子窗测得 level1 gate_pass=2/50，中间级归零是全窗非任意子窗恒成立）。可交易 alpha=L2 未做。"

> **[/ritual 结算段 · 2026-07-02]** 裁决来源：codex 裁决①-⑤（`.chanlun/review-results/codex-ritual-*.md`）+ 编排者明令「并行全部推进」授权。判决全文见 staging：GRAMMAR §1-A（673-finding 定理）+ CHOICES 673-fix（实装#12 编排者已授权）。判决摘要：Type1/Type2 判据结构互斥机制成立（1473 真实信号分解支撑）+ 接口级三分拆（type1/type2/type3 三独立函数+dispatcher）。
