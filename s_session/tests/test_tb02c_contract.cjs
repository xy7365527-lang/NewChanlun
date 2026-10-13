// #1404：既有观察器的段对象合同；不是浏览器/结构语义验收。
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const API = require('../browser/tb01c-client.js');
const fixture = require('./fixtures/tb02c/contract-seed.json');
test('段 seed 四条件与来源完整进入既有观察器', () => {
  API.validateSegmentPayload('CC-011.segment_seed', fixture);
  assert.ok(API.TB02_KINDS.includes('CC-013.segment'));
});
test('缺条件、未知笔与无效方向拒绝', () => {
  for (const mutate of [p => p.data.construction.vector.pop(), p => p.data.stroke_refs.pop(), p => {p.data.construction.vector[1] = false;}]) {
    const p = structuredClone(fixture); mutate(p);
    assert.throws(() => API.validateSegmentPayload('CC-011.segment_seed', p));
  }
});
test('不足三笔只发布等待候选，不能冒充seed', () => {
  const p = structuredClone(fixture);
  p.data.stroke_refs.pop();
  Object.assign(p.data.construction, {stroke_indices:['0','1'],length:'n=2',vector:null,overlap:null,overlap_relation:null,direction:null,failed_conditions:['fewer_than_three_strokes']});
  API.validateSegmentPayload('CC-011.seed_candidate',p);
  assert.throws(() => API.validateSegmentPayload('CC-011.segment_seed',p));
});
test('既有目录可打开C轴并显示未完整交付的证据', () => {
  const vm = require('node:vm');
  const {htmlContext} = require('./test_tb01c_browser.cjs');
  const {context, elements} = htmlContext();
  const items = ['CC-011','CC-012','CC-013'].map(id => ({id,kind:'classification',title:id,
    domain:'测试候选',implementation_status:id==='CC-011'?'implemented':'not_implemented',
    proof_status:'not_proved',run_status:'run',
    evidence:{scope:'TB-02-C',object_ids:[],waiting_reasons:['second_kind_out_of_scope']}}));
  context.catalogFixture = {items,counts:{implemented:'1',not_implemented:'2',run:'3',not_run:'0'}};
  vm.runInContext('currentState = {catalog: catalogFixture}', context);
  context.renderCatalog(context.catalogFixture);
  for (const {id} of items) {
    assert.ok(elements.get('#catalog-table tbody').innerHTML.includes(`data-catalog-id="${id}"`));
    context.renderObjects({objects:[],withdrawn_objects:[],structure_cut:'cut-16',history_mode:'AsKnown'},id);
    assert.match(elements.get('objects').innerHTML,/second_kind_out_of_scope/);
  }
});

test('真实cut1段逻辑slot不被页面当成见证整数', () => {
  const {htmlContext} = require('./test_tb01c_browser.cjs');
  const {context} = htmlContext();
  const capture = require('./fixtures/tb02c/first-cut-logical-slots.json');
  assert.equal(capture.objects.length, 2);
  for (const object of capture.objects) {
    API.validateSegmentPayload(object.kind, object.payload);
    assert.doesNotThrow(() => context.validateExactRecords(object.payload));
  }
  assert.doesNotThrow(() => context.validateExactRecords({slot:'0'}));
  assert.throws(() => context.validateExactRecords({slot:'seed:1:empty'}));
});

// #1414：新版本的逻辑slot也不能作为见证整数解析。
test('D版本逻辑slot及CC014实际对象映射', () => {
  const {htmlContext} = require('./test_tb01c_browser.cjs');
  const {context, elements} = htmlContext();
  const p = structuredClone(fixture);
  Object.assign(p, {schema_revision:'s-segment/2',semantic_version:'segment-second/1',policy_version:'seed_closed_strict_second'});
  assert.doesNotThrow(() => context.validateExactRecords(p));
  context.currentState = {catalog:{items:[{id:'CC-014',evidence:{scope:'TB-02-D',object_ids:['segment-o'],waiting_reasons:[]}}]}};
  require('node:vm').runInContext('currentState = globalThis.currentState',context);
  context.renderObjects({objects:[{object_id:'segment-o',kind:'CC-013.segment',payload:p,input_refs:[],source_coords:[],lifecycle:'active'}],withdrawn_objects:[],relations:[],structure_cut:'cut-1',history_mode:'AsKnown'},'CC-014');
  assert.ok(elements.get('objects').innerHTML.includes('segment-o'));
});
