/* #1374：只读协议负控；此处合成消息不构成 γ 或经济事实验收。 */
const test = require("node:test");
const assert = require("node:assert/strict");
globalThis.crypto ??= require("node:crypto").webcrypto;
const {verifySnapshot, sha256} = require("../browser/tb03a-client.js");
const hash = "a".repeat(64);
const discovery = {session_id:"voice-client-test",session_generation:"7",manifest_hash:hash,domains:["B:fixture"]};
const reference = object_id => ({authority_id:"fixture-authority",object_id,revision:"1",content_hash:hash});
const cut = (domain_id,commit_seq) => ({domain_id,commit_seq,root_hash:hash});
async function identity(record) {
  record.voice_id = await sha256({schema:"voice-identity/1",domain_id:"B:fixture",
    session_id:discovery.session_id,session_generation:discovery.session_generation,
    chong_id:"fixture",identity:record.identity});
}
async function seal(state) {
  const view = state.domains["B:fixture"];
  view.image_hash = await sha256(view.image);
  const body = {...state}; delete body.snapshot_token;
  state.snapshot_token = "economic-snapshot-" + await sha256(body);
  return state;
}
async function fixture() {
  const units = {quantity:{unit:"TEST-UNIT",quantum:"1"}};
  const record = {voice_id:"",binding_id:"binding-one",
    identity:{carrier_ref:reference("carrier-one"),gamma_ref:reference("gamma-one"),sigma:"Short",n:"0"},
    instrument:"TEST.CLIENT",operation_level:"5",
    structure_view:{profile_id:"fixture-profile",execution_level:"2",confirmation_level:"3",nest_depth:"1"},
    parent_voice_id:null,
    opening_basis:{fact_key:"source-one",fact_hash:hash,allocation_id:"allocation-one",e_cut:cut("E","1"),
      raw_sha256:hash,json_pointer:"/events/0",quantity:"8",units},
    evidence:{authority_id:"fixture-authority",bundle_sha256:hash,verification_sha256:hash},
    first_known_ns:"100",commit_ns:"110",commit_seq:"2",
    business_lifecycle:{status:"NotRecorded"},
    responsibility_tail:{status:"Unknown",reason:"no authoritative retirement event"}};
  await identity(record);
  const book = {chong_id:"fixture",symbol:"TEST.CLIENT",operation_level:"5",fixed_q:"12",
    actual_quantity:"8",opening_quantity:"8",units,income_book:{Pi:"0",A:"0",W:"0",R:"0"},
    known_cost_totals:{listed_component_total:"0"},known_cost_components:[],unknown:{},
    voice:{status:"Bound",records:[record]}};
  const image = {book,recorded:[],pending:[],applied:[],unmatched:[],history:[]};
  return seal({kind:"EconomicSnapshot",...discovery,mode:"Current",as_known_ns:null,
    domains:{"B:fixture":{status:"Available",cut:cut("B:fixture","2"),commit_ns:"110",image,image_hash:""}},
    completeness:"CompleteVector",snapshot_token:""});
}
const first = state => state.domains["B:fixture"].image.book.voice.records[0];
async function addChild(state) {
  const view = state.domains["B:fixture"], parent = first(state), child = structuredClone(parent);
  child.binding_id="binding-two";child.parent_voice_id=parent.voice_id;
  child.identity={carrier_ref:reference("carrier-two"),gamma_ref:reference("gamma-two"),sigma:"Long",n:"1"};
  child.first_known_ns="120";child.commit_ns="130";child.commit_seq="3";
  await identity(child);
  view.image.book.voice.records.push(child);view.cut.commit_seq="3";view.commit_ns="130";
  return child;
}
test("根代际与操作级别、证书层、区间套梯级数分别保留", async () => {
  const state = await fixture();
  assert.equal(await verifySnapshot(state,discovery),state);
  assert.deepEqual([first(state).identity.n,first(state).operation_level,
    first(state).structure_view.confirmation_level,first(state).structure_view.nest_depth],["0","5","3","1"]);
});
test("原未绑定切面仍可回看", async () => {
  const state = await fixture();
  state.domains["B:fixture"].image.book.voice={status:"MissingDependency",reason:"gamma absent",identity:null};
  await verifySnapshot(await seal(state),discovery);
});
test("真实父引用形状允许独立子代际", async () => {
  const state = await fixture();await addChild(state);
  await verifySnapshot(await seal(state),discovery);
});
const negativeCases = [
  ["更改四元身份后只重签外层图像", s => {first(s).identity.sigma="Long";}],
  ["把重开计数作为根代际", async s => {first(s).identity.n="1";await identity(first(s));}],
  ["伪造不存在的父引用", async s => {first(s).identity.n="1";first(s).parent_voice_id=hash;await identity(first(s));}],
  ["另一标的的证书绑定记录", s => {first(s).instrument="TEST.OTHER";}],
  ["另一操作级别的记录", s => {first(s).operation_level="3";}],
  ["把代际抄成区间套梯级数", s => {first(s).structure_view.nest_depth="0";}],
  ["晚于图像切面的获知记录", s => {first(s).first_known_ns="120";}],
  ["晚于图像切面的提交序号", s => {first(s).commit_seq="3";}],
  ["把尾责未知改成已清", s => {first(s).responsibility_tail.status="Cleared";}],
  ["已绑定却是空集合", s => {s.domains["B:fixture"].image.book.voice.records=[];}],
  ["重复 Voice 身份", s => {s.domains["B:fixture"].image.book.voice.records.push(structuredClone(first(s)));}],
  ["定义证据摘要缺失", s => {delete first(s).evidence.verification_sha256;}],
  ["开局数量为零", s => {first(s).opening_basis.quantity="0";}],
  ["开局数量为负", s => {first(s).opening_basis.quantity="-8";}],
  ["证据权威与对象权威不同", s => {first(s).evidence.authority_id="other-authority";}],
  ["更换 carrier 权威并重签身份", async s => {first(s).identity.carrier_ref.authority_id="other-authority";await identity(first(s));}],
  ["更换 gamma 权威并重签身份", async s => {first(s).identity.gamma_ref.authority_id="other-authority";await identity(first(s));}],
  ["父子同时提交", async s => {const child=await addChild(s);child.commit_seq=first(s).commit_seq;}],
  ["父声部提交时间晚于子声部", async s => {await addChild(s);first(s).commit_ns="131";s.domains["B:fixture"].commit_ns="131";}],
  ["子声部早于父声部获知", async s => {const child=await addChild(s);child.first_known_ns="99";}],
  ["子声部获知时父声部尚未提交", async s => {const child=await addChild(s);child.first_known_ns="105";}],
];
for (const [name,change] of negativeCases) test(name,async () => {
  const state=await fixture();await change(state);
  await assert.rejects(verifySnapshot(await seal(state),discovery));
});
