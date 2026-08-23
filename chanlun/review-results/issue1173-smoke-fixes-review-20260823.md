# #1173 烟测期修复快速轴评审（2026-08-23，本体直办）

> 范围：关票面 == 评审面。评审对象 = 烟测排障期直推 main 的四个修复 commit + 一件拾取 commit。
> 背景：2026-08-22 22:48Z 起账户级 Actions 入队故障（全部 runnable job 2s 零步失败，已用最小探针
> workflow 与全新私有仓复现定界），烟测链被阻断；以下修复均在该故障前/后的窗口内落地，且每一件
> 都被故障恢复后的端到端 run 实跑验证过（见各条证据）。

## 评审对象与结论

| commit | 内容 | 结论 |
|---|---|---|
| e11a218e77 | agent-implement/review 两 workflow 的 DeepSeek 路线各加 uv 安装步（kernel bootstrap 前置） | PASS |
| ab76975a60 | review 模型路由改「Detect model route」步运行时实读 PR 标签，五个互斥步改走 steps.route.outputs | PASS |
| 588b5d8be7 | prime-agent-provider findByIdOnHost 加按 header.id 扫描兜底（文件名 ≠ header.id 时 extraction 二段 resume 预检失败） | PASS |
| e999f0006b | agent.ts 在 GITHUB_ACTIONS 环境下覆盖 sandboxSessionsDir = runner HOME（/home/agent 不可写 EACCES） | PASS |
| 2554a62b91 | 评审工蜂的改进拾取：会话目录覆盖复用 provider 宿主常量 + shared.test.ts 新增 28 项覆盖 | PASS |

## 逐条证据

1. **e11a218e77**：无 uv 时 run 32600210208（第 3 次烟测）实测工蜂全部工具调用被 kernel bootstrap 闸挡；
   加步后 run5（32601390368）implement 半链 success 建出 Draft PR #1200，且后续每个 review run 的
   「Install uv」步均 success。
2. **ab76975a60**：修复前两次实测（32601936209/32602496362）pull_request_target label 快照在 if 条件
   里评估异常（快照含模型标签，互斥安装步却都执行、DeepSeek run 步被 skip）；修复后同 run 内
   Claude 步全 skipped、DeepSeek 步执行（32602764461 的步骤表）。最终成功 run 32643220426 复验一致。
3. **588b5d8be7**：修复前 32602764461 在「Run complete → Collecting commits」后抛
   `resumeSession ... not found under /home/runner/.prime/agent/sessions`；修复后最终 run 的 extraction
   二段（Run review agent 内部两段跑）通过并产出评审输出（Post PR review 步 success）。
4. **e999f0006b**：修复前 32641876744（入队恢复后的第一个真实执行 run）抛
   `EACCES: permission denied, mkdir '/home/agent/.prime/agent/sessions'`；修复（含 cherry-pick 至
   PR 分支）后重触发的 32643220426 全绿（Push branch / Post PR review / Mark PR ready 均 success）。
5. **2554a62b91**：拾取评审工蜂 28e2980113 的改进（常量复用 + 测试）；本地实测
   `npm run typecheck` 过、`npx tsx --test .sandcastle/agent-workflows/shared/shared.test.ts` = 28/28 pass。

## 附注

- 四修复中无生产 Rust/Python 代码改动，全部落在 workflow 文件与 .sandcastle/agent-workflows 装配层；
  ci.yml 未动。
- 「workflows: write 权限」修法已实测会破坏 workflow 注册并回滚（47205adf17 记录在案），评审通过项
  均不含该改动。
