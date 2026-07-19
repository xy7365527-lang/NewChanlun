# 现场台账（scene ledger）——跨 session/harness 恢复唯一入口

**来历**：harness-engineering improve-harness playbook 实装（2026-07-19）。基线证据：kimi session 恢复现场花 ~15 次工具调用做 wire 考古 + 编排者中转；同日两个 harness（kimi/CC）各挂归并脚本并发撞车（p124_merge 双写同一 dump）。
**规则**：任何改变活体状态的动作（进程起/退、监视器挂/杀、待裁定挂起/解除）后**立即改写本文件**；任何 session 恢复时**先读本文件，再考虑 wire 考古**。全 harness 共用（kimi/CC 一视同仁）。
**核验**：末行 `last_verified` 时间戳 + 核验者；内容必须与现实一致（090：声明=能力）。

## 活体进程

| 进程 | PID | 状态 | 喂给哪关 |
|---|---|---|---|
| p124_merge（S=8 归并，来自 merge_fix 链） | 72171 | 在跑（3:34 起，~99% CPU） | 关② 终验对账 |

## 监视器/自动脚本

| 脚本 | PID | 状态 | 说明 |
|---|---|---|---|
| p124_s2_auto_merge.sh | 57506 | **已杀**（03:36，kimi） | 使命完成（shard2 已退），杀前已点火 merge_and_check |
| p124_s2_watch.sh | 52152 | 已杀 | 同上 |
| p124_r2_merge_fix.sh | 67200 | **已杀**（03:37，kimi） | CC session 链式脚本；其 merge(72171) 保留——两 merge 撞车，保留方=72171（详见注意事项） |
| merge_and_check.sh | 68712 | 已退出（MERGE_EXIT=143） | 其 merge(68763) 被误杀；§3–7 段以缺 dump 状态跑完，**输出作废** |
| kimi 监视任务 bash-9kca542r | 75602 | 在跑 | 等 72171 退出后自动通知 |

## 待办链（按依赖序）

1. **72171 完成 → 重跑对账 §3–7**（禁重跑 merge 本体——dump 已在；手动跑：门行/产量 vs v3/dump 行数/CERT 行数/bit-exact diff，输出落 /tmp/p124_r2_check_r3.out）
2. → 填 /tmp/p124_r2_acceptance_template.md 的 ⟨PENDING⟩ 位 → **关② 收口**（验收线：terminal_confirmed ∈ 硬界 [748,1,213]、Pan=748 精确、877=110/122/31 精确，p127 §4.3-1）
3. → §5.2 η 列口径拍板 **（待编排者回 (i)/(ii)，材料 p128）**
4. → bash /tmp/stage3_execute.sh all（signal → witness → kappa → reach → rdecomp → m8；m8 须 §5.2 已裁）
5. → wave-1 实装（方案 1+2+3 同批，②收口后动 lib，验证 250k/1M diff=0 + P123_SHADOW=1）

## 待编排者裁定（挂起中）

- **§5.2 m8_e2e η 列口径**：(i) 改打 η_corrected 并列原值（倾向，additive）/ (ii) 未修正列+桥列并置+文注。材料：chanlun/review-results/p128-m8-eta-column-ruling-material-20260719.md。

## 注意事项

- **merge 撞车实录**：68763（merge_and_check 之子）被 kimi 误杀（读错进程树）；72171（merge_fix 之子）保留。两 merge 命令逐字相同、确定性输出 ⟹ 无语义损失，但对账 §3–7 必须重跑。
- shard2 最终产量：terminal_confirmed=635、covered_b=1049、missed=0（/tmp/p124_r2_s2.out 末行）。
- 主仓 /Users/silencehan/Projects/NewChanlun 绝对禁写；全部产物落 worktree。
- 双账号：kimi-old provider 已配（config.toml），配额见底 /model 切 kimi-old/k3。

## 参考仓库（耐久路径）

- **harness-engineering**（lopopolo）：`~/Projects/harness-engineering`（git clone，2026-07-19 装）。用途 = 改进本 harness 时的路由文集；入口读其 `AGENTS.md`（应用路由），改进单个作业用 `playbooks/improve-harness.md`（baseline→最早断点→最小属主干预→原生验证→全新重跑→留/改/删），仓库级评审用 `playbooks/repository-review.md`。本台账即按该 playbook 实装的第一个干预。

last_verified: 2026-07-19 03:45 EDT by kimi（72171 在跑 3.0M/4.6M 核对无误；harness-engineering 已装耐久路径）
