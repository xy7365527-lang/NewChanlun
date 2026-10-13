// #1419 L3 复跑：组合树浏览器合同 validateCenterCut 逐 cut 复核 S 写者发布对象（与 Python Q 同一份 records/axes）。
const fs = require('node:fs');
const API = require('/private/tmp/goal1323-native-20261012/tb1419-x1405-composite/s_session/browser/tb01c-client.js');
const rule = process.argv[2];
const dump = JSON.parse(fs.readFileSync(__dirname + '/l3-dump.json', 'utf8'));
let all = true;
for (const cut of dump.cuts) {
  let r = 'ACCEPT';
  try { API.validateCenterCut(cut.s_records, rule, cut.axes); } catch (e) { r = e.message.slice(0, 80); all = false; }
  console.log(`bars=${cut.bars} js_contract=${r}`);
}
console.log('JS_ALL_ACCEPT', all);
