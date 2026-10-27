# #1629 独审后尾部快速轴评审

- 评审者：claude-opus-5-5（本机 claude CLI，经 CLIProxyLocal；独立会话、非作者；只读工具）
- 发起：主控合入代理 /root/merge_train，2026-10-21
- 被审：`b75b4b940c`、`af6976de06`、`cc244dc2d6`（独审原审至 `6b270b145d`）
- 结论：APPROVE

---

**评审范围**：只看 `b75b4b940c`、`af6976de06`、`cc244dc2d6` 三个提交，对比基准是 `6b270b145d`。`rust/` 下只动了三个文件：`evidence.rs`、`stroke.rs`、`issue1588_parse_mem_probe.rs`。我没有重跑测试，测试结果来自作者的收据 `logs/tests-af6976de06.txt`（190 passed，EXIT=0）。

### (1) 非测试构建的解析代码有没有改

- **`evidence.rs` 的实际代码改动只有两处**：在 `CoordsRef::materialize` 和 `CoordsRef::count` 的遍历循环里各加了一行 `bump_iter()`。`bump_iter` 定义在 `evidence.rs:209-212`，标了 `#[inline(always)]`，函数体整个包在 `#[cfg(test)]` 里。所以在非测试构建中它是空函数，这两行对生产逻辑、计算结果和性能都没有影响。其余改动全是文档注释，`cost_probe` 计数模块本身也只在测试构建中存在。
- **`stroke.rs`**：只改了 `shared_endpoint_sources` 的文档注释，代码没动。
- **`issue1588_parse_mem_probe.rs`**：这是诊断用的探针程序，不属于解析库。改动是设置环境变量 `ISSUE1588_SKIP_FACTS_DIGEST` 后，连 `facts_counts`（Θ(n²) 的计数）也一起跳过。解析调用 `parse_layer_with_inclusion_facts` 没变。
- **`cc244dc2d6`**：只新增了 `.chanlun/review-results/issue1629/` 下的报告和日志，没有代码。
- **结论**：作者说的"非测试构建的解析代码不变"属实。

### (2) 测试锁能不能锁住、有没有奖励黑客

- **遍历计数的覆盖面扩大了，锁变严**：以前 `iters == 0` 只能抓到 `SourceCoords::iter`。现在 `materialize` 和 `count` 也计数，所以 `coords_ref_union_cost_is_decoupled_from_size` 和 `shared_stroke_facts_cost_is_decoupled_from_prefix` 这两个测试里的 `iters == 0` 断言，会把 `to_vec`、`len`、`contains` 这类整棵展开也当作违规。断言窗口（从 `reset` 到 `read`）里没有夹带测试自己调用的 `count()`：`got.count()` 在 `read()` 之后才调用，不会误计数，也不会因为窗口放宽而漏掉违规。
- **并集成本锁同样变严**：
  - 原来只测相隔 20 次追加的近版本，现在加了相隔 n/2 次的远版本，取两者的最大步数去比较。
  - 判定阈值 `per_size[2] <= per_size[0] * 4` 原样保留，没有放宽。
  - 插入点从 `3n/2+1` 改成 `3n/4+1`。我逐个核对了 n=1000、8000、64000：插入值除以 3 都余 1，都不在树里，所以 `count == n+5` 的精确断言仍然成立，没有因为插入了已有元素而让断言自动通过。
  - 实测步数 `[148, 202, 256]`，规模放大 64 倍，步数约放大 1.7 倍。如果是旧的线性实现，步数会放大约 64 倍，肯定过不了这个锁。
- **端点构造的成本锁没改**：阈值还是 3.0 倍。收据里每端点步数从 151.8 到 216.8，约 1.43 倍。
- **"返回 `a` 的指针 ⟺ `b ⊆ a`"这条注释说法成立**：对照 `union_nodes` 代码（`evidence.rs:268-301`）核对过。唯一可能的反例是 `a` 是单个叶子、而 `b` 是 `Concat` 且 `b ⊆ a`，但 `Concat` 至少有 2 个元素，装不进只有 1 个元素的 `a`，所以不可能出现。注释里写的退化情况（两树内容重叠但不共享指针时变成 Θ(min)·polylog）也和代码吻合。

### 发现列表

- **HIGH**：无。
- **MEDIUM**：无。
- **LOW-1**（`rust/src/bin/issue1588_parse_mem_probe.rs:255-273`）：设了 `ISSUE1588_SKIP_FACTS_DIGEST` 之后，计数被跳过，但汇总行仍照常打印 `coords total cap_mb=0.0 (0.0% of retained)`。读日志的人可能把它误当成真实的零占用。建议跳过时打印 `skipped`，或者干脆不打这一行。只影响诊断输出。
- **LOW-2**（同一文件）：环境变量 `ISSUE1588_SKIP_FACTS_DIGEST` 原来只控制跳过事实摘要，现在还附带跳过计数，含义扩大了。代码注释写了这一点，但以前用这个变量跑出来的日志，和现在的读数在 `coords` 这几行上不能直接对比。不需要处理，知道即可。

VERDICT: APPROVE
