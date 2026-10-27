# #1608 w3 报告（s_session_v2 / s_structure_session / s_content/** / session_*）

分支 `issue1608/w3`，基于 `ed337f26f3`，NC_TARGET=issue1608-w3。没有 push，没有开 PR。

## 提交

- `4b87d3874d` fix(#1608): s_session_v2/s_content/session_* 编译告警第一轮（cfg 门控、删残留薄包装、窄 allow）
  - 改动 15 个文件，+100/−154 行

## 验收

计数方法：`--message-format=short`，取 `: warning` 行，按（配置, 行）去重。

- 基线日志（ed337f26f3）：`/tmp/nc-remote/20261021-091657-ed337f26f3.log`
- 验收日志（4b87d3874d）：`/tmp/nc-remote/20261021-093556-4b87d3874d.log`
  - EXIT=0，五档 RC 均为 0，没有 error

| 配置 | 基线总数 | 现总数 | 基线本人文件 | 现本人文件 |
|---|---|---|---|---|
| C1 `--all-targets` | 164 | 164 | 0 | **0** |
| C2 `+backtest_bin` | 261 | 261 | 0 | **0** |
| C3 `s_session` | 451 | 382 | 75（三档去重合计） | **0** |
| C4 `s_session,json,native` | 414 | 350 | ↑ | **0** |
| C5 `s_session,sqlite` | 451 | 382 | ↑ | **0** |

- 本人文件的匹配模式：`s_session_v2/|s_structure_session\.rs|src/s_content/|src/session_[^/]*\.rs`
- 新增告警：用 `comm` 对比基线集合与现集合，没有任何新告警（所有配置、所有文件）。

## 删除清单

所删的都是两类：一是 #1394 准入迁移后只剩 `X_admitted(.., None).expect` 的薄包装，二是已被 `*_view` 版本取代、调用方早已移除的适配器。每一项都 grep 过，确认全仓没有调用方。

- **s_content/path.rs**（7 个）：check_range_root_ctx、object_key_at_ctx、bytes_between_ctx、navigate、check_range_root、object_key_at、bytes_between
  - 文档注释已移到对应的 `*_view_ctx` / `navigate_view` 上
  - 另删未用 import `CONTENT_FORMAT`
- **s_content/fragment.rs**（2 个）：public_header（已被 public_header_view 取代）、自由函数 hex（已被 FragmentText::hex 取代）
- **s_session_v2/cache.rs**（6 个）：make_room、put_root_cert、derived_get、derived_put、mark_reachable、certify_retained_originals
- **s_session_v2/evidence_store.rs**（2 个）：work_resize、append_checked
- **s_session_v2/mod.rs**（7 个）：control_normalized、ControlSchema::add_ddl、sorted_names、verify_control_schema、control_canonical_text、control_identity、control_sorted_keys
- **s_session_v2/tb02a.rs**（2 个）：validate_prices、stored_prices（已被 `_view` 版本取代）
- **s_structure_session.rs**（1 个）：parse_batch_value

合计 27 项。

## 新增 allow：12 处

全部是条目级或字段级的 `allow(dead_code, reason = "#1608：…")`。没有新增 crate 级或文件级 allow，也没有用 `let _ =`。

- **只在特定 feature 下接线**（`cfg_attr(not(feature), allow)`），3 处：
  - audit_json.rs 中的 `enum Node` 和首个 `impl OwnedJson`（只在 s_allocation_json_testonly 档使用）
  - content_store.rs 中的 `OriginalScanScratch::capacity_nodes`（`cfg_attr(not(test))`，只有测试经 v3 request_audit 读取）
- **测量或诊断读数**，4 处：
  - evidence_store.rs：Staged.staged_nodes、Staged.staged_bytes 两个字段，以及 cfg(test) 的 describe
  - s_fact_cache.rs：cfg(test) 的 last_outcome
- **待接线 API 或原型入口**，4 处：
  - evidence_store.rs：Stager::param_bytes、read_fixed_cut_value
  - work.rs：WorkSet::new
  - capture_allocation.rs：CaptureSlab::copy_body（#1394 原型）
- **测试载荷字段**，1 处：
  - work.rs 测试里 `Panics` 元组的 u8 字段（它是让元素真正占用 Layout 字节的非零尺寸载荷）

## 其余处理方式

- **`#[cfg(test)]`**：
  - session_transport::io_connection_with_deadline
  - LedgerAllocator::new、CodecVec::with_capacity
  - cache.rs 中 9 个只供测试的方法
  - evidence_store.rs 中的 split_seq、split_seq_work、Reader::touched
  - ControlHistory 的 3 个读数
  - CaptureCancellation::never 和 at_check
- **`#[cfg(feature = "s_capture_native_testonly")]`**：
  - WorkMap::try_reserve
  - read_audit_message、validate_audit_message
  - tb02a::enrich_raw_row
- **`#[cfg(all(test, feature = "s_capture_native_testonly"))]`**：
  - WorkSet::contains
  - ControlFacts::len
  - current_originals_equal_for_request
  - prototype_capture_work_on_thread
- **`#[cfg(feature = "s_allocation_json_testonly")]`**：
  - JsonBytes trait 及其 impl
  - quoted
  - CaptureCancellation::until
  - mod.rs 中的 `audit_json as control_aj` import
- **import、mut、命名、绑定**：
  - content_manifest.rs 删去未用的 NumberToken 和 `Value as V`，测试里删去多余的 `mut`
  - staged_nodes.rs 中的测试函数名改成 snake_case：`staged_nodes_preserve_frame_and_current_sql_error_gates`
  - mod.rs 中未使用的元组绑定 `(expected_key, count, published)` 改为只保留 `messages.get(..).ok_or_else(..)?` 的存在性检查，行为不变
- **按要求未动的部分**：
  - fetch_update→try_update、测试钩子的 expect、check-cfg
  - 两个 #1394 编译负例块
  - 他人负责的文件，以及 Cargo.toml 和 CI

## 范围外的残留告警（只列出，未处理）

- **`src/s_content.rs`**（父模块文件本身，不在 `s_content/**` 范围内），C3–C5 各有 4 条，基线就存在：
  - 835 行 `parse_canonical_u64` 未使用
  - 1906 行方法 `next` 和 `next_body` 未使用
  - 4405 行 `with_memo` 未使用
  - 5733 行多余的 `mut`
  - **需要主控确认归属**：如果它算我负责的范围，可以按同样的规则补一轮。
- **`src/bin/s_session_v3/**`**：在 C3 下有 235 条（去重前）。较多的文件：
  - history.rs 37 条
  - object_index.rs 20 条
  - business_json.rs 16 条
  - source_audit.rs 14 条
  - capture.rs 14 条
  - request_audit.rs 10 条
  - aux_index.rs 9 条
  - state.rs 8 条
  - control_audit.rs 8 条
  - reader.rs 6 条
- **其他范围外告警**：
  - `src/bin/../*.rs`：被 bin 以 `#[path]` 共享的 bi_engine、buysellpoint、divergence、macd、moves、zhongshu 等
  - `src/theta_v0/**`：backtest 130 条，strategy、classifier、parser
  - 各个 p* 探针 bin、strict_nest_check、recursive_t、economic_session、fugue_v3、trading、orchestrator 等

## 疑点

1. **mod.rs 的 validate_audit_phase 一带**：原 `let (expected_key, count, published) = …?` 解构出的三个值都没有读取方。git 显示这些校验已在 19cae1dc09 移进 validate_audit_phase。所以我判断这不是漏检，而是迁移后的残留。现在保留了 `?` 的存在性检查，行为不变。建议审阅者顺便确认 validate_audit_phase 确实覆盖了 key、count、published 三项校验。
2. **门控只在 v3 测试中使用的条目**：一部分 v2 条目只在 v3 的 native 测试（state.rs、audit_owner/q.rs）里用到，所以按 `all(test, native)` 或 `native` 做了门控。如果 v3 的负责人日后放宽这些调用点的 cfg，需要同步放宽这里。
3. **诊断告警被去重掩盖**：Cargo 会跨 target 去重诊断（"N duplicates"）。所以某个 test target 下不报告警，并不能证明条目在那里被用到；门控的判断都以 grep 调用方为准。
4. **s_content.rs 的归属**：见上一节。
