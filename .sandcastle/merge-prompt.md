# TASK

把以下分支逐一技术合并进**当前分支**（你所在的是归并分支，不是 main）：

{{BRANCHES}}

对每个分支：
1. `git merge <branch> --no-edit`
2. 有冲突就读两边智能解冲突
3. 解完跑验证：Rust 改动 `cargo check`；TS 改动 `npx tsc -p tsconfig.sandcastle.json`
4. 验证失败先修再合下一条

全部合完后，做一个汇总 commit（message：`merge: <涉及票号列表>——<一句话>`，附编号声明行）。

# 硬性规则（本仓裁定，覆盖官方模板）

- **不合 main、不 push、不关任何 issue**——验收与合入是编排层人工闸（#1003 裁 3）。
- 合不动的分支：跳过并在最终答复里点名原因。

相关 issue 清单：

{{ISSUES}}

完成后输出 <promise>COMPLETE</promise>。
