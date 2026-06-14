export const meta = {
  name: 'operation-duality-audit',
  description: 'Audit every URS operation point for dual accounting identity (该级别 + 次级别); flag single-identity gaps',
  phases: [
    { title: 'Enumerate', detail: '2 independent agents enumerate all URS operation points + dual-identity annotation' },
    { title: 'Verify', detail: 'per-operation adversarial: refute each dual-identity / gap claim by re-tracing code' },
  ],
}

const ROOT = '/Users/silencehan/Projects/NewChanlun'
const URS = ROOT + '/rust/src/trading/unified_recursive.rs'
const NF = ROOT + '/rust/src/trading/nested_fugue.rs'

const SPEC = [
  '## 编排者双重会计身份规格（每个操作都有双重身份，不存在单一身份操作）',
  '',
  '| 操作 | 该级别身份 | 次级别身份 |',
  '|------|-----------|-----------|',
  '| 清仓 | 平仓 N→0 | 用释放的全部资金在**反方向开仓** 0→N反向（清仓=平仓+翻转，非单纯平仓） |',
  '| 降成本 | 卖出 m 降成本 N→N-m | 开**反向**仓 m 0→m反向 |',
  '| 回补 | 买回 m N-m→N 成本降低 | 子 voice 平仓 P&L=父降成本额（同一数字两视图） |',
  '| 翻转 | 方向切换（多→空 或 空→多） | 原方向 voice 回归 + 新方向 voice 诞生（仓位连续不中断，非 exit+re-enter） |',
  '',
  '验证标准：如果某操作只有单一身份（如清仓只平仓不翻转）= 实装缺口。',
  '',
  '## 背景张力（必须正确处理）',
  '- "平多 ≠ 开空"（RNF 编排者核心）：平多与开空是**两个独立会计动作**，非单一符号翻转。',
  '  这与"清仓=平仓+翻转"**不矛盾**——正因平多≠开空，清仓才是两个独立动作（平仓+开反向）。',
  '- dual_voice.rs："翻转断面 M=N...两个独立会计动作，非单一翻转断面；同级别多空可共存"——',
  '  双向机制已存在于 dual_voice.rs，但 URS 未导入。',
  '- URS 继承 v4 单向基座：根恒多头（nested_fugue.rs pop_tail line 210 debug_assert Long）；',
  '  环23"出场=翻转=新建仓"在 URS 退化为"持有到底→清仓到现金→重新开多"。',
].join('\n')

const COMMON = [
  '你审计统一递归系统 URS（' + URS + '）的**每个操作点的双重会计身份**（编排者最重要约束）。',
  '',
  '## 审计材料（自己 Read）',
  '- URS run 主循环：' + URS + '（A 强平 255-266 / B 否定 269-282 / C 清仓 289-298 /',
  '  D 回补 302-312 / E spawn 317-386 / F 入场 389-413 / eod 454-458；操作 A-F 经 acted 互斥）',
  '- 会计原语：' + NF + '（pop_tail 194-262 / unwind_to 268-289 / Voice 84-99 / nav 103-112）',
  '',
  SPEC,
  '',
  '## 纪律',
  '1. **逐操作点追踪**：URS 每 bar 改变 chain/position/n_base 的每一处都是操作点。逐行定位。',
  '2. **双重身份逐一标注**：每个操作点标注 (该级别身份, 次级别身份)。次级别身份不存在 = 单一身份。',
  '3. **缺口=单一身份**：找出所有只有单一身份的操作（编排者点名清仓）。',
  '4. **不打补丁**：发现缺口精确描述，不淡化。区分"genesis/terminal 天然单一"（入场/eod）vs',
  '   "应双重却单一"（清仓只平仓不翻转 = 真缺口）。',
].join('\n')

phase('Enumerate')
const TABLE_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['operations', 'gaps_summary'],
  properties: {
    operations: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['op', 'code_lines', 'own_level_identity', 'sub_level_identity', 'dual_status', 'gap_note'],
        properties: {
          op: { type: 'string', description: '操作名（如 C清仓/E降成本/翻转）' },
          code_lines: { type: 'string', description: 'URS 行号' },
          own_level_identity: { type: 'string', description: '该级别会计身份' },
          sub_level_identity: { type: 'string', description: '次级别会计身份；不存在则写"缺失"' },
          dual_status: { type: 'string', enum: ['DUAL-COMPLETE', 'SINGLE-GAP', 'SINGLE-GENESIS', 'SINGLE-TERMINAL', 'PARTIAL'] },
          gap_note: { type: 'string', description: '若 SINGLE-GAP，说明应有的次级别身份及为何缺失' },
        },
      },
    },
    gaps_summary: { type: 'string', description: '所有 SINGLE-GAP 操作的汇总 + 根因' },
  },
}

const enums = await parallel([
  () => agent(COMMON + '\n\n## 任务\n独立枚举 URS 全部操作点，逐一标注双重会计身份，建完整表。' +
    '从头 Read 代码，不要预设结论。', { label: 'enum:A', phase: 'Enumerate', schema: TABLE_SCHEMA }),
  () => agent(COMMON + '\n\n## 任务\n独立枚举 URS 全部操作点，逐一标注双重会计身份，建完整表。' +
    '特别注意 gap-hunt：每个操作点都质问"次级别身份在代码哪一行体现？找不到=SINGLE-GAP"。' +
    '从头 Read 代码，不要预设结论。', { label: 'enum:B', phase: 'Enumerate', schema: TABLE_SCHEMA }),
])

const valid = enums.filter(Boolean)
// 合并两份枚举的操作名集合（取并集，按 op 名）
const opNames = Array.from(new Set(valid.flatMap((e) => e.operations.map((o) => o.op))))

phase('Verify')
const VERIFY_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['op', 'agreed_dual_status', 'own_level_verified', 'sub_level_verified', 'is_real_gap', 'refutation', 'confidence'],
  properties: {
    op: { type: 'string' },
    agreed_dual_status: { type: 'string', enum: ['DUAL-COMPLETE', 'SINGLE-GAP', 'SINGLE-GENESIS', 'SINGLE-TERMINAL', 'PARTIAL'] },
    own_level_verified: { type: 'string', description: '该级别身份的代码行号确认' },
    sub_level_verified: { type: 'string', description: '次级别身份的代码行号确认，或确认其缺失' },
    is_real_gap: { type: 'boolean', description: '是否真实实装缺口（应双重却单一；genesis/terminal 天然单一 = false）' },
    refutation: { type: 'string', description: '尝试反驳该操作的双重身份标注——重追踪代码' },
    confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
  },
}

const verdicts = await parallel(opNames.map((op) => () => {
  const claims = valid.map((e, i) => {
    const row = e.operations.find((o) => o.op === op)
    return 'enum' + (i ? 'B' : 'A') + ': ' + (row ? JSON.stringify(row) : '(未枚举此操作)')
  }).join('\n')
  return agent(
    COMMON + '\n\n## 对抗核验任务\n你是怀疑论审计员，核验操作 "' + op + '" 的双重会计身份标注。\n' +
    '两位独立枚举员对该操作的标注：\n' + claims + '\n\n' +
    '**你的任务**：独立重新 Read ' + URS + ' 和 ' + NF + ' 的相关行，自己追踪该操作改变了什么 chain/n_base/free，\n' +
    '确认 (该级别身份, 次级别身份) 各自的代码行号；判定 dual_status；判定是否真实缺口\n' +
    '（应双重却单一=true；genesis 入场/terminal eod 天然单一=false）。默认怀疑两位枚举员的标注，独立验证。',
    { label: 'verify:' + op, phase: 'Verify', schema: VERIFY_SCHEMA }
  )
}))

return { enumerations: valid, verified: verdicts.filter(Boolean) }
