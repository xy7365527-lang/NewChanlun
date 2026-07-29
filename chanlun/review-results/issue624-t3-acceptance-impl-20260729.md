# #624 T3 验收环实施车收口报告（claude opus）

> 日期：2026-07-29；性质：实施车 stdout 收口报告原文
> 原始日志：/tmp/issue624-dispatch-20260729.log

全部四件事落地，验证链全绿。收口如下。

## 1 · commit SHA 链（逐个可编译，全部实测）

| SHA | 内容 |
|---|---|
| `3180f8f446` | ① 对拍测试（F4/F6/F7/F8 五测）+ F1 迁入 `level_view` |
| `148c814312` | ② golden 锚（`golden_log.rs` + fixtures JSONL） |
| `66cce932dc` | ③ 三档消费方登记表（落进 `retrace_ledger/mod.rs` 模块头） |
| `8b8905def2` | ④ 删除 `first_retrace_replay.rs` + 注册行（裁定 A 5 删） |
| `1eb76f104b` | Standards 订正：golden 场景测 55 行超硬杠，拆两条 |
| `16417473cb` | 对拍报告入库 |

## 2 · 对拍报告 + fixtures 逐条统计

`chanlun/review-results/issue624-fixture-replay-diff-20260729.md`

九条 fixtures 先全量枚举（F1–F9，含各自输入与旧输出），再立**语义映射表**（旧 15 项词汇 → 新词汇，逐条标「同/加强/合并/有意改变/无对应面」），最后对拍：

- **PASS 4**（F4 同 move→`NotAdjacent`；F6 晚到成功→静默吸收；F7 Supersede→Restart→新档谱系载荷；F8 空输入）；**FAIL 0**
- **不覆盖 2**（F3/F5）：seed→唯一 CompletedMove 映射及三个 fail-closed 码随裁定 A 删除。这是净减语义，报告 §四 C 显式登记：职责归 provider/`level_view` 侧，删除不改任何现有生产行为（旧模块零调用点），但将来接真实 provider 时须在上游重新落地
- **迁移 1**（F1）/ **已被主模块覆盖 1**（F2）/ **同义反复零损失 1**（F9）

## 3 · golden 锚 + 重放一致

`rust/tests/fixtures/issue624_retrace_ledger_events.golden.jsonl`（12 行 / 3608 字节 / 11 条修订，digest `0x808b849b9ba1d112`）。六场景：判胜、判败、Restart 新档、改口处死、迟到吸收、死人挂号。三层证据：逐字节相等 / 指纹钉常量且**对 11 条各改一次都验 `assert_ne`** / 从磁盘经真实 JSONL 边界 `load()`→`fold()` 复现锚定账本态并跑全量不变量。

迟到吸收与死人挂号**不在 golden 里**——它们按裁定六走 audit 流，由独立一条测试从警报计数与 audit 事件两侧正面钉死，报告里写明这是裁定六的可观测证据而非覆盖缺口。

## 4 · 消费方登记（0/3，全部未接线）

登记表同时进代码（`retrace_ledger/mod.rs` §消费方登记）与报告：死亡证明 → `trading::center_book::CenterBook`；证据包（+备战面）→ 交易层；短差档 → `signal` 一线 #606/#607。`grep` 实证：`src/` 内除 `classifier/mod.rs:92` 的模块注册行外零引用，与「0/3」一致。

## 5 · 删除段 + 全绿

删 572 行模块 + 3 行注册（**未删其他文件**；`closed_loop/{buy,sell}.rs` 的同名 `first_retrace` 字段未碰）。`StrictCompletedPair` / `RetraceOutcome` 两枚仍在用的域词汇迁入账本，derive 与字段一个 bit 不改。

验证：基线 `2195/0/137` → 收口 `2197 passed / 0 failed / 138 ignored`（+11 新 / −9 旧 fixture，ignored +1 = golden 重生成入口）；`--no-fail-fast` 全 target 零红；`cargo test --doc retrace_ledger` 的 `compile_fail` doctest 仍生效；`cargo check --all-targets` 通过；lib-test 警告数 53 与基线同，本票所改文件零警告；最长新函数 34 行、最长文件 269 行。

## 6 · 偏离 / 存疑

- **偏离**：F1（`C2VersionTuple` 三版本钉死）不是 firstRetrace 语义面，按「不得减测试覆盖」迁到了 `level_view/tests/projection_pairing.rs` 而非账本测试群——这动了删除授权范围外的一个文件，但是**新增**不是删除。
- **存疑（须你裁）**：§四 C 那条净减——seed→CompletedMove 唯一性证明现在仓内彻底没有了。裁定 A 的前提是「零生产调用点」，成立；但接真实 provider 那天这条 fail-closed 要有人补。是否现在就开一张 provider 侧的接线/补证票，归你定。
- **未做（不在执行车授权内）**：票面 AC 的「编排者终审记录在票」与「两轴 + 影子评审票」——前者是你的落字，后者需另派评审车。
