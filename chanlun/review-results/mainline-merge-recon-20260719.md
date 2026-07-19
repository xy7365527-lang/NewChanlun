# 主线归并对账（2026-07-19，#117，编排者=claude-fable-5/438f5dc7）

## 1. 态势
- merge-base：fe03fd5c28（task-103）
- main 独有 7 commit：task-106/107/108（区间套原生实装+级别标定+收敛探针）+ 待裁2 nest_isolation_guard（f838b914bc、83c31e86aa）
- 分支独有 1 commit：640609071d 收口提交（52 文件 rust +26633：#105 forward-find、dual_ledger、recognize_nested、A10 守恒、η additive、harness 成套、090 镜像修复、11 关矩阵）

## 2. 冲突面与解法
- 125↔7 触碰文件仅 2 交集：
  - `rust/Cargo.toml`：块1 #101 注释措辞 → 双侧语义合一；块2 add/add → 并集（p107+p108+p105 三 bin 全保留，补齐 p108 required-features）；bin 名无重复
  - `rust/src/theta_v0/classifier/mod.rs`：git 自动合并干净
- 归并落点：scratch 分支 `merge-mainline-20260719`（worktree /tmp/merge-mainline-20260719），merge commit `fcd3214f18`。**main 未动**。
- checkout 需 `GIT_LFS_SKIP_SMUDGE=1`（`.chanlun/block-topology/relations.jsonl` LFS 对象缺失，指针落盘，不入 rust 构建路径）

## 3. 终局门（cargo test --lib，fresh target）
- 判定：**PASS** — `test result: ok. 1736 passed; 0 failed; 128 ignored`（6.20s，0 warning；较 1732 门多出 main 侧 task-106/107/108 新增测）

## 4. 收编（已执行，2026-07-19）
- 已收编：产物提交 `9f2629e6b6`（090 条目1/2 应用+11 关矩阵+harness 台账+worktree 盘点）→ merge `cad69b2ef8`（零冲突）；main 本体终局门二跑 **PASS**（1736/0/128，6.12s）
- scratch worktree /tmp/merge-mainline-20260719 已移除，分支标签 `merge-mainline-20260719` 保留可溯
- 残留登记（未认领、不判弃）：`stash@{0}` = p92_nest_replay_postruling.rs / p93_invalidseed_probe.rs / strict_nest_check.rs 主仓改动（strict_nest 与归并版全等；p92/p93 有分歧，待认领裁定）
- 残留登记（工作树在场）：`p107_level_calib.rs` +51 行未提交增补（修正一 `P107_L0NN` classifier 自检 + 修正二 `P107_MAT` 分级距离矩阵，只读报表），疑似 task-107 会话遗单，待认领后补测入库
- untracked p100/p101 源与归并版逐字节全等，已由归并版收编
- 未入库残留（非本线，留置）：p43-replay-cfull-v2 文档改动、skills-lock.json、.agents/、.kimi-code/、.claude/skills/* 若干
- 附带清理项：`rust/.cargo/config.toml` 的 rustc-wrapper 仍指 /tmp/kimi-nest-mainline/.chanlun/locks/cargo_gate.sh（透传）——收编时建议改指仓内相对路径或退役
