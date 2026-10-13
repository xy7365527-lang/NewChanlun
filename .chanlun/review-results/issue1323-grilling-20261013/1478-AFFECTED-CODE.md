# #1478 受影响代码清单（中枢外缘 GG/DD 的遍历范围落码与延伸测试的末成员端点）

> 票：[#1478](https://github.com/xy7365527-lang/NewChanlun/issues/1478)；教义正本：[`zhongshu.md`](../../definitions/zhongshu.md)（中枢；落点 §3、§4.1 端点条款、§9.0 行号订正与 R21–R27／L9–L11／P14–P16）。
> 基线：**main `465174240b989a1d99f99a047fbb46b3ee215f49`**。行号于该基线逐行核过。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归下表「归属」列的实施票，本清单本身不改任何代码（本次提交仅有的一处代码改动是 #1344 的一行零行为注释，见 #1344 清单）。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/theta_v0/classifier/recursive_tower.rs:525-527` | main `465174240b` | R21：延伸循环体（`while` 在 `:524`）的 `c.dd`／`c.gg` 改为仅 `Zn`（`(j−i)` 偶数位）参与 | #1419（并入 R8 批次） |
| Rust | `rust/src/theta_v0/classifier/recursive_tower.rs:553-578` | main `465174240b` | R22：≥9 段升级子窗 dd/gg 改为仅 `Zn` | #1429 |
| Rust | `rust/src/theta_v0/classifier/center.rs:170/:176` | main `465174240b` | R23：`compute_gg`／`compute_dd`（调用点 `:257-258` `center_from_segments`、`:294-295` `center_from_window`）改为取第 1、3 段；端点相连时值不变 | #1419 |
| Rust | `rust/src/theta_v0/classifier/ref_v1.rs:150-151/:157-158` | main `465174240b` | R25：Lean 参考实现的 Rust 镜像（seed `max3`／`min3`、延伸聚合），与 L9／L10 同拍（parity 契约） | #1419 同拍 |
| Rust | `rust/src/theta_v0/classifier/center.rs:312` | main `465174240b` | R26：`classify_relation`（中心定理二）以 `Zn` 外缘为输入；`Zn` 外缘落地前不得验收 | #1424 |
| Rust | `rust/src/theta_v0/types.rs:125` | main `465174240b` | R18：`Center` 不加 G/D 字段（不动） | — |
| Rust | `rust/src/zhongshu.rs:72-87` | main `465174240b` | R24：旧线「max(所有段)」；随 TB-10 收编或退役，不参准入 | TB-10 |
| Fixture | `rust/tests/fixtures/theta_v0_center_parity.json` | main `465174240b` | R27：与 R8 同一次 regen | #1419 |
| Lean | `formal/Origin/CenterConstruct.lean:408-417` | main `465174240b` | L9：`extendWeak` 聚合（`:417`）改为仅 `Zn`（奇偶位） | #1419 同拍 |
| Lean | `formal/Origin/CenterConstruct.lean:442-443` | main `465174240b` | L10：`refStep` 的 `gg0 := max3 h1 h2 h3`／`dd0 := min3 l1 l2 l3` 改为 `max(g₁,g₃)`／`min(d₁,d₃)`；`:70-71` doc 注释同步 | #1419 同拍 |
| Lean | `formal/Origin/CenterConstruction.lean:104/:108` | main `465174240b` | L11：`computeGG`／`computeDD` 改为取第 1、3 段（与 R23 对应） | #1419 同拍 |
| Lean | `formal/Origin/CenterFull.lean:53-59` | main `465174240b` | L8：`CenterFull` 不加 G/D 字段（不动） | — |
| Python | `src/newchan/a_zhongshu_v1.py:79-80/:100-101/:108-109` | main `465174240b` | P14：字段注释「max(所有段的 high)」、seed、延伸聚合；随 TB-10 收编或退役 | TB-10 |
| Python | `src/newchan/a_center_v0.py:78-80/:159` | main `465174240b` | P15：唯一按 `Zn` 算的实现（死码）；登记，不升正本 | — |
| Python | `s_session/s_tb02_contract.py` | main `465174240b` | P16：新增 `validate_center_payload`，校验 GG/DD/G/D 与 `Zn` 成员一致 | #1419 |
| Doc | `.chanlun/definitions/zhongshu.md` | main `465174240b` | 三行头、§3 G/D 行与「落码补齐」段、§4.1 端点条款与退场条件、§5.4、§9.0 行号订正、§9 新增行、§9.4（本次提交已落） | 本票落文档 |
| Doc | `CONTEXT.md` | main `465174240b` | 词条「Z 走势段（Zn）」（本次提交已落） | 本票落文档 |

## 本轮复核订正（相对工位草稿）

- 工位草稿把 Lean seed 外缘记在 `CenterConstruct.lean:70-71`；那两行是 doc 注释，代码在 `:442-443`（`refStep`），另有 `CenterConstruction.lean:104`／`:108` 的 `computeGG`／`computeDD` 同样三段全取，本清单补为 L11。
- 工位草稿漏列 `rust/src/theta_v0/classifier/ref_v1.rs`（Lean 参考实现的 Rust 镜像，seed `:150-151`、延伸 `:157-158`）；它与 L9／L10 受同一 parity 契约约束，补为 R25。
- 工位草稿记 `a_zhongshu_v1.py:79-100`；实际是 `:79-80`（字段注释）、`:100-101`（seed）、`:108-109`（延伸聚合）。
- 工位草稿记 seed 外缘在 `center.rs:241`／`:283`；那是 `center_from_segments`／`center_from_window` 的函数头，三段全取发生在 `compute_gg`／`compute_dd`（`:170`／`:176`），调用点 `:257-258`、`:294-295`。
- `zhongshu.md` §9 旧表行号（基线 `e87a3612f5`）已在 §9.0 订正：R3 `:274→:312`、R4 `:66→:84`、R5 `:747→:524`、R7 `:757-804→:553-578`、R8 `:804→:581`、R9 `:772→:549`、R13 `:570→:293`、R14 `:650→:363`、R18 `:122-133→:125`、R19 `:56→:71`，R10／R11 迁到 `recursive_tower.rs:391/:397/:434` 与 `pipeline.rs:367`；R6、R12、R20 未重核。
- 落地约束（根裁定第 5 条）：塔与 Lean 并入 #1419 的 R8 批次，同拍改、同一次基线重生成；不允许出现「S 发 `Zn` 外缘、塔留全单元外缘」的中间态。

基线说明：工位草稿以 main `f95a7f3e` 为基线（#1344 部分行以 TB02D `03239158` 为基线）；本清单不靠差异推断，全部行号直接在 main `465174240b` 上用 `git show` 逐行核过。
