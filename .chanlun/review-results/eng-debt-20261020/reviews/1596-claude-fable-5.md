# PR #1596 独立评审（#1588 memory 分片）

- PR：xy7365527-lang/NewChanlun#1596「#1588 memory 分片：解析回放逐 bar 写时整表拷贝修复 + 事实坐标容量收缩」
- 分支 / 头提交：`eng-debt-1588/memory` @ `e52cbb0171`（base main）
- 评审人：claude-fable-5（非作者，只评审；未改代码、未推送、未评论 PR）
- 日期：2026-10-20

## 结论：PASS-WITH-NITS

两条修复（E-b：`classify_at` 在 append 前丢弃上一 bar 的 ParseLayer；E-a 可隔离部分：`stroke.rs::sorted` 去重后 `shrink_to_fit`）正确、不改解析结果、不降测试覆盖、无掩盖。我独立在 windows-5090 重跑了探针与新测试，并自建反证提交验证了新测试的区分力，数字与 PR 表格逐项一致。剩余 Θ(n²) 主体转票 #1597 符合 #1588 票面「修 / 判环境问题 / 转票」三选一的处置规则。两个 nit 均在诊断探针与证据记录层面，不阻塞合入。

## 轴①：正确性 / 风险

### 是否误改行为
- `incremental.rs:101` `self.last_l0 = None;` 置于 `append` 之前。`last_l0()` 的全部消费方（`runner.rs:355-358/574/739`、`theta_pi_diff.rs:291`、`econ_positive.rs:285`、`p938_seed_moved_composition.rs:178`、各 tests）都是同一 bar 内 `classify_at(i)` 之后立即读取，没有在 `classify_at` 之前读上一 bar layer 的用法。访问器语义不变。
- 逐位对拍：我在头提交重跑 `classifier 20000 50` 滚动摘要 `2a3cd44f…e2f1`、`last_l0` 摘要 `2b142ac1…`，与修前日志（`da78044035`）完全一致；`facts 5000` 的 layer/facts 摘要 `98fa48d6…1160` / `8d178804…bbb2` 与收缩容量前一致。
- `shrink_to_fit`：增量热路径 `endpoint_at(.., expand_sources=false)` 下 `sources` 为空 `vec![]`（`stroke.rs:223-229`），`shrink_to_fit` 对空 Vec 是空操作；作者在 `c262c34b73`（含 shrink）上重跑 `classifier 49995` 得 4,554.4 MB，与 shrink 前 4,554.0 MB 一致，确认不影响生产回放开销。仓库内无依赖 `Vec::capacity()` 的代码（grep 无命中）。

### 是否掩盖问题 / 降覆盖
- 未删改任何既有测试；新增回归测试 `classify_at_keeps_shared_parse_tables_in_place`（`incremental.rs:936-989`）。
- 反证（我自建）：撤回 `self.last_l0 = None;` 一行后，新测试在 `bar 1: merged_bars 被整表拷贝` 失败，探针 `classifier 5000` 累计分配 109.4 MB → 1,470.4 MB、`20000` 937.8 MB → 20,511.7 MB，摘要不变。断言用 Rc 地址判定可靠：`make_mut` 在旧块仍存活时分配新块，地址必不同，不存在地址复用造成的假阴性。
- 剩余持有点排查：`ParseLayerIncr::new` 共 33 处调用方，除 PR 已改的 5 处外，其余（`capture_oos.rs:248`、`profile.rs`、`p122/p123/p124/p409/p985/cp_capability_smoke/strict_nest_check` 等）均为循环体内 `let l0 = parser.append(bar)`，迭代末即释放，无跨 bar 持有。`OwnedIncrementalClassifier`（`streaming.rs:96-112`）不持有 ParseLayer，不受影响。
- PR 自列的 4 个关注点：
  1. `classify_at` 内部 / 并发读 `last_l0()`：无（上述）。
  2. `fill`/`runner` 的 `Rc::clone(&l0.strokes)`：`pi_theta_fill_loop_overlay` 内 `strokes_i`（`fill.rs:5163`）是单 bar 局部变量，迭代结束释放；`admission.rs:656-657` 克隆的是独立全量 `parse_layer` 的 Rc，不与增量状态共享。新测试逐 bar 断言 strokes 的 Rc 地址不变，也从经验上排除了 TowerCache 暗持有。
  3. `capacity` 依赖：无。
  4. Rc 地址判定：可靠（上述）。

### E-a 处置
- 20k 留存 9,530.9 MB → 6,518.1 MB，仍是二次增长（99.5% 为展开来源坐标容量）。主体转 #1597（OPEN，含测量数据与方案建议），理由（需改公开事实类型、消费方在 #1488/#1493/#1582 在途区域、调用方均为测试/oracle 不在生产热路径）成立。

## 轴②：#1588 票面验收

- 票面第 4 项「带完整事实的解析入口内存随前缀二次增长（尾窗 2 万根约 9.3 GB）」：本 PR 把问题拆成 E-b（生产回放路径 O(n²) 分配，**已修**，49,995 根 127.9 GB → 4.6 GB，耗时 6.2 s → 2.0 s）与 E-a（完整事实输出 schema Θ(n²)，**可隔离部分已修**，主体**转票 #1597**）。符合验收「逐条 现象、复现命令与输出、根因、处置（修/判为环境问题并给证据/转票）」。
- 证据落点：`.chanlun/review-results/eng-debt-20261020/memory/REPORT.md` 有复现命令、四份远端日志 ID、修前/修后/反证表格。主控 `LEDGER.md` 尚未在 main 上出现，需主控汇总时把本分片两条（E-b 修、E-a 转 #1597）以及下方「遗留观察」登记进去。
- 「每个修复非作者独审」：本文即是。「不删测试」：满足。

## 逐条发现

| # | 严重度 | 位置 | 发现 | 建议 |
|---|---|---|---|---|
| 1 | Nit | `rust/src/bin/issue1588_parse_mem_probe.rs:263` | `held = None;` 触发 `unused_assignments` warning（作者 `c262c34b73` 日志 `cargo check` 已报 `issue1588_parse_mem_probe.rs:263:13`，PR 描述未提）。语义正确（赋值即 drop 旧值），只是编译器把它判为死写。 | 改为 `drop(held.take());`（与 `s_structure_session.rs:3113` 写法一致），顺便消 warning。可合入后顺手修。 |
| 2 | Nit | `REPORT.md` 反证一行；远端日志 `20261020-151858-0265c224cd.log` | 作者的反证日志命令行因中文字符串转义出错（`bash: ... command not found`）而部分失败，且尾行 `EXIT=0` 与 `test result: FAILED` 不符（grep 管道吞了退出码）。日志正文仍支持结论，且反证提交未推送、无法直接复跑。我已用本地临时 ref 独立重做反证，结论成立。 | 证据记录里注明「反证提交未推送，复现方式=撤回 `incremental.rs:101` 一行后跑 `classify_at_keeps…` 与 `classifier 49995`」即可；日志的 EXIT 口径以后用 `set -o pipefail`。 |
| 3 | Info | `REPORT.md` 条目 1「修后残余」 | `replay-drop` 累计分配仍略超线性（20k 426.5 MB → 49,995 根 2,018.4 MB，约 ×4.7 对应 ×2.5 的 n），作者归因于段状态不可复用时重建 + 笔尾重扫，存活峰值线性。只记为「遗留观察」未转票。 | 不阻塞；建议主控在 LEDGER 的「观察项」登记，避免下次被当成新发现。 |
| 4 | Info | `rust/src/theta_v0/parser/mod.rs:319-323` | `append` 文档补了持有合同，但合同只靠文档约束，未来新调用方仍可能写成 `l = parser.append(b)`。 | 可选：后续在 `ParseLayerIncr` 上加一个 `append_into(&mut self, slot: &mut Option<ParseLayer>)` 之类先 take 再 append 的便捷入口。不属本 PR 范围。 |

## 我实际复核过的事实与命令

1. `gh pr view 1596` / `gh pr diff 1596` / `gh issue view 1588` / `gh issue view 1597`：PR 描述、diff（11 文件 +547/−0）、票面验收、转票存在且 OPEN。
2. 本地源码阅读：`incremental.rs:59-115`（结构与 `classify_at`）、`streaming.rs:36-112`（Owned 变体不持 layer）、`runner.rs:347-375`、`fill.rs:746-780, 5163`（`strokes_i` 单 bar 局部）、`admission.rs:640-670`（独立全量 parse）、`stroke.rs:140-147, 207-260, 440-460`（`sorted` 两处调用；非展开路径 `sources` 为空）、`s_structure_session.rs:3095-3118`；grep 全仓 `last_l0()`、`Rc::clone(&l0.`、`ParseLayerIncr::new`（33 处）、`.capacity()`（0 处非 bin）。
3. 作者引用的 4 份远端日志（WSL `~/nc-remote/logs/`）逐份拉回核对：`20261020-145238-da78044035`（修前）、`20261020-150150-7775135c0f`（修后 + 9/166 测试）、`20261020-151858-0265c224cd`（反证）、`20261020-152224-c262c34b73`（shrink 后 facts 曲线 + fmt/check/测试）。表格数字与 PR 一致。
4. 独立重跑（头提交 `e52cbb0171`，`nc-remote eng-debt-1588/memory rust …`，日志 `/tmp/nc-remote/20261020-173127-e52cbb0171.log`）：
   - `classifier 5000`：alloc_total 109.4 MB，`last_l0` 摘要 `9c26b466…`（= PR）
   - `classifier 20000 50`：alloc_total 937.8 MB，rolling `2a3cd44f…e2f1`（= 修前）
   - `replay-drop 20000`：426.5 MB；`replay-hold 5000`：1,386.0 MB（= PR）
   - `facts 5000`：retained 439.2 MB，coords 占 98.2%，layer/facts 摘要 `98fa48d6…` / `8d178804…`（= PR）
   - `cargo test --lib theta_v0::backtest::incremental::tests::classify_at_keeps`：1 passed
5. 独立反证（本地临时 ref `refs/review/1596-counter` = HEAD 撤回 `incremental.rs:101` 一行，`git commit-tree` 构造、未碰工作树、跑完即删；日志 `/tmp/nc-remote/20261020-173355-9755108096.log`）：
   - `classifier 5000`：1,470.4 MB；`classifier 20000 50`：20,511.7 MB，摘要与修后相同
   - 新测试 FAILED：`assertion left == right failed: bar 1: merged_bars 被整表拷贝`
6. 工作树在评审结束时 `git status` 干净，临时 ref 已删除，无推送。
