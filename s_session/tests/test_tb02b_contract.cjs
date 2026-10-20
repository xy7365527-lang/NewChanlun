// #1405（#1343）：既有观察器的新笔对象合同（v1 AsKnown 并存、v2 同价/极值区形状）；不是浏览器验收。
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const assert = require('node:assert/strict');
const API = require('../browser/tb01c-client.js');
const V2 = {schema_revision:'s-new-bi/2', semantic_version:'new-bi-dual-coordinate/2', policy_version:'standard_raw_gap_3;same-price/keep-earlier+extreme-region/1'};
const V1 = {schema_revision:'s-new-bi/1', semantic_version:'new-bi-dual-coordinate/1', policy_version:'standard_raw_gap_3'};
const endpoint = (kind, anchor, price, roots, first, last, sources) => ({kind, merged_index:anchor, group_anchor:anchor, price, extreme_roots:roots, raw_first:first, raw_last:last, source_coords:sources, sealed_at:null, waiting_reasons:[]});
const asV1 = (e, position) => { const {raw_first, raw_last, ...rest} = e; return {...rest, raw_position:position}; };
const top = endpoint('TOP', '5', '13', ['5', '6'], '5', '6', ['4', '5', '6', '7']);
const later = endpoint('TOP', '9', '13', ['9'], '9', '9', ['8', '9', '10']);
const bottom = endpoint('BOTTOM', '1', '2', ['1'], '1', '1', ['0', '1', '2']);
const conditions = {merged_gap:'3', raw_between_actual_extrema:'3', top_price:'13', bottom_price:'2', vector:[true, true, true], failed_conditions:[], waiting_reasons:[]};
const pair = () => ({...V2, view_role:'main', object_generation:'1', slot:['1', '1', '5'],
  data:{old:bottom, new:top, endpoint_kind_pair:'BOTTOM/TOP', conditions, comparison:null, selection:null, retained_anchors:[], evidence:[], waiting_reasons:[]}});
const same = () => ({...V2, view_role:'main', object_generation:'1', slot:['1', '5', '9'],
  data:{old:top, new:later, endpoint_kind_pair:'TOP/TOP', conditions:null, comparison:'EQ', selection:'KEEP', retained_anchors:['5'], evidence:['same_kind_equal_price_keep_earlier'], waiting_reasons:[]}});

test('v2 极值区端点与确定条件进入观察器', async () => {
  await API.validateBiPayload('CC-008.new_bi_pair', pair());
  await API.validateBiPayload('CC-008.endpoint', {...pair(), slot:'fx:1:5', data:top});
  for (const mutate of [p => {p.data.new.raw_last = '4';}, p => {p.data.new.raw_first = '6';}, p => {p.data.new.raw_position = '5';},
    p => {delete p.data.new.raw_last;}, p => {p.data.conditions.raw_between_actual_extrema = null;}, p => {p.data.conditions.vector[1] = null;},
    p => {p.policy_version = 'standard_raw_gap_3';}, p => {p.data.new.waiting_reasons = ['raw_extreme_identity_tie'];}]) {
    const p = structuredClone(pair()); mutate(p);
    await assert.rejects(() => API.validateBiPayload('CC-008.new_bi_pair', p));
  }
});
test('v2 同型同价只能 KEEP 在先者并带证据', async () => {
  await API.validateBiPayload('CC-009.same_kind', same());
  for (const mutate of [d => Object.assign(d, {selection:'UNDETERMINED', retained_anchors:['5', '9']}), d => Object.assign(d, {selection:'REPLACE', retained_anchors:['9']}),
    d => {d.evidence = [];}, d => {d.retained_anchors = ['9'];}, d => {d.waiting_reasons = ['same_kind_endpoint_identity_tie'];}]) {
    const p = structuredClone(same()); mutate(p.data);
    await assert.rejects(() => API.validateBiPayload('CC-009.same_kind', p));
  }
});
test('v1 旧对象按 AsKnown 仍可读，v1/v2 字段不得混用', async () => {
  const old = structuredClone(same()); Object.assign(old, V1); delete old.data.evidence;
  Object.assign(old.data, {selection:'UNDETERMINED', retained_anchors:['5', '9'], waiting_reasons:['same_kind_endpoint_identity_tie'],
    old:asV1(old.data.old, null), new:asV1(old.data.new, '9')});
  await API.validateBiPayload('CC-009.same_kind', old);
  const mixed = structuredClone(old); mixed.data.evidence = ['same_kind_equal_price_keep_earlier'];
  await assert.rejects(() => API.validateBiPayload('CC-009.same_kind', mixed));
});

// #1405：会话规则→新笔合同代际绑定须在 Q3.validateState 完整执行点拒；拒因与 py 侧 test_tb02b_contract.py 同一句原文。
const BI_RULE_MISMATCH = '新笔合同与cut规则版本不符';
const RULE_NEW_BI_2 = 's2-axis-quantifiers+new-bi/2+segment-second/1', RULE_NEW_BI_1 = 's2-axis-quantifiers+new-bi/1+segment-second/1';
const PROFILE = 'ohlc_integer_tb02a_v1';
const AXES = ['CC-001', 'CC-002', 'CC-003', 'CC-004', 'CC-005', 'CC-006', 'CC-007', 'CC-054', 'CC-056', 'CC-008', 'CC-009', 'CC-010', 'CC-055', 'CC-011', 'CC-012', 'CC-013', 'CC-014'];
function frozenCatalog() {
  // 由冻结正本还原完整目录（静态列逐值、数值按源头投影为文本），使目录摘要门照常通过。
  const declared = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'catalog', 'signed-catalog.json'), 'utf8'));
  const wire = v => typeof v === 'number' ? String(v) : Array.isArray(v) ? v.map(wire) : v && typeof v === 'object' ? Object.fromEntries(Object.entries(v).map(([k, x]) => [k, wire(x)])) : v;
  const items = declared.items.map(i => ({id:i.id, ...wire(Object.fromEntries(['kind', 'title', 'domain', 'branches'].map(k => [k, i[k] ?? (k === 'branches' ? [] : '')]))),
    implementation_status:'not_implemented', proof_status:'not_proved', run_status:'not_run', evidence:{}}));
  return {catalog_revision:API.TB02_CATALOG.catalog_revision, items, counts:{implemented:'0', not_implemented:String(items.length), run:'0', not_run:String(items.length)}};
}
test('公开Q3零代入口保留具名历史规则并拒绝未知规则', async () => {
  function emptyState(rule) {
    const common = {session_id:'empty-rule-contract',generation:'0',structure_cut:'cut-0',catalog_revision:API.TB02_CATALOG.catalog_revision,
      index_frontier:'',scope:{structure:'CompleteCut',economic:'not_started'},input_frontier:'-1',seq_range:{from:'0',to:'-1'},catalog_run_status:'not_run',catalog_evidence:{}};
    return {snapshot:{...common,profile_id:PROFILE,profile_hash:'a'.repeat(64),rule_revision:rule,history_mode:'AsKnown',as_of_generation:'0',
      objects:[],withdrawn_objects:[],raw_history:[],witnesses:[],relations:[],observations:[]},
      catalog:{...frozenCatalog(),...common},
      cut:Object.fromEntries(['session_id','generation','structure_cut','catalog_revision','index_frontier'].map(k=>[k,common[k]]))};
  }
  for (const protocol of ['s-session/2','s-session/3']) {
    for (const rule of ['s2-axis-quantifiers','s2-axis-quantifiers+new-bi/1','s2-axis-quantifiers+new-bi/2',
      's2-axis-quantifiers+new-bi/1+segment-first/1',RULE_NEW_BI_1,RULE_NEW_BI_2])
      await API.Q3.validateState(emptyState(rule),protocol);
    await assert.rejects(API.Q3.validateState(emptyState('unknown-rule'),protocol),/cut规则修订未声明/);
  }
});
// 单个完整 typed 端点对象的 cut；内容身份按该 cut 的会话规则闭合，使合同代际绑定成为唯一可能的拒因。
async function endpointCut(generation, rule, protocol = 's-session/2') {
  const payload = {...(generation === 2 ? V2 : V1), view_role:'main', object_generation:'1', slot:'fx:1:5', data:generation === 2 ? top : asV1(top, '5')};
  const record = {object_id:null, object_revision:'1', kind:'CC-008.endpoint', batch_id:'batch-1', fact_key:API.canonical(['CC-008.endpoint', payload.slot]), payload,
    input_refs:[], source_coords:[], first_known_generation:'1', first_known_cut:'cut-1', published_generation:'1',
    withdrawn_generation:null, withdrawal_reason:null, superseded_by:null, lifecycle:'active'};
  const identity = {kind:record.kind, fact_key:record.fact_key, payload, input_refs:[], profile_id:PROFILE, rule_revision:rule};
  record.object_id = protocol === 's-session/3' ? await API.Q3.V.identity(0x20, identity, 'obj3-') : 'obj-' + await API.sha256(identity);
  if (protocol === 's-session/3') record.batch_id = 'batch3-' + 'b'.repeat(64);
  const scope = {structure:'CompleteCut', economic:'not_started'};
  const profile_hash = 'a'.repeat(64);
  const axes = rule.endsWith('segment-second/1') ? AXES : AXES.slice(0, 13);
  const evidence = {profile_id:PROFILE, profile_hash, rule_revision:rule, structure_cut:'cut-1', scope, batch_id:'batch-1',
    ...Object.fromEntries(['classified_objects','windows_total','merged_bars','effective_source_positions','withdrawals','replaces','insufficient_knowledge','domain_not_satisfied'].map(k=>[k,'0'])),
    axes:Object.fromEntries(axes.map(a => {
      const segment = ['CC-011','CC-012','CC-013','CC-014'].includes(a);
      return [a, {impl_status:'not_implemented', proof_status:'not_proved', run_status:'not_run', evidence:{
        scope:segment?'TB-02-D':'TB-02-A', object_ids:[], waiting_reasons:[],
        ...(a==='CC-056'?{raw_revisions:[],withdrawals:[],replaces:[]}:{}),
        ...(segment?{delivered_subobligations:['finite_complete_prefix_strict_second','independent_stroke_and_segment_break_facts'],
          deferred_subobligations:['general_equal_identity','general_scan_continuation','global_G006_bridge']}:{}),
      }}];
    }))};
  const shared = {session_id:'contract', generation:'1', structure_cut:'cut-1', catalog_revision:API.TB02_CATALOG.catalog_revision,
    index_frontier:'batch-1', scope, input_frontier:'-1', seq_range:{from:'0',to:'-1'}, catalog_run_status:'not_run', catalog_evidence:evidence};
  const snapshot = {...shared, profile_id:PROFILE, profile_hash, rule_revision:rule, history_mode:'AsKnown', as_of_generation:'1',
    objects:[record], withdrawn_objects:[], witnesses:[], relations:[], observations:[], raw_history:[]};
  const catalog = {...frozenCatalog(),...shared};
  for (const item of catalog.items) if (evidence.axes[item.id]) item.evidence = evidence.axes[item.id].evidence;
  return [{catalog, snapshot, cut:shared}, protocol];
}
test('new-bi/2 会话 cut 里出现 s-new-bi/1 对象被拒', async () => {
  await API.Q3.validateState(...await endpointCut(1, RULE_NEW_BI_1));
  await assert.rejects(API.Q3.validateState(...await endpointCut(1, RULE_NEW_BI_2)), {message:BI_RULE_MISMATCH});
});
test('new-bi/1 会话 cut 里出现 s-new-bi/2 对象被拒', async () => {
  // B 片浏览器那一轮正是此组合（会话规则 new-bi/1、对象 s-new-bi/2）；终版绑定须拒。
  await API.Q3.validateState(...await endpointCut(2, RULE_NEW_BI_2));
  await assert.rejects(API.Q3.validateState(...await endpointCut(2, RULE_NEW_BI_1)), {message:BI_RULE_MISMATCH});
});


test('v3 A/B 新旧规则各取13轴，跨代/未知规则与多轴拒绝', async () => {
  for (const version of [1, 2]) {
    const rule = 's2-axis-quantifiers+new-bi/' + version;
    const state = await endpointCut(version, rule, 's-session/3');
    await API.Q3.validateState(...state);
    assert.equal(Object.keys(state[0].snapshot.catalog_evidence.axes).length, 13);
    await assert.rejects(API.Q3.validateState(...await endpointCut(3-version, rule, 's-session/3')), {message:BI_RULE_MISMATCH});
    state[0].snapshot.catalog_evidence.axes['CC-014'] = {};
    await assert.rejects(API.Q3.validateState(...state), /cut规则目录轴/);
  }
  await assert.rejects(API.Q3.validateState(...await endpointCut(2, 'unknown', 's-session/3')), /cut规则修订未声明/);
});

test('v3知识域绑定完整规则且拒绝历史域混入新规则', async () => {
  for (const version of [1,2]) {
    const rule = 's2-axis-quantifiers+new-bi/' + version;
    const [state, protocol] = await endpointCut(version, rule, 's-session/3');
    const r = state.snapshot.objects[0];
    r.kind = 'CC-054.knowledge_state';
    r.fact_key = API.canonical([r.kind, 'TB-02-A']);
    r.payload = {scope:'TB-02-A',input_frontier:'-1',known_facts:['raw_ohlc','inclusion_trace','group_provenance'],waiting_reasons:[],requests:[],
      domain:version===1?'established_direction_without_extreme_identity_competition':'established_direction_with_extreme_root_sets'};
    const identify = async () => { r.object_id = await API.Q3.V.identity(0x20, {kind:r.kind,fact_key:r.fact_key,payload:r.payload,input_refs:[],profile_id:PROFILE,rule_revision:rule}, 'obj3-'); };
    await identify(); await API.Q3.validateState(state, protocol);
    r.payload.domain = version===2?'established_direction_without_extreme_identity_competition':'established_direction_with_extreme_root_sets';
    await identify();
    await assert.rejects(API.Q3.validateState(state, protocol), /知识域与cut规则版本不符/);
  }
});

// 图值访问的 TestOnly reader：由真实 content 编码校验闭包，逐字段借出 Value；不是浏览器/分页实跑。
async function graphPayload(payload, check = () => {}) {
  const C = require('../browser/s-content-v3.js'), V = API.Q3.V;
  const encoded = await C.encode(C.parseJSON(JSON.stringify(payload))), graph = await C.GraphView.create(encoded.nodes);
  const tags = new Map(encoded.nodes.map(n => [n.root, n.tag]));
  const reader = {check,
    async *entries(v) { check(); for (const e of graph.entries(v.ref)) { check(); yield {key:e.key, value:new V.Value(undefined, reader, e.value_ref)}; } },
    async scalar(v) { check(); return v.kind==='null'?null:v.kind==='bool'?tags.get(v.ref.root)==='02':graph.text(v.ref); },
  };
  return new V.Value(undefined, reader, encoded.ref);
}
test('异步图值保留根集合/区间合同与期限检查，原件不改写', async () => {
  for (const mutate of [null, p=>{p.data.new.extreme_roots=[];},p=>{p.data.new.source_coords=['4','5','7'];},p=>{p.data.new.raw_last='4';}]) {
    const p=structuredClone(pair()); if (mutate) mutate(p);
    const before=JSON.stringify(p); let checks=0;
    const graph=await graphPayload(p,()=>checks++);
    if (mutate) await assert.rejects(API.validateBiPayload('CC-008.new_bi_pair',graph));
    else await API.validateBiPayload('CC-008.new_bi_pair',graph);
    assert(checks>0); assert.equal(JSON.stringify(p),before);
  }
  const graph=await graphPayload(pair(),()=>{throw Error('期限已过');});
  await assert.rejects(API.validateBiPayload('CC-008.new_bi_pair',graph),/期限已过/);
});
