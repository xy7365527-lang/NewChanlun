# ADR 0026：扫描唯一化的替代防线——Lean 完整语义镜像 + 对拍签收，P1 退役以镜像签收为前置

**日期**：2026-08-18（原裁 0025；2026-08-22 谱系重建致原编号被 lav-shadow 契约占用，重编号 0026，内容一字未改）
**裁定人**：编排者，走 [\[grilling\] 替代防线选型：P1 硬门退役后装配层不变量由什么接管 #1058](https://github.com/xy7365527-lang/NewChanlun/issues/1058)（map [#1055](https://github.com/xy7365527-lang/NewChanlun/issues/1055)）逐问拍板（五问全出：形态/射程/时序/签收判据/落文档）
**状态**：已采纳。本 ADR 只定防线选型与换防时序；镜像的阈值细节（签收五件事）进 spec 填，镜像本体走图外实施票（codex）。

## 背景

map #1055 已裁「生产扫描成为唯一扫描」：3a 把生产两路（`extract_signals_with_hist_anchored` signal.rs:1644 与 `cand_event::observations_for_level` observe.rs:270）收成一趟扫描；P1 对照臂（`level_cand_delta` recursive_tower.rs:2142 独立装配线 + `strict_nest_check` P1 硬门逐位对拍）随 3b 退役。历史考古（[#1056](https://github.com/xy7365527-lang/NewChanlun/issues/1056)，报告 `chanlun/review-results/scan-unify-history-archaeology-20260818.md`）坐实两点：

1. **P1 独有射程 = 装配层 drift**（两条手工镜像装配线改不同步，prelude 漏改类）；#668/#670 判据/覆盖事故当年由影子评审 + 真值表 bin + 300k 窗完备性实测抓出，**均非 P1**。3a 合一线后「两条线」结构本身消失 ⟹ 该类错结构性消灭。
2. **生产准入 nest 不消费 CandDeltaEvent**（gate-on 走 `build_nest_certificate` econ_positive.rs:997 与 `nest_index::build_nest_certificate_index` nest_index.rs:262，吃 `NestCandidateEvent`）⟹ P1 退役的生产面风险 ≈ 0；CandDeltaEvent 面消费点（穷举检索式 `grep -rn "assemble_certificates" rust/src --include="*.rs"`，已查到的有 8 处）= strict_nest_check、p123、p116、p92、p107、p124、cp_capability_smoke 各 bin + opsem_dump sidecar（backtest 库，env `OPSEM_DUMP_DIR` 门控未设=no-op，opsem_dump.rs:419/:496）。

## 裁定

### 一、防线形态 = Lean 完整语义镜像（四选一中的 C，定档 a）

- **镜子照谁**：3a 统一扫描的装配语义——prelude（排序守卫/方向锚/趋势门解析/first_match_idx/A 段缓存/λ_C episode 定界）+ 事件装配 + c_p 生命周期附着（考古报告第一节所列面）。
- **实施交 codex，图外实施票**（本图为纯决策图，两段式口径）；镜像 spec 在 #1057 收口后出。
- **不越界论证**：本图 Out of scope 禁的是「判据/教义**口径变更**」；镜像钉的是**既有**语义、不改口径，与边界不冲突。
- **未选项**：真值表/合成电池（A）与朴素重算对拍（B）不选为独立防线——A 只覆盖抽样点、B 只覆盖缓存层，均不接管「装配语义逐位一致」这条 P1 原不变量；「接受残余风险」（D）不成立为独立答案（见裁定四的明写余量）。

### 二、签收判据 = Rust↔Lean 提取对拍 + 试点定理

- 统一扫描装配输出 vs 镜像判定，**指定窗 0 mismatch** + **试点定理无 `sorry`**；缺任一项不算签收（无对拍的镜子照不到 Rust，防线名存实亡）。
- 对拍桥有先例可循：`formal/Origin/` 的 EngineBridge / LedgerBridge / BspEventBridge。
- 阈值五件事（窗数及不重叠证明 / 每窗各自过线 vs 合计 / 覆盖率 / 分层 / 点估计或置信下界及方法）**进 spec 填**，本 ADR 不代填。
- 镜像为**对拍锁/形式化正本，不参与生产判定**（同 #799 收敛通则：不得成为同一判断的第二档；对照臂退役后它是测试锁不是第二条装配线）。

### 三、切换时序：签收在先，退役在后（无空窗期）

- 3b 的关票门 = **镜像对拍过签收线**；签收未过，P1 硬门与对照装配线不退役。
- 依赖链：**#1057（3a 统一提取记录形状）→ 镜像 spec → 3b**。镜像靶子是 3a 后的统一扫描，3a 形状未定镜像无从起。

### 四、残余风险声明

- 镜像覆盖之外（抽样面之外的语义、未来新增判据面）由影子评审 + 升档条款兜底；此残余风险**接受并明写**，不作为独立防线。
- P1 退役后若镜像签收被后续裁定推翻或范围扩张，防线的再评估走新票，不回写本 ADR。

## 受影响代码清单

- `rust/src/bin/strict_nest_check.rs` P1 硬门（P1Checker :423，checkpoint 全比对 :419-660 一带）——3b 退役对象；
- CandDeltaEvent 消费面 8 处：strict_nest_check / p123_fast_replay / p116_turnpoint_anchor_existence / p92_nest_replay_postruling / p107_level_funnel_audit / p124_merge / cp_capability_smoke 各 bin + opsem_dump sidecar（:224）；
- `rust/src/theta_v0/classifier/tower_cache.rs` 前沿缓存锁步（cached_candidate_observations :91 / cached_first_third* :123-132）——3a 合锁对象（#1057 裁定面）；
- `rust/src/theta_v0/classifier/nest.rs` 对照侧装配 assemble_certificates*（:1296/:1309/:1336）；
- 3b 实施票（图外，随 spec 出图时开，关票门挂本 ADR 裁定二签收线）。

## 关联

- 图：[#1055](https://github.com/xy7365527-lang/NewChanlun/issues/1055)；前置：[#1056](https://github.com/xy7365527-lang/NewChanlun/issues/1056) 考古；上游：[#1057](https://github.com/xy7365527-lang/NewChanlun/issues/1057) 3a；下游：[#1061](https://github.com/xy7365527-lang/NewChanlun/issues/1061) ADR-0005 修订（对照臂退役理由由本 ADR 供料）。

## 补充（[#1060](https://github.com/xy7365527-lang/NewChanlun/issues/1060)，2026-08-18）：strict_nest_check 名分订正

- **受影响代码清单第 1 条订正**：`rust/src/bin/strict_nest_check.rs` **不退役**——3b 转型为**镜像对拍执行器**：P1 硬门位换成 Rust↔Lean 提取对拍门；「每 checkpoint + 末根终态、逐级全比对、FAIL 停线」硬门机器原样保留，只换对拍对象（P1 线 → Lean 镜像）。切换时点 = 3b（本 ADR 裁定三签收门之后），无空窗期。
- **P2 面校验去向**：并入镜像对拍面（c_p 附着即本 ADR 裁定一镜像靶子的组成部分），不另立 P2 专项校验（同物两查撞 #799 收敛通则）；P2 证据载体 = 统一记录 + cp_ownership 投影（#1059 已裁）。
- strict_nest_check 的逐 bar 因果诊断（非门）保留（诊断模式 ≠ 生产模式，#799 限定词二），转镜像锚定，细节随图外 spec。
