# #1603 独审后尾部快速轴评审

- 评审者：claude-opus-5-5（本机 claude CLI，经 CLIProxyLocal；独立会话、非作者；只读工具）
- 发起：主控合入代理 /root/merge_train，2026-10-21
- 被审：`59374c8e73`、`c126e53032`（独审第 2 轮原审至 `7fd1ac3f8c`）
- 结论：APPROVE（LOW-1：直写 null 无负控；LOW-2：AwaitingPolicy 文档注释未同步）

---

## 评审结论（#1603 尾部两个提交，只读）

**结论：可以通过，没有 HIGH / MEDIUM 问题，只有两条 LOW。**

### 59374c8e73 的改动是否正确

- **语义正确，符合 ADR 0041。** 改动在 `read_u32` 里加了一个判断：字段如果直接写成 `null` 或 `"AwaitingPolicy"`，先判为「未签」（AwaitingPolicy）并直接返回。不回退到 6 或 12/26/9 这类备用值，符合 ADR 0041 第二节「可调字段不得有备用值」和第三节第 2 条「未签就是 AwaitingPolicy，不是第二个取值」。
  - 判断顺序没问题：这一步在「必须是对象」的检查之前，所以裸写的数字（如 `"l_max": 6`）仍然判为格式错（Invalid），已有的负控 `bare` 继续守住这一点。
  - 规范写法 `{"value": null / "AwaitingPolicy"}` 原来的处理路径没有变。
- **负控确实能抓住回退。** 新增的 `bare_unsigned` 用例写的是 `"l_max":"AwaitingPolicy"`。如果删掉新加的判断，这个字符串不是对象，会被判成 Invalid，`matches!(…AwaitingPolicy…)` 就会失败。所以「整段删掉」这种回退一定会让测试变红。
- 我在 worktree 里跑了 `cargo test --release --lib policy_records`，4 个测试全部通过。

### c126e53032 是否只是格式化

- 是，没有语义变化。我把每个文件去掉空白、去掉结尾逗号后逐一比对：
  - `pipeline.rs`、`tower_policy.rs`、`config.rs` 三个文件内容完全相同。
  - `policy_records.rs` 只剩一种差异：rustfmt 把 `=> expr,` 形式的分支改成了 `=> { expr }`，两种写法等价。
- 字符串字面量（包括测试里的原始 JSON 串）内容都没变。`cargo fmt --check` 也是干净的。

### 发现

- **LOW-1 `policy_records.rs` `read_u32`：直写 `null` 这条分支没有负控。**
  - 新用例只覆盖了 `"AwaitingPolicy"` 字符串。
  - 如果有人只删掉 `entry.is_null() ||`，`"l_max": null` 会退回成 Invalid，而现有测试全部仍然通过。
  - 后果不严重：Invalid 同样会阻止运行，不会用上备用值，只是报错类型不对。
  - 建议补一个 `{"l_max": null}` → AwaitingPolicy 的用例。
- **LOW-2 `PolicyError::AwaitingPolicy` 的文档注释没跟着改。**
  - 注释仍写「字段缺失，或 `value` 为 `null` / `"AwaitingPolicy"`」，没有提到这次新增的「字段本身直写」这种形式。
  - 另外，记录头的 `version` 字段写 `"AwaitingPolicy"` 字符串时仍判 Invalid（写 `null` 才判 AwaitingPolicy），和字段这边的口径略有不一致。这一点是改动前就有的，不在本次 diff 范围内，只记一笔。

VERDICT: APPROVE
