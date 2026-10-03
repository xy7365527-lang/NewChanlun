# Stage28 独立审查待审包

状态pending，`semantic=not_reviewed`。本轮作者＋机器证据不冒充独评。

盲读材料：[ClockEdgeSpec.lean](ClockEdgeSpec.lean)、DCSpec.Whole/Run/Unit、MovingQuote.leg和实际Stroke.velocity/impulse。比较工位另收 [开工卡](Stage28WorkCard.md)、[证明](ClockEdgeProof.lean)、[统计检查器](stage28_inspect.mjs)、[报告](Stage28ClockAndEdges.md)、[声明锁](stage28-declaration-lock.json) 与 [证据摘要](stage28-evidence.json)，不接作者对话。

重点审查：两时钟是否在声明的有限域严格递增；是否将不同完整带时间历史冒充同一输入；Whole是否真实完整；数值力度反例是否被扩大成已确认背驰；零时长0是否被视作有效速度；一般选边公式是否直接调用原leg；“全程固定”是否核每个模型状态；98.26%的分母是否为构造段绝对变化而非收益/方差；延迟扰动是否改变原生逻辑块；是否擅自选定原子或时钟进入完成判据。

工作树根目录，用新的仓外输出目录复现：

```bash
LEAN_PATH="$PWD/formal/.lake/build/lib/lean:/Users/silencehan/Documents/Codex/research-evidence/issue1467/lift-boundary-verified-v1/lean-verification-runs/70afcb63cad748cebe219c90e28e4091/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/direct-quote-verified-v1/lean-verification-runs/6fb6a2f71c86428997a7be2a9148863e/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/moving-quote-v1-checked/lean-verification-runs/dab78e23087045249ac84871dd73c164/lib" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/clock-edge-project-v1 \
  --target-file ClockEdgeProof.lean --declaration ClockEdge.exact_root \
  --expected-type 'ClockEdge.CLOCK_TARGET ∧ ClockEdge.ZERO_TARGET ∧ ClockEdge.EDGE_TARGET' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/clock-edge-reproduce-1

node .chanlun/review-results/issue1467-f1-proof/stage28_inspect.mjs \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/native-quote-v1 \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/clock-edge-data-reproduce-1
```

数据命令只读取已有压缩行输出，并核对压缩、解压与报告SHA，不重新回放或保存展开行情。复核现有Lean证据时以相同LEAN_PATH运行 `lean_evidence.py --manifest /Users/silencehan/Documents/Codex/research-evidence/issue1467/clock-edge-v1-check3/run-manifest.json`。源、依赖、环境变化后不能继承旧通过；失败记录原样保留。
