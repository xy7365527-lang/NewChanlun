# #648 T2/T3 留痕：管线抽离 pipeline.rs + LOW-12 清零（classifier 格局收口完成）

- 日期：2026-08-16
- 票据：#648 T2/T3（#786 Q3 改写稿；T1 留痕见 `issue648-t1-test-extraction-20260816.md`）
- commit：T2 = `1d48829129`；T3 = `61d7baaba8`（main，镜像同步）

## T2 移动清单（纯移动，零行为变更）

| 落点 | 内容 | 行数 |
|---|---|---|
| `pipeline.rs`（新） | 23 个管线 fn（classify 族/segment_to_unit/unit_to_segment/detect_centers_*/extract_* 等）+ `LevelState`/`Classification` 两 struct 及 impl + 2 个 cfg(test) 探针静态 | 2026 |
| `mod.rs`（残余） | 文档头 + 36 个 mod 声明 + pub use 再导出 + 子模块共享 prelude + `mod tests;` | **164** |

名分对账（改写稿义务）：pipeline.rs 文件头明载「纯移动抽取，非 C6（#745）所删旧孤儿复活」（旧孤儿 = kimi 线 416 行死壳，内容不同源）。

## 机械适配留痕（无语义改动）

- pipeline.rs 自带 use 块（super 链 +1；14 个根模块名 `use super::{…}` 再绑定；`super::env_registry` → `super::super::env_registry` ×3）；
- **mod.rs 私有 use 块实证为子模块 prelude 中枢**（约 30 个子文件 `use super::*` 消费）⟹ 保留并转 `pub(crate)`（crate 外不可见，公共接口面不变），按 **test 构建** lint 精修死绑定（删 ParseLayer/Cow/RefCell/Direction/Stroke 等 9 名；教训：`cargo check --lib` 不含 cfg(test)，test-only 消费会被误判 unused，裁撤必须以 test 构建为准）；既存死 import `cache_series_ok` 一并为测试消费正名；
- `classify_level`/`extract_first_third_for_level` 升 `pub(crate)`（tests 子树消费）；
- bsp 白名单两访问器点随迁重锚（mod.rs:224/256 → pipeline.rs:103/135，dated 注记）。

## T3 退役清单（LOW-12 残余，#733 移交）

`classifier/cp_replay_diagnostics.rs`、`classifier/oracle_probe.rs`（内容已迁 diag/ 的根级死壳）+ `wverify_run/{report,m8,issue71_chi_gamma}.rs`（未声明死文件；LOW-9 登记的 ShortDiff 旧名残留随件同清）。五件均无 mod 声明，不编译、零行为面。

## 前后对照

- 测试计数：T2 后 / T3 后全量 lib 均 **2685 passed / 0 failed / 151 ignored**，与 T1 后基线逐位一致；
- `check --all-targets` 0 错（探针 bin 含）；fmt 0 Diff；test 构建警告 62 == 基线（零新增，反清 1 个既存死 import）；
- **终态**：mod.rs 6421 → 164 行薄壳 = #643(b) kimi 拆分格局的正房形状（勘察报告 §结论的 T1+T2 路径全程走完）。

## 验收逐条（改写稿 Acceptance）

1. 公共接口 parity：`pub use pipeline::*` + prelude pub(crate) 化，`classifier::Classification` 等路径全仓零变化（全仓编译不动一处消费方为实证）✓
2. `cargo test --lib` 计数不动（2685/0/151 三步逐位一致）；`check --all-targets` 0 错 ✓
3. 红线：signal.rs GOLDEN / nest_lifecycle.rs / 证书真值路径 / lean_parity §7 零触碰（diff 面只含 mod.rs/pipeline.rs/bsp.rs/tests/*）✓
4. 留痕：本报告 + T1 报告 ✓
