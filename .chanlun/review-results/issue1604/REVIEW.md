# #1604 非作者独审记录

审查者：Claude Code 子代理 code-reviewer。它与实施者同模型，但在独立上下文里运行，未参与实施；只读，未跑 cargo。按授权只运行了 `PYTHONPATH=src pytest tests/test_segment_policy_1604.py`，19 passed。
实施者：Claude Code（sandcastle 工位，分支 `sandcastle/issue-1604`）。
审查对象：`b388916287`（单提交），看的是与 `HEAD~1` 的全量 diff。

> 说明：同模型子代理不等于异质独审。合入门若要求跨模型或人工独审，仍需另行补做。

结论：**PASS-with-notes**（无阻断项）

## 1. 共同验收

- **只读版本化记录，输出写 id+版本：PASS**
  - 读取：`policy_records.rs:279-350` 只经 `include_str!` 编入的 JSON 取值；`config.rs:84-96` 的 `from_active_records` 是唯一入口，`:119-121` 的 `impl Default` 读坏即 panic。
  - 输出：`parser/segment.rs:394-413` 的 `SegmentPolicy::from_config` 把记录 label 与字段值写进 `SegmentFacts.policy`；`TailWindowBound` 同样带 `policy`。
  - 全仓 rg：`TAIL_WINDOW` 只剩旧线 `rust/src/segment.rs:426`（TB-10，范围外）。`Tangency::Closed/Open` 字面量只出现在 `config.rs` 的定义和测试里。
- **生产路径无常数：PASS**
  - `source_guard_no_constants_or_in_place_writes` 与 `source_guard_tangency_only_from_record` 两道守卫都在。
  - `ParseConfig {` 的构造点只有 `config.rs:87`。
- **缺值即停、不回退：PASS**
  - `read_str` 与 `read_u32` 同口径。
  - v1 预算记录显式判 `Invalid`，防止把缺字段静默当成 0。
- **负控：PASS**。Rust 负控覆盖缺记录、缺字段、null/AwaitingPolicy、非法 token、id 错、负数/裸数、v1；Python 侧 11 例。
- **变异体：PASS**。open 与 closed 必须分出差异（`flipped > 0`），另有签署值断言和源码字面量守卫，三道防线叠加。
- **等值对拍：PASS**。签署值等于改前常数；360 例语料上 tw=3/7 与 ∞ 的段、尾部和段事实（`checks` 诊断轨迹除外）逐位相同。

## 2. `probe_tail_excluded`

- **tw ≥ 3 时恒不置位：论证成立。**
  - 下标 i 在长度首次达到 i+3 时定型；这时 `tail_lo = i+3-tw ≤ i`，所以 i 一定落在主循环范围内，当场判定。
  - tw ≥ 3 时 `finalized_end == end`，不会走到每次重探 `n-2` 的分支。
- **本轮修掉的 n-2 漏标：修复必要且正确。**
  - 原先 tw=1 时，含末元素的候选既不在窗内，也不在探查范围内。
  - 修复后，`probed_upto` 只推进到定型边界。
- 阻塞期间 `probed_upto`/`last_checked` 不前进，解除阻塞后一次补齐，无遗漏。
- 命中即置粘性标记后早停，不会漏报。
- 增量和全量用的是同一个 `FeatureSeqState` 方法；tw=1 有逐例对拍。

## 3. tangency 跨实现

- `separates` 单独定义，保留改前的比较符号，所以旧线 f64 在 NaN 上与改前逐位一致。新线是整型，用 `!meets` 也没有风险。
- 以下几处在 closed 档下都逐位复现改前口径：旧线 `segment.rs` 两处、Python v1 两处、`segment_policy.py`。
- Python v0 从严格 `<` 改为 closed `<=`：
  - 这是 #246 早已裁定的口径，登记见 `.chanlun/definitions/xianduan.md` X-14/X-38 与 tangency 量化报告，本票只是把它落到代码。
  - `IMPLEMENTATION.md` §7 如实登记了这一点，不算遗漏。
- `theta_v0_lean_parity.rs` 改为读运行时记录：记录一旦改签 open，Lean 未参数化（#1626）的缝就会暴露出来，这是有意设计。

## 4. 其他

- 三个 `#[path]` bin 都补了 `theta_v0` 转接模块。
- `s_session_v2/tb02c.rs` 读 `facts.policy`，缺记录时返回 Err。`tail_cache_window:0`（第二序列）一行未受影响。

## 阻断项

无。

## 建议项（不阻塞）

1. `ThetaConfig::try_from_active_records` 的 `..ThetaConfig::default()` 会把记录多解析一次。与 #1603 同模式，不是 bug。
2. `docs/reference-theta-v0.md` 未同步两份新记录，建议跟进。
3. clippy 零新增告警未验证（远端未装 cargo-clippy），已如实登记。
4. 探查路径的 `second_sequence(..., 0)` 不复用增量缓存。尾窗极小、探查区间极大时会有重复计算；签署值 7 下不构成问题。
