import assert from 'node:assert/strict';
import fs from 'node:fs';
import {EncodingError,normalize,encode,recordCode,bucket,active0} from './effective.mjs';
let assertions=0;const eq=(a,b)=>{assert.deepEqual(a,b);assertions++;};
const equivalent=[
 [['q','2','4'],['q','1','2']],
 [['q','-1','-2'],['q','1','2']],
 [['q','0','-99'],['q','0','1']],
 [['n','0003'],['n','3']],
 [['z','-000'],['z','0']],
 [['b','ABff'],['b','abFF']],
 [['l',[['q','6','8'],['n','0002']]],['l',[['q','3','4'],['n','2']]]],
];
for(const [a,b] of equivalent){eq(encode(a),encode(b));eq(normalize(normalize(a)),normalize(a));}
const different=[
 [['q','1','2'],['q','1','3']],
 [['n','1'],['z','1']],
 [['z','1'],['q','1','1']],
 [['l',[['n','1'],['n','2']]],['l',[['n','2'],['n','1']]]],
 [['n','9007199254740992'],['n','9007199254740993']],
];
for(const [a,b] of different)eq(encode(a)===encode(b),false);
eq(normalize(['q','18014398509481986','2']),['q','9007199254740993','1']);
const invalid=[['q','1','0'],['n','1e3'],['z',1],['x','1'],['b','a'],['n','1','ignored'],['n','1\n'],['z','-1\r']];
for(const x of invalid){assert.throws(()=>normalize(x),EncodingError);assertions++;}
const base={rho:'aa',q:'0',s:'0',u:'1',F:['l',[['b','50'],['q','1','2']]]};
const alt={...base,F:['l',[['b','50'],['q','2','4']]]};
eq(recordCode(base),recordCode(alt));eq(bucket([base,alt]).kind,'single');
eq(bucket([base,{...base,F:['l',[['b','50'],['q','2','3']]]}]).kind,'ValueConflict');
eq(bucket([{...base,proofId:'must stay outside R'}]).kind,'EncodingError');
eq(bucket([{...base,F:['q','1','0']}]).kind,'EncodingError');eq(bucket([]).kind,'empty');
const history=[['n','4'],['q','1','2'],['b','ff']];
const active=active0(history,'AA','00','1');
const empty=active0(history,'aa','0','3');
eq(active[1][0],['l',[['b','aa'],['n','0'],['n','1']]]);
eq(active[1].slice(1),[['n','3'],['b',Buffer.from('AwaitingEvidence').toString('hex')],['n','1'],['n','3'],['n','2']]);
eq(empty[1].slice(1),[['n','3'],['b',Buffer.from('EmptyBuffer').toString('hex')],['n','3'],['n','3'],['n','0']]);
eq(active0([],'aa','0','0')[1][5],['n','0']);
assert.throws(()=>active0(history,'aa','0','4'),EncodingError);assertions++;
assert.throws(()=>active0(history,'aa','0','-1'),EncodingError);assertions++;
const result={scope:'仅ER1新增有限值表示、桶判等/冲突、Active；未运行旧扫描、EQ64、行情',
 equivalentCases:equivalent.length,differentCases:different.length,invalidNodeCases:invalid.length,
 assertions,active,empty,allAssertionsPassed:true};
if(process.argv[2])fs.writeFileSync(process.argv[2],JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify({assertions,allAssertionsPassed:true}));
