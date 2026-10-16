// #1420 段合同 v3（s-segment/3）浏览器侧锁：三态终点、K1 游标、续扫检查点与同 cut 引用；人工构造只检合同，不作结构语义或运行验收。
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const API = require('../browser/tb01c-client.js');
const seed = require('./fixtures/tb02c/contract-seed.json');

const SEQ = 'CC-012.feature_sequence', SEG = 'CC-013.segment', VER = 'CC-056.segment_version';
const RULE_V2 = 's2-axis-quantifiers+new-bi/1+segment-second/1', RULE_V3 = 's2-axis-quantifiers+new-bi/1+segment-second/2';
const RULE_COMBINED = 's2-axis-quantifiers+new-bi/2+segment-second/2', RULE_BI2_D = 's2-axis-quantifiers+new-bi/2+segment-second/1';
const PROFILE = 'ohlc_integer_tb02a_v1';
const TERMINALS = {CONFIRMED: ['CONFIRMED', 'CONFIRMED', 'COMPLETE'], KILLED: ['KILLED_BY_NEW_EXTREME', 'KILLED', 'COMPLETE'],
  OPEN: ['OPEN_AT_PREFIX_END', 'PENDING', 'COMPLETE'], BUDGET: [null, 'PENDING', 'INCOMPLETE_RESOURCE_BOUND']};
const clone = structuredClone;
const envelope = (v3, data, slot) => ({schema_revision: v3 ? 's-segment/3' : 's-segment/2', semantic_version: v3 ? 'segment-second/2' : 'segment-second/1',
  policy_version: 'seed_closed_strict_second', view_role: 'main', object_generation: '1', slot, data});
const elements = (ids = ['0', '1', '2']) => ids.map(i => ({high: '400', low: '200', stroke_idx: i, stroke_indices: [i]}));
const hit = () => ({elements: elements(), top_fractal: true, bottom_fractal: false, through_stroke: '2'});
function cursor({segDir = 'UP', b = '0', extreme = '600', granted = '64', consumed = '2', sealed = null, kill = null, matched = null, standard = elements()} = {}) {
  return {schema: 'second-fold-cursor/1', rule_revision: 'second-candidate-terminal/1', budget_policy: 'second-seq-scan-budget/1',
    budget_granted: granted, candidate: {seg_dir: segDir, b_stroke: b, candidate_extreme: extreme}, input_stroke_count: '3',
    from_stroke: String(BigInt(b) + 1n), through_stroke: '2', consumed_len: consumed, consumed_digest: '0123456789abcdef',
    dir_state: segDir, standard: clone(standard), breach_watermark: '528', semantic_state: sealed, killed_by: clone(kill), matched: clone(matched)};
}
// 与 test_tb02w_contract.py 同一构造：P＝600 未越过；KILLED 取 P＝500 被笔 2 高点 528 严格越过。
function second(terminal, v3 = true) {
  const [semantic, status, coverage] = TERMINALS[terminal], confirmed = terminal === 'CONFIRMED', killed = terminal === 'KILLED';
  const extreme = killed ? '500' : '600', truncated = terminal === 'BUDGET';
  const granted = {CONFIRMED: '64', KILLED: '64', OPEN: null, BUDGET: '2'}[terminal];
  const kill = killed ? {stroke_idx: '2', price: '528', candidate_extreme: extreme, comparison: '>'} : null;
  const matched = confirmed ? hit() : null, checks = [confirmed ? hit() : {...hit(), top_fractal: false}];
  const scan = {from_stroke: '1', through_stroke: '2', input_stroke_count: '3', resource_truncated: truncated, tail_cache_window: '0'};
  const facts = {raw_elements: elements(), standard: elements(), steps: [], checks, from_stroke: '1', through_stroke: '2',
    input_stroke_count: '3', resource_truncated: truncated, matched};
  if (v3) {
    Object.assign(facts, {candidate_extreme: extreme, breach_watermark: '528', semantic_state: semantic, killed_by: kill, coverage});
    Object.assign(scan, {semantic_state: semantic, coverage, budget: {policy: 'second-seq-scan-budget/1', granted},
      cursor: cursor({extreme, granted, sealed: confirmed || killed ? semantic : null, kill, matched})});
  }
  return envelope(v3, {entity_id: 'second:segment:1:1', segment_id: 'segment:1:1', role: 'SECOND_SEQUENCE', direction: 'UP',
    member_direction: 'UP', stroke_refs: clone(seed.data.stroke_refs), scan, facts, second_sequence_status: v3 ? status : (confirmed ? 'CONFIRMED' : 'PENDING')}, 'second:segment:1:1');
}
function first(status, v3 = true) {
  return envelope(v3, {entity_id: 'first:segment:1:1', segment_id: 'segment:1:1', role: 'FIRST_SEQUENCE', direction: 'UP', member_direction: 'DOWN',
    stroke_refs: clone(seed.data.stroke_refs), second_sequence_status: status,
    scan: {from_stroke: '0', through_stroke: '2', input_stroke_count: '3', resource_truncated: false, tail_cache_window: '7'},
    facts: {raw_elements: elements(), standard: elements(), steps: [], checks: [{elements: elements(), first_fractal: true, gap: true,
      gap_closed_by_c: false, provisional: false, turn_hypothesis_pair: ['0', '1']}]}}, 'first:segment:1:1');
}
function segment(result, v3 = true) {
  const known = {generation: '1', input_frontier: '13', receipt_id: 'r', received_at: 't', semantic_commit_ns: null};
  const strict = {elements: elements(), raw_gap: true, gap_closed_by_c: false, effective_case: 'CASE_TWO', closure_operator: '<=', closure_left: '200', closure_right: '400'};
  const done = result === 'CASE_TWO', budget = result === 'PENDING_BUDGET';
  return envelope(v3, {entity_id: 'segment:1:1', entity_revision: '1', seed_id: 'seed:1:1', sequence_id: 'first:segment:1:1',
    sequence_object_id: 'first-object', second_sequence_id: 'second:segment:1:1', second_sequence_object_id: 'second-object',
    construction: clone(seed.data.construction), result: budget ? 'PENDING_CASE_TWO' : result,
    waiting_reasons: result === 'KILLED_CASE_TWO' ? ['second_candidate_killed_by_new_extreme'] : [], termination_attempts: [],
    geometric_end: done ? {stroke_index: '2', group_anchor: '13', extreme_roots: ['13'], price: '528'} : null, trigger_stroke: done ? '2' : null,
    formed_known_at: known, terminated_known_at: done ? clone(known) : null, stroke_refs: clone(seed.data.stroke_refs), strict,
    fact_vector: {stroke_broken: true, new_segment_formed: done ? true : budget ? null : false, segment_broken: done ? true : budget ? null : false,
      stroke_break_witness: {earlier_stroke: '0', later_stroke: '2', earlier_price: '384', later_price: '528', comparison: '>='},
      scan_from_stroke: '0', scan_through_stroke: '2', scan_complete: !budget,
      termination_witness: done ? {kind: 'CASE_TWO', strict: clone(strict), second_match: hit(), construction: clone(seed.data.construction), minimum_count_met: true, through_stroke: '2'} : null}},
  'segment:1:1');
}
const checkpoint = (options = {}) => cursor({granted: '2', consumed: '2', standard: elements(['1', '2']), ...options});
function version(roundStrokes = '2', cursors = [checkpoint()]) {
  return {...envelope(true, {input_frontier: '13', waiting_reasons: [], known_at: {generation: '1', input_frontier: '13', receipt_id: 'r', received_at: 't', semantic_commit_ns: null},
    delivered_scope: 'seed_strict_first_and_second_terminal', deferred_scope: ['general_equal_identity', 'global_G006_bridge'],
    scan_budget: {policy: 'second-seq-scan-budget/1', round_strokes: roundStrokes}, scan_cursors: cursors}, 'TB-02-W')};
}
const mutated = (p, change) => { const q = clone(p); change(q.data); return q; };
function rejects(kind, p, reason) { assert.throws(() => API.validateSegmentPayload(kind, p), reason === undefined ? Error : {message: reason}); }

test('三种 SECOND 终态（含预算耗尽 null）与 FIRST 镜像状态通过', () => {
  for (const terminal of Object.keys(TERMINALS)) API.validateSegmentPayload(SEQ, second(terminal));
  for (const status of ['NOT_APPLICABLE', 'PENDING', 'CONFIRMED', 'KILLED']) API.validateSegmentPayload(SEQ, first(status));
  for (const result of ['CASE_TWO', 'KILLED_CASE_TWO', 'PENDING_CASE_TWO', 'PENDING_BUDGET']) API.validateSegmentPayload(SEG, segment(result));
  API.validateSegmentPayload(VER, version());
  API.validateSegmentPayload(VER, version('0', []));
  // v2 旧对象照旧通过。
  API.validateSegmentPayload(SEQ, second('CONFIRMED', false));
  API.validateSegmentPayload(SEG, segment('CASE_TWO', false));
});

test('终态、覆盖、失效见证与游标的关键反例', () => {
  const withHit = d => { d.facts.matched = hit(); d.facts.checks = [hit()]; };
  rejects(SEQ, mutated(second('KILLED'), withHit), 'SECOND命中与状态不符');
  rejects(SEQ, mutated(second('CONFIRMED'), d => { d.scan.coverage = d.facts.coverage = 'INCOMPLETE_RESOURCE_BOUND'; }), 'SECOND覆盖与语义/截断不一致');
  rejects(SEQ, mutated(second('BUDGET'), d => { d.scan.resource_truncated = d.facts.resource_truncated = false; }), 'SECOND覆盖与语义/截断不一致');
  rejects(SEQ, mutated(second('OPEN'), d => { d.second_sequence_status = 'KILLED'; }), 'SECOND状态未绑定语义终点');
  const bothKills = change => d => { change(d.facts.killed_by); change(d.scan.cursor.killed_by); };
  rejects(SEQ, mutated(second('KILLED'), bothKills(k => { k.comparison = '<'; })), '失效见证未严格越过候选极值');
  rejects(SEQ, mutated(second('KILLED'), d => { bothKills(k => { k.price = '500'; })(d); d.facts.breach_watermark = '500'; }), '失效见证未严格越过候选极值');
  rejects(SEQ, mutated(second('KILLED'), d => { d.facts.breach_watermark = '529'; }), '越界水位未绑定失效见证');
  rejects(SEQ, mutated(second('OPEN'), d => { delete d.scan.cursor.consumed_digest; }));
  rejects(SEQ, mutated(second('OPEN'), d => { d.scan.cursor.schema = 'second-fold-cursor/2'; }), 'K1游标版本不符');
  rejects(SEQ, mutated(second('OPEN'), d => { d.scan.cursor.consumed_digest = '0123456789ABCDEF'; }), 'K1游标指纹不规范');
  rejects(SEQ, mutated(second('OPEN'), d => { d.scan.cursor.semantic_state = 'OPEN_AT_PREFIX_END'; }), 'K1游标封存了未决终态');
  rejects(SEQ, mutated(second('KILLED'), d => { d.scan.cursor.killed_by.price = '529'; }), 'K1游标失效见证与事实不同');
  rejects(SEQ, mutated(second('OPEN'), d => { d.scan.cursor.consumed_len = '3'; }), 'K1游标消费范围越界');
  rejects(SEQ, mutated(second('OPEN'), d => { d.scan.cursor.candidate.b_stroke = '1'; }), 'K1游标起点不是b笔之后');
  // 版本混用：v3 信封配 v2 形状、v2 信封出现 KILLED。
  rejects(SEQ, {...second('CONFIRMED', false), schema_revision: 's-segment/3', semantic_version: 'segment-second/2'});
  rejects(SEQ, mutated(second('OPEN', false), d => { d.second_sequence_status = 'KILLED'; }));
});

test('SECOND预算消费不越额，资源截断必须有限，完整扫描仍准无限预算', () => {
  const grant = value => d => { d.scan.budget.granted = d.scan.cursor.budget_granted = value; };
  for (const terminal of Object.keys(TERMINALS)) {
    API.validateSegmentPayload(SEQ, mutated(second(terminal), grant('2')));
    rejects(SEQ, mutated(second(terminal), grant('1')), 'SECOND游标消费超过扫描预算');
    if (terminal !== 'BUDGET') API.validateSegmentPayload(SEQ, mutated(second(terminal), grant(null)));
  }
  rejects(SEQ, mutated(second('BUDGET'), grant(null)), '资源截断缺有限扫描预算');
  const provisional = mutated(second('CONFIRMED'), grant(null));
  Object.assign(provisional.data.scan.cursor, {semantic_state: null, matched: null});
  API.validateSegmentPayload(SEQ, provisional);
  for (const value of ['-1', '0', '1.5', '01', 1, true, false]) rejects(SEQ, mutated(second('CONFIRMED'), grant(value)));
  for (const value of ['-1', '1.5', '01', 2, true, false]) rejects(SEQ, mutated(second('OPEN'), d => { d.scan.cursor.consumed_len = value; }));
});

test('段结果与 CC-056 范围/槽/续扫检查点反例', () => {
  rejects(SEG, segment('KILLED_CASE_TWO', false), '冒称交付第二种终结');
  rejects(SEG, mutated(segment('KILLED_CASE_TWO'), d => { d.second_sequence_id = d.second_sequence_object_id = null; }), '第二种缺SECOND引用');
  rejects(VER, {...version(), slot: 'TB-02-D'}, '段语义槽身份不符');
  rejects(VER, mutated(version(), d => { d.delivered_scope = 'seed_strict_first_and_finite_second'; d.deferred_scope = ['general_equal_identity', 'general_scan_continuation', 'global_G006_bridge']; }), '段交付范围不符');
  rejects(VER, mutated(version(), d => { delete d.scan_cursors; }));
  rejects(VER, version('0'), '预算0却登记续扫检查点');
  rejects(VER, version('2', [checkpoint({granted: null})]), '续扫检查点预算不符');
  rejects(VER, version('2', [checkpoint(), checkpoint()]), '续扫检查点候选身份重复');
  rejects(VER, version('2', [checkpoint({consumed: '3'})]), '续扫检查点预算不符');
  rejects(VER, version('2', [checkpoint({sealed: 'KILLED_BY_NEW_EXTREME'})]), 'K1游标见证与封存终态不符');
  API.validateSegmentPayload(VER, version('2', [checkpoint(), checkpoint({extreme: '601'}), checkpoint({segDir: 'DOWN', sealed: 'CONFIRMED', matched: hit()}),
    checkpoint({b: '4', sealed: 'KILLED_BY_NEW_EXTREME', kill: {stroke_idx: '2', price: '601', candidate_extreme: '600', comparison: '>'}})]));
});

// #1420：合同夹具补齐 Q3 公共 validateState 所需的同 cut 包；保留人工合同域，不当生产输入证据。
const AXES = ['CC-001','CC-002','CC-003','CC-004','CC-005','CC-006','CC-007','CC-008','CC-009','CC-010','CC-011','CC-012','CC-013','CC-014','CC-054','CC-055','CC-056'];
const KNOWN_RULES = ['s2-axis-quantifiers','s2-axis-quantifiers+new-bi/1',
  's2-axis-quantifiers+new-bi/1+segment-first/1',RULE_V2,RULE_V3,
  's2-axis-quantifiers+new-bi/2',RULE_BI2_D,RULE_COMBINED];
function axisSet(rule, objects = []) {
  const continued = [RULE_V3,RULE_COMBINED].includes(rule);
  const legacy = AXES.filter(cid => !['CC-008','CC-009','CC-010','CC-011','CC-012','CC-013','CC-014','CC-055'].includes(cid));
  const bi = AXES.filter(cid => !['CC-011','CC-012','CC-013','CC-014'].includes(cid));
  const selected = rule === KNOWN_RULES[0] ? legacy : [KNOWN_RULES[1],'s2-axis-quantifiers+new-bi/2'].includes(rule) ? bi : rule === KNOWN_RULES[2] ? AXES.filter(cid => cid !== 'CC-014') : AXES;
  return Object.fromEntries(selected.map(cid => {
    const e = {scope:'TB-02-A',object_ids:[],waiting_reasons:[]};
    if (['CC-011','CC-012','CC-013','CC-014'].includes(cid)) {
      const kind = cid === 'CC-014' ? SEG : cid === 'CC-012' ? SEQ : cid === 'CC-013' ? SEG : 'CC-011.segment_seed';
      e.object_ids = objects.filter(o => o.kind === kind).map(o => o.object_id);
      if (rule === KNOWN_RULES[2]) e.scope = 'TB-02-C';
      else Object.assign(e, {scope:continued ? 'TB-02-W' : 'TB-02-D',
        delivered_subobligations:['finite_complete_prefix_strict_second','independent_stroke_and_segment_break_facts',...(continued ? ['general_scan_continuation'] : [])],
        deferred_subobligations:['general_equal_identity',...(continued ? [] : ['general_scan_continuation']),'global_G006_bridge']});
    }
    if (cid === 'CC-056') Object.assign(e,{raw_revisions:[],withdrawals:[],replaces:[]});
    return [cid,{impl_status:'not_implemented',proof_status:'not_proved',run_status:e.object_ids.length ? 'run' : 'not_run',evidence:e}];
  }));
}

function ruleState(generation, rule) {
  const state = fullState({objects:[],withdrawn_objects:[],raw_history:[]}, rule);
  if (generation === '0') {
    const initial = {generation:'0',structure_cut:'cut-0',index_frontier:'',input_frontier:'-1',
      seq_range:{from:'0',to:'-1'},catalog_run_status:'not_run',catalog_evidence:{}};
    Object.assign(state.snapshot, initial, {as_of_generation:'0'});
    Object.assign(state.catalog, initial);
    Object.assign(state.cut, {generation:'0',structure_cut:'cut-0',index_frontier:''});
    for (const item of state.catalog.items) item.evidence = {};
  }
  return state;
}

test('公开Q3零/一代保留八个具名规则，已核状态换成未知规则也拒绝', async () => {
  for (const protocol of ['s-session/2','s-session/3']) {
    for (const generation of ['0','1']) {
      for (const rule of KNOWN_RULES) await API.Q3.validateState(ruleState(generation,rule),protocol);
      const state = ruleState(generation,KNOWN_RULES[0]);
      await API.Q3.validateState(state,protocol);
      state.snapshot.rule_revision = 'unknown-rule';
      if (generation !== '0') state.snapshot.catalog_evidence.rule_revision = 'unknown-rule';
      await assert.rejects(API.Q3.validateState(state,protocol),/cut规则修订未声明/);
    }
  }
});
function fullState(snapshot, rule) {
  const scope = {structure:'CompleteCut',economic:'not_started'}, axes = axisSet(rule,snapshot.objects);
  const common = {session_id:'contract-only',generation:'1',structure_cut:'cut-1',catalog_revision:API.TB02_CATALOG.catalog_revision,
    index_frontier:'batch',scope,input_frontier:'13',seq_range:{from:'0',to:'13'},catalog_run_status:'run'};
  const profile = {profile_id:PROFILE,profile_hash:'a'.repeat(64),rule_revision:rule};
  const evidence = {...profile,structure_cut:'cut-1',scope,batch_id:'batch',axes};
  for (const k of ['classified_objects','windows_total','merged_bars','effective_source_positions','withdrawals','replaces','insufficient_knowledge','domain_not_satisfied']) evidence[k]='0';
  common.catalog_evidence = evidence;
  const declared = require('../catalog/signed-catalog.json');
  const wire = v => typeof v === 'number' ? String(v) : Array.isArray(v) ? v.map(wire) : v && typeof v === 'object' ? Object.fromEntries(Object.entries(v).map(([k,x]) => [k,wire(x)])) : v;
  const items = declared.items.map(i => {
    const axis = axes[i.id];
    return {id:i.id,...wire(Object.fromEntries(['kind','title','domain','branches'].map(k => [k,i[k] ?? (k === 'branches' ? [] : '')]))),
      implementation_status:axis?.impl_status ?? 'not_implemented',proof_status:'not_proved',run_status:axis?.run_status ?? 'not_run',evidence:axis?.evidence ?? {}};
  });
  const counts = {implemented:'0',not_implemented:String(items.length),run:String(items.filter(i => i.run_status === 'run').length),not_run:String(items.filter(i => i.run_status === 'not_run').length)};
  return {snapshot:{...snapshot,...common,...profile,history_mode:'AsKnown',as_of_generation:'1',witnesses:[],relations:[],observations:[]},
    catalog:{...common,items,counts},cut:Object.fromEntries(['session_id','generation','structure_cut','catalog_revision','index_frontier'].map(k => [k,common[k]]))};
}
async function typed(kind, payload, rule) {
  const fact_key = API.canonical([kind, payload.slot]);
  const object_id = 'obj-' + await API.sha256({kind, fact_key, payload, input_refs: [], profile_id: PROFILE, rule_revision: rule});
  return {object_id, object_revision: '1', kind, batch_id: 'batch', fact_key, payload, input_refs: [], source_coords: [],
    first_known_generation: '1', first_known_cut: 'cut-1', published_generation: '1', withdrawn_generation: null, withdrawal_reason: null, superseded_by: null, lifecycle: 'active'};
}
async function cut(firstPayload, secondPayload, segmentPayload, rule = RULE_V3) {
  const a = await typed(SEQ, firstPayload, rule), b = await typed(SEQ, secondPayload, rule);
  const s = clone(segmentPayload); s.data.sequence_object_id = a.object_id; s.data.second_sequence_object_id = b.object_id;
  return {objects: [a, b, await typed(SEG, s, rule)], withdrawn_objects: [], raw_history: [], generation: '1'};
}

test('SECOND预算反例重算对象哈希及同cut引用后仍被公开Q3拒绝', async () => {
  // 这套人工段对象采用 s-session/2 的 obj- 身份域；不冒充 S3 已发布段事实。
  const protocol = 's-session/2';
  for (const rule of [RULE_V3, RULE_COMBINED]) {
    for (const [granted, message] of [['1', 'SECOND游标消费超过扫描预算'], [null, '资源截断缺有限扫描预算']]) {
      const bad = second('BUDGET');
      bad.data.scan.budget.granted = bad.data.scan.cursor.budget_granted = granted;
      // cut 先重新计算 FIRST/SECOND 身份，再重绑段引用并重算段哈希；错误只在预算语义。
      const snapshot = await cut(first('PENDING'), bad, segment('PENDING_BUDGET'), rule);
      assert.equal(snapshot.objects[2].payload.data.second_sequence_object_id, snapshot.objects[1].object_id);
      await assert.rejects(API.Q3.validateState(fullState(snapshot, rule), protocol), {message});
    }
    for (const [terminal, result] of [['CONFIRMED', 'CASE_TWO'], ['KILLED', 'KILLED_CASE_TWO'], ['OPEN', 'PENDING_CASE_TWO']]) {
      const unlimited = second(terminal);
      unlimited.data.scan.budget.granted = unlimited.data.scan.cursor.budget_granted = null;
      const snapshot = await cut(first(TERMINALS[terminal][1]), unlimited, segment(result), rule);
      await API.Q3.validateState(fullState(snapshot, rule), protocol);
    }
  }
});

test('同cut引用：KILLED_CASE_TWO 绑 KILLED、PENDING 绑 PENDING，v3 段不收 v2 序列，规则绑定 schema', async () => {
  const run = async (snapshot, rule = RULE_V3) => API.Q3.validateState(fullState(snapshot,rule),'s-session/2');
  for (const [result, terminal] of [['CASE_TWO', 'CONFIRMED'], ['KILLED_CASE_TWO', 'KILLED'], ['PENDING_CASE_TWO', 'OPEN'], ['PENDING_BUDGET', 'BUDGET']])
    await run(await cut(first(TERMINALS[terminal][1]), second(terminal), segment(result)));
  await run(await cut(first('CONFIRMED', false), second('CONFIRMED', false), segment('CASE_TWO', false), RULE_V2), RULE_V2);
  for (const [result, terminal] of [['CASE_TWO','CONFIRMED'],['KILLED_CASE_TWO','KILLED'],['PENDING_CASE_TWO','OPEN'],['PENDING_BUDGET','BUDGET']])
    await run(await cut(first(TERMINALS[terminal][1]), second(terminal), segment(result), RULE_COMBINED), RULE_COMBINED);
  await run(await cut(first('CONFIRMED', false), second('CONFIRMED', false), segment('CASE_TWO', false), RULE_BI2_D), RULE_BI2_D);
  await assert.rejects(run(await cut(first('CONFIRMED', false), second('CONFIRMED', false), segment('CASE_TWO', false), RULE_COMBINED), RULE_COMBINED), /段合同与cut规则版本不符/);
  await assert.rejects(run(await cut(first('KILLED'), second('KILLED'), segment('KILLED_CASE_TWO'), RULE_BI2_D), RULE_BI2_D), /段合同与cut规则版本不符/);
  await assert.rejects(run(await cut(first('PENDING'), second('OPEN'), segment('KILLED_CASE_TWO'))), {message: '段结果未绑定SECOND终态'});
  await assert.rejects(run(await cut(first('KILLED'), second('KILLED'), segment('PENDING_CASE_TWO'))), {message: '段结果未绑定SECOND终态'});
  const unknown = mutated(segment('KILLED_CASE_TWO'), d => Object.assign(d.fact_vector, {new_segment_formed: null, segment_broken: null, scan_complete: false}));
  await assert.rejects(run(await cut(first('KILLED'), second('KILLED'), unknown)), {message: '段事实可知性未绑定SECOND扫描证据'});
  // v3 段配 v2 序列：引用闭合先于逐对象 rule→schema 绑定执行，拒因落在引用闭合；v3 对象挂 v2 规则则落在 schema 绑定。
  await assert.rejects(run(await cut(first('CONFIRMED', false), second('CONFIRMED', false), segment('CASE_TWO'))), {message: '段引用缺同cut序列对象'});
  await assert.rejects(run(await cut(first('KILLED'), second('KILLED'), segment('KILLED_CASE_TWO'), RULE_V2), RULE_V2), {message: '段合同与cut规则版本不符'});
});

test('v3 逻辑slot不被页面当成见证整数', () => {
  const {htmlContext} = require('./test_tb01c_browser.cjs');
  const {context} = htmlContext();
  for (const p of [version(), second('KILLED'), segment('KILLED_CASE_TWO')]) assert.doesNotThrow(() => context.validateExactRecords(p));
});


test('W目录保留未完成父轴，同时公开本片续扫证据', () => {
  const {htmlContext} = require('./test_tb01c_browser.cjs');
  const {context, elements} = htmlContext();
  const evidence = {scope:'TB-02-W', object_ids:[], waiting_reasons:['second_sequence_resource_bound'],
    delivered_subobligations:['general_scan_continuation'], deferred_subobligations:['general_equal_identity','global_G006_bridge']};
  context.catalogFixture = {items:[{id:'CC-014',kind:'classification',title:'CC-014',domain:'本片候选',
    implementation_status:'not_implemented',proof_status:'not_proved',run_status:'waiting',evidence}],
    counts:{implemented:'0',not_implemented:'1',run:'0',not_run:'0'}};
  vm.runInContext('currentState = {catalog: catalogFixture}',context);
  context.renderCatalog(context.catalogFixture);
  assert.match(elements.get('#catalog-table tbody').innerHTML,/data-catalog-id="CC-014"/);
  context.renderObjects({objects:[],withdrawn_objects:[],structure_cut:'cut-1',history_mode:'AsKnown'},'CC-014');
  assert.match(elements.get('objects').innerHTML,/general_scan_continuation/);
  assert.match(elements.get('objects').innerHTML,/global_G006_bridge/);
});


test('目录scope、子义务和未知规则在Q3按具名修订拒绝混用', async () => {
  for (const rule of [RULE_V2,RULE_V3]) {
    const axes = axisSet(rule);
    await API.validateTB02Axes(axes,null,rule);
    await assert.rejects(API.validateTB02Axes(axes,null,rule === RULE_V2 ? RULE_V3 : RULE_V2),/轴范围错误/);
    await assert.rejects(API.validateTB02Axes(axes,null,'unknown-rule'),/未声明的目录规则修订/);
    const bad = clone(axes);bad['CC-014'].impl_status='implemented';
    await assert.rejects(API.validateTB02Axes(bad,null,rule),/有限交付冒称全轴完成/);
    const missing = clone(axes);missing['CC-014'].evidence.deferred_subobligations=[];
    await assert.rejects(API.validateTB02Axes(missing,null,rule),/第二种子义务范围不符/);
  }
  const sample = await cut(first('KILLED'),second('KILLED'),segment('KILLED_CASE_TWO'));
  const state = fullState(sample,RULE_V3);state.snapshot.rule_revision='unknown-rule';state.snapshot.catalog_evidence.rule_revision='unknown-rule';
  await assert.rejects(API.Q3.validateState(state,'s-session/2'),/cut规则修订未声明/);
});
