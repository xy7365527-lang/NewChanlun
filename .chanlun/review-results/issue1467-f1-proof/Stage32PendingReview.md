# Stage32 独立审查待审包

状态pending，`semantic=not_reviewed`。

盲读工位收[ExitLiftSpec.lean](ExitLiftSpec.lean)、[ExitSupportSpec.lean](ExitSupportSpec.lean)及实际导入的SourceTriplet/Cover/RetilePacket/CenterExit/DC定义。比较工位另收[开工卡](Stage32WorkCard.md)、两份证明、[报告](Stage32ExitLiftReadiness.md)、两份声明锁和[证据摘要](stage32-evidence.json)。不接作者对话或预设结论。

重点攻击：观察索引与确认单元位置是否混同；原前缀和完整DC见证是否真正保留；是否把未确认尾计入来源；至少三成员是否越权套到所有连接或全部F₁；扩大支撑的否定是否保留了全部明示前提；首末中枢归属与三个孩子顺序是否被偷换；正面重分组是否跨开原中枢窗口；candidate与严格几何是否被冒充完整Move；是否证明实际重切触发、完成/同级及新发生身份。

在工作树根使用证据摘要记录的LEAN_PATH运行下列精确检查，输出目录须新建且不覆盖旧证据。加强根依赖已核验的初始根，其LEAN_PATH比初始根多一项 `exit-lift-v1-checked/lean-verification-runs/ce5412601f63429cbeb5cb4d40730cab/lib`。其余上游路径全部保存在摘要中。

```bash
export LEAN_PATH="$(node -p 'require("./.chanlun/review-results/issue1467-f1-proof/stage32-evidence.json").checks.initial.LEAN_PATH')"
python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/exit-lift-project-v1 \
  --target-file ExitLiftProof.lean --declaration ExitLift.exact_root \
  --expected-type 'ExitLift.PREFIX_TARGET ∧ ExitLift.SOURCE_TARGET ∧ ExitLift.FROZEN_TARGET ∧ ExitLift.LATE_TARGET' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/exit-lift-reproduce-1

export LEAN_PATH="$(node -p 'require("./.chanlun/review-results/issue1467-f1-proof/stage32-evidence.json").checks.support.LEAN_PATH')"
python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/exit-support-project-v1 \
  --target-file ExitSupportProof.lean --declaration ExitSupport.exact_root \
  --expected-type 'ExitLift.PREFIX_TARGET ∧ ExitLift.SOURCE_TARGET ∧ ExitLift.FROZEN_TARGET ∧ ExitLift.LATE_TARGET ∧ ExitSupport.NONEMPTY_TARGET ∧ ExitSupport.GROW_TARGET ∧ ExitSupport.SPLIT_TARGET' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/exit-support-reproduce-1
```

初始项目中的NineRegroupSpec.lean是仓内原声明的逐字副本，不是新关系；其SHA收入本轮源表。复核原证据时使用各自LEAN_PATH运行 `lean_evidence.py --manifest <摘要中的manifest路径>`。源、依赖或模块解析改变后不能继承通过。记录的几何对照不是生产实现或已采纳定义。
