# Stage30 独立审查待审包

状态pending，`semantic=not_reviewed`。原文范围分析与机器证明必须分别审查。

盲读工位收 [CoarseClockSpec.lean](CoarseClockSpec.lean)、ClockEdge原定义及相关DC/力度定义。比较工位另收 [开工卡](Stage30WorkCard.md)、[证明](CoarseClockProof.lean)、[报告](Stage30BaseAndClockScope.md)、[声明锁](stage30-declaration-lock.json) 和 [证据摘要](stage30-evidence.json)，不接作者对话。

来源审查核084:52/54/56与081续排:110的归属、bi.md及beichi.md的名分；区分允许替代F₁与承认任意F₁合格，不把“无需复刻笔”误读为“可改原义f₂”。数学审查核兼容域非空、两种细时钟严格性、价格和桶记录是否逐项相同、参考是否固定、NO_SELECTOR量词、PARTIAL的soundness范围及是否越权推广到实际Coinbase协议或统计不可预测。

工作树根目录，用新输出目录复现：

```bash
LEAN_PATH="$PWD/formal/.lake/build/lib/lean:/Users/silencehan/Documents/Codex/research-evidence/issue1467/lift-boundary-verified-v1/lean-verification-runs/70afcb63cad748cebe219c90e28e4091/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/direct-quote-verified-v1/lean-verification-runs/6fb6a2f71c86428997a7be2a9148863e/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/moving-quote-v1-checked/lean-verification-runs/dab78e23087045249ac84871dd73c164/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/clock-edge-v1-check3/lean-verification-runs/14c54f0aafae4193921edef9d9fbf923/lib" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/coarse-clock-project-v1 \
  --target-file CoarseClockProof.lean --declaration CoarseClock.exact_root \
  --expected-type 'CoarseClock.WITNESS_TARGET ∧ CoarseClock.AMBIGUITY_TARGET ∧ CoarseClock.NO_SELECTOR_TARGET ∧ CoarseClock.PARTIAL_TARGET' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/coarse-clock-reproduce-1
```

只复核现有证据时以相同LEAN_PATH运行 `lean_evidence.py --manifest /Users/silencehan/Documents/Codex/research-evidence/issue1467/coarse-clock-v1/run-manifest.json`。源、依赖、模块解析或环境变化后不能直接继承通过。已有未完成独评状态不能被新内核检查抹去。
