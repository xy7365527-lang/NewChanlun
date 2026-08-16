# #648 T1 留痕：classifier/mod.rs 内联测试抽离（classifier/tests/ 主题七件）

- 日期：2026-08-16
- 票据：#648 T1（#786 Q3 改写稿；勘察 `chanlun/review-results/issue786-q3-classifier-arch-review-20260816.md`）
- commit：`f4503ab64a`（main，镜像 @ 同点）

## 移动清单（纯移动，零行为变更）

| 落点 | 内容 | 行数 |
|---|---|---|
| `tests/fixtures.rs` | 16 个共享夹具（seg/bars_from_closes/cache_candidate/synthetic_segments/chain_fixture/PrefixCacheReuse 等，pub(super)） | 432 |
| `tests/incremental_parity.rs` | 26 测试：增量 vs 全量对拍族（incremental_tower/candidate_event_stream/chain_certificate_book/causal/forest_epoch/cascade_reset 等） | 1537 |
| `tests/pipeline_geometry.rs` | 19 测试：classify 管线几何 + 查询/诊断 + 真实数据 census 探针 | 1655 |
| `tests/third_class_anchor.rs` | 4 测试：#486/q7 anchor 门族 | 315 |
| `tests/operation.rs` | 3 测试：operation 分解族 | 109 |
| `tests/rebase_txn.rs` | 2 测试：rebase 接缝族 | 159 |
| `tests/incremental_profile.rs` | 尾部兄弟 `mod incremental_profile`（#[ignore] 真实数据标度） | 81 |

源：`mod.rs` 内联 `mod tests`（原 :2149-:6338）+ `mod incremental_profile`（原 :6340-:6421）。抽离后 `mod.rs` 6421→2151 行（管线段未动，属 T2 面）。

## 前后测试计数对照

- 搬前（本批基线 `c9fa040dd3` 干净树）：**2685 passed / 0 failed / 151 ignored**；
- 搬后（`f4503ab64a`）：**2685 passed / 0 failed / 151 ignored**——逐位不动；
- 移动件数：内联区 `#[test]` 55 件 → tests/ 七件 55 件，零丢失；
- `cargo check --all-targets` 0 错（探针 bin 含）；fmt 0 Diff。

## 机械适配留痕（纯移动必需的引用层调整，无语义改动）

- 各主题件头部补 `use super::super::*;`（classifier 根）+ `use super::fixtures::*;`；按内容探测补 `ParseLayer`/`Direction`/`MoveKind` 显式 import；
- 测试体内的局部 `use super::…` 路径统一降一级（super 链 +1）；
- 夹具晋升 `pub(super)`（classifier::tests 子树内共享）；
- bsp 白名单一夹具点随迁重锚：`mod.rs:4574` → `tests/pipeline_geometry.rs:1565`（dated 注记，内容重核不变）。

## 残余

T2（管线 ~2148 行 → pipeline.rs 复名）与 T3（LOW-12 死文件清零搭车）未动，#648 保持 open。

## 勘误（2026-08-16，影子评审 #997 回报后补记）

- **机械适配实际五条**：报告列了四条，漏列 rustfmt 重排（55 件测试体 17 件纯格式化差异、9 件换行形态——rustfmt 缩进敏感性：0 缩进 vs 4 缩进产出不同形态），零行为影响；
- **计数订正**：抽离后 mod.rs 实际 **2150 行**（报告 2151）、mod 声明实际 **35**（报告 36，系搬前数）；
- **incremental_profile 模块路径变化**（classifier::incremental_profile → classifier::tests::incremental_profile）未在报告单列——零外部引用，登记即可。
