# 异质诊断：中断点快照 stale 修复（codex diagnose 模式 / 独立看代码，不依赖工位自陈）

## 结论

写侧根因被真正抓到，修复充分且无新坐标系分叉。读侧无需改代码（本来就正确）。

## 1. 四候选根因逐一核实

| 候选 | 判定 | 依据 |
|------|------|------|
| (a) 写侧取缓存 HEAD 非实时 | **证伪** | `interrupt_materializer.py:144` 与 `ceremony_scan.py:70` 都是实时 `git rev-parse HEAD` subprocess，无缓存 |
| (b) hook 未每次触发 materialize | **成立（真根因）** | `materialize()` 是正确的实时机械生成器，但此前只挂在 `ceremony_scan --materialize-interrupt` 下，无 hook 触发。三个写侧调用方（PreCompact / Stop 两处 / ceremony_push）都经 `write_session.sh`，却只把 `.interrupt-point.md` 当只读输入复制进 session，从不重写 → 永久停在首次写入的 HEAD |
| (c) 读侧信任叙事字段不重算 | **证伪** | `goal_reducer.reduce_goal` 是纯函数从 events+git 重算；`ceremony_scan` 实时取 HEAD 喂 reducer。读侧从不信任快照（注释明示"base_head 不匹配即降级，reducer 重算为准"） |
| (d) scan 与 reducer 双路径分歧 | **证伪** | scan 唯一路径 `_load_current_goal → reduce_goal`，无第二条算工位路径 |

## 2. 新坐标系分叉核查（644b）

无分叉。写侧（`interrupt_materializer.py:144`）与读侧（`ceremony_scan.py:70`）用**同一命令** `git rev-parse HEAD`。
- 写侧 cwd=repo root，读侧 cwd=`.chanlun/goals/`，但 `git rev-parse HEAD` 从同仓库任意子目录结果相同（git 自动向上找 .git）→ HEAD 口径一致。
- `base_head_stale: true` 在重新 materialize 后仍为 true，**不是 bug**：`base_head`=goal 创建时 HEAD（`fab1f08`，真实历史 commit），`git_head`=当前 HEAD（`b29d3d4`）。stale 正确反映"goal 创建后又推进了 commit"——这是 spec §9 设计意图（降级为参考），重新 materialize 只刷新 git_head，不应抹掉 stale 信号。

## 3. 写侧/读侧边界冲突核查

无竞态。写侧只改 `write_session.sh`（+1 行调用 + 注释）；读侧 working tree 无任何改动（`ceremony_scan.py`/`goal_reducer.py`/`interrupt_materializer.py` 均未改）。两侧不碰同一文件。

## 4. 过度工程核查（ponytail）

最小正确修复，无过度工程。修复 = 在三个写侧调用方的汇聚点（`write_session.sh`）插一行无条件 `python scripts/interrupt_materializer.py`，单点拦截覆盖 Stop/compact/push 三条路径，不在每个 caller 各加。符合"共享函数单点 guard"。

## 5. 验证

- materialize 幂等：连跑两次 git_head 一致（b29d3d4）✓
- 三写侧调用方均经 write_session.sh：PreCompact(precompact-save.sh:54)、Stop(ceremony-completion-guard.sh:101/734)、push(ceremony_push_and_rescan.sh:25)✓

## 边界条件（结论翻转条件）

1. 若某条新的中断点写入路径**绕过** `write_session.sh` 直接写 `.interrupt-point.md` → 该路径会重新引入 stale，需同样补 materialize。当前三条路径已全覆盖。
2. 若 `interrupt_materializer.py` 调用失败（`|| true` 吞错），session 会拿到上次的旧 projection——失败被静默。当前为"尽力刷新不阻断 session"的设计权衡，可接受；若要求强一致需去掉 `|| true` 并显式报错。

## 影响声明

诊断对象：`scripts/write_session.sh`（写侧，已改）、`scripts/interrupt_materializer.py` / `goal_reducer.py` / `ceremony_scan.py`（读侧，未改）。诊断本身不改任何文件。data.rs 改动与本机制无关（theta_v0 回测数据层）。
