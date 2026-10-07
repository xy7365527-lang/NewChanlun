# #1404 R3 semantic-core 完整性事前推导

本次仅只读核验源码、冻结输入和小型终态 JSON；未启动服务、未打开任何 SQLite、未读取 semantic-core 作为计数依据，未审阅主控正在修改的比较工具。本文为本票工作草稿。

结论：每个 arm 的完整预期为 **5 × 28 + 17 + 1 + 1 + 2 = 161 条**，来自输入和采集控制流，不能由实际输出条数反推。四个 arm 是 `first_up-a`、`first_up-b`、`first_down-a`、`first_down-b`。

## 冻结依据

R3 根目录为 `/Volumes/AgentStorage/issue1404-formal-r3-20261007`。每个 arm 的 `evidence/source-before-run/MANIFEST.json` 绑定了采集器和 Python DDL 的原文副本；本次核验四份副本的 SHA 均符合各自清单。主要冻结副本如下，编号以 `first_up-a/evidence/source-before-run/` 为准：

- `0002-tb02a_runtime.py`：`collect_core` 第 182 行起、`execute` 第 257 行起。
- `0039-tb02c_runtime.py`：第 20 行起，在基础记录落盘之后做正常恢复并追加两条记录。
- `0054-tb01c_runtime.py`：第 28 行 `TABLES`，第 80 行 `request_for`，第 187 行 `send`。
- `0010-s_query_integrity.py`：第 30 行 `LEGACY_SCHEMA`、第 166 行 `CONTROL_SCHEMA`、第 196 行 `EXPECTED_V2_OHLC`；第 211 行起核权威模式。
- `0016-s_tb02_contract.py`：第 22 行 `SCHEMA`。
- `0047-tb02c_harness.py`：冻结计划生成逻辑。

`RUN-PLAN.json` 的 fixture SHA 为 `32d00df0b3e349411b79e34e284242bce5b63dce97a03b6cb2eb17a8dec27aa4`。冻结 `raw-ledger.json` 的两个 case 各 28 根；四份 `messages.jsonl` 均含严格顺序 `op-0000` 至 `op-0027`，每条唯一 raw 事件的 `seq` 为 `0` 至 `27`，payload hash 逐项正确。每份 clock 含这 28 个操作及唯一 `recover-normal`，共 29 项。两 arm 的消息字节相同：上行 SHA `c9d3239819df5ac521764dfa2c607d9cbbd5f94706aeac2b851b1b8b84351396`，下行 SHA `bc201fc5470b8b87e93cbfe37b495d03e1e1fd67d60c419ad73ba3545fa752c6`。

## 记录布局及身份

下表行号是从控制流计算出的 1 起始位置。

| 预期位置 | kind | 数量 | 必须保留的顺序与身份 |
|---|---|---:|---|
| 1–28 | `actual_ingest_result` | 28 | `operation` 为文本 `0` 至 `27`；`result` 为该原消息的完整 Received/response。`Run.send` 已验证 S 回应与原请求绑定、`Committed`、`accepted_seq=i`。 |
| 29–56 | `final_receipt` | 28 | `operation=i`；请求 message_id=`final-receipt-i`，producer_id=`tb01c-input-driver`，producer_epoch=`1`；payload 四元原身份取 messages[i]，另含 op=`receipt`；完整 response，Committed、accepted_seq=i、S producer_epoch=1。 |
| 57–112 | `public_candidate` | 56 | 对每个 i=0..27，先 `live-i`，再 `known-i`，不可先聚合全部 live。live 的 client=`live`，i=0 用 load，其后 watch；known 的 client=`history`、op=`load`、mode=`AsKnown`、generation=文本 i+1。 |
| 113–140 | `public_candidate` | 28 | `revisit-i`，i=0..27；client=`history`、op=`load`、mode=`AsKnown`、generation=文本 i+1。运行时已与对应 known 的完整 candidate 相等比较。 |
| 141–157 | `authoritative_table` | 17 | 表名按字典序，含空表；columns 按 DDL/PRAGMA 列顺序；rows 按 canonical JSON 字节排序，不得丢列、丢行或替换 payload。 |
| 158 | `authoritative_schema` | 1 | rows 为完整 `(type,name,tbl_name,sql)`，按 type/name 排序，含自动索引记录。 |
| 159 | `lifecycle_semantics` | 1 | value 必须为 `{}`。本片 fault_index=None，正常恢复不会改写基础 self.lifecycle。 |
| 160 | `normal_recovery_cut` | 1 | generation=`16`，before/after 各为完整公共 candidate，二者相等。 |
| 161 | `normal_recovery_cut` | 1 | generation=`28`，before/after 各为完整公共 candidate，二者相等。 |

各 kind 的顶层键集依次为 `{kind,operation,result}`、`{kind,operation,request,response}`、`{kind,command,candidate}`、`{kind,table,columns,rows}`、`{kind,rows}`、`{kind,value}`、`{kind,generation,before,after}`。计数正确仍不足，必须同时核身份、顺序、键集和无额外记录。

会话身份分别是 `tb02c-first_up` / `tb02c-first_down`，source_namespace 分别为 `testonly.ohlc.tb02c-first_up` / `testonly.ohlc.tb02c-first_down`。session_generation、source_epoch、原消息 producer_epoch、原消息 writer_epoch 全为文本 `1`；原消息 producer_id=`tb02c-input`。同方向 a/b 保持这些业务身份一致，仅进程、控制目录、DB/socket/端口不同。

注意：实际 HTTP 查询共 88 条，但基础 core 的 public_candidate 只有 84 条。额外四条 `before-normal-16`、`before-normal-28`、`after-normal-16`、`after-normal-28` 在 `super().collect_core()` 写完后才执行，其四份完整 candidate 嵌入最后两条正常恢复记录；不得再补计四条 public_candidate。实际停机/换代/重启回执另存 `normal-recovery.json`，不混入空 lifecycle_semantics。

## 17 张权威表可由冻结 DDL 取得

AST 读取 `LEGACY_SCHEMA + CONTROL_SCHEMA + tb02.SCHEMA` 的字符串即可取得清单，不必连接数据库，也不要 import `s_query_integrity`，其模块初始化会打开内存 SQLite。结果恰为 TB01C 的 15 张表加 `raw_ohlc`、`structure_facts`，按名排序：

```text
141 batches
142 catalog
143 meta
144 objects
145 observations
146 raw_events
147 raw_ohlc
148 relations
149 s_clock_events
150 s_delivery_policy
151 s_delivery_refs
152 s_input_messages
153 s_protocol_meta
154 structure_deltas
155 structure_facts
156 witnesses
157 writer_epoch_history
```

本片 profile 为 `ohlc_integer_tb02a_v1`、协议为 s-session/2，因此对应 `EXPECTED_V2_OHLC`，不是任选四种历史模式之一。DDL 没有 AUTOINCREMENT，不产生 sqlite_sequence。列顺序/约束来自上述 DDL；表行实际内容必须完整比较，不能用表名或哈希代替。Rust 建表来源对应 `s_structure_session.rs:160`、`s_session_v2/mod.rs:12`、`s_session_v2/tb02a.rs:6`，TB-02-C 在既有结构事实表发布，不新增专用表。

## 四 arm 终态证据

四个 arm 均有下列证据，已逐份读取小型 JSON：

- `<arm>/EXIT.json`：exit=0。
- `<arm>/evidence/RESULT.json`：`CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER`，cleanup.errors=[]。该通用文件的 semantic_oracle/browser 占位字段仍为 not_evaluated，不能覆盖下面专属成功回执。
- `<arm>/evidence/normal-recovery.json`：stop=stopped、recover=recovered、start=ready；cuts=[16,28]、equal=true；使用事前 clock 的 recover-normal，writer 换代为 2。
- `<arm>/evidence/query-oracle.json`：status=passed。
- `<arm>/evidence/browser/RESULT.json`：status=passed、cuts=["16","28"]、page_errors=[]，Chromium 153.0.8010.12；同目录有 HAR、CC-011/012/013 文本和 developing.png。

总进程原回执仍为 `EXECUTION.json` exit=1，原因是旧比较器拒绝超 16 MiB 单记录。以上只能证明采集终态成功和可独立制定完整布局，不将其改写为完整双跑已通过；本轮未运行或审阅正在修复的比较器。
