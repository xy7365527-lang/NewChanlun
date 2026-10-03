# Stage25 独立审查待审包

状态pending，`semantic=not_reviewed`。父模型切换限制仍在，作者检查不替代独评。

盲读：[LocatedSpec.lean](LocatedSpec.lean)、原MovingQuote/DirectQuote的Runs、locate、Completed。比较工位另收[开工卡](Stage25WorkCard.md)、[证明](LocatedProof.lean)、[机器摘要](stage25-lean-verification-summary.json)与[源指纹](source-sha256-stage25.json)。

重点攻击：Span是否独立于程序；位置覆盖是否遗漏事件；MATCH反向是否真正覆盖任意独立Completed；EOF是否被偷偷当作反号；一般因果结论是否允许不同后缀/长度；是否只证明边界却声称所有对象字段精化；模型事件计数是否被混成捕获时间；符号块确认是否被偷换为几何或完整走势完成。

工作树根目录，用新输出目录复现：

```bash
LEAN_PATH="$PWD/formal/.lake/build/lib/lean:/Users/silencehan/Documents/Codex/research-evidence/issue1467/direct-quote-verified-v1/lean-verification-runs/6fb6a2f71c86428997a7be2a9148863e/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/moving-quote-v1-checked/lean-verification-runs/dab78e23087045249ac84871dd73c164/lib" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/located-project-v1 --target-file LocatedProof.lean --declaration LocatedProof.exact_root --expected-type 'LocatedProof.PROGRAM_TARGET ∧ LocatedProof.MATCH_TARGET ∧ LocatedProof.CAUSAL_TARGET ∧ LocatedProof.WITNESS_TARGET' --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake --direct --build-timeout 60 --strict-exit --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/located-reproduction-1
```

读取现有证据时使用相同LEAN_PATH运行`lean_evidence.py --manifest /Users/silencehan/Documents/Codex/research-evidence/issue1467/located-v1-check2/run-manifest.json`。源、依赖或环境改变后不能直接继承通过状态。
