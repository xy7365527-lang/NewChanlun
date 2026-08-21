# 扫描唯一化历史考古：#1056（P1/CandDeltaEvent/nest 对照侧/strict_nest_check）

- 日期：2026-08-18 ｜ 票：[#1056](https://github.com/xy7365527-lang/NewChanlun/issues/1056)（map #1055 子票）
- 性质：只读考古（喂「替代防线选型」「nest 对照侧重接形态」「strict_nest_check 新使命」三张 grilling 票）
- 方法：代码 + ADR + 仓内报告核对；行号以 main @ 2026-08-18 为准

## 一、CandDeltaEvent 字段的写入者与消费者

载体：`rust/src/theta_v0/classifier/recursive_tower.rs:1287`（约 20 字段族）。

**写入者（两段）**：

1. `level_cand_delta`（recursive_tower.rs:2142）——预扫描段循环写出：`level`/`side`/`divergence_confirm_src`（=别名 `confirm_src`）/`interval`（=别名 `c_episode_interval`）/`a_interval`/`c_episode_start`/`c_episode_interval`/`enter_src`/`cand_delta`/`pan_div_diag` + `cp_ownership` 边（`CandDeltaCpEdge`）。
2. 塔侧 c_p 生命周期通道（`cp_event_objects` recursive_tower.rs:1992 一带 + `relaxed_cand_delta_entries` :1379 附着 Closed 对象）——写出：`c_interval_full`/`b_parent`/`c_structure`/`third_class_in_c`/`cp_certificate_confirm_src`/`full_trend_c_qualified`/`full_trend_evidence`。

**消费者清单（按字段面）**：

| 消费者 | 读的字段面 | 用途 |
|---|---|---|
| `strict_nest_check`（bin） | P1：cand_delta/confirm_src/side；P2：c_episode_start/interval/b_parent.source_interval/c_interval_full（:879-894） | P1 硬门对拍 + P2 父区间证据 |
| `p123_fast_replay`（:1305-1360） | cand_delta/c_episode_start/confirm_src/side/b_parent.source_interval | old-semantics 基线计数 + 旧证书 |
| `p116_turnpoint_anchor_existence`（:407/:983） | cand_delta + 证书装配面 | P1 基线对账 |
| `p92_nest_replay_postruling`（:272） | interval 族 + 证书装配 | 旧路径审计 |
| `p107_level_funnel_audit`（:406） | 旧路径事件 | 逐级分解参照 |
| `p124_merge`（:750） | 旧路径事件 | 合并对账 |
| `cp_capability_smoke`（bin） | c_structure/full_trend/cp_ownership 全 c_p 证书面 | 证书承载能力冒烟 |
| `opsem_dump` sidecar（backtest 库 :199-231，env `OPSEM_DUMP_DIR` 门控，未设=no-op） | cand_delta/confirm_src/side | P1 计数 + strict_nest 对照 |
| `nest.rs` d_parent_interval 族（:1071-1115） | cand_delta/side/confirm_src/c_episode_start/c_episode_interval/a_interval/enter_src/interval | 对照侧链装配 |

## 二、nest 对照侧消费面与生产消费状态

**关键分叉：仓内「nest」是两条线，只有一条消费 CandDeltaEvent。**

- **生产准入 nest（gate-on）**：`build_nest_certificate`（econ_positive.rs:997）输入 = `tower + BspBits + hist`；`nest_index::build_nest_certificate_index`（nest_index.rs:262）输入 = `Vec<NestCandidateEvent>`（**另一类型**）。admission.rs:62-69 门 = env `THETA_NEST_CERT_GATE=="1"`，默认未设 ⟹ π 门整体跳过、全部路径逐字节不变（bit-exact 回归锁在案）。**→ 生产准入路径不消费 CandDeltaEvent；P1 退役零触碰。**
- **对照/诊断 nest（CandDeltaEvent 面）**：`assemble_certificates*`（nest.rs:1296/:1309/:1336）消费 CandDeltaEvent。调用点穷举（检索式 `grep -rn "assemble_certificates" rust/src --include="*.rs"`）：strict_nest_check bin:1584、p92 bin:302、p123 bin:1355、opsem_dump:224（backtest 库内 sidecar，**env 门控**：`OPSEM_DUMP_DIR` 未设 ⟹ 全方法 no-op，opsem_dump.rs:419/:496）、runner_tests:121。**→ 无生产准入调用；唯一库内可达路径是 env 门控的诊断 dump 侧车。**
- **typed 线分叉**：`assemble_typed_certificate*`（nest.rs:894/:1030）吃 `NestCandidateEvent`、不吃 CandDeltaEvent；库内调用点 = nest_index.rs:296（`build_nest_certificate_index`，生产 gate-on 路径）+ bin 探针（p92/p102b/p108/p109/p111/p123）+ turn_class.rs 测试（cfg(test)）。

结论：CandDeltaEvent 面 = 有实现、有测试锁、无生产消费（对照侧），与 ADR-0005 例外面一致；「nest 证书」生产准入走 `NestCandidateEvent` 另一条线（tower+BSP 源）。

## 三、strict_nest_check 承重面

- **P1 硬门**（strict_nest_check.rs:419-660 一带）：每 `STRICT_NEST_P1_CHECKPOINT`（默认 1000）bar + 末根终态，逐级 `cand_delta=true 事件 (confirm_src, side)` 多重集 ≟ `bsp buy1/sell1 (source_index, side)` 多重集；不一致 ⟹ FAIL + 差异样例，停线。另有逐 bar 因果诊断（非门，250k 前缀 8 处确认撤回在案）。
- **P2 面**：父区间证据（`b_parent`/`c_structure`/`c_interval_full`/`full_trend_*`）喂 nest 链装配与审计（p123/p116/cp_capability_smoke）。
- **P1 退役的连带**：P1 硬门变重言式 ⟹ 删；P2 读数若要保留需新载体——生产侧 `LevelState.cp_ownership` 已在产 c_p 生命周期对象，投影可行，不必另造扫描。

## 四、#668/#670 事故面由谁抓出；P1 不在场谁补

- **#668/#670（bsp_bridge 判据/覆盖错）的抓取设备** = #670 影子评审三轮 FAIL + 真值表 bin（`p_issue668_bsp_key_truth`）+ 300k 窗查询完备性实测（`edges_for_bsp_point` 7/29）+ 三窗验收 bin。**都不是 P1/strict_nest_check**——P1 硬门只对拍「BSP 装配线 vs P1 装配线」，bsp_bridge 的键/覆盖错不在其射程。
- **P1 独有射程** = 「两条 Rust 装配线逐位一致」（prelude/装配 drift——镜像漏改类）。在案证据：strict_nest_check 600k 前缀 0 mismatch（绿）；FAIL 停线机制存在，近期零触发记录未查实（如实标注，不冒充有抓过实例）。
- **若 P1 退役**：#668/#670 类错仍有真值表 + 影子评审 + 验收 bin 兜底；装配 drift 类失去唯一机器——这正是「替代防线选型」（#1058）要补的洞。

## 五、P1 plan + #551 单源后的剩余射程

- P1 出处：规格文件 `strict-nesting-divergence-plan-20260708.md` **已归档出仓**（#504 清收 commit 7798007580：tar `/tmp/newchanlun-graveyard-20260728/review-results-archive-20260728.tar.gz`，清单 `.chanlun/review-results/archive-sweep-20260728.md`）；`strict_nest_check.rs` 头注释（:4）仍指向旧路径 = 悬空引用。在树正本 = `chanlun/escalate/strict-nesting-rulings-20260708.md`（三裁决：Cand^δ≔背驰段谓词、盘整背驰不入链、确认时点=完成时）。
- #551 后判据核心单源：两条线同读 `first_structural_gates`/`judge_first_cached`。P1 独立部分 = prelude（排序守卫/方向锚/趋势门解析/first_match_idx/A 段缓存/λ_C episode 定界）+ 事件装配 + c_p 生命周期附着。
- **剩余射程 = 只抓「装配层两线 drift」**；不抓判据错、不抓 bsp_bridge 键错（诚实边界）。历史上 prelude 靠手工镜像（p117 037:20 破 b 包络两条线各落一次即例）。

## 六、对 3b 的考古结论（喂票面）

1. 生产准入 nest **不**消费 CandDeltaEvent ⟹ P1 退役的生产面风险 ≈ 0（gate-on 路径不动）。
2. P1 退役的真实受害面 = 6 个审计 bin + strict_nest_check P1 门 + P2 证据载体的生产者。
3. 替代防线须覆盖的洞 = **装配层 drift**（#1058 的靶心），不是判据错。
4. P2 证据（c_p 生命周期）已在生产 `LevelState.cp_ownership`（pipeline.rs:63，`Rc<Vec<CpScanOwnership>>`，与 centers 1:1，Pending→Closed 单调推进）产出；**但生产侧「事件确认快照不在这里回填」**（该字段文档原话）——确认快照的附着仍在 P1/审计面 ⟹ 投影载体可行，快照回填口径正是 #1059/#1060 要裁的缝。
