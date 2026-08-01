## ⚠ 本票的读数是在错的树上量的——主控独立复跑查出

派出的子代理报「2169 passed / 0 failed」，**该数字属实，但它跑的树不是当前在写的代码**。

### 三层事实（主控本地实测，2026-07-30）

**一、CI 盯的分支落后当前主线 545 个提交。**

```
$ git rev-list --count origin/main-rewritten..main
545
$ git log --oneline -1 origin/main-rewritten
61ed615393 feat(audit): #558 claude_audit 迁 mcp v2……
```

`.github/workflows/ci.yml` 的触发器是 `on: push/pull_request → branches: [main-rewritten]`。该分支是 `main` 的只读镜像（CI 触发器挂它，`main` 不推 origin），**镜像已停止同步**。⟹ **CI 当前跑的全部检查，测的都是 545 个提交之前的代码**，与 `cargo test` 接不接入无关。

**二、子代理的 0 failed 来自那棵旧树。** 它如实报告了「worktree 初始 checkout 在一条极旧的孤立历史线（`19b4015927`）上」，随后切到 `origin/main-rewritten`——切对了 CI 分支，但**没意识到该分支本身已落后 545 提交**。

**三、当前真实代码上有 2 条测试是红的，且正是 Lean↔Rust 对拍断言。**

```
rust/tests/theta_v0_classifier_parity.rs
  ✗ type1_buy_broke_and_diverge_matches_lean_istype1  (:528)
      assertion `left == right` failed:
      Lean IsType1（brokeCenter ∧ IsDivergence）⟺ rust buy1 置位
      left: 0 / right: 1
  ✗ signal_extraction_emits_no_second_class_only      (:629)
      panicked: 产第一类（趋势背驰：C 段破最后中枢 ∧ C<A 面积）
```

复跑两次稳定复现（`cargo test --test theta_v0_classifier_parity` → `17 passed; 2 failed`）。**核实非本会话引入**：当前分支相对 `main` 的 12 个提交 `git diff --stat main..HEAD -- rust/` 为**空**，全是文档。

### 对本票与上游裁定的影响

- 本票的验收「CI 上 `cargo test` 真的执行且为绿」**在当前形态下会成立但无意义**——CI 盯的旧树上确实是绿的。
- [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 裁定十一的附带原则「验收判据必须真的被 CI 执行才算数」**需要加一句**：还得是**在当前代码上**执行。「只编译不执行」之外，本仓还有第二种空转形态——**执行了，但执行在旧树上**。
- [#804](https://github.com/xy7365527-lang/NewChanlun/issues/804) 裁定一把总缝规则的机械锁定为对拍测试锁，其落地前提由「CI 跑 `cargo test`」升级为「**CI 在当前代码上跑 `cargo test`**」。
- [#806](https://github.com/xy7365527-lang/NewChanlun/issues/806)「一二档的锁都已打好、没接进任何 gate」再加一层：**接进去也未必看得见**。

### 本票的处置建议（待编排者拍）

本票**不关**，拆成先后两截：

1. **先解决 CI 分支归属**（同步 `main-rewritten` 到 `main`，或把触发器改挂 `main`）——**涉及 push，属不可逆动作，须编排者决定**；
2. 本票的 `ci.yml` 改动（`rust-check` 末尾追加两步 `cargo test`，沿用现有缓存、既有 `#[ignore]` 未动）**改法本身正确**，但它坐在 `origin/main-rewritten`（旧线）上的分支 `fix/810-ci-cargo-test` @ `d7e9798309`，**未合入、未 push**；等第 1 步定了再重新落到正确的基。

两条红对拍另开票记录，见下方链接。
