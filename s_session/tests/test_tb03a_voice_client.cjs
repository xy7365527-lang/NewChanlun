/* #1374：只读协议负控；此处合成消息不构成 γ 或经济事实验收。 */
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");
globalThis.crypto ??= require("node:crypto").webcrypto;
const clientModule=process.env.TB03B_REVIEW_CLIENT || "../browser/tb03a-client.js";
const {verifySnapshot, sha256} = require(clientModule);
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

// #1394：声明财务版本后不能静默丢失锁/责任族。仅协议单位测试。
const financialDiscovery={...discovery,finance_revision:"tb03b-finance/1"};
async function financeFixture(){
  const state=await fixture();
  state.domains["B:fixture"].image.finance={schema_revision:"tb03b-finance/1",rights:[],decisions:[],slots:[],responsibilities:[]};
  return seal(state);
}
test("财务版本声明的空创世图像可读，后续记录不以此替代成功",async()=>{
  await verifySnapshot(await financeFixture(),financialDiscovery);
});
test("已声明财务能力时丢失整族不能重签外层掩盖",async()=>{
  await assert.rejects(verifySnapshot(await fixture(),financialDiscovery));
});
test("B 不得持有伪公共责任",async()=>{
  const state=await financeFixture();state.domains["B:fixture"].image.finance.responsibilities=[{responsibility_id:"fake",amount:"30"}];
  await assert.rejects(verifySnapshot(await seal(state),financialDiscovery));
});
test("Prepared 不得给场所调用资格",async()=>{
  const state=await financeFixture();state.domains["B:fixture"].image.finance.decisions=[{decision_id:"fake",state:"Prepared",execution_eligibility:"Execute"}];
  await assert.rejects(verifySnapshot(await seal(state),financialDiscovery));
});

// 正例逐字读取已封存 native HTTP 原件，不用手造 plan/grant/receipt hash。
const nativeOracle = process.env.TB03B_NATIVE_ORACLE;
function nativeFixture() {
  const raw=fs.readFileSync(path.join(nativeOracle,"http-actual/snapshot-current.response.json"));
  assert.equal(crypto.createHash("sha256").update(raw).digest("hex"),process.env.TB03B_NATIVE_SNAPSHOT_SHA256);
  const response=require("../browser/tb03a-client.js").parse(raw.toString("utf8"));
  const state=response.payload;
  return {state,discovery:{session_id:state.session_id,session_generation:state.session_generation,
    manifest_hash:state.manifest_hash,domains:Object.keys(state.domains),finance_revision:"tb03b-finance/2"}};
}
async function sealNative(state) {
  for(const view of Object.values(state.domains))if(view.status==="Available")view.image_hash=await sha256(view.image);
  const body={...state};delete body.snapshot_token;
  state.snapshot_token="economic-snapshot-"+await sha256(body);return state;
}
test("/2 native 完整财务切面按真实内容消费",{skip:!nativeOracle},async()=>{
  const {state,discovery}=nativeFixture();
  assert.equal(state.completeness,"CompleteVector");
  assert.equal(await verifySnapshot(state,discovery),state);
});
test("/2 不能丢finance整族后靠重签外头冒完整",{skip:!nativeOracle},async()=>{
  const {state,discovery}=nativeFixture();
  for(const view of Object.values(state.domains))delete view.image.finance;
  await assert.rejects(verifySnapshot(await sealNative(state),discovery));
});

function alphaRecords(state) {
  const image=state.domains["B:alpha"].image;
  return [image.finance.decisions[0],...image.history.filter(h=>["PrepareDecision","CommitDecision"].includes(h.kind)).map(h=>h.record)];
}
const nativeMutations = [
  ["重签 plan_hash 与收据仍须拒绝",s=>alphaRecords(s).forEach(d=>{d.prepared.plan_hash=hash;if(d.local_commit_receipt)d.local_commit_receipt.plan_hash=hash;}),/原计划摘要/],
  ["重签 grant_id 与收据仍须拒绝",async s=>{
    alphaRecords(s).filter(d=>d.grant).forEach(d=>{d.grant.original.grant_id=hash;d.local_commit_receipt.grant_id=hash;});
    const e=s.domains["B:alpha"].image.history.find(e=>e.kind==="CommitDecision");e.request.payload.grant_id=hash;e.request.payload_hash=await sha256(e.request.payload);
  },/grant 派生摘要/],
  ["commit_hash 必须重算",s=>alphaRecords(s).filter(d=>d.local_commit_receipt).forEach(d=>d.local_commit_receipt.commit_hash=hash),/本地提交摘要/],
  ["grant 金额不能另写",s=>alphaRecords(s).filter(d=>d.grant).forEach(d=>d.grant.original.amount="31"),/金额|绑定/],
  ["Prepared 切面序号不能漂",s=>alphaRecords(s).filter(d=>d.grant).forEach(d=>d.grant.original.prepared_cut.commit_seq="2"),/切面/],
  ["财务事件接收时刻不能倒序",s=>s.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision").received_ns="73",/知识时态/],
  ["持久 Evidence 不接受 pending",s=>alphaRecords(s).forEach(d=>d.structure_evidence.pending_revision=true)],
  ["持久 Evidence 不接受 null 时刻",s=>alphaRecords(s).forEach(d=>d.structure_evidence.commit_ns=null)],
  ["持久 Evidence 不接受未来时刻",s=>alphaRecords(s).forEach(d=>d.structure_evidence.commit_ns="1000")],
  ["持久 Evidence 不接受未知格式",s=>alphaRecords(s).forEach(d=>d.structure_evidence.content_format="s-content/99")],
  ["basis 引用 extent 必须七字段",s=>alphaRecords(s).forEach(d=>d.structure_evidence.basis_ref.extent="8")],
  ["运输 proof_nodes 禁入持久 Evidence",s=>alphaRecords(s).forEach(d=>d.structure_evidence.proof_nodes=[])],
  ["S 引用必须显式 /2",s=>alphaRecords(s).forEach(d=>delete d.prepared.plan.s_ref.schema_revision)],
  ["/2 image 不接受 /1 族",s=>s.domains.E.image.finance.schema_revision="tb03b-finance/1"],
  ["对端 proof 改前根不能靠外层重签",s=>s.domains["B:alpha"].image.history.find(e=>e.kind==="CommitDecision").witness.grant_proof.previous_root=hash,/实际持久图像根/],
  ["对端当前 epoch 时刻不得晚于获知",s=>s.domains["B:alpha"].image.history.find(e=>e.kind==="CommitDecision").witness.grant_proof.current_epoch_at_ns="102",/首次获知/],
];
for(const [name,mutate,reason] of nativeMutations)test("/2 native 负控："+name,{skip:!nativeOracle},async()=>{
  const {state,discovery}=nativeFixture();await mutate(state);
  await assert.rejects(verifySnapshot(await sealNative(state),discovery),reason);
});
for(const version of [null,"tb03b-finance/99"])test("财务 discovery 未知值拒绝 "+version,async()=>{
  await assert.rejects(verifySnapshot(await fixture(),{...discovery,finance_revision:version}));
});
const {verifyOriginal,parse,Client}=require(clientModule);
function nativeOriginal(name,expected) {
  const raw=fs.readFileSync(path.join(nativeOracle,"http-actual",name));
  assert.equal(crypto.createHash("sha256").update(raw).digest("hex"),expected);
  return parse(raw.toString("utf8")).payload;
}
const originalCases=[
  ["decision","B:alpha","alpha","original-decision-alpha.response.json","cec2cfe17b335d60cfff38546e67c7e87fa4cbbbcb6b58f8aeaca38893aa3c73"],
  ["grant","E","57e3d76efd49570b2969255c23dbcf96d4f5d6c30cce86dfb788f2b277e4b9f9","original-grant-e.response.json","3301d8b5728596119474fdc07da7b02bb230aabf073f540366e28de894397657"],
];
for(const [kind,domain,id,file,sha] of originalCases){
  test("/2 native 全量原件 "+kind,{skip:!nativeOracle},async()=>{
    const {state,discovery}=nativeFixture(),o=nativeOriginal(file,sha);
    assert.equal(await verifyOriginal(o,state,discovery,domain,kind,id),o);
  });
  for(const [name,mutate] of [
    ["错向量",o=>o.cuts.E.root_hash=hash],
    ["错类型",o=>o.kind="FakeOriginal"],
    ["错请求对象",o=>{if(kind==="decision")o.original.decision.decision_id="other";else o.original.grant.grant_id=hash;}],
    ["错回执原件",o=>{const r=kind==="decision"?o.original.receipts[0]:o.original.receipt;r.record.extra="tampered";}],
    ["错回执时刻",o=>{const r=kind==="decision"?o.original.receipts[0]:o.original.receipt;r.first_known_ns="999";}],
  ])test("/2 native 原件负控 "+kind+" "+name,{skip:!nativeOracle},async()=>{
    const {state,discovery}=nativeFixture(),o=nativeOriginal(file,sha);mutate(o);
    await assert.rejects(verifyOriginal(o,state,discovery,domain,kind,id));
  });
}
test("/2 query envelope 使用 discovery 唯一版本",async()=>{
  const client=new Client({fetcher:async()=>{throw new Error("unused");}});
  client.discovery={...discovery,finance_revision:"tb03b-finance/2",operation_timeout_ms:"100",max_reply_bytes:"100000",producer_epoch:"1"};
  let request;
  client.fetchBounded=async(path,options)=>{
    request=parse(options.body);const payload={ok:true};const ph=await sha256(payload);
    return {...request,source_namespace:"economic-observe/replies",source_epoch:"1",producer_id:"Q:"+discovery.session_id,
      payload,payload_hash:ph,causal_refs:[Object.fromEntries(["source_namespace","source_epoch","message_id","payload_hash"].map(k=>[k,request[k]]))],
      message_id:"reply-"+await sha256([request.source_namespace,request.source_epoch,request.message_id,ph])};
  };
  await client.query({op:"snapshot"});assert.equal(request.schema_revision,"economic-session/2");
});
test("/2 固定向量允许各域独立切面，不能伪加全局原子性",{skip:!nativeOracle},async()=>{
  const {state,discovery}=nativeFixture();state.mode="AsKnown";
  for(const domain of ["B:alpha","B:beta"]){
    const v=state.domains[domain],current=v.image.finance.decisions[0];
    const e=v.image.history.find(h=>h.kind==="PrepareDecision");
    v.cut=current.grant.original.prepared_cut;v.commit_ns=e.commit_ns;
    v.image.finance.decisions=[e.record];v.image.history=v.image.history.filter(h=>BigInt(h.commit_seq)<=BigInt(e.commit_seq));
  }
  assert.equal(await verifySnapshot(await sealNative(state),discovery),state);
});

// /1 合法七字段 FinancialBasis 的定向回归，不把此协议向量称为原生运行验收。
test("/1 合法七字段 FinancialBasis 作用域回归",()=>{
  const vm=require("node:vm"),source=fs.readFileSync(path.join(__dirname,"../browser/tb03a-client.js"),"utf8");
  const context={module:{exports:{}},require:()=>require("../browser/tb01c-client.js"),globalThis};
  vm.runInNewContext(source.replace("return Object.freeze({Client,","return Object.freeze({evidence,Client,"),context);
  const basis={kind:"FinancialBasis",session_id:"old-valid-session",session_generation:"1",writer_epoch:"1",pending_revision:false,commit_ns:"70",snapshot:{objects:[]}};
  const expected={session_id:basis.session_id,session_generation:basis.session_generation,finance_revision:"tb03b-finance/1"};
  assert.doesNotThrow(()=>context.module.exports.evidence(basis,expected,71n));
  assert.throws(()=>context.module.exports.evidence({...basis,pending_revision:true},expected,71n));
  assert.throws(()=>context.module.exports.evidence({...basis,commit_ns:"72"},expected,71n));
});

const legacyOraclePath=process.env.TB03B_LEGACY_NATIVE_ORACLE;
function legacyOracle() {
  const raw=fs.readFileSync(legacyOraclePath);
  assert.equal(crypto.createHash("sha256").update(raw).digest("hex"),process.env.TB03B_LEGACY_NATIVE_SHA256);
  return parse(raw.toString("utf8"));
}
async function legacyFixture(stage) {
  const oracle=legacyOracle(),discovery={session_id:oracle.session_id,session_generation:oracle.session_generation,
    manifest_hash:oracle.manifest_hash,domains:Object.keys(oracle.domains),finance_revision:"tb03b-finance/1"};
  const domains={};
  for(const [domain,data] of Object.entries(oracle.domains)){
    const seq=domain==="E"?(stage==="PrepareDecision"?"3":"4"):(stage==="CommitDecision"?"4":"3");
    const c=data.commits.find(x=>x.seq===seq);
    assert.equal(c.image.finance.schema_revision,"tb03b-finance/1");
    domains[domain]={status:"Available",cut:{domain_id:domain,commit_seq:c.seq,root_hash:c.root_hash},commit_ns:c.commit_ns,image:c.image,image_hash:await sha256(c.image)};
  }
  const state={kind:"EconomicSnapshot",session_id:oracle.session_id,session_generation:oracle.session_generation,manifest_hash:oracle.manifest_hash,
    mode:"AsKnown",as_known_ns:null,domains,completeness:"CompleteVector"};
  // 仅给真实 DB image/cut 加公开快照包裹，plan/grant/receipt 和持久根逐字来自 Rust。
  await sealNative(state);return {state,discovery,oracle};
}
for(const stage of ["PrepareDecision","Reserve","CommitDecision"]){
  test("/1 Rust 非空原生事务 "+stage,{skip:!legacyOraclePath},async()=>{
    const {state,discovery}=await legacyFixture(stage);
    assert.equal(await verifySnapshot(state,discovery),state);
  });
  for(const [name,change] of [["pending",e=>e.pending_revision=true],["未来时刻",e=>e.commit_ns="1000"],["空时刻",e=>e.commit_ns=null],["多余运输字段",e=>e.proof_nodes=[]]]){
    test("/1 Rust Evidence 负控 "+stage+" "+name,{skip:!legacyOraclePath},async()=>{
      const {state,discovery}=await legacyFixture(stage),domain=stage==="Reserve"?"E":"B:alpha";
      change(state.domains[domain].image.history.find(h=>h.kind===stage).witness.structure);
      await assert.rejects(verifySnapshot(await sealNative(state),discovery));
    });
  }
}
for(const [name,mutate,reason] of nativeMutations.slice(0,6))test("/1 Rust 派生负控 "+name,{skip:!legacyOraclePath},async()=>{
  const {state,discovery}=await legacyFixture("CommitDecision");await mutate(state);
  await assert.rejects(verifySnapshot(await sealNative(state),discovery),reason);
});
test("/1 Rust 实际决定与 grant 原回执",{skip:!legacyOraclePath},async()=>{
  const {state,discovery,oracle}=await legacyFixture("CommitDecision"),cuts=Object.fromEntries(Object.entries(state.domains).map(([d,v])=>[d,v.cut]));
  for(const domain of ["B:alpha","B:beta"]){
    const d=state.domains[domain].image.finance.decisions[0];
    const receipts=Object.values(oracle.domains[domain].receipts).filter(r=>["PreparedReceipt","LocalCommitReceipt"].includes(r.kind));
    const reply={kind:"EconomicOriginal",session_id:state.session_id,session_generation:state.session_generation,manifest_hash:state.manifest_hash,cuts,domain_id:domain,
      original:{kind:"EconomicDecision",domain_id:domain,decision:d,receipts}};
    assert.equal(await verifyOriginal(reply,state,discovery,domain,"decision",d.decision_id),reply);
  }
  const receipt=Object.values(oracle.domains.E.receipts).find(r=>r.kind==="ReserveReceipt");
  for(const grant of receipt.record.grants){
    const reply={kind:"EconomicOriginal",session_id:state.session_id,session_generation:state.session_generation,manifest_hash:state.manifest_hash,cuts,domain_id:"E",
      original:{kind:"EconomicGrant",domain_id:"E",grant,receipt}};
    assert.equal(await verifyOriginal(reply,state,discovery,"E","grant",grant.grant_id),reply);
  }
});

async function scopeFixture(version) {
  if(version==="/1")return legacyFixture("Reserve");
  const fixture=nativeFixture(),state=fixture.state;
  state.mode="AsKnown";
  for(const domain of ["B:alpha","B:beta"]){
    const v=state.domains[domain],d=v.image.finance.decisions[0],e=v.image.history.find(h=>h.kind==="PrepareDecision");
    v.cut=d.grant.original.prepared_cut;v.commit_ns=e.commit_ns;
    v.image.finance.decisions=[e.record];v.image.history=v.image.history.filter(h=>BigInt(h.commit_seq)<=BigInt(e.commit_seq));
  }
  return fixture;
}
function replacePreparedScope(image, field, value) {
  for(const d of image.finance.decisions)d.structure_evidence[field]=value;
  for(const e of image.history)if(e.kind==="PrepareDecision"){
    e.witness.structure[field]=value;e.record.structure_evidence[field]=value;
  }
}
async function resignPeerScope(p, discovery, field, value) {
  replacePreparedScope(p.image,field,value);replacePreparedScope(p.current_image,field,value);
  p.receipt.record=p.image.history.find(e=>e.kind==="PrepareDecision").record;
  for(const [image,c,previous_root,commit_ns] of [[p.image,p.receipt.cut,p.previous_root,p.commit_ns],[p.current_image,p.current_cut,p.current_previous_root,p.current_commit_ns]]){
    c.root_hash=await sha256({domain_id:p.domain_id,session_id:discovery.session_id,session_generation:discovery.session_generation,
      commit_seq:c.commit_seq,previous_root,commit_ns,image_hash:await sha256(image)});
  }
}
for(const version of ["/1","/2"])for(const [field,value] of [["session_id","foreign-session"],["session_generation","987"]]){
  const enabled=version==="/1"?legacyOraclePath:nativeOracle;
  test(version+" Evidence 同域header负控 三位置一致改 "+field,{skip:!enabled},async()=>{
    const {state,discovery}=await scopeFixture(version);
    replacePreparedScope(state.domains["B:alpha"].image,field,value);
    await assert.rejects(verifySnapshot(await sealNative(state),discovery),/结构依据与可信会话不同/);
  });
  test(version+" Evidence 同域header负控 递归peer历史一致改 "+field,{skip:!enabled},async()=>{
    const {state,discovery}=await scopeFixture(version),image=state.domains.E.image;
    const reserve=image.history.find(e=>e.kind==="Reserve"),p=reserve.witness.prepared_proofs.find(p=>p.domain_id==="B:alpha");
    await resignPeerScope(p,discovery,field,value);
    for(const slot of [reserve.record,...image.finance.slots])for(const g of slot.grants)if(g.owner==="B:alpha")g.prepared_cut=p.receipt.cut;
    for(const r of image.finance.responsibilities)if(r.grant && r.grant.owner==="B:alpha")r.grant.prepared_cut=p.receipt.cut;
    await assert.rejects(verifySnapshot(await sealNative(state),discovery),/结构依据与可信会话不同/);
  });
}

const clone=value=>parse(require("../browser/tb03a-client.js").canonical(value));
async function associationFixture(version, stage) {
  if(version==="/1")return legacyFixture(stage);
  return stage==="CommitDecision"?nativeFixture():scopeFixture(version);
}
function originalGrantCopies(image) {
  return [...image.finance.slots.flatMap(s=>s.grants),...image.history.filter(e=>e.kind==="Reserve").flatMap(e=>e.record.grants),...image.finance.responsibilities.filter(r=>r.grant).map(r=>r.grant)].filter(g=>g.owner==="B:alpha");
}
async function signCurrent(p,discovery) {
  p.current_cut.root_hash=await sha256({domain_id:p.domain_id,session_id:discovery.session_id,session_generation:discovery.session_generation,
    commit_seq:p.current_cut.commit_seq,previous_root:p.current_previous_root,commit_ns:p.current_commit_ns,image_hash:await sha256(p.current_image)});
}
const associationMutations=[
  ["Prepare固定Q与最坏量不能脱离原账","PrepareDecision",s=>alphaRecords(s).forEach(d=>{d.prepared.fixed_q="987";d.prepared.worst_quantity="987";})],
  ["Prepare原Voice标的不能漂","PrepareDecision",s=>alphaRecords(s).forEach(d=>d.prepared.voice.instrument="OTHER.INSTRUMENT")],
  ["prepared_ns须原提交时刻","PrepareDecision",s=>alphaRecords(s).forEach(d=>d.prepared.prepared_ns="71")],
  ["Prepare账本版本须原事件seq","PrepareDecision",async s=>{
    const image=s.domains["B:alpha"].image;
    for(const d of new Set(alphaRecords(s))){const p=d.prepared;p.plan.expected_book_version="1";p.book_version_before="1";p.book_version_after="2";p.plan_hash=await sha256(p.plan);}
    const e=image.history.find(e=>e.kind==="PrepareDecision");e.request.payload.plan_hash=e.record.prepared.plan_hash;e.request.payload_hash=await sha256(e.request.payload);
  }],
  ["Prepare原权利金额须配实际权利","PrepareDecision",s=>alphaRecords(s).forEach(d=>d.prepared.right_source.amount="987")],
  ["Prepare历史不可丢当前决定和权利","PrepareDecision",s=>{s.domains["B:alpha"].image.finance.decisions=[];s.domains["B:alpha"].image.finance.rights=[];}],
  ["Abort不得伪造历史或早于Prepare","PrepareDecision",s=>{
    const f=s.domains["B:alpha"].image.finance;f.decisions=clone(f.decisions);Object.assign(f.decisions[0],{state:"AbortedBeforeCommit",abort_reason:"fake",abort_ns:"71"});
  }],
  ["Reserve捕获申请者epoch必须原计划","Reserve",s=>s.domains.E.image.history.find(e=>e.kind==="Reserve").witness.prepared_proofs[0].current_epoch="2"],
  ["Reserve捕获当前Prepared不能已Abort","Reserve",async(s,d)=>{
    const p=s.domains.E.image.history.find(e=>e.kind==="Reserve").witness.prepared_proofs[0];
    Object.assign(p.current_image.finance.decisions[0],{state:"AbortedBeforeCommit",abort_reason:"fake",abort_ns:"71"});await signCurrent(p,d);
  }],
  ["Reserve捕获当前seq不能越原Prepared","Reserve",async(s,d)=>{
    const p=s.domains.E.image.history.find(e=>e.kind==="Reserve").witness.prepared_proofs[0];p.current_cut.commit_seq="4";await signCurrent(p,d);
  }],
  ["E原grant计划ID必须原Prepared","Reserve",s=>originalGrantCopies(s.domains.E.image).forEach(g=>g.plan_id="other-plan")],
  ["E原grant申请epoch必须原Prepared","Reserve",s=>originalGrantCopies(s.domains.E.image).forEach(g=>g.applicant_epoch="2")],
  ["E原grant Eepoch必须Reserve请求","Reserve",s=>originalGrantCopies(s.domains.E.image).forEach(g=>g.e_epoch="2")],
  ["E原grant量纲必须原Prepared","Reserve",s=>originalGrantCopies(s.domains.E.image).forEach(g=>g.units.quantity.unit="OTHER-UNIT")],
  ["Prepare请求业务ID须原决定","PrepareDecision",async s=>{const e=s.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision");e.request.payload.decision_id="other-id";e.request.payload_hash=await sha256(e.request.payload);}],
  ["Reserve请求业务ID须原slot","Reserve",async s=>{const e=s.domains.E.image.history.find(e=>e.kind==="Reserve");e.request.payload.slot_id="other-slot";e.request.payload_hash=await sha256(e.request.payload);}],
  ["Commit捕获E当前不能删除slot","CommitDecision",async(s,d)=>{const p=s.domains["B:alpha"].image.history.find(e=>e.kind==="CommitDecision").witness.grant_proof;p.current_image.finance.slots=[];await signCurrent(p,d);}],
  ["Commit捕获E当前不能已关闭申请","CommitDecision",async(s,d)=>{
    const p=s.domains["B:alpha"].image.history.find(e=>e.kind==="CommitDecision").witness.grant_proof;
    p.current_image.finance.closed_applicants=[{slot_id:"slot",owner:"B:alpha",applicant_epoch:"1",applicant_generation:"1"}];await signCurrent(p,d);
  }],
];
for(const version of ["/1","/2"])for(const [name,stage,mutate] of associationMutations)test(version+" 可见关联负控 "+name,{skip:!(version==="/1"?legacyOraclePath:nativeOracle)},async()=>{
  const {state,discovery}=await associationFixture(version,stage);await mutate(state,discovery);
  await assert.rejects(verifySnapshot(await sealNative(state),discovery));
});

const supplementaryMutations=[
  ["历史量不能用Prepared自述替代","PrepareDecision",s=>alphaRecords(s).forEach(d=>{d.prepared.actual_quantity="5";d.prepared.worst_quantity="7";})],
  ["最坏量必须精确原数量和","PrepareDecision",s=>alphaRecords(s).forEach(d=>d.prepared.worst_quantity="7")],
  ["当前实际量须已应用事实历史","PrepareDecision",s=>s.domains["B:alpha"].image.book.actual_quantity="5"],
  ["原Voice方向必须Long","PrepareDecision",s=>alphaRecords(s).forEach(d=>d.prepared.voice.identity.sigma="Short")],
  ["原Voice操作级别不能漂","PrepareDecision",s=>alphaRecords(s).forEach(d=>d.prepared.voice.operation_level="99")],
  ["原权利来源不能漂","PrepareDecision",s=>alphaRecords(s).forEach(d=>d.prepared.right_source.source_ref="other-source")],
  ["不可丢单独权利族","PrepareDecision",s=>s.domains["B:alpha"].image.finance.rights=[]],
  ["当前权利版本必须原Prepare末尾","PrepareDecision",s=>s.domains["B:alpha"].image.finance.rights[0].current_version="3"],
  ["Prepare历史不可重复","PrepareDecision",s=>{const i=s.domains["B:alpha"].image;i.history.push(clone(i.history.find(e=>e.kind==="PrepareDecision")));}],
  ["E原grant attempt必须原计划","Reserve",s=>originalGrantCopies(s.domains.E.image).forEach(g=>g.attempt_id="other-attempt")],
  ["E原grant generation必须原计划","Reserve",s=>originalGrantCopies(s.domains.E.image).forEach(g=>g.applicant_generation="2")],
  ["E原grant policy必须原Reserve","Reserve",s=>originalGrantCopies(s.domains.E.image).forEach(g=>g.policy.cap="999")],
  ["E已授信不可丢完整slot","Reserve",s=>s.domains.E.image.finance.slots=[]],
  ["E Reserved不可只留部分原申请","Reserve",s=>{
    const i=s.domains.E.image;i.finance.responsibilities=i.finance.responsibilities.filter(r=>r.owner!=="B:beta");
    for(const slot of [i.finance.slots[0],i.history.find(e=>e.kind==="Reserve").record]){slot.grants=slot.grants.filter(g=>g.owner!=="B:beta");slot.retained_total="50";}
  }],
  ["Commit请求业务ID必须原决定","CommitDecision",async s=>{const e=s.domains["B:alpha"].image.history.find(e=>e.kind==="CommitDecision");e.request.payload.decision_id="other-id";e.request.payload_hash=await sha256(e.request.payload);}],
  ["B原grant Eepoch必须原Reserve","CommitDecision",async(s,discovery)=>{
    const image=s.domains["B:alpha"].image,prep=image.history.find(e=>e.kind==="PrepareDecision").record;
    const gp=image.history.find(e=>e.kind==="CommitDecision").witness.grant_proof;
    for(const captured of [gp.image,gp.current_image])originalGrantCopies(captured).forEach(g=>g.e_epoch="2");
    gp.receipt.record=gp.image.history.find(e=>e.kind==="Reserve").record;
    gp.receipt.cut.root_hash=await sha256({domain_id:"E",session_id:discovery.session_id,session_generation:discovery.session_generation,
      commit_seq:gp.receipt.cut.commit_seq,previous_root:gp.previous_root,commit_ns:gp.commit_ns,image_hash:await sha256(gp.image)});
    await signCurrent(gp,discovery);
    for(const d of alphaRecords(s).filter(d=>d.grant)){
      d.grant.original.e_epoch="2";d.grant.e_cut=gp.receipt.cut;const r=d.local_commit_receipt;r.e_cut=gp.receipt.cut;
      r.commit_hash=await sha256([prep,d.grant.original,r.e_cut,r.book_version_after,r.commit_ns]);
    }
  }],];
for(const version of ["/1","/2"])for(const [name,stage,mutate] of supplementaryMutations)test(version+" 关联补充负控 "+name,{skip:!(version==="/1"?legacyOraclePath:nativeOracle)},async()=>{
  const {state,discovery}=await associationFixture(version,stage);await mutate(state,discovery);
  await assert.rejects(verifySnapshot(await sealNative(state),discovery));
});
const associationOraclePath=process.env.TB03B_ASSOCIATION_NATIVE_ORACLE;
async function associationNativeFixture(kind) {
  const raw=fs.readFileSync(associationOraclePath);
  assert.equal(crypto.createHash("sha256").update(raw).digest("hex"),process.env.TB03B_ASSOCIATION_NATIVE_SHA256);
  const oracle=parse(raw.toString("utf8")).cases[kind],domains={};
  for(const [domain,data] of Object.entries(oracle.domains)){
    const c=data.commits.at(-1);domains[domain]={status:"Available",cut:{domain_id:domain,commit_seq:c.seq,root_hash:c.root_hash},commit_ns:c.commit_ns,image:c.image,image_hash:await sha256(c.image)};
  }
  const discovery={session_id:oracle.session_id,session_generation:oracle.session_generation,manifest_hash:oracle.manifest_hash,domains:Object.keys(domains),finance_revision:"tb03b-finance/1"};
  const state={kind:"EconomicSnapshot",session_id:oracle.session_id,session_generation:oracle.session_generation,manifest_hash:oracle.manifest_hash,mode:"Current",as_known_ns:null,domains,completeness:"CompleteVector"};
  return {state:await sealNative(state),discovery};
}
for(const kind of ["abort-closed","awaiting"])test("/1 Rust 合法关联状态 "+kind,{skip:!associationOraclePath},async()=>{
  const {state,discovery}=await associationNativeFixture(kind);
  assert.equal(await verifySnapshot(state,discovery),state);
  if(kind==="abort-closed"){
    assert.equal(state.domains["B:alpha"].image.finance.decisions[0].state,"AbortedBeforeCommit");
    assert.equal(state.domains["B:alpha"].image.finance.rights[0].locked_amount,"30");
    assert.equal(state.domains.E.image.finance.slots[0].retained_total,"90");
  }else{
    assert.equal(state.domains.E.image.finance.slots[0].state,"AwaitingAllocationPolicy");
    assert.deepEqual(state.domains.E.image.finance.slots[0].grants,[]);
  }
});
test("原Prepare数量用原cut事实，不能把后来持量倒灌",{skip:!nativeOracle},()=>{
  const vm=require("node:vm"),source=fs.readFileSync(path.join(__dirname,"../browser/tb03a-client.js"),"utf8");
  const context={module:{exports:{}},require:()=>require("../browser/tb01c-client.js"),globalThis};
  vm.runInNewContext(source.replace("return Object.freeze({Client,","return Object.freeze({bookBefore,Client,"),context);
  const {state}=nativeFixture(),image=state.domains["B:alpha"].image;
  // 仅测试原cut查询的时态边界，以下后来变动是协议向量，不称原生后续事实验收。
  image.book.actual_quantity="7";
  const event={event:"known-delta",family_id:"family-one",deltas:{units:"3"}};
  image.recorded.push({fact_key:"later",event,raw_utf8:JSON.stringify({units:image.book.units,events:[event]})});
  image.history.push({kind:"ApplyAllocation",commit_seq:"5",commit_ns:"120",application:{status:"Applied",fact_key:"later"},event});
  assert.equal(context.module.exports.bookBefore(image,"3"),4n);
  assert.equal(context.module.exports.bookBefore(image,"6"),7n);
});

async function inputBindingFixture(version,kind) {
  if(kind==="CloseApplicants") {
    const old=await associationNativeFixture("abort-closed");
    if(version==="/1")return old;
    const fixture=nativeFixture(),image=fixture.state.domains.E.image,view=fixture.state.domains.E;
    // /2 Close 是协议派生向量：只复用真实 /1 Close 业务记录，加到真实 /2 E4；不冒称 /2 原生 Close 运行。
    const close=clone(old.state.domains.E.image.history.find(e=>e.kind==="CloseApplicants"));
    close.request.schema_revision="economic-session/2";close.request.session_id=fixture.state.session_id;close.request.session_generation=fixture.state.session_generation;
    close.witness={input_raw:image.history.find(e=>e.kind==="Reserve").witness.input_raw};
    image.history.push(close);image.finance.closed_applicants=close.record.closed.map(c=>({...c,slot_id:close.record.slot_id}));
    const previous_root=view.cut.root_hash;view.commit_ns=close.commit_ns;view.cut={...view.cut,commit_seq:close.commit_seq};
    view.cut.root_hash=await sha256({domain_id:"E",session_id:fixture.state.session_id,session_generation:fixture.state.session_generation,
      commit_seq:view.cut.commit_seq,previous_root,commit_ns:view.commit_ns,image_hash:await sha256(image)});
    return fixture;
  }
  if(kind==="ImportResponsibility") {
    if(version==="/1")return legacyFixture("PrepareDecision");
    const fixture=nativeFixture(),raw=fs.readFileSync(path.join(nativeOracle,"http-actual/watch-from-init.response.json"));
    assert.equal(crypto.createHash("sha256").update(raw).digest("hex"),"6915432369584c9f88df1e187348fc33eb9c2544558efeecbac99b6980d68a97");
    const c=parse(raw.toString("utf8")).payload.domains.E.changes.find(c=>c.cut.commit_seq==="3");
    fixture.state.domains.E={status:"Available",...c};fixture.state.mode="AsKnown";return fixture;
  }
  return associationFixture(version,kind);
}
function mutateInput(event,change) {
  const data=parse(event.witness.input_raw);change(data);event.witness.input_raw=require("../browser/tb03a-client.js").canonical(data);return data;
}
const inputBindingMutations=[
  ["Prepare原政策期限不得过期","PrepareDecision",s=>mutateInput(s.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision"),d=>d.policy.valid_through_ns="1")],
  ["Prepare期限按commit而非first-known","PrepareDecision",s=>mutateInput(s.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision"),d=>d.policy.valid_through_ns="71")],
  ["Prepare政策币种必须原账权利","PrepareDecision",s=>mutateInput(s.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision"),d=>d.policy.currency="OTHER")],
  ["Prepare政策money_unit必须原账","PrepareDecision",s=>mutateInput(s.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision"),d=>d.policy.money_unit="OTHER")],
  ["Prepare政策quantity_unit必须原账","PrepareDecision",s=>mutateInput(s.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision"),d=>d.policy.quantity_unit="OTHER")],
  ["Prepare仅原model能力","PrepareDecision",s=>mutateInput(s.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision"),d=>d.policy.model="OTHER")],
  ["Prepare仅原quantum能力","PrepareDecision",s=>mutateInput(s.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision"),d=>d.policy.quantum="2")],
  ["Prepare仅原allocation政策","PrepareDecision",s=>mutateInput(s.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision"),d=>d.policy.allocation_policy_id="OTHER")],
  ["Import原amount须baseline原式","ImportResponsibility",s=>{const i=s.domains.E.image,e=i.history.find(e=>e.kind==="ImportResponsibility");e.record.original.amount="0";i.finance.responsibilities[0].amount="0";}],
  ["Import原owner须E baseline","ImportResponsibility",s=>{const i=s.domains.E.image,e=i.history.find(e=>e.kind==="ImportResponsibility");e.record.original.owner="OTHER";i.finance.responsibilities[0].owner="OTHER";}],
  ["Import请求与record ID须baseline同一","ImportResponsibility",async s=>{const e=s.domains.E.image.history.find(e=>e.kind==="ImportResponsibility");e.request.payload.responsibility_id="OTHER";e.record.responsibility_id="OTHER";e.request.payload_hash=await sha256(e.request.payload);}],
  ["Close本次请求按owner唯一","CloseApplicants",async s=>{const i=s.domains.E.image,e=i.history.find(e=>e.kind==="CloseApplicants");e.record.closed.push(clone(e.record.closed[0]));e.request.payload.closed=clone(e.record.closed);e.request.payload_hash=await sha256(e.request.payload);i.finance.closed_applicants=e.record.closed.map(c=>({...c,slot_id:e.record.slot_id}));}],
];
for(const version of ["/1","/2"])for(const [name,kind,mutate] of inputBindingMutations)test(version+" 原输入关联负控 "+name,{skip:!(version==="/1"?legacyOraclePath:nativeOracle) || (kind==="CloseApplicants" && !associationOraclePath)},async()=>{
  const {state,discovery}=await inputBindingFixture(version,kind);await mutate(state);
  await assert.rejects(verifySnapshot(await sealNative(state),discovery));
});

const inputParseMutations=[
  ["输入exact形状",d=>d.extra="unexpected"],
  ["cap规范非负整数",d=>d.policy.cap="1.0"],
  ["期限规范非负整数",d=>d.policy.valid_through_ns="1.0"],
  ["baseline规范非负整数",d=>d.baseline.amount="-1"],
  ["覆盖事实exact摘要",d=>d.baseline.covered_facts[0].fact_hash="bad"],
  ["覆盖事实身份唯一",d=>d.baseline.covered_facts.push(clone(d.baseline.covered_facts[0]))],
  ["未选权利币种仍须政策币种",d=>d.rights[1].currency="OTHER"],
  ["原权利身份和owner唯一",d=>d.rights.push(clone(d.rights[1]))],
  ["原权利owner必须已配置域",d=>{d.rights[1].owner="B:foreign";d.plans[1].owner="B:foreign";}],
  ["未选plan exact形状",d=>d.plans[1].extra="unexpected"],
  ["原plan决定身份唯一",d=>d.plans.push(clone(d.plans[1]))],
  ["原plan plan_id唯一",d=>d.plans[1].plan_id=d.plans[0].plan_id],
  ["原plan owner attempt唯一",d=>{const a=d.plans[0],b=d.plans[1];b.owner=a.owner;b.right_id=a.right_id;b.attempt_id=a.attempt_id;}],
  ["原plan整数正值",d=>d.plans[1].expected_book_version="0"],
  ["原plan完整责任不得溢出",d=>d.plans[1].limit_price="9223372036854775807"],
  ["未选S引用摘要仍须合法",d=>d.plans[1].s_ref.object_hash="bad"],
  ["未选O量仍须原plan量",d=>d.plans[1].o_ref.quantity="2"],
  ["原plan权利版本不能倒置",d=>d.rights[1].version="2"],
  ["slot结束来源代际必须原输入",d=>d.slots[0].input_end.source_epoch="OTHER"],
  ["slot结束须完整请求hash",d=>d.slots[0].input_end.complete_request_hash=hash],
  ["slot watermark正整数",d=>d.slots[0].input_end.watermark="0"],
  ["slot身份唯一",d=>d.slots.push(clone(d.slots[0]))],
  ["slot不得空且覆盖全部plan",d=>d.slots=[]],
  ["slot每owner只能一项申请",d=>{d.plans[1].owner=d.plans[0].owner;d.plans[1].right_id=d.plans[0].right_id;}],
];
for(const version of ["/1","/2"])for(const [name,change] of inputParseMutations)test(version+" 输入解析负控 "+name,{skip:!(version==="/1"?legacyOraclePath:nativeOracle)},async()=>{
  const {state,discovery}=await associationFixture(version,"PrepareDecision");
  mutateInput(state.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision"),change);
  await assert.rejects(verifySnapshot(await sealNative(state),discovery));
});
test("原输入字节上限沿Rust既有16MiB",{skip:!nativeOracle},()=>{
  const vm=require("node:vm"),source=fs.readFileSync(require.resolve(clientModule),"utf8");
  const context={module:{exports:{}},require:()=>require("../browser/tb01c-client.js"),globalThis,TextEncoder};
  vm.runInNewContext(source.replace("return Object.freeze({Client,","return Object.freeze({inputWitness,Client,"),context);
  const {state}=nativeFixture(),e=state.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision");
  e.witness.input_raw+=" ".repeat(16*1024*1024);
  assert.throws(()=>context.module.exports.inputWitness(e),/原 Rust 字节上限/);
});
for(const version of ["/1","/2"]){
  test(version+" 原政策合法commit期限闭边界",{skip:!(version==="/1"?legacyOraclePath:nativeOracle)},async()=>{
    const {state,discovery}=await associationFixture(version,"PrepareDecision");
    const e=state.domains["B:alpha"].image.history.find(e=>e.kind==="PrepareDecision");mutateInput(e,d=>d.policy.valid_through_ns=e.commit_ns);
    assert.equal(await verifySnapshot(await sealNative(state),discovery),state);
  });
  test(version+" 原政策单位与原导入合法对应",{skip:!(version==="/1"?legacyOraclePath:nativeOracle)},async()=>{
    const {state,discovery}=await inputBindingFixture(version,"ImportResponsibility");
    assert.equal(await verifySnapshot(await sealNative(state),discovery),state);
    assert.equal(state.domains.E.image.finance.responsibilities[0].amount,"20");
    assert.equal(state.domains.E.image.finance.responsibilities[0].owner,"E:baseline");
  });
  for(const mode of ["native-business","expired-exception","unsorted-unique"])test(version+" 合法Close对应 "+mode,{skip:!(version==="/1"?legacyOraclePath:nativeOracle) || !associationOraclePath},async()=>{
    const {state,discovery}=await inputBindingFixture(version,"CloseApplicants");
    const i=state.domains.E.image,e=i.history.find(e=>e.kind==="CloseApplicants");
    if(mode==="expired-exception")mutateInput(e,d=>d.policy.valid_through_ns="1");
    if(mode==="unsorted-unique"){
      e.record.closed=[{owner:"B:beta",applicant_epoch:"1",applicant_generation:"1"},...e.record.closed];
      e.request.payload.closed=clone(e.record.closed);e.request.payload_hash=await sha256(e.request.payload);
      i.finance.closed_applicants=e.record.closed.map(c=>({...c,slot_id:e.record.slot_id}));
    }
    assert.equal(await verifySnapshot(await sealNative(state),discovery),state);
  });
}
test("/1 原生Abort保锁可在政策过期后中止",{skip:!associationOraclePath},async()=>{
  const {state,discovery}=await associationNativeFixture("abort-closed");
  mutateInput(state.domains["B:alpha"].image.history.find(e=>e.kind==="AbortDecision"),d=>d.policy.valid_through_ns="1");
  assert.equal(await verifySnapshot(await sealNative(state),discovery),state);
  assert.equal(state.domains["B:alpha"].image.finance.rights[0].locked_amount,"30");
});
for(const version of ["/1","/2"])for(const kind of ["ImportResponsibility","Reserve","CommitDecision"])test(version+" 原政策过期作用于 "+kind,{skip:!(version==="/1"?legacyOraclePath:nativeOracle)},async()=>{
  const {state,discovery}=await inputBindingFixture(version,kind),domain=kind==="CommitDecision"?"B:alpha":"E",i=state.domains[domain].image,e=i.history.find(e=>e.kind===kind);
  const data=mutateInput(e,d=>d.policy.valid_through_ns="1");
  if(kind==="ImportResponsibility"){e.record.original.policy=clone(data.policy);i.finance.responsibilities[0].policy=clone(data.policy);}
  if(kind==="Reserve")for(const g of [...i.finance.slots.flatMap(s=>s.grants),...i.history.filter(e=>e.kind==="Reserve").flatMap(e=>e.record.grants),...i.finance.responsibilities.filter(r=>r.grant).map(r=>r.grant)])g.policy=clone(data.policy);
  await assert.rejects(verifySnapshot(await sealNative(state),discovery),/在提交时已过期/);
});
for(const version of ["/1","/2"])test(version+" 不同Close请求可重复owner且无排序政策",{skip:!(version==="/1"?legacyOraclePath:nativeOracle) || !associationOraclePath},async()=>{
  const {state,discovery}=await inputBindingFixture(version,"CloseApplicants"),view=state.domains.E;
  const first=view.image.history.find(e=>e.kind==="CloseApplicants"),second=clone(first),data=parse(second.witness.input_raw);
  // 第二个不同slot请求是协议向量，验证同owner可跨请求复用；不冒称原生DB已有第二提交。
  const plan=clone(data.plans.find(p=>p.owner==="B:alpha"));
  Object.assign(plan,{decision_id:"second-close-plan",plan_id:"second-close-plan",attempt_id:"second-close-attempt",slot_id:"second-close-slot"});data.plans.push(plan);
  data.slots.push({slot_id:plan.slot_id,applicants:[plan.decision_id],input_end:{kind:"ControlledInputEnd",source_id:data.source_id,source_epoch:data.source_epoch,watermark:"1",complete_request_hash:await sha256([plan.decision_id])}});
  second.witness.input_raw=require("../browser/tb03a-client.js").canonical(data);
  second.request.message_id+="-second";second.request.payload.slot_id=plan.slot_id;second.record.slot_id=plan.slot_id;
  second.request.payload_hash=await sha256(second.request.payload);
  const vm=require("node:vm"),source=fs.readFileSync(path.join(__dirname,"../browser/tb03a-client.js"),"utf8");
  const context={module:{exports:{}},require:()=>require("../browser/tb01c-client.js"),globalThis,TextEncoder};
  vm.runInNewContext(source.replace("return Object.freeze({Client,","return Object.freeze({eventBinding,Client,"),context);
  assert.equal(await context.module.exports.eventBinding(first,view,discovery),undefined);
  assert.equal(await context.module.exports.eventBinding(second,view,discovery),undefined);
});
for(const version of ["/1","/2"])for(const [name,reason] of [["null",null],["空文本",""],["布尔",false],["对象",{}],["数组",[]]])test(version+" Close reason原类型负控 "+name,{skip:!(version==="/1"?legacyOraclePath:nativeOracle) || !associationOraclePath},async()=>{
  const {state,discovery}=await inputBindingFixture(version,"CloseApplicants"),e=state.domains.E.image.history.find(e=>e.kind==="CloseApplicants");
  e.request.payload.reason=reason;e.record.reason=reason;e.request.payload_hash=await sha256(e.request.payload);
  await assert.rejects(verifySnapshot(await sealNative(state),discovery),/关闭申请原因.*缺失/);
});
for(const version of ["/1","/2"])for(const [label,reason] of [["空白"," \n\t"],["数字文本","0"]])test(version+" Close reason合法非空文本不新增trim政策 "+label,{skip:!(version==="/1"?legacyOraclePath:nativeOracle) || !associationOraclePath},async()=>{
  const {state,discovery}=await inputBindingFixture(version,"CloseApplicants"),e=state.domains.E.image.history.find(e=>e.kind==="CloseApplicants");
  e.request.payload.reason=reason;e.record.reason=e.request.payload.reason;e.request.payload_hash=await sha256(e.request.payload);
  assert.equal(await verifySnapshot(await sealNative(state),discovery),state);
});
