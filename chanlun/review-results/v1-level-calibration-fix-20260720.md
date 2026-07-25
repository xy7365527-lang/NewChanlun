# V1 级别标定严格化实装：区间包含判据探针 + 91 证书对账（路 A，2026-07-20）

任务：按 `v1-level-calibration-attribution-20260720.md` §4 路 A 实装——口径显式分离 + 判据
升级为区间包含，**不动任何赋值**（nest.rs 只读引用；`rust/Cargo.toml` 零改；决策层
typed_ledger/TW/sep_legs/塔主结构逐字节未触，judge_at 一个 bit 不动）。
工位：worktree `/tmp/kimi-nest-mainline`（分支 kimi-nest-mainline-20260717）。
代码行号锚均为本 worktree 当前行号。

## 0. 实装物

- 探针 bin：`rust/src/bin/v1_level_calib_fix.rs`（只读断言探针，新增文件，唯一代码改动）。
  - 窗口包含链上爬：`climb`（rust/src/bin/v1_level_calib_fix.rs:165）——lvl j+1 候选点
    离开窗口 ∋ 当前点 src，逐级 +1，备忘定型（最近优先、最大高度）。
  - 窗口区间取自 `LevelState.centers`（:364-366：src 前最后一个 end_index < src 的中枢；
    回退 start_index <= src，计数照实）——消除 p108 的 `BspPoint.center` nocenter 空洞
    （裁定建议③，`p108-interval-probe-20260717.md:48-50`）。
  - 单节极值距离 `dist_of`（:211，买=窗口 argmin low / 卖=argmax high，|极值bar−src|，
    p108 检验 3 同口径）；存在性单调上爬 `climb_mono`（:249——单调为链局部性质，按节备忘精确）。
  - T1 配对：`pred = exec - 1`（:524；`event_bsp_book_level`，nest.rs:532-537；教义锚
    037:16/024:18/043:26）。
  - 门控等价物：既有探针经 Cargo.toml `required-features = ["backtest_bin"]` 解门控；
    本 bin 自动发现（Cargo.toml 零改纪律），以 crate 级 cfg 等价实装（:45-49 头注、
    :655-663 双 main）——无 feature 时编译为提示性 main，默认构建不破。
- 运行输出：`/tmp/v1_fix.out`（BTC 全量 4,613,599 bars，输入 `/tmp/p92_dump_full.txt`
  91 证书：A 口径 46 / B 口径 45，exec=1×87 / exec=2×4，#100 对账集）。

## 1. 判据定义（新旧对照，全部在探针内同运行计算）

| 判据 | 定义 | 出处 |
|---|---|---|
| OLD_PT | 任意级绑定 bar 处同侧 BSP lvl 集合 ∋ exec（同点硬判据） | 断裂原始出处；p108 检验 1 已证 70-92% 结构性缺失 |
| OLD_DIST | 最近 lvl==exec 同侧点 |dt|≤1440（P107_SAMELVL 复现，bar 距离） | p107 :33 |
| NEW_ID | 绑定 bar 处 lvl 集合 ∋ (exec−1)（T1 身份层） | p117 T1，nest.rs:532-537 |
| NEW_ANCHOR | 最近 lvl0 同侧端点 |dt|≤1440（路 A iii：绑定一律锚 lvl0） | p107 结论 1/4，:62/:69 |
| NEW_CHAIN | 从 lvl0 锚点窗口包含上爬可达高度 h ≥ exec−1（路 A ii 核心） | p108 裁定建议①，:45-46 |
| NEW_MONO | 链上 |极值bar−src| 随级别下降非增；witness（见证链）与 exist（存在性）双口径 | p108 裁定建议②，:47 |

## 2. 91 证书对账：新旧判据对照表（探针实测输出）

| exec | n | OLD_PT 同点 | OLD_DIST bar 距离 | NEW_ID T1 身份 | NEW_CHAIN 窗口包含 |
|---|---|---|---|---|---|
| 1 | 87 | 8（9.2%） | 31（35.6%，strong=8 weak=23 far=56 none=0） | 79（90.8%） | **87（100.0%）** |
| 2 | 4 | 0（0.0%） | 0（0.0%，far=4） | 0（0.0%） | 2（50.0%）⚠ 样本不足 |
| 合计 | 91 | 8（8.8%） | 31（34.1%） | 79（86.8%） | **89（97.8%）** |

- 绑定基线复现：`bound_any=91/91`（p101/p107 基线）、`bound_l0=91/91`（lvl0 锚定，
  P107_MATRIX lvl0 far=0 的路 A (iii) 复现）。
- 上爬可达高度分布（`V1_HEIGHT`）：exec=1 → lvl0×64 / lvl1×8 / lvl2×5 / lvl3×7 / lvl4×3
  （锚点虽为 lvl0 端点，23/87 的锚点结构在窗口包含链下可达 lvl≥1——嵌套关系的正向读出）；
  exec=2 → lvl3×2（idx 87/88，h=3 ≥ pred=1，一致）、lvl0×2（idx 89/90，见 §3.1）。
- 对照说明（090 照实）：任务书「旧 bar 距离判据的 0%」在 exec=2 组字面上成立（0/4），
  exec=1 组实测为同点 9.2% / 同级 bar 距离 35.6%——p107 的结论表述是「无同级对应」
  （far=56/87=64% 主导，`p107-level-calib-20260717.md:33`），非字面 0%。本表以实测为准。
- 分支口径说明：本 worktree 与 main 的 classifier 状态有微差（OLD_DIST exec=1 实测
  strong=8 weak=23 far=56 vs p107 main 实测 strong=8 weak=22 far=57，:29）——
  本报告全部数字以本 worktree 实测为准，p107/p108 数字仅作定性对照。

## 3. 残存归因（090：照实否定合格）

### 3.1 NEW_CHAIN 残存 2/91（idx 89/90，exec=2）= 数据右缘确认延迟，非判据断裂

- 两张证书 judge_max=4613104、bsp_bar=4613104（dt=0），lvl0 锚点 src=4613084，
  距数据末端（4613599）仅 495 bars；ids 区间为 837024-4613084 / 828966-4613084 的
  巨型开放段（`V1_TAG idx=89/90`）。
- 量化归因（`V1_EDGE`）：sell 侧 lvl1 最大 src=4608702 < 锚点 4613084——**数据内根本
  不存在 src ≥ 锚点的 lvl1 同侧确认点**；离开窗口 ⊂ [·, src] ⟹ 包含锚点的 lvl1 窗口
  结构上不可能已确认。即「爬不上去」是高级别事件确认延迟（需后续 bar），不是窗口
  包含判据的反例。
- 与归因文档一致：「exec≥2 仅 4 张样本，其中 2 张在数据右缘——证据不足，不下判定」
  （`v1-level-calibration-attribution-20260720.md:110-111`）。可判定的 exec=2 子集
  （idx 87/88，非右缘）2/2 一致，且上爬高度 h=3（≥ top−1=1，覆盖整个证书递归跨度）。
- 结论：路 A 下「断裂」对全部**可判定**证书消失（87/87 + 2/2 = 89/89，100%）；
  残存 2 张为右缘不可判定项，落账为「数据右缘确认延迟」，留待数据延伸后复测。

### 3.2 NEW_MONO 残存 1/25（idx 48）= 跨 src 链中间节点 d=0，p108 同 src 口径不直接覆盖

- 单调收敛双口径：witness（高度优先定型见证链）21/25（84%）；**exist（存在性——
  单调约束下可达高度 = 无约束可达高度）24/25（96%）**。
- 唯一残存 idx 48（Short）：path 0:3066658 > 1:3068007 > 2:3069866 > 3:3081052，
  dists 0:231 > 1:0 > 2:1859 > 3:13045——lvl1 节点恰为自身窗口极值（d=0），
  单调要求下级 d ≤ 0 ⟹ 任何下级链节都不可能满足，存在性检查同样到不了 lvl3。
- 归因：p108 的 114/114 单调收敛（`p108-interval-probe-20260717.md:38`）是在
  **同 src 多级组**上实测的——窗口嵌套（contain 100%）+ 同一 src 使 |极值−src| 单调
  成为嵌套的直接推论；本探针的跨 src 上爬链每节 src 不同，中间节点「点=极值」（d=0）
  会对下级施加不可能的约束。这是**检验对象的推广**（同 src 组 → 跨 src 链），不是
  p108 经验事实的反例，也不是窗口包含判据的失败（该证书 chain_hit=true，h=3 ≥ pred=0
  本身一致）。另注意 p108 自身实测 lvl0 exact0 仅 3.86%（:39-41）——「点≠极值是常态」，
  单调收敛作为力度门设计约束时应采用存在性口径并容许此类中间 d=0 破例。
- 断言 A5（存在性单调 100%）因此失败，exit=1，照实落账（见 §4）。

### 3.3 区间来源核验

- `windows=27252`，`fallback_center=21`（0.08% 用回退规则），**nocenter=0（全级别）、
  nocenter3=0**——`LevelState.centers` 口径完全消除 p108 的 nocenter 空洞
  （p108 中枢口径 nocenter 对数 1,271/670/396/219，:33-34），裁定建议③落地。

## 4. 断言输出（V1_ASSERT，exit code 携带）

```text
V1_ASSERT name=A1_bind_any pass=true detail=bound_any=91/91
V1_ASSERT name=A2_anchor_l0 pass=true detail=bound_l0=91/91
V1_ASSERT name=A3_routeA_sup pass=true detail=new_chain=89/91 vs old_pt=8/91
V1_ASSERT name=A4_nocenter3 pass=true detail=nocenter3=0
V1_ASSERT name=A5_mono pass=false detail=exist_mono=24/25 witness_mono=21/25
V1_DONE
exit=1
```

A1-A4 通过；A5 失败 = §3.2 残存（1 例跨 src 中间 d=0 节点），已归因落账。
汇总行：`V1_SUMMARY certs=91 bound_any=91 bound_l0=91 old_pt=8/91 new_id=79/91
new_chain=89/91 mono=21/25 mono_ex=24/25 nocenter3=0`。

## 5. 回归验证

- `cargo test --release --lib`：**1770 passed; 0 failed; 129 ignored**（finished in 0.96s）
  ——全绿零变红（本 worktree 基线即 1770；任务书所引 1755 为 main 基线，本分支多 15 个）。
  探针为 bin，不进 lib 测试面。
- 编译双门控验证：`cargo build --release --bin v1_level_calib_fix`（无 feature，走提示性
  main）与 `cargo build --release --features backtest_bin --bin v1_level_calib_fix` 均过。
- 探针只读：不改判据、不改赋值、不写主路径状态；lib 源文件零改动（git status 中
  rust/src 下的 M 项均为工位进入前既有改动，与本任务无关）。

## 6. 结论

路 A（不动赋值、口径分离、判据升级为区间包含）下，91 证书「级别标定断裂」**对全部
可判定证书消失**：窗口包含判据一致性 89/89=100%（exec=1 87/87；exec=2 可判定 2/2），
对照旧判据同点 8.8% / 同级 bar 距离 34.1%。残存两项均已归因：右缘确认延迟 2 张
（§3.1，不可判定非反例）、跨 src 单调 1 例（§3.2，检验对象推广非 p108 反例）。
「级别」口径分口径显式：nest exec/top = 塔索引/证书递归深度，classifier lvl = 窗口合成
级别，T1 移位 exec=k ↔ lvl=k−1 为两口径间的唯一合法桥（本探针 :524 实装此桥）。

## 复现

```text
cd rust && cargo run --release --features backtest_bin --bin v1_level_calib_fix -- /tmp/p92_dump_full.txt
```
