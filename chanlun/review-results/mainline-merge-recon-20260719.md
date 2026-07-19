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

## 4. 收编条件（用户拍板项）
- 门全绿后：main fast-forward 不可行（双向分叉），收编 = main merge scratch 分支（或直接 merge kimi 分支复现同解）
- 附带清理项：`rust/.cargo/config.toml` 的 rustc-wrapper 仍指 /tmp/kimi-nest-mainline/.chanlun/locks/cargo_gate.sh（透传）——收编时建议改指仓内相对路径或退役
