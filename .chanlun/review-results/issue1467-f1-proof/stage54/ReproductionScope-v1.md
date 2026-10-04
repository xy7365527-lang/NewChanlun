# Stage54 复现范围 v1

2026-10-05，#1467。根目录 R 为 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun`；仓外证据 E 为 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage54`。

## 初始审计的冻结输入

原始清单 `initial-audit-evidence/source-and-contract-lock.json` 包含可变 `ResearchProgress.md`。冻结后，主控插入 `stage54-current` 临时段，导致原样 live 校验失败。不得修改旧锁、旧校验器或覆盖实时 Progress 来隐藏该失败。

历史字节保存在 `E/initial-input-snapshots/ResearchProgress.pre-stage54.md`：68340字节，SHA-256 `7bf6386ef17ff4c273e1af3cad33ce2b4ab71d3de2e9ae6bce08e534660b9405`。恢复过程只移除本轮临时段，保留他人已有6行；`recovery-receipt.json` 记录恢复。此快照留仓外，不把他人的未提交修改带进研究提交。

审查者在 `E/initial-review/locked-packet` 复制固定的47件输入，以该历史快照补齐唯一漂移项，并执行未改作者校验器。复验结果见 `E/initial-review/locked-snapshot-receipt.json`，交付绑定见 `delivery-receipt.json`。原始失败及 `live-author-recheck-receipt.json` 仍保存。完整逐行/摘要检查通过的范围为5作者件、41来源合同、3先行件、228摘录行；各分组有重复引用，不能相加为独立文件数。

复现应使用这份锁定副本及其回执。该本机路径不是公共下载地址；只有研究分支的读者尚不具备该历史 Progress 字节，不能宣称可从当前仓根无条件复现初始审计。新 Stage54 清单绑定快照和回执的身份，但不将可变 Progress 全文件加入新冻结清单。

## 其余验证

- D54 的独评命令与原始12输出比较见 `independent-exit-release-review.md`，仓外证据为 `E/candidate-review/`。无需为本次文字整合再次跑不变枚举。
- 成员来源原包的独评在 `E/source-review/`；修订复核在 `E/direction-review/`。必须采用修订后的v2清单及对应审查，旧包的原义映射不获单独通过。
- 本轮整合只核所绑定文件的摘要、链接和采用范围，不将这种核验称作新的独立数学审查。后续汇总范围审查另行记录。
- `formal/` 未改，未运行 fixture 漂移或全仓测试；没有新市场数据、训练、收益回测或生产验证。
