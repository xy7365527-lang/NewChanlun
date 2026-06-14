export const meta = {
  name: 'accounting-duality-audit',
  description: 'Audit nested_fugue.rs accounting against the section-8 same-difference duality (538/540) — 6 dimensions, each adversarially verified',
  phases: [
    { title: 'Audit', detail: 'one agent per duality dimension: trace spec vs code, verdict CONFORMS/DIVERGES/GAP/REFINES' },
    { title: 'Verify', detail: 'adversarial skeptic re-traces code to refute each finding' },
  ],
}

const ROOT = '/Users/silencehan/Projects/NewChanlun'
const SPEC = ROOT + '/docs/nested_fugue_accounting.md'
const CODE = ROOT + '/rust/src/trading/nested_fugue.rs'
const URS = ROOT + '/rust/src/trading/unified_recursive.rs'

const COMMON = [
  '你是会计审计员，审计缠论嵌套递归赋格的**会计双重性**实装（538号/540号谱系）。',
  '',
  '## 双重性（538号，必读背景）',
  '同一笔物理交易有两个会计视图：父级别视图=降成本（cost_basis 降低）/ 子级别视图=独立头寸',
  '（SHORT，capital=父释放现金，独立 P&L）。核心恒等式：child.P&L 恒等 father.cost_reduction',
  '（同一数字两身份）。物理层存一次，视图按需导出。守恒：Σ(链上 units)=N_base（恒仓）。',
  '',
  '## 审计材料（自己 Read，不要只信任何人的转述）',
  '- 规格：' + SPEC + '（§1 基本量/§2 卖出/§3 买回/§5 递归嵌套/§7 earning/§8 全局不变量）',
  '- 代码：' + CODE + '（Voice 结构 84-99；nav() 103-112；settle_phase 151-180；',
  '  pop_tail 194-262；unwind_to 268-289；run 主循环守恒守卫在 655-660 附近）',
  '- 复用方：' + URS + '（URS bit-exact 复用上述原语，line 310 清仓门除外）',
  '',
  '## 审计纪律（严格性 = 蜂群语法规则）',
  '1. **逐行追踪 + 数值演算**：不要泛泛说"大致符合"。给出具体行号 + 一个具体数值轨迹',
  '   （如：u=250, basis=104, c=96 → leftover=?, reduce=?, cost_pool=?）。',
  '2. **规格 vs 代码逐条对照**：找出规格声明与代码实际行为的**每一处差异**：',
  '   - CONFORMS：代码忠实实装规格',
  '   - REFINES：代码比规格更完整（规格是理想化，代码处理了规格未提的情形）',
  '   - DIVERGES：代码与规格冲突（规格说 A，代码做 B，不可调和）',
  '   - GAP：规格声明的不变量在某些路径下不成立（规格不完整/声明膨胀）',
  '3. **不打补丁不绕过**：发现矛盾就精确描述矛盾，不要"修复"或淡化。矛盾是最有价值的产出。',
  '4. **诚实定级**：L0（纯代码/定义追踪）/ L2（需真实数据验证）。标注结论等级。',
].join('\n')

const DIMS = [
  {
    key: 'D1-store-once',
    title: '同一笔存一次 + 视图按需导出（§8.2 + nav 单真值）',
    prompt: [
      '审计维度 D1：**同一笔物理交易只在 ledger 出现一次，视图按需导出**（§8 不变量2）。',
      '追踪 Voice 结构（84-99）是否是单一物理账本条目；nav()（103-112）是否单一真值',
      '（free + 多头 units×c + 空头 capital）；settle_phase（151-180）如何从单条目导出视图 P&L。',
      '判定：代码是"存一次+导出"还是"父子各存一份可能不一致"？空头视图未实现 P&L 存在哪里？',
      '（提示：nav 注释 101-102 称"空头视图未实现 P&L 在回补时以缩水/剩余现金物化"——这是存一次形式吗？）',
    ].join('\n'),
  },
  {
    key: 'D2-pnl-identity',
    title: 'child.P&L 恒等 father.cost_reduction（§8.3）',
    prompt: [
      '审计维度 D2：**核心双重性恒等式 child.P&L 恒等 father.cost_reduction**（§8 不变量3，§3 规格）。',
      '这是双重性的心脏。追踪 pop_tail 的 Short-child 分支（215-246，子空头回补=父降成本）。',
      '规格 §3 说："assert child.P&L == father.cost_reduction（双视图一致）"，无条件。',
      '代码：leftover = capital − u_back×c；reduce = leftover.min(parent.cost_pool)；',
      'parent.cost_pool 减 reduce；excess = leftover − reduce 走 earning 或 free。',
      '**关键问题**：恒等式在所有情形成立吗？分三种情形逐一数值演算：',
      '(a) 子空头盈利且 cost_pool 充足（c < basis，profit 小）；',
      '(b) 子空头盈利但 cost_pool 耗尽（profit > cost_pool → excess 走 earning）；',
      '(c) 子空头**亏损**（c > basis，价格涨，u_back<u，shortfall>0）。',
      '每种情形：child.P&L = ? father.cost_reduction(=reduce) = ? 两者相等吗？',
      '若某情形不等，§8.3 无条件恒等式是 GAP（规格不完整）还是 DIVERGES？',
    ].join('\n'),
  },
  {
    key: 'D3-conservation-N',
    title: 'Σunits=N_base 守恒 + N_base 动态语义（§8.1 vs §1）',
    prompt: [
      '审计维度 D3：**守恒律 Σunits=N_base + N_base 的存在论语义**。',
      '规格 §1："N 建仓后恒定（26课一开始买够不加仓）。只有 earning 阶段可增加。"',
      '规格 §8 不变量1："Σ(所有活跃 voice 的 units) = N（根 voice 恒仓）"。',
      '代码：主循环守恒守卫（655-660 附近）检验 |Σunits − n_base| < eps；但 pop_tail line 220',
      '把 n_base 减去 shortfall（亏损回补时 N **减少**），line 233 把 n_base 加 dq（earning **增加**）。',
      '**关键矛盾**：规格 §1 说 N 只增不减（只有 earning 增），但代码 line 220 在亏损时**减少** n_base。',
      'N_base 到底是"建仓股数"（恒定，26课）还是"当前在手单位"（动态，随盈亏增减）？',
      '这是规格内部不一致（§1 恒定 vs §8.1 重算守卫 vs 代码动态）还是有调和读法？',
      '数值演算 shortfall 路径，判定 DIVERGES/GAP/REFINES。',
    ].join('\n'),
  },
  {
    key: 'D4-earning-asymmetry',
    title: 'earning 双重性的多空不对称（§7）',
    prompt: [
      '审计维度 D4：**earning 阶段的多空不对称**（§7 + 538号 §4）。',
      '规格 §7："多空对称——会计完全对称，方向是 voice 属性，earning 机制与方向无关。"',
      '规格 §7 还说："空头降成本…cost≤0（空头免费）→纯利润→增加空头仓位"。',
      '代码 pop_tail 两分支：',
      '- Short-child→Long-parent（226-236）：at_point 时 excess 走 earning，parent.units 加 dq（多头**增股**）。',
      '- Long-child→Short-parent（247-259）：line 249 注释"空头 earning（挣负股数）L0 不可构造——',
      '  现金留在 capital"；line 256 只把 nrf_short_earning_hits 加 1（计数），**不增 units**。',
      '**关键问题**：规格 §7 声称多空对称，但代码只在多头父构造 earning 增仓，空头父 earning',
      '不可构造（只计数）。这是 DIVERGES（代码违反规格对称性声明）还是规格声明膨胀（§7 对称性',
      '在空头侧 L0 不可构造，规格未察）？参照 538号 §4 + 谱系 project_put_option_short_earning',
      '（空头 earning 在凸性载体=期权才消解）。判定并定级。',
    ].join('\n'),
  },
  {
    key: 'D5-nav-mtm',
    title: 'NAV 逐市估值 vs 空头未实现递延（§8.4）',
    prompt: [
      '审计维度 D5：**NAV 定义 vs nav() 实装的逐市估值差异**（§8 不变量4）。',
      '规格 §8 不变量4："total_NAV = initial_capital + Σ(已关闭 voice 的 P&L) + 未实现 P&L"。',
      '代码 nav()（103-112）：free + Σ(多头 units×c) + Σ(空头 capital)。',
      '**关键问题**：空头 voice 存活期间，其贡献是 capital（=开空时持有现金，固定），**不是**',
      'capital + (basis−c)×units（逐市）。即 nav() **不**对空头逐市估值——空头未实现盈亏递延到',
      '回补才物化（line 101-102 注释）。',
      '数值演算：子空头 units=250, basis=104（开空价），当前 c=96（空头浮盈 8×250=2000）。',
      'nav() 报告该 voice 贡献 = capital = 250×104 = 26000（固定）；真实逐市 = capital +',
      '(104−96)×250 = 28000。差 2000 未计入。',
      '这意味着：(a) 空头存活期 equity 曲线**不逐市** → MDD 在空头相位可能**低估**；',
      '(b) 但最终 nav（全部回补后）正确。规格 §8.4"+未实现 P&L"在空头存活期成立吗？',
      '判定 §8.4 是 GAP（空头未实现未计入 nav）还是有调和（capital 已是 1x 逐仓名义，递延有界）？',
      '注意这影响 R4(MDD) 指标口径——但跨变体一致（同一 nav 函数），比较公平。',
    ].join('\n'),
  },
  {
    key: 'D6-recursive-3layer',
    title: '递归嵌套三层守恒（§5：孙→子→父）',
    prompt: [
      '审计维度 D6：**递归嵌套的三层会计守恒**（§5）。',
      '规格 §5："子 voice 做降成本，孙 voice(k-2) direction:LONG，守恒 father.shares +',
      'child.units + grandchild.units == N"。即区间套下探产生 ≥3 层 voice 链。',
      '追踪 pop_tail 的 Long-child 分支（247-259，孙多头平仓回流子空头父）：',
      'line 250 proceeds = v.units×c；line 251 parent.capital 加 proceeds（现金回流父 capital）；',
      'line 252 parent.units 加 v.units（父在手恢复）；line 253-255 profit 递减 parent.cost_pool。',
      '**关键问题**：',
      '(a) 三层及以上守恒 Σunits=N 在 unwind_to 逐层解栈（268-289，"附庸的附庸不是我的附庸"）',
      '   时是否每层正确回流直接父层？',
      '(b) 孙是多头（在反弹中做多=对子空头降成本），proceeds 回流父 capital（非 free）——',
      '   这与 D2 的子空头回流父（多头）路径对称吗？双重性在递归相邻层是否一致复用？',
      '(c) 538号 §2.2 称 CL/BTC 实测链长 4-5（depth≥3 真实涌现）。结构上代码支持任意深度吗？',
      '数值演算一个 3 层链的守恒。判定 CONFORMS/REFINES/GAP。',
    ].join('\n'),
  },
]

phase('Audit')
const FINDING_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['dimension', 'invariant', 'verdict', 'evidence_trace', 'spec_vs_code', 'severity', 'level', 'genealogy_note'],
  properties: {
    dimension: { type: 'string' },
    invariant: { type: 'string', description: '审计的具体不变量（规格条款）' },
    verdict: { type: 'string', enum: ['CONFORMS', 'REFINES', 'DIVERGES', 'GAP'] },
    evidence_trace: { type: 'string', description: '具体行号 + 数值轨迹（演算）' },
    spec_vs_code: { type: 'string', description: '规格声明 vs 代码实际行为的精确差异；若无差异说明为何一致' },
    severity: { type: 'string', enum: ['L0-canonical-correct', 'spec-incomplete', 'spec-code-tension', 'declaration-inflation', 'benign-divergence'] },
    level: { type: 'string', enum: ['L0', 'L2-needed'] },
    genealogy_note: { type: 'string', description: '与 534/537/538/540 或 project_* 谱系的关系' },
  },
}

const VERDICT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['finding_holds', 'refutation_attempt', 'correction', 'confidence'],
  properties: {
    finding_holds: { type: 'boolean', description: '原审计发现经独立重追踪后是否成立' },
    refutation_attempt: { type: 'string', description: '尝试如何反驳——重追踪代码，找反例' },
    correction: { type: 'string', description: '若发现错误，给出修正的数值轨迹/行号；若无误写"无修正"' },
    confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
  },
}

const results = await pipeline(
  DIMS,
  (d) => agent(
    COMMON + '\n\n## 你的审计维度\n' + d.title + '\n\n' + d.prompt + '\n\n' +
    '产出结构化审计发现。evidence_trace 必须含具体行号 + 数值演算。',
    { label: 'audit:' + d.key, phase: 'Audit', schema: FINDING_SCHEMA }
  ),
  (finding, d) => agent(
    COMMON + '\n\n## 对抗性核验任务\n你是怀疑论审计员。下面是关于"' + d.title + '"的审计发现。\n' +
    '**你的任务是反驳它**——独立重新 Read ' + CODE + ' 和 ' + SPEC + ' 的相关行，自己数值演算，\n' +
    '尝试证明这个发现是错的（追踪错误/数值算错/规格读错/遗漏调和读法）。\n' +
    '默认怀疑：除非独立重追踪确认，否则 finding_holds=false。\n\n' +
    '## 待核验的审计发现\n' + JSON.stringify(finding, null, 1) + '\n\n' +
    '## 维度原始要求\n' + d.prompt + '\n\n产出核验判决。',
    { label: 'verify:' + d.key, phase: 'Verify', schema: VERDICT_SCHEMA }
  ).then((v) => ({ dimension: d.key, title: d.title, finding, verdict: v }))
)

return results.filter(Boolean)
