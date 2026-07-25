# R1-R3 裁定确认清单（5a/5b 线形化实装前置）——待编排者一次性确认（2026-07-20）

- **工位**：核查工位（worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`）。本文件只读核查产物，零代码改动、零 git mutation。
- **来源**：issue #67 任务②；裁定本体已写在两卡内（卡内签字 ≠ 实装授权，preflight §3.3 口径），本清单仅汇总待确认点，请编排者对每项回「确认 / 驳回 / 改裁」。

## R1：5b 两条失效链构造性签字（含 TURN 残余风险继承）

- **卡内位置**：`wave1-plan5b-implementation-card-20260719.md` §2 逐函数读域表（:40-49，8 行函数封闭）+ §4 失效链①（:71-91，e_src 水位公式 + ①a/①b/①c 失效条件 + 充分/不过度论证）+ §5 失效链②（:93-112，≥2 后继块门 + ②a/②b/②c + 充分/不过度）+ §9 自封「签字完毕」（:140-142）。
- **待确认要点**：
  1. 接受 §4/§5 的两链构造性论证作为 memo 失效判定依据（水位 ① 用严格 `<`；② 用 `b_idx+2 < blocks.len()` 与 TURN `ready=m-2` 同构）。
  2. **接受 §5 残余风险声明**（:112）：继承 p116_turnpoint_anchor_existence.rs:19-23 的 TURN 已声明残余——「末窗一次重扫产 ≥2 子中枢的 bar 上理论上仍可能提前落盘」；本 memo 接受度与生产 TURN 落盘同级，拦截网 = V0 身份校验 + `P123_SHADOW=1` 全程对拍 + 全量双跑 diff（残余兑现表现为 shadow mismatch / dump diff，不停线于隐性错误）。

## R2：5a 游标存放位置 → bin `LevelDerived` 旁

- **卡内位置**：`wave1-plan5a-implementation-card-20260719.md` §2.2（:61-74）三案取舍：否决 (a) lib 内带键查询、否决 (b) bin `RunEntry` 旁、采纳 (c) bin `LevelDerived` 旁（per-level `ConfirmCursorStore`）。
- **决定项**：shadow oracle 必须能走真冷路径——store 在 bin ⟹ `P123_SHADOW=1` forced 重估传 `Cold`/新建空 store 即真全量 oracle；store 在 lib/TowerCache 内 ⟹ 两路径共享同一 store，陈旧游标两侧同错，`mismatches=0` 成假象，对拍失效。
- **次要理由**：寿命对齐（`LevelDerived` 正是 lower_legs/lower_gen/lower_ends 持有者，同域同寿命）；seam 最小（新增 `assemble_level_view_resident`，旧入口委托 `None`，`assemble_level_view` 69 处调用点签名不动——preflight §2.1 已将卡内「~30 处」措辞更正为实测 69 处）；不扩 TowerCache 硬契约（mod.rs:1417-1426「不加 lineage epoch」在案）。

## R3：5b memo 存放位置 → bin `RunEntry` 旁

- **卡内位置**：`wave1-plan5b-implementation-card-20260719.md` §3（:51-58）。
- **理由**：键语义是 run 语境的——center 来自 **run 投影**种子（evaluate_run :1109-1111）、kind 门来自 **run 的 blocks**（:1112）；`LevelDerived` 是 per-level（跨 run 共享 lower_legs），水位可 per-level，memo 值不行。lib 内持 memo（TowerCache 挂载，B3 area-memo 先例存在）会使 `provide_nest_candidate_events` 输出依赖隐式跨调用状态、破坏「同入参同输出」显式性，且 shadow 不可控。显式参数 + bin 持有 ⟹ forced 路径传 `None`（5b 卡 §7 第一要点）。

## 附：R4 一致性观察（非裁定项，无需回）

preflight §3.3-R4：5a 放 per-level（LevelDerived）、5b 放 per-run（RunEntry），两卡各自论证自洽、无冲突；但两卡同改 B3-B5（evaluate_run 签名 + dirty/shadow 两调用点），排期上须同工位或先后衔接——preflight §4 建议同一实装工位三段推进（段一 lib 暴露面【已落】→ 段二 5a → 段三 5b）。

## 确认方式

对 R1（含 1、2 两个子项）、R2、R3 各回「确认 / 驳回 / 改裁」即可；全部确认后 5a/5b 硬阻塞②解除（issue #67 结论档 `wave1-5a5b-blocker-clearance-20260720.md` §3 同步登记终态）。
