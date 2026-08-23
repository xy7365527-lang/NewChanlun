# Agent Roster — 2026-08-22

| Agent | 类型 | 模型 | 任务 | 关联票 | 状态 |
|---|---|---|---|---|---|
| `sandcastle-1168-implementer` | Sandcastle 实装工蜂 | `deepseek/deepseek-v4-pro` | S2 五态 fail-closed Shadow Ranker 接口契约 | #1168 | 已完成（`3189ab45ff`；宿主验收绿，待 main FF） |
| `sandcastle-1168-reviewer` | Sandcastle 独立评审工蜂 | `deepseek/deepseek-v4-pro` | 复审五态、非 RANKED 无排序与 ADR 0025 对齐 | #1168 | 已完成（`ed39224075`；宿主验收绿，待 main FF） |
| `sandcastle-1169-implementer` | Sandcastle 实装工蜂 | `deepseek/deepseek-v4-pro` | S3 隔离 provider 运行面与凭据绑定 | #1169 | 已完成（`c551eed1f4`；宿主验收绿，待 main FF） |
| `sandcastle-1169-reviewer` | Sandcastle 独立评审工蜂 | `deepseek/deepseek-v4-pro` | 复审空环境、endpoint-key 绑定、出网/挂载边界 | #1169 | 已完成（`75ec32918e`；宿主验收绿，待 main FF） |

## /triage 全量盘点（2026-08-22）

| 类型 | 模型 | 任务 | 状态 |
|---|---|---|---|
| rlm 子代理 | 随会话继承 | triage-a：debt 票 #916 #928 #948 #964 #1095 #1096 调研+推荐（只读，不写 GitHub） | 完成；阶段2：起草 brief 评论 | 完成；标签+评论已由本体贴发（19 标签 + 20 评论，2026-08-22） |
| rlm 子代理 | 随会话继承 | triage-b：task/SPEC 票 #1049 #1100 #1126 #1128 #1160 调研+推荐（只读，不写 GitHub） | 完成；阶段2：起草 brief 评论 | 完成；标签+评论已由本体贴发（19 标签 + 20 评论，2026-08-22） |
| rlm 子代理 | 随会话继承 | triage-c：impl/smoke 票 #1168 #1169 #1170 #1171 #1172 #1173 调研+推荐（只读，不写 GitHub） | 完成；阶段2：起草 brief 评论 | 完成；标签+评论已由本体贴发（19 标签 + 20 评论，2026-08-22） |
| rlm 子代理 | 随会话继承 | triage-d：#1174 map + #1182 bug + #1046 冲突背景 调研+推荐（只读，不写 GitHub） | 完成；阶段2：起草 brief 评论 | 完成；标签+评论已由本体贴发（19 标签 + 20 评论，2026-08-22） |
| `merge-1169-conflict-codex` | Codex CLI 冲突解算工蜂 | `gpt-5.6-sol` | 仅解 rust/lib.rs 的 lav_rank + lav_runtime 并集 | #1169 | 已完成：并集解冲突、宿主暂存 |
| `sandcastle-1170-implementer` | Sandcastle 实装工蜂 | `deepseek/deepseek-v4-pro` | S4 pairwise 相对排序与原子 Run Record | #1170 | 首轮仅探索后异常退出、无 commit |
| `sandcastle-1170-reviewer` | Sandcastle 独立评审工蜂 | `deepseek/deepseek-v4-pro` | 复审原生协议、coverage/usage、失败留痕与 no-failover | #1170 | 未启动：首轮实装无 commit |
| `sandcastle-1170-implementer-retry1` | Sandcastle 实装恢复工蜂 | `deepseek/deepseek-v4-pro` | 收敛实现 S4，禁重复探针 | #1170 | 失败：13 分钟仍只探索，0 commit |
| `sandcastle-1170-reviewer-retry1` | Sandcastle 独立评审工蜂 | `deepseek/deepseek-v4-pro` | 复审 S4 fail-closed/Run Record | #1170 | 未启动 |
| `sandcastle-1170-implementer-retry2` | Sandcastle 实装恢复工蜂 | `openai-codex/gpt-5.6-sol` | S4 直接实现，禁研究/联网 | #1170 | 已完成初版（`12b2c7c7bc`），主进程异常退出 |
| `sandcastle-1170-reviewer-retry2` | Sandcastle 独立评审工蜂 | `openai-codex/gpt-5.6-sol` | 复审 S4 fail-closed/Run Record | #1170 | 未启动：主进程异常 |
| `sandcastle-1170-reviewer-recovery` | Sandcastle 独立评审恢复工蜂 | `openai-codex/gpt-5.6-sol` | 修正初版 Borda/六态/Run Record 偏差 | #1170 | 完成（`743a379f02`） |
| `sandcastle-1171-implementer` | Sandcastle 实装工蜂 | `openai-codex/gpt-5.6-sol` | S5 replay/deletion/budget/redline | #1171 | 失败（递归转派 provider 403/402/abort，0 commit） |
| `sandcastle-1171-reviewer` | Sandcastle 独立评审工蜂 | `openai-codex/gpt-5.6-sol` | S5 ADR 0025/#1149 合规复审 | #1171 | 无效完成（递归转派后未取回 findings） |
| `sandcastle-1171-implementer-direct-retry` | Sandcastle 直接实装恢复工蜂 | `openai-codex/gpt-5.6-sol` | 禁止递归转派，直接实施 S5 | #1171 | 完成（`fdad245a68`） |
| `sandcastle-1171-reviewer-direct-recovery` | Sandcastle 独立评审恢复工蜂 | `openai-codex/gpt-5.6-sol` | 禁止递归转派；只审 S5 精确 diff | #1171 | 未完成（留 4 文件未提交修正） |
| `sandcastle-1171-reviewer-finalize` | Sandcastle 独立评审收尾工蜂 | `openai-codex/gpt-5.6-sol` | 接手修正、完成测试并提交 | #1171 | 完成（`60500a2141`） |
| `sandcastle-1172-implementer` | Sandcastle 直接实装工蜂 | `openai-codex/gpt-5.6-sol` | S6 指标报告与图外 spec 草案 | #1172 | 完成（`b37d60779b`，NO-GO） |
| `sandcastle-1172-reviewer` | Sandcastle 独立评审工蜂 | `openai-codex/gpt-5.6-sol` | 复核 #1162/#1149 计数与门判定 | #1172 | 完成（无修正） |
