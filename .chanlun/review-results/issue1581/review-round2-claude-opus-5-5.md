# #1581 非作者独审第 2 轮（归档入仓）

- 原件：外盘 \`/Volumes/AgentStorage/issue1581-review/result-r2.md\`（负责人 /root/owner_1581 发起，claude-opus-5-5 经 CLIProxyLocal）；第 1 轮审 \`c9b8ee96c2\`，第 2 轮审 \`2163a39728\`
- 入仓：主控合入代理 /root/merge_train，2026-10-21，原文未改

---

第 2 轮结论：第 1 轮的两个 must_fix 都已解决，判 **PASS**。第 1 轮的 should_fix 里还剩 3 条没处理，降级保留在下面。我没有动仓库文件；对拍和格式检查是我自己跑的，变异判断是读代码推的，没有实际改代码跑一遍。

## 1. rebase 后实现有没有变

没变。`git range-diff` 里旧分支的 6 个提交和新分支一一对应，全部显示为 `=`（内容逐字相同）。

新增的两个提交只改了两个文件：
- `pipeline.rs` 加了 2 行文档：在 `classify_incremental` 的「## 边界」节写明带标记的层会被拒读，以及拒读后各项输出是什么。这一条就是第 1 轮 should_fix 要的那句话，内容和代码行为对得上。
- `tb1581_tests.rs` 加了新测试。

`mod.rs`、`tail.rs`、`profile.rs` 这两个提交里都没碰过。

## 2. 新测试 `tb1581_marker_reaches_incremental_and_batch_layers` 能不能锁住标记写入

**能锁住。** 逐个推演把写入点改回 `None` 的后果：

| 改动 | 结果 |
|---|---|
| 增量路径 `mod.rs:428` 改成 `None` | 测试遍历所有 K 线都找不到带标记的现场，最后 `hit.expect(...)` 报错，测试红 |
| 批量路径 `mod.rs:215` 改成 `None` | `assert_eq!(batch.segments_parse_invalid_from, Some(p))` 失败，测试红；`batch == layer` 也会失败 |
| facts 路径 `mod.rs:492` 改成 `None` | `facts_layer` 的标记断言失败，`facts ≡ 增量` 也失败，测试红 |
| 增量路径传给 `build_tail` 的标记改成 `None` | 尾部出现 `PendingSegment`，`has_pending_segment` 断言失败，测试红 |

**尾部正控成立。** 在同一现场、同一个测试开关下，用 `divide_segments_with_tail` 和 `build_tail(..., None)` 重新算一遍尾部，确实有 `PendingSegment`。这说明尾部过滤真的去掉了东西，不是空转。

**classifier 断言的取舍可以接受。** 命中现场是 seed=33、bar=451、`segments=0`。这时没有任何已确认段，classifier 本来就不会有产出，所以这里的「classifier 无产出」断言证明不了拒读，属于空转。这一点作者已在注释里写明。

classifier 入口的拒读，实际由 `tb1581_classifier_refuses_marked_layer_with_warm_cache` 锁住，那个测试有「不带标记时有产出」的正控。增量层到 classifier 之间只是读一个字段，所以「标记确实写进三路」加上「入口拒读」两个测试合起来就闭环了。

两个小毛病，不影响结论：
- 测试只能在流首段触发门失败（测试开关 TestOnly 只作用于首段），覆盖不到「已有确认段之后才失败」的场景。这一场景现在由热缓存测试和 ignored 的 seed 4 逐 bar 测试承担。
- 测试开关是线程局部变量，循环里如果中途报错不会被复位。但 cargo 每个测试单独一个线程，不会影响其他测试。

## 3. 正控对拍

逐位相同。我把 `base2-digest.log`（基线 eeda264996）和 `impl2-tests.log`（2163a39728）里的 7 行 `tb1581 digest`（seed 0–5 各一行，加 ALL=6f97754fd3d6199e）抽出来 diff，输出 `DIGEST_SAME`。`impl2-tests.log` 里 tb1581 一共 6 个测试，全部通过。

## 4. seed 4 反例回报 #1493

已闭环：
- d738acddf1 已在新的基点 e0b57c2d66 里。它新增 `tb1493_production_incremental_has_no_fail_closed_marker`：用逐字复制的 `tb1581` 合成 K 线，锁住生产增量路径全程没有标记，注释里点名了 seed=0x1581_0004、bar 1999–2035、`Some(100)`。
- `impl2-tests.log` 的 probe 行显示，seed 0–5 在新 HEAD 上 `marker_runs` 全部为空，第 1 轮的反例已不复现。

还有一个第 1 轮登记给 #1493、和本 PR 无关的事项：`IncrSegments::append` 三个提前返回的分支可能留下过期标记。本轮没有看到它已被处理的证据，需要 #1493 那边继续跟。

## 5. `cargo fmt --check`

我实跑确认：差异只在 `tb02c_tests.rs`（952、964 行）和 `tb1493_replay_tests.rs`（48、84、609 行），作者的文件格式是干净的。这归 #1493，不扣本 PR，但两边合并前 CI 的格式检查会是红的。

## 仍未处理的第 1 轮 should_fix

1. **绕过标记的下游还没登记。** `capture_oos.rs`、`wverify_run.rs` 里的 `cascade_dual`、诊断模块 `cp_recall_audit` 和 `cand_delta` 仍然直接读段账本，不看标记。`docs`、`analysis`、`.chanlun` 里没有搜到登记，tracker 也没有对应的跟进票。
2. **`tb1581_injected` 的 classifier 断言还没有正控。** 本轮新提交没碰这个测试。
3. 第 1 轮要求的「增量/批量/facts 标记相等」的证据，已由新测试里 `batch == layer`、`facts_layer == layer`（比较时包含标记字段）补上，这一条可以划掉。

```json
{"verdict":"PASS","must_fix":[],"should_fix":["另开跟进票或在 §15 登记仍直接读 l0.segments、绕过标记的下游（backtest/capture_oos.rs 最需要处理、wverify_run.rs 的 cascade_dual、diag/cp_recall_audit、diag/cand_delta），避免 classifier 拒读而 capture_oos 照常交易，构成同一判断宽严两档（#799）——本轮未见登记","tb1581_injected 里 classify(&bad) 的断言补正控（同一输入去掉标记时有非空产出）——本轮未改","新测试命中现场 segments=0，classifier 无产出的断言是空转，建议在断言旁注明「入口拒读由热缓存测试锁」或删去该断言，以免被误读为证据","（归 #1493）fmt 差异在 tb02c_tests.rs/tb1493_replay_tests.rs，合并前须由 #1493 修复，否则 CI 格式检查是红的；IncrSegments::append 提前返回分支可能留下过期标记一事仍待 #1493 处理"]}
```
