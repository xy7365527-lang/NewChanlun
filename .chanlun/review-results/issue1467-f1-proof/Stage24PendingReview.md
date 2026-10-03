# Stage24 独立审查待审包

状态pending，`semantic=not_reviewed`。父模型切换限制未解除，作者核对不替代独评。

盲读材料：[当前声明](BookQuoteSpec.lean)及原MovingQuote的Step、Update、QuoteOf、construct。比较工位另收[开工卡](Stage24WorkCard.md)、[证明](BookQuoteProof.lean)、[机器摘要](stage24-lean-verification-summary.json)及[源指纹](source-sha256-stage24.json)。

重点审查：计数覆盖的是全部模型价位还是仅最优报价；取消/执行是否证明数量平衡；双方轨迹合法是否被漏写；前缀之外的事件是否参与当前证明；Read.none是否错误包含“暂不可知”；Reads列表是否完整覆盖状态；不同列表见证是否真实可执行；报价相等是否被扩大为订单身份/FIFO、轨迹存在性或真实捕获时钟结论。

从工作树根目录，用新输出目录复现：

```bash
LEAN_PATH="$PWD/formal/.lake/build/lib/lean:/Users/silencehan/Documents/Codex/research-evidence/issue1467/direct-quote-verified-v1/lean-verification-runs/6fb6a2f71c86428997a7be2a9148863e/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/moving-quote-v1-checked/lean-verification-runs/dab78e23087045249ac84871dd73c164/lib" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/book-quote-project-v1 --target-file BookQuoteProof.lean --declaration BookQuote.exact_root --expected-type 'BookQuote.COUNT_TARGET ∧ BookQuote.PREFIX_TARGET ∧ BookQuote.WIRE_TARGET ∧ BookQuote.WITNESS_TARGET' --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake --direct --build-timeout 60 --strict-exit --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/book-quote-reproduction-1
```

只读现有证据时，使用相同LEAN_PATH运行`lean_evidence.py --manifest /Users/silencehan/Documents/Codex/research-evidence/issue1467/book-quote-v1/run-manifest.json`。源、依赖或环境变化后需重新检查。
