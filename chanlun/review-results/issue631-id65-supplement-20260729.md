# #631 补记：ID-6.5 第三次漂移登记（level_origin 删除致 GOLDEN 二次翻转）

- 日期：2026-07-29｜票据：issue #631（处置，sonnet 常规档）｜commit：`d9f1860124`
- 工位：`/tmp/kimi-nest-mainline`
- 前置：`issue610-digest-guard-attribution-20260728.md`（归因）；`issue610-id65-supplement-20260729.md`（ID-6.5 补齐，其「边界条件」节明文要求：删除路径下「再次诚实重锚**并登记**……不沿用本次值静默放行」）；`owner-attribution-fix-readings-20260724.md` §5（center 面登记）

## 登记表（第三次，格式同前两次）

| 面 | 旧值 | 新值 | 归因 |
|---|---|---|---|
| `extract_signals_bit_exact_digest_guard` GOLDEN 实际值（level_origin 删除面） | `0xe6a2_63e3_43e4_3845`（#610 重锚值） | `0xe371_3897_d9bf_978c` | **#631 删除 `BspPoint.level_origin` 字段**（`bbbd8f89fa` 顺带入库的恒 0 空转字段，14 文件清定义+构造点+PartialEq 分量）⟹ 247 点 Debug 串逐条尾部的 `, level_origin: 0` 消失，摘要二次翻转。处置选择与 main 分支 #455（`2d1abf9786`，grilling 裁定照删）同构；新值与 main #455 提交记录的历史值**逐位相同**——两独立分支对同一改动的实跑结果一致，交叉印证。`signal.rs` 常量注释按 #308 先例记历史值五级链（0xe6a2→0x90c7→0x56ed→0x37d2→0x06b3）+ 引入提交 + 重锚日期，且显式声明「不自定义 Debug 补回假字段掩盖翻转」。per-case 对拍 `extract_signals_bit_exact_vs_orig_per_case` 仍全绿（字段删除不改变六 bit/pivot/center/struct_break_dir 语义） |

## 结论

- `level_origin` 族的 ID-6.5 履行记录至此闭环：引入面（#610 补记）+ 删除面（本件）+ center 面（07-24 §5）三段齐。
- 本分支此前从未并入 main #455（分叉在其前），本次为独立复现同一处置；两线 GOLDEN 历史值链在 `0xe371…` 汇合。

## 边界条件

- 后续任何改动再触 BspPoint Debug 串形状（字段增删/类型改写/Derive 调整），照本例第三次登记续链，不得静默放行。
