# #1653 Sandcastle 评审工蜂结论（摘录）

来源：宿主 `.sandcastle/logs/sandcastle-issue-1653-reviewer.log`（review base `02b5b571eb`，2026-10-21T08:32:19Z 起，08:39:02Z 完成）。以下为日志中评审结论原文。

---

评审通过，追加了 1 个修正 commit。issue 要的四项都完整落地，功能语义没动。我修的问题只有一个文档注释。

**已看的文件**（diff 基准为 `02b5b571eb`）：
- `rust/src/theta_v0/classifier/pipeline.rs`
- `rust/src/theta_v0/parser/tb1581_tests.rs`
- `rust/src/theta_v0/policy_records.rs`
- `rust/src/bin/s_session_v2/mod.rs`

**结论**
- **拒读终止态（#1581）**：新增 `TowerTermination::ParseInvalid { from }`，标记串是 `PARSE_INVALID`；拒读早返回改记它。我在全仓搜过，没有别处对这个枚举做穷举 `match`，所以加一个变体不会让其他代码编译失败。`is_resource_truncated` 对新态返回 false，符合语义。tb1581 的两处断言锁，加上"无标记时为 NATURAL"的正控，在远端都跑绿了。
- **直写 `null` 负控**：和 `read_u32` 里的 `entry.is_null()` 分支一一对应，断言精确到 `id` 和 `field`。
- **`AwaitingPolicy` 注释**：我对照了 `parse_header` 和 `read_u32` 的实现。注释里写的"`version` 缺失或为 `null` 同判此类"与代码一致。
- **`rule_revision` 缺失文案**：改成 `Option<&str>` 后，文案明确说"meta 缺 rule_revision 键"。拒绝的条件和原来等价：键缺失时原来比的是空串，同样不等于当前规则版本，照样拒绝。新测试沿用了同文件 #1576 测试的夹具写法。
- **commit message**：有票号和编号声明，改动没超出 issue 范围，也没写入密钥。

**修正**
- `eeaabb2186 review: #1653——ClassifyOutput.termination 字段注释补 ParseInvalid 态`：`ClassifyOutput.termination` 的字段注释原来只写了"自然终止／资源截断"两种，已和新增的第三态对齐。

**远端验证**（nc-remote，提交 `eeaabb2186`）
- `cargo test --locked tb1581`：3 个通过，另有 3 个 ignored，本来就是手动跑的对拍和探针测试。
- `cargo test --locked policy_records`：4 个通过，包括 `negative_controls_missing_unsigned_invalid`。
- `cargo test --locked --features s_session --bin s_structure_session v2_serve_stale`：2 个通过。日志里能看到 #1576 那条通过；"2 passed"说明另一条，也就是本分支新加的缺键测试，同样通过。
- `RUSTFLAGS="-D warnings" cargo check --features s_session --bins --lib`：**失败，报 88 个错误**。这些全是 dead code 未使用、多余的 `mut`、调用已弃用方法之类的告警。我没在改动行里看到任何一处，`pipeline.rs` 里那 3 处也不在本分支的改动段内，看起来是远端工具链对存量代码报的。但 nc-remote 不接受对基线提交 `02b5b571eb` 打包（报 "Refusing to create empty bundle"），基线上没跑成，所以"不是本分支引入的"只是推断，没有验证。验收要求的 `-D warnings` 零告警，以 CI 结果为准。
- 不在我这次范围内、交给编排层 / CI 的：`cargo test` 默认档和 s_session 档的全量测试；验收要求的红绿两次 CI run（旧实现红、新实现绿）；记录到 `.chanlun/review-results/`。

<promise>COMPLETE</promise>
