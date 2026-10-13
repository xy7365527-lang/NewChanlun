// #1405（#1343）：既有观察器的新笔对象合同（v1 AsKnown 并存、v2 同价/极值区形状）；不是浏览器验收。
'use strict';
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

test('v2 极值区端点与确定条件进入观察器', () => {
  API.validateBiPayload('CC-008.new_bi_pair', pair());
  API.validateBiPayload('CC-008.endpoint', {...pair(), slot:'fx:1:5', data:top});
  for (const mutate of [p => {p.data.new.raw_last = '4';}, p => {p.data.new.raw_first = '6';}, p => {p.data.new.raw_position = '5';},
    p => {delete p.data.new.raw_last;}, p => {p.data.conditions.raw_between_actual_extrema = null;}, p => {p.data.conditions.vector[1] = null;},
    p => {p.policy_version = 'standard_raw_gap_3';}, p => {p.data.new.waiting_reasons = ['raw_extreme_identity_tie'];}]) {
    const p = structuredClone(pair()); mutate(p);
    assert.throws(() => API.validateBiPayload('CC-008.new_bi_pair', p));
  }
});
test('v2 同型同价只能 KEEP 在先者并带证据', () => {
  API.validateBiPayload('CC-009.same_kind', same());
  for (const mutate of [d => Object.assign(d, {selection:'UNDETERMINED', retained_anchors:['5', '9']}), d => Object.assign(d, {selection:'REPLACE', retained_anchors:['9']}),
    d => {d.evidence = [];}, d => {d.retained_anchors = ['9'];}, d => {d.waiting_reasons = ['same_kind_endpoint_identity_tie'];}]) {
    const p = structuredClone(same()); mutate(p.data);
    assert.throws(() => API.validateBiPayload('CC-009.same_kind', p));
  }
});
test('v1 旧对象按 AsKnown 仍可读，v1/v2 字段不得混用', () => {
  const old = structuredClone(same()); Object.assign(old, V1); delete old.data.evidence;
  Object.assign(old.data, {selection:'UNDETERMINED', retained_anchors:['5', '9'], waiting_reasons:['same_kind_endpoint_identity_tie'],
    old:asV1(old.data.old, null), new:asV1(old.data.new, '9')});
  API.validateBiPayload('CC-009.same_kind', old);
  const mixed = structuredClone(old); mixed.data.evidence = ['same_kind_equal_price_keep_earlier'];
  assert.throws(() => API.validateBiPayload('CC-009.same_kind', mixed));
});
