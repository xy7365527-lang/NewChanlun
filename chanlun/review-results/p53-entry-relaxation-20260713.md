# P53：`cand_delta` 几何入口放宽实装（P0 全纳裁决）

- 日期：2026-07-13
- 分支：`p0-replay-dparent`
- 基准 HEAD：`3d162754bf`
- 数据：BTC 1m 全量 4,613,599 bar
- 配置：`ThetaConfig::default()`
- 直接权威：P0 #53“入口放宽，P52 的 202 例全部纳入事件集”
- 上游材料：`p52-recall-upper-bound-audit-20260712.md`、
  `p51-pending20-triage-20260712.md`、701 号谱系

## 0. 结论

【已验证】P53 已把生产生命周期中经现行 `judge_third_cert` 闭合的稳定 `B_p/c_p` 对象全部事件化为
`CandDeltaEntryEvent`。放宽后的几何入口集合 `E_new` 与 P52 独立直扫上界 `U` 逐对象相等：

```text
stable_objects=2630
upper_success=213
event_objects=213
aligned_success=213
missed_candidates=0
event_only_failures=0
```

【已验证】来源分桶严格复现 P52 的对象差集：

```text
旧 cand_delta=true 且几何成功：11
旧 cand_delta=false 且几何成功：9
旧事件中无同 CpId、稳定对象直接事件化：193
合计：213
```

【已验证】旧 `CandDeltaEvent.cand_delta` 仍表示算法背驰历史真值，不被改写。原 31 个
`cand_delta=true` 事件记录保持存在：其中 11 个桥接到 `E_new`；另外 20 个因终点几何失败不进入
几何入口，但仍逐例保留 `ConsolidationReviewPending`，终点分桶继续是 `DIR=11 / REENTER=9`。

★ Insight ─────────────────────────────────────

- “放宽事件入口”不等于把背驰 D 真值改义成第三类几何。独立事件类型使两类证据可同时保真。
- 193 个无旧事件对象没有可诚实填写的 `a_interval/divergence_confirm_src`；从 Closed 对象事件化可
  避免伪造背驰字段。
- 9 个旧 false 对象实际只在 `c_structure` 携 CpId；来源匹配必须继承 P52 的
  `cp_ownership` 优先、`c_structure` 回退规则。

─────────────────────────────────────────────────

## 1. 改动 diff 摘要

### 1.1 生产入口事件

【已验证】`rust/src/theta_v0/classifier/recursive_tower.rs` 新增（行号锚点为本提交前工作树）：

- `CandDeltaEntryOrigin`（`recursive_tower.rs:1201`）
  - `ExistingCandDeltaTrue`
  - `ExistingCandDeltaFalse`
  - `StableCpGeometry`
- `CandDeltaEntryEvent`（`recursive_tower.rs:1222`）：保存 level、side、稳定归属边、完整 `c_p` 结构、third 证书、首次确认时点、
  第 20/22 行及完整趋势资格证据、来源分桶；
- `relaxed_cand_delta_entries`（`recursive_tower.rs:1239`）：只遍历 `CpLifecycleStatus::Closed` 对象并事件化。

Closed 对象若缺 `departure_move_id / departure_interval / c_structure / third_class_in_c /
cp_certificate_confirm_src`，或证书身份不一致，立即硬失败；不允许静默过滤。
【已验证】生产函数只按 `CpLifecycleStatus::Closed` 与对象证书遍历，不含 202 例 ID 白名单；
`L2#31/L1#140`、`L2#764/L1#3401` 只出现在 `#[cfg(test)]` 回归夹具中。

【已验证】`rust/src/theta_v0/classifier/mod.rs:561` 新增 `cand_delta_entry_tower`，逐级构造放宽后的入口事件集。
它读取生产 `Classification.levels[*].cp_ownership`，不是 P52 诊断直扫的第二套真值。

### 1.2 P52 独立验收

【已验证】`rust/src/bin/cp_capability_smoke.rs:388-451` 同时计算并硬校验：

1. P52 `cp_recall_upper_bound_audit` 的独立 `success_keys`；
2. P53 `cand_delta_entry_tower` 的 `event_keys`。

两集合不相等时逐例输出：

- `P53_EXCEPTION kind=MISSING_RELAXED_ENTRY`（`cp_capability_smoke.rs:438`）；
- `P53_EXCEPTION kind=ENTRY_WITHOUT_GEOMETRY`（`cp_capability_smoke.rs:444`）；

随后非零退出。正常路径输出 `P52_RECALL_* / P52_CASE`；增量过程计数改用
`P53_FRONTIER_*` 前缀，使 batch 与逐 bar 增量的全部 `P52_` 行可以原样零差异比较。

### 1.3 P52 基线继承

【已验证】本次变更集合同时承接 P52 尚未提交的只读审计、frontier 计数器与 3 条 P52 回归测试；没有改写 P52
报告正文，只在报告末尾追加 P53 交叉引用。

## 2. `E` 放宽前后对比

| 口径 | 总计 | L1 | L2 | L3 | L4 |
|---|---:|---:|---:|---:|---:|
| `E_old`：旧 `cand_delta=true` | 31 | 7 | 13 | 9 | 2 |
| `U`：P52 独立几何成功 | 213 | 136 | 53 | 17 | 7 |
| `E_new`：`CandDeltaEntryEvent` | 213 | 136 | 53 | 17 | 7 |
| `U \ E_new` | 0 | 0 | 0 | 0 | 0 |
| `E_new \ U` | 0 | 0 | 0 | 0 | 0 |

【已验证】来源分桶：

| 来源 | 总计 | L1 | L2 | L3 | L4 |
|---|---:|---:|---:|---:|---:|
| `ExistingCandDeltaTrue` | 11 | 1 | 6 | 3 | 1 |
| `ExistingCandDeltaFalse` | 9 | 4 | 2 | 2 | 1 |
| `StableCpGeometry` | 193 | 131 | 45 | 12 | 5 |

### 2.1 原 193 例路径

【已验证】这些对象没有同 CpId 的旧事件，不能伪造算法背驰字段。P53 直接读取生产生命周期 Closed 对象的
稳定边与 third 证书，产 `StableCpGeometry` 入口事件。回归测试固定 P52 第 1 例
`L1 / B=L2#31 / cp_departure=L1#140 / retest=L1#141`。

### 2.2 原 9 例 `CAND_DELTA_FALSE` 路径

【已验证】9 例旧事件的 `cand_delta=false` 保持不变；P53 用同 CpId 的 Closed third 证书赋予独立几何入口
资格。真实数据上这 9 例没有 `cp_ownership`，但 `c_structure` 携带完整
`(level, B_p, departure, source_start)`，故匹配规则是：

```text
event.cp_ownership
  OR ELSE event.c_structure
```

回归测试固定 P52 的 `L1 / B=L2#764 / cp_departure=L1#3401 / retest=L1#3402`，并显式构造
`cp_ownership=None + c_structure=Some + cand_delta=false`。

## 3. P51 20 例在放宽后的状态

【已验证】P51 的 20 例是 `E_old \ U`，不是 P52 的 202 个 `U \ E_old`。P53 没有把集合方向混同：

| 终点原子 | 数量 | P53 后几何入口 | 分类状态 |
|---|---:|---|---|
| `DIRECTION_PAIR_MISMATCH` | 11 | 不进入 | `ConsolidationReviewPending` |
| `RETEST_REENTERS_B` | 9 | 不进入 | `ConsolidationReviewPending` |
| 合计 | 20 | 0 | 20 例逐例保留 |

状态变化只发生在集合计数口径：它们属于旧算法背驰事件集，但不属于新几何入口集。以下语义不变：

- 历史 `CandDeltaEvent` 不删除、不改字段；
- `ThirdCertificateLifecycle=Pending`；
- `FullTrendQualification=NotQualifiedAsOf(T_end)`；
- `ClassificationReview=ConsolidationReviewPending`；
- `Censoring=RightCensoredAt(T_end)`；
- 不自动证明盘整背驰，不回填历史时点。

## 4. 回归测试

【已验证】新增 3 条（`rust/src/theta_v0/classifier/recursive_tower.rs:3146`、`:3175`、`:3229`）：

1. `p53_relaxes_original_no_event_l2_31_l1_140`；
2. `p53_relaxes_original_cand_delta_false_l2_764_l1_3401`；
3. `p53_preserves_existing_cand_delta_event_records`。

第三条同时覆盖：已有几何成功事件继续进入、已有几何失败事件继续留在 review 侧、调用前后旧事件
vector bit-exact 相等。

## 5. 验收命令与输出

### 5.1 定向与编译

```bash
cd rust
cargo test p53_ --lib
cargo check --all-targets
cargo build --release --bin cp_capability_smoke
```

【已验证】2026-07-13 本 worktree fresh 重跑：全部 exit 0；定向测试 `3 passed / 0 failed`。

### 5.2 cargo test 全量

```bash
cd rust
cargo test --quiet > /tmp/p53-cargo-test-20260713.log 2>&1
awk '/test result:/{passed+=$4; failed+=$6; ignored+=$8} \
     END{printf "passed=%d failed=%d ignored=%d\n",passed,failed,ignored}' \
    /tmp/p53-cargo-test-20260713.log
```

输出：

```text
passed=1620 failed=0 ignored=133
```

【已验证】P52 基线为 `1617/0/133`；P53 fresh 全量测试为 `1620/0/133`，净增 3 passed，
无回退。主库首行是
`1566 passed / 0 failed / 127 ignored`。

### 5.3 P52 batch 全量

```bash
CP_SMOKE_DATA_ROOT=/Users/silencehan/Projects/NewChanlun \
CP_SMOKE_MAX_BARS=4613599 \
CP_SMOKE_MODE=batch \
CP_SMOKE_VERBOSE=0 \
CP_SMOKE_P50_CERTS=1 \
CP_SMOKE_P52_AUDIT=1 \
target/release/cp_capability_smoke > /tmp/p53-recall-batch-20260713.log
```

关键输出：

```text
P52_RECALL_SUMMARY levels=1-4 stable_objects=2630 upper_success=213 event_objects=213 aligned_success=213 missed_candidates=0 missed_cand_delta_false=0 missed_before_event=0 event_only_failures=0
P53_ENTRY_SUMMARY old_e=31 new_e=213 existing_cand_delta_true=11 existing_cand_delta_false=9 stable_cp_geometry=193 legacy_records=55 legacy_review=20
```

【已验证】2026-07-13 fresh batch：exit 0，`real=3.07s`；`P53_EXCEPTION` 为 0 行。

### 5.4 P52 逐 bar 增量全量

```bash
CP_SMOKE_DATA_ROOT=/Users/silencehan/Projects/NewChanlun \
CP_SMOKE_MAX_BARS=4613599 \
CP_SMOKE_VERBOSE=0 \
CP_SMOKE_P50_CERTS=1 \
CP_SMOKE_P52_AUDIT=1 \
target/release/cp_capability_smoke > /tmp/p53-recall-incremental.log
```

【已验证：本任务留存全量运行证据】该逐 bar 进程从 `2026-07-12T23:46:08Z` 运行到
2026-07-13，源码修改时间早于其 release 构建与运行；exit 0。关键输出：

```text
P52_RECALL_SUMMARY levels=1-4 stable_objects=2630 upper_success=213 event_objects=213 aligned_success=213 missed_candidates=0 missed_cand_delta_false=0 missed_before_event=0 event_only_failures=0
P53_ENTRY_SUMMARY old_e=31 new_e=213 existing_cand_delta_true=11 existing_cand_delta_false=9 stable_cp_geometry=193 legacy_records=55 legacy_review=20
P53_FRONTIER_TOTAL pending_fallbacks=845771 tail_reinherits=27043570 certificate_clear_recomputes=1100854
real 37992.73
user 15983.55
sys 21450.82
```

【已验证】`P53_EXCEPTION` 为 0 行；20 个 `P53_LEGACY_REVIEW_CASE` 仍严格分为
`DIRECTION_PAIR_MISMATCH=11 / RETEST_REENTERS_B=9`。

### 5.5 batch / 增量 P52 零差异

```bash
rg '^P52_' /tmp/p53-recall-batch-20260713.log > /tmp/p53-batch-p52-20260713.txt
rg '^P52_' /tmp/p53-recall-incremental.log > /tmp/p53-incremental-p52-20260713.txt
diff -u /tmp/p53-batch-p52-20260713.txt /tmp/p53-incremental-p52-20260713.txt
```

【已验证】fresh batch 与上述全量增量留存日志两侧均为 218 行，`diff -u` exit 0、stdout 为空，
即全部 `P52_` 行零差异；两份抽取文件 SHA-256 均为
`6c5599dc161a3a3a72dd79bc800d8391fa8e1a6e7b9a820f0412c49e0bc936e4`。

```text
218 /tmp/p53-batch-p52-20260713.txt
218 /tmp/p53-incremental-p52-20260713.txt
diff_exit=0
```

### 5.6 例外归因

【已验证】batch 与逐 bar 增量两路径的 `P53_EXCEPTION` 均为 0 行，因此没有未归因例外。
原 `E_old \\ U` 的 20 个对象不属于入口漏纳例外：两路径都逐例输出
`P53_LEGACY_REVIEW_CASE`，严格归因为 11 个 `DIRECTION_PAIR_MISMATCH` 与 9 个
`RETEST_REENTERS_B`，状态均为 `ConsolidationReviewPending`。没有静默删除或改写旧事件。

### 5.7 工作树验收

最终提交前执行：

```bash
git diff --check
git status --short
git status --short --branch
```

【已验证】提交前 `git diff --check` exit 0。精确 `git add` 仅包含本任务 6 个文件，但当前 Codex
sandbox 将本 worktree 的 `.git` 挂载为只读，创建 `.git/index.lock` 时返回
`Operation not permitted`（exit 128）；因此未执行无意义的 `git commit`，不以未提交状态冒充 clean。

```text
git_diff_check=0
git_add_exit=128
git_add=blocked_by_read_only_git_dir
git_commit=not_run_no_index
post_commit_status=blocked_by_environment
```

## 6. 遗留风险与有效域

1. 【已验证】`CandDeltaEntryEvent` 是独立几何入口事件；现有 strict-chain 消费者仍读取算法背驰
   `CandDeltaEvent`。【推断】本 P0 裁决没有授权把 213 例直接接入 strict-chain、
   交易、订单或历史入场时点；若要改变消费集合，必须另案明确因果时点与 nesting 区间字段，
   不能给 193 例伪造背驰区间。
2. 【已验证】`E_new=U` 依赖生产生命周期 Closed 与 P52 独立直扫当前逐对象相等；smoke 已把不等
   转为硬失败。【推断】未来修改生命周期 writer 或 `judge_third_cert` 时仍须保留该双路验收。
3. 【已验证】9 个 false 路径依赖 `c_structure` 回退仅用于同 CpId 来源识别，不把 structure 反写成历史
   `cp_ownership`，也不改变 `cand_delta=false`。
4. 【已验证】P51 20 例的独立盘整背驰证书、第 20/22 行完整资格与完成分解能力缺口仍未解决；P53 只守住其
   分类语义。
5. 【已验证】frontier 计数是过程诊断，不属于 batch/增量最终 `P52_` 等值集合；P53 使用独立
   `P53_FRONTIER_*` 前缀保留过程差异。

## 7. 谱系

【已验证】701 号已更新为 **已实装待复核**，记录 P0 对 P52 历史禁区的后续授权、`11/9/193`
分路、20 例保护线和全量复核门。【推断】迁入 settled 需要独立复核，本任务不自行迁移。
