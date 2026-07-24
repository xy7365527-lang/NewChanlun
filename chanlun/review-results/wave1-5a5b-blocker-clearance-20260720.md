# wave-1 5a/5b 硬阻塞清理核查档（issue #67，2026-07-20）

- **工位**：核查工位（worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`）。只读核查 + 落档：零代码改动、零 git mutation、主仓只读、`rust/Cargo.toml` 未触。
- **输入**：issue #67、`wave1-5a5b-preflight-check-20260719.md` §6（硬阻塞三项）、`wave1-static-verification-20260719.md` §8、`l4-lib-exposure-20260719.md`、两实装卡、`.chanlun/scene-ledger.md`、`/tmp/w1_full_*` 产物。

## 结论总表

| 硬阻塞 | 状态 | 依据 |
|---|---|---|
| ①a wave-1 验收封口（双跑「在跑」收口） | **已清**（编排者裁定在案） | §1 |
| ①b wave-1 未提交改动 | **仍存**（本工位禁 git mutation，转编排者行动项；性质已被 ①a 裁定部分消解） | §1.4 |
| ② R1-R3 裁定确认 | **待编排者确认**（唯一剩余闸门） | §2 + `chanlun/escalate/r1-r3-ruling-confirmation-checklist-20260720.md` |
| ③ lib 暴露面复核 | **已清** | §3（`cargo test --release --lib` 1770/0 全绿，对接点逐项核对一致） |

## 1. 阻塞①：全量双跑终态核实 + 验收裁定登记位置

### 1.1 双跑实际终态（/tmp 产物实证）

- 脚本 `/tmp/w1_full_double_run.sh`：8 片 p124_shard_replay → p124_merge 归并 → `diff /tmp/w1_full_m_dump.txt /tmp/p124_r2_m_dump.txt`。
- 产物 `/tmp/w1_full_s0..s7` + `.out`（2026-07-19 07:05-07:12）：**6/8 片完成**（s0/s1/s3/s4/s6/s7 末行均为 `P124_MISSED ...` 收尾，PREFIX 段走到 4.5M/4.6M）；**shard2 停在 PREFIX 3.5M/4.6M、shard5 停在 4.0M/4.6M**——中止现场与 scene-ledger「6/8 片已完成、shard2 至 3.0M+ 时停」一致。
- **归并阶段从未启动**：`/tmp` 无 `w1_full_m_dump.txt` / `w1_full_m.out` / `w1_full_vs_g2.diff`（ls 实证）。`w1_full_vs_g2.diff`（dump diff=0 的封口产物）不存在 ⟹ static 档 §8 末行「在跑」即终止于此状态。
- 在线计数一致证据复核：`w1_full_s2.out` 的 PREFIX 里程碑（1.5M=9714/14332、2.0M=8271/16763、2.5M=6846/20655、3.0M=5389/24476，pending/views）与 static 档 §8 登记的旧跑在线信号**逐字一致**。

### 1.2 裁定登记位置（任务①核心核实项）

编排者裁定「总重跑也不用了，3M 一致已证明」登记于 **worktree `.chanlun/scene-ledger.md`「已完成（本台账登记后）」节**：

- `:16` 中止令（2026-07-19，编排者令）：6/8 片已完成、shard2 至 3.0M+ 时停；先完全解决超线性（5a/5b），再做一次性总重跑。
- `:17` wave-1 验证状态如实登记（未封 4.6M）：静态证明 + 1732/0 + 250k/1M 逐字一致 + SHADOW 5,278 检查 0 mismatch + 在线计数一致至 3.0M。
- `:18` **wave-1 验收定谳（2026-07-19，编排者裁定）**：总重跑封印**取消**——「3M 都一致已经证明了」——wave-1 按「静态证明 + 1732/0 + 250k/1M 逐字一致 + SHADOW 0 mismatch + 在线计数一致至 3.0M」验收成立，4.6M dump diff 不再要求。

### 1.3 090 照实警示（不夸大封口）

- 该登记**自身处于未提交状态**（`.chanlun/scene-ledger.md` 在本 worktree `git status` M 列表内；登记所在的「已完成」节整段在未提交 diff 中）。主仓 `.chanlun/scene-ledger.md` 仍是旧版（无 :16-18 三条，已核）。裁定效力不受影响（编排者口头/会话裁定在先，台账登记在后），但**台账固档需随 wave-1 提交一并落库**——与 §1.4 同一行动项。
- `m8-readiness-checklist-20260719.md:92`（G-3）仍按旧口径写「S=8 一次性总重跑封印未落」——该文写作时点早于/同于定谳，此句对 wave-1 验收口径**已被 scene-ledger:18 取代**，阅读时以定谳为准。
- 定谳豁免的是 **wave-1 自身**的 4.6M 封口；5a/5b 本体卡的验收协议（5a §5-6 / 5b §7-5 的全量双跑 diff=0）未被豁免，届时按 scene-ledger:16「wave-1+5a/5b 合并封印」执行。

### 1.4 wave-1 未提交（①b 子项）

- 现状：`git status` 仍有 17 个 rust 源文件 M（classifier 五文件 = wave-1 方案 1/2/3；`mod.rs` 含 l4 暴露面；`runner.rs`/`exit.rs`/`interp.rs`/`ledger.rs`/`overlay_state.rs` 等 = 后续 m8/e2e 修复），全部未提交。本工位纪律禁 git mutation，**不代办提交**。
- 性质变化：preflight §6-1 的阻塞理由是「验收基线无固定码身份」——①a 定谳已豁免 wave-1 自身的经验封口；剩余实质需求 = 5a/5b 实装的**同码基线**需要一个固定码身份（两卡 §5/§7 的 250k/1M diff=0 对拍基线）。**行动项（编排者/实装工位）：5a/5b 动 code 前安排一次提交固档（含 scene-ledger 台账）**。

## 2. 阻塞②：R1-R3 裁定确认清单（已落 escalate/）

清单全文：`chanlun/escalate/r1-r3-ruling-confirmation-checklist-20260720.md`。三项卡内已裁、待 Lead/编排者一次性确认：

| # | 裁定项 | 卡内位置 | 待确认要点 |
|---|---|---|---|
| R1 | 5b 两条失效链构造性签字 | 5b 卡 §2/§4/§5/§9 | 接受两链构造性论证；**接受 §5 TURN 残余风险继承**（末窗一次重扫产 ≥2 子中枢的 bar 理论上仍可能提前落盘；拦截网 = V0 + SHADOW 全程对拍 + 双跑 diff） |
| R2 | 5a 游标放 bin `LevelDerived` | 5a 卡 §2.2 | 决定项 = shadow oracle 走真冷路径（store 在 lib 则双染假绿）；寿命对齐 + seam 最小（69 处调用点签名不动，preflight §2.1 已更正卡内「~30 处」措辞） |
| R3 | 5b memo 放 bin `RunEntry` | 5b 卡 §3 | 键语义是 run 语境（center 来自 run 投影种子、kind 门来自 run blocks）；显式参数保 shadow 可控（forced 传 None） |

附 R4 观察（非裁定项）：5a per-level / 5b per-run 无冲突，但两卡同改 B3-B5 调用点，须同工位三段推进（preflight §4）。

## 3. 阻塞③：lib 暴露面复核（l4-lib-exposure-20260719.md）

### 3.1 测试闸门（本工位实测）

```
$ cargo test --release --lib   # 2026-07-20，本 worktree
test result: ok. 1770 passed; 0 failed; 129 ignored; 0 measured; 0 filtered out; finished in 1.01s
```

- 与任务给定基线 **1770 passed 一致，全绿零变红**。l4 档登记的 1737 是其时点口径（后续工位新增 33 项测试至 1770）；暴露面 5 单测（`freeze_boundary_matches_signal_inline`、`tower_water_levels_empty_tower/single_segment/multi_segment_append/frontier_rewrite`）在 release 网内（mod.rs:3625/3651/3673/3697/3770 实证存在）。

### 3.2 暴露面与两卡对接点逐项核对

| 对接点 | 卡内要求 | 实装现状（本工位 grep 实证） | 判定 |
|---|---|---|---|
| L1 `LevelCache.confirmed_len` | 5a 卡 §2.3(a)（mod.rs:765 结构内 + :1825 旁维护） | mod.rs:838 字段 + mod.rs:1900 `lc.confirmed_len = prefix_count;`（l4 档行号 +4 漂移，语义锚完好） | ✅ |
| L2 `TowerCache.tower0_confirmed_len` | 5a 卡 §2.3(a)（classify 开头写入） | mod.rs:942 字段 + mod.rs:1557 写入 + mod.rs:1053 `clear()` 归零 | ✅ |
| L3 `tower_confirmed_len(level)`（legs 量纲） | 5a 卡 §2.3(a) 签名草稿 | mod.rs:994-1000，与卡内草稿逐字一致（level==0 上行；缺级 `map_or(0,…)`） | ✅ |
| L4 `freeze_boundary` 公式函数 | 5b 卡 §3.3-1：signal.rs 提取 + `extract_first_third_resume` 改调（单一来源） | **偏离（已登记）**：自由函数落 mod.rs:954；signal.rs:1168-1172 内联公式**仍在**（本工位 sed 实证逐字同）；双源一致性由网格对拍（mod.rs:3625）+ 生产交叉锁（mod.rs:3697 ⑤ `cached_first_third_count`）双锁；signal.rs 改调留 5b 实装工位（其授权面内）。l4 §4 已照实登记 | ✅（偏离已闭环登记） |
| L5 `last_freeze_boundary` + `freeze_boundary(level)` getter（source 坐标量纲） | 5b 卡 §3.3-2/3：无条件更新（不挂 07a 分支）+ `Option<usize>` getter | mod.rs:842 字段 + mod.rs:1901 无条件更新 + mod.rs:1006 getter（越界 None） | ✅ |
| 量纲红线（preflight :65） | legs 界 vs source 界不可互换，键名/注释显式 | rustdoc 三处显式标注（字段/访问器/维护点，l4 §8 diff 实证）；两卡比较点（5a gate `w_now >= cur.k0` / 5b ①a `seg_end < e_src`）各对正量纲 | ✅ |
| 单测 ×5 | l4 §5 覆盖矩阵 | 五测试全部在案（行号见 §3.1），release 网内绿 | ✅ |

### 3.3 边界声明复核（090）

l4 §3 的两条实测发现（水位值路径相关、重写 bar 水位回退）已写入 rustdoc，消费方（5a gate / 5b ①a）均只承诺证书语义、不跨血缘比水位值——与两卡用法一致。l4 §6「不声明 5a/5b 能力」声明与实装面一致（L6-L9/B1-B6 仍未实装，grep 零命中——preflight §1 行 13 结论复核仍成立：`ConfirmKey`/`ConfirmCursor`/`PanMemo`/`confirm_cursors`/`pan_memo` 等在 `rust/src/` 零命中）。

## 4. 给编排者的行动项汇总

1. **R1-R3 确认**：回 `chanlun/escalate/r1-r3-ruling-confirmation-checklist-20260720.md`（唯一剩余硬闸门）。
2. **提交固档**：5a/5b 动 code 前安排一次提交（17 个 M 源文件 + scene-ledger 台账），固定 5a/5b 验收的同码基线码身份（本工位禁 git mutation，不代办）。

## 5. 090 / v3 合规声明

- 本档为只读核查产物：所有「已清/待确认」判定均以实证为据（/tmp 产物 ls/grep、scene-ledger 原文、cargo test 实测输出、mod.rs/signal.rs 行号实证），不声明任何新能力。
- 无概率/统计推断、无回测验证、无 EMH 假设；全部结论为代码事实 + 文本比对 + 编排者裁定登记位置核实。
- 行号锚时效：mod.rs/signal.rs 行号 = 2026-07-20 本 worktree 工作树状态（HEAD `640609071d` + 全部未提交改动）；scene-ledger 行号 = 同状态。
