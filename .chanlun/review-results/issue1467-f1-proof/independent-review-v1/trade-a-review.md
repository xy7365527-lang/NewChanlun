# 代码审查报告

日期：2026-10-04。绑定 #1468，审查成交 A 臂网格特征导出。结论：PASS，仅适用于 `trade-history-grid-a/1` 探索导出及本报告列明的合同。没有发现置信度超过 80% 的需修复问题。

审查工作树：`/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun`。独立评审代理执行；未修改被审程序、测试、合同、原始输入或既有导出。唯一写入是本报告。未训练模型、选择特征、测量预测效果或收益，未验收完整 F₁/F₂ 或递归价值。

## 概念层质询结果

通过。本改动实现观察时间、成交历史与外生网格，不实现笔、中枢、背驰或递归判定。定义回溯以指定的 `ExperimentDesignCandidate-v1.md`、`Stage16CausalViews.md`、`Stage35ExternalTarget.md` 和原 `order_label_probe.mjs` 为本次合同；未扩大为缠论教义或上游交易所回放审查。未发现概念矛盾，故未进入谱系矛盾检索。

- 外生网格：`trade_grid_features.mjs:77–98` 从第一条采集记录之后的 UTC 100ms 刻度起逐格输出，未以成交、订单、盘口刷新或标签触发决策。独立检查删除内部非成交行后，所有身份和特征保持一致。起止范围来自共同观察范围，符合候选合同。
- 因果截止：`:56–64` 与 `:82–89` 均只消费 `capture<t`，且 `t<=watermark`。同刻成交保持接收次序；当前水位桶不进入任何决策，EOF 不封闭它。逐行截断和同刻截断检查通过。
- 观察起点与未知：窗口是否足够使用第一条采集时间，不使用第一笔成交或盘口 ready 时间。初始化前的已知成交保留。未见成交的价格、年龄为 null；不足一秒的窗口统计为 null。足够观察长度后的零仅表示未观察到成交。文档明确 `trade_stream_completeness=not_established`，未将缺口期间的观察计数宣称为完整交易所统计。
- 信息隔离：进入累积器的成交仅有 capture、price、quantity。sequence 只用于入口重复拒绝。quote、status、line、side、标签与目标确认时间均不影响 A 特征。审计资格单独输出，未回传特征计算。
- 精度与重复：价量由十进制整数字符串转 BigInt；累计量和价格输出为字符串。同序号重复拒绝，不合并不同序号但相同价量的成交。跨序号同 trade_id 的唯一性明确依赖 Stage16 上游，导出视图不携带 trade_id，本次不能据此独立证明该上游性质。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 0 | note |

测试中的朴素参考直接过滤原始行并重新求和，未调用被审解析器、投影或滑动游标。它仍是作者的另一实现，文档对此表述准确。本次另写数值纳秒 oracle，直接用合成事件的整数时间计算期望值，避开两个实现共同依赖时间字符串解析的问题。

`featuresAt` 的使用前提是传入 `tradeInput` 产生的有序历史及与该历史一致的共同起点、水位；本次不把该函数当作对任意未验证数组的防御性接口。`auditRows` 的缺口标记表达窗口内收到过 gap 状态，且完整性始终未知，不是无缺口证书。

## 实际执行

Node：v22.23.1。下列检查均 exit 0。三个被审文件为未跟踪新文件，staged/unstaged diff 不显示其正文，审查读取了完整文件。

```sh
node --check .chanlun/review-results/issue1467-f1-proof/trade_grid_features.mjs
node .chanlun/review-results/issue1467-f1-proof/trade_grid_features_test.mjs \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-v1/rows.jsonl \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/trade-grid-a-v1
```

结果：25/25 项通过。真实输入 SHA 校验通过，20,868 行、205 笔新到成交、572 个网格点。测试验证全部真实特征与原始行参考相同、共同身份和截止/资格与 profile/2 相同、6 个真实前缀切点、未来成交价量扰动，以及已有导出的内容和文件哈希。保存导出没有被重写。

以下独立检查以 shell heredoc 直接运行，未另建脚本。结果为 256 份历史、5,120 个决策、4,096 个逐行前缀、5,120 次公开截止接口对拍全部通过，内部非成交事件删除不变式也通过。

```sh
node --input-type=module <<'NODE'
import assert from 'node:assert/strict';
import {tradeInput, gridFeatures, featuresAt} from './.chanlun/review-results/issue1467-f1-proof/trade_grid_features.mjs';
const E = 1609459200000000000n, G = 100000000n, W = 1000000000n;
const stamp = n => new Date(Number(n / 1000000n)).toISOString().slice(0,19) + '.' + String(n % 1000000000n).padStart(9, '0') + 'Z';
function make(events) { return events.map((e,i) => ({schema:'coinbase-causal-view/1', product_id:'SYNTHETIC', available_at_capture:stamp(e.t), line:i+1, status:e.status ?? 'waiting_snapshot', new_trade:e.p === undefined ? null : {known_at_capture:stamp(e.t), scale:100000000, sequence:String(i+1), price:String(e.p), quantity:String(e.q)}})); }
let histories = 0, decisions = 0, prefixes = 0, cutoffChecks = 0;
for (let k=0;k<256;k++) {
 const origin=E+BigInt(k%7)*10000001n;
 const events=[{t:origin}];
 for(let j=0;j<8;j++) {
  const t=origin+BigInt(j)*200000000n;
  events.push({t});
  if (k & (1<<j)) {
   events.push({t,p:9007199254740993000n+BigInt(j),q:9007199254740993001n+BigInt(j)});
   if(j%2===0) events.push({t,p:9007199254740993100n+BigInt(j),q:7n});
  }
 }
 events.push({t:origin+2000000000n});
 const rows=make(events), actual=gridFeatures(tradeInput(rows));
 for (const d of actual) {
  const t=BigInt(d.decision_ns), past=events.filter(e=>e.p!==undefined && e.t<t), recent=past.filter(e=>e.t>=t-W), last=past.at(-1), full=t-origin>=W;
  const expected={last_trade_price:last ? String(last.p):null,seen_trade_count:String(past.length),since_last_trade_ns:last?String(t-last.t):null,observed_trade_count_1s:full?String(recent.length):null,observed_trade_quantity_1s:full?String(recent.reduce((s,e)=>s+e.q,0n)):null};
  assert.equal(t%G,0n); assert.deepEqual(d.features,expected); decisions++;
  const x=tradeInput(rows); assert.deepEqual(featuresAt(x.trades,t,origin,events.at(-1).t),expected); cutoffChecks++;
 }
 for(let n=1;n<=rows.length;n++) {const p=gridFeatures(tradeInput(rows.slice(0,n)));assert.deepEqual(p,actual.slice(0,p.length));prefixes++;}
 const filtered=rows.filter((r,i)=>i===0 || i===rows.length-1 || r.new_trade!==null);
 assert.deepEqual(gridFeatures(tradeInput(filtered)),actual);
 histories++;
}
console.log(JSON.stringify({independent_numeric_time_oracle:true,histories,decisions,prefixes,cutoffChecks,interior_nontrade_event_removal_invariant:true}));
NODE
```

## 被审版本

以下均为本次直接读取并计算的 SHA256。源码根目录为工作树的 `.chanlun/review-results/issue1467-f1-proof/`。

| 文件 | SHA256 |
|---|---|
| trade_grid_features.mjs | `7dba29c959e9ff04cea98c123cff567d902631d047e593cb26575781690c4499` |
| trade_grid_features_test.mjs | `c829d2f1f1f6c70918b94e397a88edc81b09c15791c60cf4ebb1f59f6c325292` |
| TradeGridA-v1.md | `9d08f523110c49f3d5df28478bae08aab3d34ba6bb5162c883311d0d5703c8fa` |
| ExperimentDesignCandidate-v1.md | `fa5cc0c57fadf723629969267ae58ab85ac5bd478ac7ab9ebe742f0565c57dac` |
| Stage16CausalViews.md | `5cc8292241275d4a580e3e05f1eeaa7cd39c513cc22c5e25a6388851a7ecc1e1` |
| Stage35ExternalTarget.md | `23140dcd39546fbe6e6c268b92f86ae2b6fb87e4b1b08f87a109794418f7feeb` |
| order_label_probe.mjs | `d909d6aac8b222cde57c3c35730bbae822fe9cf2481a16a78157b3e02c5553da` |
| 仓外真实 rows.jsonl | `a52374d3d1fc922eca44a692cc4243db2a083ee93346bc5b2181292ae66a47a5` |

未打印原始行情行或身份。未下载数据，未重新审查 Stage16 原始回放与 trade_id 去重实现，未读取已保存标签文件作额外独立核验。已有测试对当前标签程序的对齐及导出哈希给出的证据不能替代这些未执行项目。
