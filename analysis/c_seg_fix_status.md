# C段边界修复状态（2026-06-11 验证）

## 结论

**修复已在 Rust 引擎落地，无需新改动。** 本次任务执行的是验证收尾：编译 + 单测全过。

任务描述的缺陷（趋势背驰C段被截断在 move 边界 `seg_end = comp_end`，导致 C段恒空、背驰永不触发）由两个已落地的修复共同消除：

| 修复 | 位置 | 内容 |
|------|------|------|
| 修复A（递归层 seg_end 对齐） | `rust/src/level.rs:217-275` | `moves_from_level_zhongshus` 的 seg_end 不再直接取 `last_zs.comp_end`：非末组 = 下一组首中枢 `comp_start - 1`，末组 = `num_components - 1`（含离开中枢的突破组件）。锚点 `last_seg_s1` 仍 = `comp_end`，不随 seg_end 扩展 |
| B2（C段越界极值定义） | `rust/src/divergence.rs:405-417` | 趋势 C 段终点不被 `mv.seg_end` 截断：搜索窗口 = `[c_start, 下一 settled 中枢 seg_end]`（无则 n-1），`c_end = trend_extreme_seg(...)` 取窗口内趋势极值段（走势转折点，第24课） |

增量器连带修正（`buysellpoint.rs:751-761`）：B2 后 `seg_c_end` 跨 move 不再全局单调，div frontier / stable_anchor 门控已相应调整。

## 验证结果

- `cargo build --release`：**通过**（需 `PATH="/usr/bin:$PATH"` 前置规避 `.local/bin/cc` 劫持，已知问题）
- `cargo test`：**74 passed / 0 failed / 1 ignored**
  - ignored 项为 `c_segment_verify::c_segment_oklo_447k_per_level_bsp`（OKLO 447K 全量逐 bar 长测试，设计为显式运行，非失败）
  - C段相关单测在跑且通过：`level::tests::fix_a_non_last_move_extends_to_next_group_start` 等

## 边界条件

- 本验证仅覆盖 Rust 单测层（合成数据，L1 等级）。真实数据效果（type1 1→374 / type2 0→360，L4 首次出现 8 个 type1）由前序 session 三标的全量重跑确认（commit 36c6a033）。
- 若 B2 窗口上限定义（下一 settled 中枢 seg_end）被修改，closed move 永久性保证（`divergence.rs:569`）随之失效。
- 长测试 `c_segment_oklo_447k_per_level_bsp` 已于 2026-06-11 显式运行通过（`cargo test --release -- --ignored`）：
  L2 type1/type2 = 374/360，L1' 60/58，L4 8/8，L5 1/1，与 commit 36c6a033 报告逐位一致。
  产物落盘 `analysis/data_cache/c_segment_fix_rust_verify.json`。
- 仍未完成：`test_bitexact_large_real_streaming`（BZ 真实全量 slow，jetsam×2），替代覆盖已在
  `engine_c_segment_fix.md` 声明。

## 影响声明

- 本次未改动任何源文件。仅执行编译与单测验证，并落盘本状态文件。
- 谱系引用：`engine_bsp_gap_diagnosis`（BSP缺口根因诊断）→ 修复A + B2；相关 commit：0359f0dc（Python oracle 同步）、36c6a033（B2 落地 + 三标的重跑）。
