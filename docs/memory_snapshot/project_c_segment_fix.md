---
name: c-segment-fix-DONE
description: 引擎C段边界修复——已完成并验证（2026-06-11）。修复A level.rs:217-275 + B2 divergence.rs:405-417已提交（commit 5739ec4f/0359f0dc/36c6a033）。cargo test 74/0；447K ignored守卫显式跑过：L2 type1/type2=374/360, L4=8/8。唯一残留：BZ slow测试jetsam×2未跑完（替代覆盖已声明）。下游解锁：tranche entry 3→4、REV配对重做、全量回测重跑
metadata: 
  node_type: memory
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## 根因
level.rs:258（Python: a_zhongshu_level.py:240）递归层move的seg_end ≡ last_zs.comp_end。
离开中枢后到转折前的C段被截断为空→趋势背驰永不触发→高级别type1/type2=0→tranche空定义域。
缠师24课C段终于转折点（走势完成处），代码截断在move工程边界——偏离原文。

## 修复方案B2
C段越界到下一中枢内的趋势极值段。具体：
- buysellpoint层的divergence_segments()或c_segment计算中，c_end不截止于comp_end
- 而是延伸到：走势的价格极值段（趋势方向最高/最低价的那个段）
- OKLO模拟：L2 type1从1暴增到373 confirmed，type2从0到360 confirmed

## 精确代码位置
### Rust
- rust/src/buysellpoint.rs：找divergence_segments或c_start/c_end计算
- rust/src/moves.rs：move的seg_end定义可能需要扩展
- rust/src/orchestrator.rs：递归层的走势构造

### Python（同步改保持oracle一致）
- src/newchan/a_buysellpoint_v1.py：_detect_type1里的c_segments/c_start/c_end
- src/newchan/a_zhongshu_level.py:240：seg_end定义

## 验证
- 笔/线段/中枢层输出必须完全不变
- L2 type1 > 100（修复前=1）
- L4 type1 > 0（修复前=0）
- type2 > 0（修复前=0）
- cargo test + 回测在Rust闭环里跑，不走Python链路

## 诊断报告
analysis/engine_bsp_gap_diagnosis.md（由引擎BSP缺口诊断任务产出）

**Why:** 这是"吃到每一笔"北极星的最大引擎层阻塞
**How to apply:** 新session第一优先级——直接改代码+验证

Related: [[session-organic-fugue]], [[recursive-regularization]]


## Wave1下游重验（2026-06-11本session，两工蜂并行，质询通过）
- **tranche**：旧"空定义域定理"证伪。OKLO buy1_ladders={2,3,4}，entry 3→4，(2,3]=1层非空（L4 buy=5/sell=3，与447K守卫side-split逐位吻合）。报告analysis/tranche_recheck_post_csegfix.md。仅证结构非空，n_rev_tranche_adds触发率待全量回测
- **REV配对**：9e0504c6实为完整矩阵且磁带=最终磁带；HEAD复现守卫通过（tape_fp逐位同）。V2o唯一跨标的稳健（ΣΔ+591.6pp，胜率45→52%）；V1f旧+607.5pp是C段bug伪影（新仅+57.6）；BRN最优翻转为V2p（逃逸型2/3否证）。θ_depth不需随type1重标定（度量中枢带宽，与触发位是不同几何对象），但θ=1%量级失配需ATR相对化扫描。报告analysis/rev_pairing_post_csegfix.md
- 两任务零引擎源码改动，未commit

## 下一波待办
- tranche全量回测（n_rev_tranche_adds实际触发率，M2/D3依赖）
- θ_depth ATR相对化扫描（预注册）
- 高层type1锚×REV闭腿（C段修复新打开的工位）
- 谱系补录：532子结算（tranche定义域兑现）+"C段越界极值定义"结算编号
- Git commit工作区遗留（analysis改动+本波两份报告）
