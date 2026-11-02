# 收据 workers/runner-review-0049/REVIEW.md

- 原件：`workers/runner-review-0049/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`df0a3e67522fc086502b70ce94038c0b52bda4087cfbd61fb61c50847d800979`
- 字节数：5615；行数：52

## 关键行（行号: 原文）

```
1: # #1488 六 driver runner 适配独审（0049）
3: - 评审对象：`cdfe51a317108d28aec5be550dad10f034dd3c49`（detached），差异 `git diff fff166540e cdfe51a317`
6: - **结论：FAIL**（1 条阻断：SOURCE_FILES 口径和旧版不一致，从 580 文件降成 3 文件）
8: ## 关键事实：作者用错了基线
17: - 作者报告写“两份 CI 文件在 HEAD 与 5cb7253d46 上 blob 相同……即确为先前独审、跑在 5cb7253d 上的版本”，这个判断有误。`5cb7253d` 树里放的是更早的 9160 时代 runner。run 39095659808 校验的对象是 5cb7253d 源码，但它用的 runner 出自 654a7f。
19: ## 逐项核查
21: ### (1) 差异范围：PASS（有一处附带说明）
25: ### (2) REVIEWED_SOURCE/TREE：PASS
30: ### (3) SOURCE_FILES：**FAIL**
36: - 影响评估：`identity()` 还做了几项 git 层面的检查——clean checkout、HEAD==GITHUB_SHA、REVIEWED_SOURCE^{tree}、祖先关系、尾差恰为 CI_FILES。所以缩口径**目前看不出能直接利用的绕过路径**。不过它去掉了旧版那层独立于 git 对象模型的全量内容绑定，没有满足指令里“口径与旧版相同，未漏列”这一条。
37: - 修复建议：把 SOURCE_FILES 换成 fff166 上 `rust/` 全部 580 个文件的哈希，报错文案改回 580 口径。完整映射已经算好，放在 `workers/runner-review-0049/SOURCE_FILES-580-fff166.json`（sha256 `de5bd699…1cbf`），每一项都用提交对象和工作树两种方式算过，结果一致，可以直接作为下一稿的对照。改完需要重新独审。
39: ### (4) TESTS：PASS
40: 在 `fff166540e:rust/src/bin/s_session_v3/driver.rs` 里，六个名字各出现恰好一次 `fn <name>`，都在 `mod tests`（第 465 行）之内。每个函数上方都是 `#[cfg(feature = "s_capture_native_testonly")]` 加 `#[test]`，没有 `#[ignore]`。这个 feature 在 FEATURES 里。新锁名在第 2127 行。旧锁名 `…releases_candidate_after_successful_normal_append` 在源码里只出现在第 2112 行的文档
42: ### (5) 尾差：PASS
45: ### (6) LEGACY_RUNNER：PASS
46: `.github/scripts/run-1488-delta-ci.py` 的哈希：fff166 树里是 `2c94e9e8…97dc`，工作树里也是这个值，和 `LEGACY_SHA256` 一致。`git diff 9160fc9d fff166540e` 里没有这个文件，常量保持原值是对的。
48: ### (7) 凭据与外部写入：PASS
51: ## 附带意见（非阻断）
52: - 作者报告里“确为先前独审版本”的溯源结论需要更正。后续以 654a7f 为基线时，应该写明它不在 fff166 的祖先链上。
```
