# Stage23 独立审查待审包

状态pending，`semantic=not_reviewed`。当前父模型切换限制未解除，未绕过运行时限制派发。

盲读工位先收[模型声明](MovingQuoteSpec.lean)、[构造声明](MovingQuoteWireSpec.lean)和实际导入定义，复述有效域、Step、Flow、分块、读出和七项根目标。比较工位另读[开工卡](Stage23WorkCard.md)、[证明](MovingQuoteProof.lean)、[复用记录](stage23-proof-reuse.json)、[机器摘要](stage23-lean-verification-summary.json)与[源指纹](source-sha256-stage23.json)。

重点攻击：价格单位列表及连续删除见证是否被误说成完整MBO/FIFO；正负压力是否被偷换为成交方向；报价单调性是否真实由更新/极值推出；端点是否使用同一个边界状态；分块是否保留price等全部模型字段；整体长度拒绝与块内无几何是否被混淆；整侧耗尽后有无补0或回填；构造接口是否被误说成验证全部来源；完成边界与真实捕获时钟的缺口是否保留；是否把局部Origin核心当成完整F₂。

从工作树根目录使用新证据目录复现：

```bash
LEAN_PATH="$PWD/formal/.lake/build/lib/lean:/Users/silencehan/Documents/Codex/research-evidence/issue1467/direct-quote-verified-v1/lean-verification-runs/6fb6a2f71c86428997a7be2a9148863e/lib" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/moving-quote-project-v1 --target-file MovingQuoteProof.lean --declaration MovingQuote.exact_root --expected-type 'MovingQuote.RUN_TARGET ∧ MovingQuote.STEP_TARGET ∧ MovingQuote.LEG_TARGET ∧ MovingQuote.CAUSAL_TARGET ∧ MovingQuote.WITNESS_TARGET ∧ MovingQuote.OUTPUT_TARGET ∧ MovingQuote.WIRE_WITNESS_TARGET' --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake --direct --build-timeout 60 --strict-exit --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/moving-quote-reproduction-1
```

读取现有证据时用相同LEAN_PATH运行`lean_evidence.py --manifest /Users/silencehan/Documents/Codex/research-evidence/issue1467/moving-quote-v1-checked/run-manifest.json`。复用证明已在新事件类型上重新编译，不能只凭文本相同继承旧语义结论。
