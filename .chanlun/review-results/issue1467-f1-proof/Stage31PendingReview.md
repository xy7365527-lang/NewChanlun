# Stage31 独立审查待审包

状态pending，`semantic=not_reviewed`。来源解释与形式目标分别审查。

盲读材料：[TurnAuditSpec.lean](TurnAuditSpec.lean)及实际Origin.Turn/Divergence定义；比较工位另收[开工卡](Stage31WorkCard.md)、[证明](TurnAuditProof.lean)、[报告](Stage31TurnCoverage.md)、[声明锁](stage31-declaration-lock.json)、[证据摘要](stage31-evidence.json)。原文范围仅043/044所引行，另核当前Rust候选载体，不接作者对话。

重点攻击：是否把抽象q无Div/r有Div例子冒称真实小转大发生；是否把原Turn同级载荷的调用约定误说成类型强制；是否用必要条件反推发生；是否将来源Forward必要关系提升为完整让位；是否把值相等当成同一发生实例；是否反过来把value≠value当成充分身份修复；是否将原接口已声明的开口包装成全部旧定理失效。

工作树根目录，用新输出目录复现：

```bash
LEAN_PATH="$PWD/formal/.lake/build/lib/lean" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-audit-project-v1 \
  --target-file TurnAuditProof.lean --declaration TurnAudit.exact_root \
  --expected-type 'TurnAudit.COVERAGE_TARGET ∧ TurnAudit.SELF_TARGET ∧ TurnAudit.IDENTITY_TARGET' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-audit-reproduce-1
```

冷环境先在formal目录执行 `lake build Origin.Turn`。只复核现存证据时以相同LEAN_PATH运行 `lean_evidence.py --manifest /Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-audit-v1/run-manifest.json`。源、工具、模块解析或环境变化后不能继承通过；旧研究的独评待办不被本次机器检查抹去。
