# Stage19 独立审查待审包

状态pending，`semantic=not_reviewed`。父模型切换限制未解除，未绕过CUA安全限制派发。作者自查与Lean精确检查分别保留，不登记独评通过。

盲读工位只收[ComposeAuditSpec.lean](ComposeAuditSpec.lean)、[ComposeMinimalSpec.lean](ComposeMinimalSpec.lean)及实际导入定义，先复述量词、输入来源和结果。独立比较工位再对照[开工卡](Stage19WorkCard.md)、[证明](ComposeAuditProof.lean)、[最短性证明](ComposeMinimalProof.lean)、[机器摘要](stage19-lean-verification-summary.json)及[源绑定](source-sha256-stage19.json)。不要把作者结论当作审查结果。

需重点攻击：

- 反例是否真正调用正式MovesComposedFrom、windowCenters、WellFormed与Move.interval？有没有另造弱接口？
- 重复的是完整原单元，还是仅共享端点？所给窗口允许重叠是否被错误扩大为所有其他见证也必然重叠？
- 六条最短性是否限定在至少两个上级对象的该类反例，是否冒充所有失败类型的最小值？
- 旧DC和正量证明是否实际进入根，而不是只有手写几何列表？
- 子中心本身的外缘是否正确？0下界由哪一步引入？下一层WellFormed与正确Origin拒绝为何能并存？
- 是否误伤canonicalWindows或推断未经运行的生产消费者？是否把良构当作完整走势终结证书？

当前源和外部项目已按摘要绑定。重跑用新输出目录，并预留声明提取日志空间。工作树根目录：

```bash
LEAN_PATH="$PWD/formal/.lake/build/lib/lean:/Users/silencehan/Documents/Codex/research-evidence/issue1467/lift-boundary-verified-v1/lean-verification-runs/70afcb63cad748cebe219c90e28e4091/lib" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/compose-audit-project-v1 --target-file ComposeAuditProof.lean --declaration ComposeAudit.exact_root --expected-type 'ComposeAudit.OVERLAP_TARGET ∧ ComposeAudit.RANGE_TARGET' --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake --direct --build-timeout 60 --strict-exit --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/compose-audit-reproduction-1
LEAN_PATH="$PWD/formal/.lake/build/lib/lean:/Users/silencehan/Documents/Codex/research-evidence/issue1467/lift-boundary-verified-v1/lean-verification-runs/70afcb63cad748cebe219c90e28e4091/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/compose-audit-v1-checked/lean-verification-runs/e0fff3aa5da843c3a33719390e0b6d82/lib" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/compose-minimal-project-v1 --target-file ComposeMinimalProof.lean --declaration ComposeMinimal.exact_root --expected-type 'ComposeMinimal.WITNESS_TARGET ∧ ComposeMinimal.LOWER_BOUND_TARGET' --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake --direct --build-timeout 60 --strict-exit --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/compose-minimal-reproduction-1
```

第二命令明确复用已检查主根模块；若改用第一次复现的新模块，须更新LEAN_PATH并重新绑定依赖，不能保留旧哈希冒充同一运行。读取现有证据而不重编译时，使用同一LEAN_PATH运行`lean_evidence.py --manifest`。
