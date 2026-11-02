# #1653 非作者独审（claude-fable-5-1，只读）

- 审查对象：PR #1662，分支 `sandcastle/issue-1653`（`ecc72af992` 实装 + `41fb2e53ce` 评审工蜂注释修正），base origin/main `02b5b571eb`。
- 路由：Claude Code CLI 经 CLIProxyLocal（本机代理端口 8318），模型 `claude-fable-5-1`；实装与评审工蜂均为 `claude-opus-5-5`，本审查非作者。
- 工具限制：Read/Grep/Glob 与只读 git；未跑 cargo。审查提示原文：`prompt.md`。
- 日期：2026-10-21。

---

结论：PASS

## 审查范围

`git diff 02b5b571eb...HEAD`，4 个文件、2 个提交（`ecc72af992` 实装 + `41fb2e53ce` 注释修正）。全部只读，未跑 cargo，未调 gh。

## HIGH

无。

## MEDIUM

无。

## LOW

1. **参考文档未同步新终止态** — `docs/reference-theta-v0.md:31`、`rust/src/theta_v0/classifier/mod.rs:35`、`rust/src/theta_v0/classifier/pipeline.rs:467-472`（`classify` 的函数级文档）仍只描述「自然终止 / INCOMPLETE_RESOURCE_BOUND」两支，没有提到 `PARSE_INVALID`。票面只要求枚举与注释同步（已在 `pipeline.rs:407-413` 的枚举文档和 `pipeline.rs:546-551` 的 `classify_incremental` 边界段落做到），所以不阻断；但下游读参考文档的人会以为 `code()` 只有两个值。建议顺手补一句。

2. **模块级缺值文档仍漏「`value` 键缺失」一例** — `rust/src/theta_v0/policy_records.rs:15-16` 写的是「字段缺失、字段直写或 `value` 为 `null`/`"AwaitingPolicy"`」，而 `read_u32` 在 `policy_records.rs:177` 对 `{"l_max":{}}`（对象里没有 `value` 键）也判 `AwaitingPolicy`。变体文档 `policy_records.rs:70-71` 已经写全了（含 `value` 缺失、`version` 缺失/null），只是模块头部没跟上。这是基线就有的遗漏，不是本 PR 引入的。

3. **正控断言依赖夹具规模** — `rust/src/theta_v0/parser/tb1581_tests.rs:230-234` 断言 `warm.termination.code() == NATURAL`。`warm` 是 2000 根合成 bar 在第 1700 根处的全量分类，若将来夹具或 `l_max` 变化导致塔到达上界，这条会变成 `INCOMPLETE_RESOURCE_BOUND` 而红，但红的原因与本票无关。作者声明默认档全绿，我未跑 cargo，仅提示这是一个与 `l_max=6` 耦合的断言。

## 逐项核对

**1. ParseInvalid 新变体的全部消费点。** 全仓 grep `TowerTermination`、`.termination`、`is_resource_truncated`、`INCOMPLETE_RESOURCE_BOUND`、`"NATURAL"`、`PARSE_INVALID`：

- 生产代码里读 `ClassifyOutput.termination` 的只有 `pipeline.rs:402-404` 的 `is_resource_truncated()`，它用 `matches!(…, ResourceBound{..})`，`ParseInvalid` 自然落到 false——拒读不会被误判为资源截断。
- `code()`（`pipeline.rs:440-446`）是唯一的 `match`，三个变体全覆盖，编译期穷尽。测试里 `tests/tower_policy.rs:44-55` 的 `match` 有 `other =>` 兜底，不受影响。
- `TowerTermination` 和 `ClassifyOutput` 都没有 `Serialize` 派生（`pipeline.rs:383`、`:414`），`code()` 在生产代码中没有调用者——没有任何已入仓夹具或对拍输出含这个标记。S 会话侧 `s_structure_session.rs:5802`、`s_tb02_contract.py:661`、浏览器 JS 里的 `INCOMPLETE_RESOURCE_BOUND` 都是线段第二种候选的 **scan coverage** 字段，与塔终止态是两套东西，不受影响；S 侧的 `PARSE_INVALID_REASON = "segment_parse_invalid"`（`tb02c.rs:19`）是独立常量，不与新 `"PARSE_INVALID"` 串接。
- 空 ParseLayer 早返回（`pipeline.rs:717-724`）仍记 `Natural`，且位于 `classify_incremental_inner` 内，而拒读检查在 `classify_incremental` 入口（`pipeline.rs:565`）先于它——带标记的空层也会正确记 `ParseInvalid`，与枚举文档「空输入 ⟹ 自然终止」的分界一致。

**2. 断言锁。** `tb1581_tests.rs:186-191` 和 `:222-225` 两处均为 `assert_eq!(out.termination, ParseInvalid{from: …})`，把早返回换回 `Natural` 两处都会红——锁是实的。前者的 `unwrap()` 安全：`:164` 已断言 `seg_start == Some(0)`，`:165` 断言与 `bad.segments_parse_invalid_from` 相等。`from` 值语义正确：`parser/mod.rs:118-122` 定义该字段为失败段的 `seg_start` 笔号，与枚举字段文档「失败段起点笔号」一致。正控存在：`:230-234` 同一现场无标记时为 `NATURAL`，同时 `:226-229` 锁住 `code()` 映射。

**3. null 负控 ↔ `read_u32`。** `policy_records.rs:378-385` 的 `{"l_max":null}` 走 `:171` 的 `entry.is_null()` 分支。我手推了删掉该分支的后果：`entry.as_str()` 对 null 返回 `None`，落到 `:174-176` 的 `as_object()` 失败 ⟹ `Invalid`，与注释「只删 `is_null` 分支会退成 Invalid」一致，断言用 `assert_eq!` 精确到 `field: "l_max"`，比旁边的 `matches!` 严。`AwaitingPolicy` 变体文档（`:70-71`）列的四种来源逐条对上：字段缺失 `:168`、直写 null/AwaitingPolicy `:171`、`value` 缺失 `:177`、`value` 为 null/AwaitingPolicy `:179-180`、`version` 缺失/null `:132-138`。

**4. rule_revision 拒绝条件等价性。** 旧：`unwrap_or_default()` 得 `""`，`"" != RULE_REVISION` ⟹ 拒。新：`None != Some(RULE_REVISION)` ⟹ 拒；`Some(r)` 分支与旧完全相同。拒绝集合不变。文案：缺键时显示「meta 缺 rule_revision 键」，有键时格式不变。新测试 `mod.rs:3372-3404` 用 `DELETE FROM meta WHERE key='rule_revision'`（`meta` 表确为 key/value 结构，`s_structure_session.rs:400`），并断言 `VersionMismatch：` 前缀、含缺键文案、不含 `rule_revision=\`\``、socket 未绑定。`serve` 在到达 `:2492` 前调用的 `verify_writer_epoch`/`verify_reachable_root`/`active`/`check_clock`/`load_profile` 都不读 `rule_revision`（grep 证实），所以删键后确实走到目标分支而非在更早处以别的错误退出。

**5. 范围与注释。** 改动严格落在票面四项内，无额外重构。注释带 `#1653`/`#1603`/`#1576`/`#1581` 票号与本仓既有惯例一致（周边代码全是这种写法）。第二个提交只补了 `ClassifyOutput.termination` 字段注释一句，无行为变化。

## 实际看过 / 未核实

**看过**：完整 diff；`pipeline.rs` 376-590、706-730；`tb1581_tests.rs` 140-246；`policy_records.rs` 1-260、340-398；`s_session_v2/mod.rs` 2460-2545、3225-3404；`tests/tower_policy.rs` 20-60；`parser/mod.rs:118-122`；全仓 grep 结果（`TowerTermination`、`termination`、`code()` 消费、`rule_revision` 读点、`meta` 表结构、`RULE_REVISION`）。

**未核实**：没有跑 `cargo test`（默认档与 `s_session` 档）、没有跑 `-D warnings`，「新断言旧红新绿」与「全绿」仅凭静态推演与作者声明；LOW-3 的正控是否确为 `NATURAL` 需实跑才能确认；没有核对 `docs/` 或 ADR 是否有本票要求的其他文档更新（票面未列）。
