# #1370 TB-01-A 修复报告（本地耐久性 + 精确整数 wire）

> 票：GitHub Issue #1370（TB-01-A，SPEC #1340 / 图 #1323 / SPEC 载荷 #1339 R2）。
> 基线：`efc1ddc4d6c015f4f8c6d44f904d704a88308b46`；分支 `codex/1323-tb01-a-1370`。
> 编号声明：本提交中的 #1370 仅指 GitHub Issue。
> 本报告是工作草稿，不覆盖旧 IMPLEMENTATION/AC/review 报告，不以自修 PASS 冒充根最终验收。

本轮修复范围：`rust/src/bin/s_structure_session.rs` + 同票 Rust 靶向回归测试（bin 内 `#[cfg(test)]`）+
`s_session/s_readonly_server.py`。**未改** `s_session/browser/index.html`（根并行修复并独立提交）。

---

## 1. DUR-H01 —— 正常并发追加后永久留下 begun（活性缺口）

**反例原状**：advance A 在 Begin 后与 accept 追加 1 条交错，旧推进按前沿变化退出 1，但 `advance_state`
仍为 `begun:tokenA:genA:frontierA`，后续所有 advance 永远被拒（错误只建议 reset/TB-05）。不是杀进程恢复。

**修法**：保留 epoch/token/generation 所有权。新增 `cancel_own_begin`，仅在 Commit 检测到「已知前沿变化、
本 attempt 尚未公布结构 Commit」时调用；它在同一 IMMEDIATE 写事务内 CAS 核 `writer_epoch==WRITER_EPOCH`、
`advance_state==begun:{token}:{gen}:{frontier}`、`begin_token==token`、published `generation==gen-1` 全部匹配
后才把本 attempt 的推进占用置回 `idle` 并记录 `last_cancelled_begin`。任一不匹配即不清除（不清别人 Begin、
不把未知中断自动放行）；崩溃/悬挂修订仍保留 Begin/闭门（TB-05）。

**实际命令/退出码**（5000 条合法事件 + 1 条交错追加）：
- advance A：exit **1**，stderr 含「本 attempt 已正常取消并释放推进权」；之后 `advance_state=idle`。
- 后续 advance B：exit **0**（generation=1、raw_events=5001、windows_total=4999）。
- 探针脚本：`/tmp/s1370_interleave.py`（poll Begin → accept 追加 → 核 A 退出 + 状态 + B 成功）。

**边界**：只清理「持有本 Begin 的活着的操作自己已知失败的尝试」；新代际/不同 token/已发布 generation
变化时零清理。重复同输入身份仍复用原收据，无 reset 删库解锁。

## 2. DUR-H02 —— S.AcceptInput 写事务绕过 writer_epoch

**反例原状**：受控库 `writer_epoch=2` 时 advance 拒 StaleWriter，accept 却仍接纳 3 条并持久收据。

**修法**：抽出 `verify_writer_epoch`，AcceptInput 在其 IMMEDIATE 写事务内、任何 raw/收据/profile/meta
写入前先过同一 fence；Advance 的 Begin 与 Commit 也复用同一谓词（去掉两处手写 `epoch != "1"`）。
epoch 不匹配 → StaleWriter、零接纳、零收据、零 profile 更新。

**实际命令/退出码**：受控 `UPDATE meta SET value='2' WHERE key='writer_epoch'` 后 accept → exit **1**
（`StaleWriter：writer_epoch=\`2\``），`raw_events` 行数 **0**。

**边界**：本片当前固定 `WRITER_EPOCH="1"`（不实现监督器/跨主机接管换代），只做受控前件拒绝。

## 3. DUR-M01 —— 相同事件重放改写同一 cut 的 profile 来源

**反例原状**：相同 profile_id 仅 note 字节变化后重放，事件全部 replay 返回旧收据，accept 却覆盖
`meta.profile_hash`；同一 cut/index_frontier 的 Snapshot 声称新 hash，已封存 batch 仍旧 hash。

**修法**：session 的 `profile_id/profile_hash` 在**首次接纳时固定**；后续 accept 若 profile_id 不同 → 拒绝；
若 profile_id 相同但规范字节哈希不同 → `IdentityConflict` 拒绝（零收据/零覆盖）。`input_profile`/
`input_file_hash` 仍记录本次输入档案，不属 cut 来源绑定。

**实际命令/退出码**：首 accept(4-branch fixture) → advance；再以同 profile_id 改 note 的 profile accept →
exit **1**（`IdentityConflict：profile_id=... 已绑定规范哈希 ... 本次字节哈希 ... 不同`）；
`meta.profile_hash` 前后均为 `0e9dfc...` 未变。

**边界**：profile 修订（新版本/新输入绑定）另行承接，本片不覆盖旧 cut。

## 4. 三个同价输入无法发布域外事实（observations 唯一约束）

**反例原状**：全新库接纳 `[10000,10000,10000]` 后 advance exit 1，`UNIQUE constraint failed:
observations.observation_id`。根因：全等三K无方向时 parser 不合并（merged==raw==3），raw 级域前件与
merged 级 `DomainNotSatisfied` 对同一窗口 `[0,1,2]` 各报一次 `adjacent_inclusion`，observation 身份
`(kind, window_start, window_mid, window_end, reason)` 相同 → 主键冲突。

**修法**：observation 身份即事实身份；对 `(kind, window_start, window_mid, window_end, reason)` 去重
（同一事实只发布一次），不删不同窗口/原因的合法事实。域外数据发布 `domain_not_satisfied`，不判为分类
成功、不造第五 Other 分型。

**实际命令/退出码**：`[10000,10000,10000]` advance → exit **0**；snapshot `objects=0`、
`observations=[{kind:domain_not_satisfied, reason:adjacent_inclusion, window:[0,1,2]}]`。
0/1/2 点知识不足对照仍通过（insufficient_knowledge）。

**边界**：去重键不含 `detail`（detail 是证据不是身份）；不同窗口/原因保留。

## 5. WIRE-M01 —— 精确整数 wire 投影（无 JS safe 上限字段字符串化）

**反例原状**：`input_refs[].raw_refs[].seq`、`witnesses[].raw_bars[].seq`、`witnesses[].merged_source_index`、
`accept.results[].seq`、`advance.generation` 以 JSON number 外发，无 `<=2^53-1` 上界，JS Number 可舍入。

**修法**：内部计算/排序/SQLite INTEGER 语义不变；只在「明确外发投影」转规范十进制字符串。

**具体字段投影合同（外部 wire，只转这些）**：
| 字段 | 内部（不变） | 外发（本修复） |
|---|---|---|
| `accept.results[].seq`（接纳序） | i64 | 字符串 |
| `advance.generation` | i64 | 字符串 |
| `snapshot.objects[].object_revision` | i64=1 | 字符串 `"1"` |
| `snapshot.objects[].window_start/mid/end` | i64 | 字符串 |
| `snapshot.objects[].input_refs[].merged_index` | usize | 字符串 |
| `snapshot.objects[].input_refs[].raw_refs[].seq` | i64 | 字符串 |
| `snapshot.witnesses[].slot` | usize∈{0,1,2} | 字符串 |
| `snapshot.witnesses[].merged_source_index` | usize | 字符串 |
| `snapshot.witnesses[].raw_bars[].seq` | i64 | 字符串 |
| `snapshot.observations[].window_start/mid/end`（非 null） | i64 | 字符串（null 仍 null） |

**不改**：batch canonical JSON 的数值（内部封存，规范字节/对象身份/hash 不变）、catalog 有限分类码与
counts、evidence 计数、comparisons `prev/cur`（已字符串）、merged OHLC 与 raw `price/ts/volume`（已字符串）、
`strict_up/held`（布尔）。Rust `read_raw_events` 内部 `ev["seq"]` 仍是数值（`as_i64` 计算路径不退化），
`build_object_and_witnesses`/对象身份/批次仍用数值内容寻址。

**实际命令/退出码**：Rust snapshot 与 Python `/api/snapshot` 对同一库逐字段核对，上述字段均为 `str` 且
Rust/Python 一致；>2^53 价格/ts/source_coord 仍字符串无损往返。`cargo test --features s_session --bin
s_structure_session` → 2 passed（`canonical_integer_rejects_non_canonical`、
`wire_projection_stringifies_coordinates_and_seq`）。

**边界**：不把「原生小接纳序样例无损」当成全接纳序域已证；不全局递归改写已签目录；不替 JS 前端转字符串
（服务端投影，前端只读文本）。

## 6. 受影响检查（真实退出码）

| 命令 | 退出码 | 结果 |
|---|---:|---|
| `cargo fmt -- --check` | 0 | 通过 |
| `cargo check --all-targets`（default） | 0 | 通过 |
| `cargo check --features s_session --bin s_structure_session` | 0 | 通过 |
| `cargo build --features s_session --bin s_structure_session` | 0 | 通过 |
| `cargo clippy --features s_session --bin s_structure_session` | 0 | 本片文件 0 命中（lib 既有 warning 不计） |
| `cargo test --lib local_shape` | 0 | 4 passed |
| `cargo test --features s_session --bin s_structure_session` | 0 | 2 passed |

## 7. 未验 / 根接

- 真实 macOS launcher 与 GUI 浏览器、fullfsync 前件（根在 macOS 接，本容器未声称已过）。
- 独立 wire 复核（根接回）；本报告 Rust/Python 类型一致只是实施侧自核，非独评。
- 跨进程杀/重启/修订/Delta/Gap/背压/分页（TB-01-B/C、TB-05/07），本片不承担。
