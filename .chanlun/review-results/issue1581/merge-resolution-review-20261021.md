# #1581 PR #1584 合并提交冲突解法评审

- 评审者：claude-opus-5-5（本机 claude CLI，经 CLIProxyLocal；独立会话、非作者；只读工具）
- 发起：主控合入代理 /root/merge_train，2026-10-21
- 被审：合并提交 `c8fb5db957`（origin/main → codex/1581-downstream-parse-invalid），冲突仅 `rust/src/theta_v0/classifier/pipeline.rs` 拒读早返回补 #1603 `termination`/`policy` 字段
- 结论：APPROVE（附非阻断跟进建议：拒读路径另立 `TowerTermination::ParseInvalid`）

---

## 评审结论

### (1) 拒读路径标 `Natural` 是否合适

**结论：这次合并里这样写是合格的，不用改。但这里留着一处语义上的隐患，建议另开跟进。**

**支持这样写的理由：**

- **和既有空输入路径完全一致。** 空 ParseLayer 的早返回在 `pipeline.rs:697-712`，做法是 `cache.clear()`，返回空的 Classification、tower 和 operations，并标 `TowerTermination::Natural`。#1581 的拒读路径输出同样的空产物，再标 `Natural`，与它逐字段同口径。`TowerTermination` 的文档（`pipeline.rs:408`）也把「空 ParseLayer」明确列在自然终止名下。
- **`ResourceBound` 用在这里是错的。** 它要带 `l_max`、`pending_units` 和 `budget`，表示「还能往上建，但预算用完了」。拒读和预算无关，`pending_units` 根本填不出来，硬标反而会误导。
- **#1581 要的 fail-closed 已经由产物本身做到。** 不出中枢、走势、买卖点和 operations，候选事件簿只读不写。termination 只是附带的元信息，不承担阻断。
- **`policy: TowerPolicyStamp::of(config)` 写得对。** 它满足 #1603 共同验收「每份输出写出所用记录 id 与版本号」，而且只读 config，与是否拒读无关。

**隐患在哪里：**

#1603 把 `Natural` 和 `INCOMPLETE_RESOURCE_BOUND` 当成「覆盖完整」与「覆盖不完整」的二分来用。拒读的真实含义却是「这一层输入无效，什么都没读」，不属于其中任何一种。

如果将来某个下游（例如 #1603 票面写到的「截断标记会进入 S 会话输出」）只看 `termination.code()`，它在拒读的 bar 上会读到 `NATURAL`。这和「输入为空、塔自然到顶」分不开，等于把拒读当成了完整覆盖。

**这次为什么不阻断：** 我在 `rust/src/` 下 grep 过，生产代码里读 `ClassifyOutput.termination` 的地方，除了 `pipeline.rs` 自己的 `is_resource_truncated()`，一个都没有，其余都是测试。所以现在不会有人读错，风险只是潜在的。

**建议的最小改法（另开跟进，或在 #1603 的输出接入 S 会话之前做完）：**

1. 给枚举加一个变体 `TowerTermination::ParseInvalid { from: usize }`，`code()` 返回 `"PARSE_INVALID"`。拒读早返回改成 `termination: TowerTermination::ParseInvalid { from: l0.segments_parse_invalid_from.unwrap() }`（拿不到值就用 `if let`）。`is_resource_truncated()` 不受影响。
2. 在 `TowerTermination` 文档里写明：拒读既不算覆盖完整，也不算资源截断。
3. 在 `tb1581_classifier_refuses_marked_layer_with_warm_cache` 里补一句断言，锁住 termination 的取值。现在这条测试没有断言 termination 和 policy，把拒读路径的 termination 改成别的值，测试也不会变红。

如果不想动枚举，至少在 `TowerTermination` 文档里注明「拒读路径也记 `Natural`，要区分拒读请读 `l0.segments_parse_invalid_from`」，再补上第 3 条的断言。

### (2) 合并有没有丢掉任何一方的改动

**两边都没丢。**

- **与 main 一侧（`c8fb5db957^2`）对比：** 差异只有 #1581 的 5 个文件（`pipeline.rs` +19，`parser/mod.rs`、`profile.rs`、`tail.rs`、`tb1581_tests.rs`）。
- **#1581 的改动有没有原样带进来：** 把「merge-base → #1581 分支」的 diff 和「main → 合并结果」的 diff 逐行比对：
  - parser 下 4 个文件只有 hunk 行号偏移，内容一致。
  - `pipeline.rs` 只多出冲突解法的 3 行（`termination`、`policy` 和那行注释），外加解构从三元组改成四元组。
- **#1603 等 main 上的改动有没有被吞：** 用 numstat 比对「merge-base → main」和「#1581 分支 → 合并结果」，199 个文件里只有 `pipeline.rs` 一处不同（96 行变成 99 行，正好是上面那 3 行）。也就是说，main 上的改动全部保留，也没有带进多余内容。
- **主路径：** 改成 `let (classification, tower, operations, termination) = classify_incremental_inner(...)`，和 main 上 #1603 的写法一致。

### 验证

- `cargo test --lib tb1581`：3 个通过，3 个按设计 `#[ignore]` 没跑。
- `cargo test --lib tower_policy`：4 个通过。
- 工作区没有改动，只有原来就在的 `?? .rev-issue1581.md`。

VERDICT: APPROVE
