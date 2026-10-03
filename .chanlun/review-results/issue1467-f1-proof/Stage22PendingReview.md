# Stage22 独立审查待审包

状态pending，`semantic=not_reviewed`。当前父模型切换限制仍在，未绕过工具限制派发。

盲读材料：[当前声明](CenterExitSpec.lean)、实际Frame.pair、Retile.Local、Origin构造器与Formal.CenterTrichotomy定义。比较工位另收[开工卡](Stage22WorkCard.md)、[证明](CenterExitProof.lean)、[机器摘要](stage22-lean-verification-summary.json)及[源绑定](source-sha256-stage22.json)。

重点审查：成功首对是否真实来自共同可见前缀；退出关系是否只是所声明的字段契约而非全F₂；相对偏移是否与观察索引混淆；未来rest的可见性是否仍需调用者证明；两条Whole与正量见证是否完整；中心数据是否真实派生；扩张是否验证核心分离和原谓词，而非只有else；NoEarlyChoice是否被越权扩大到统计不可预测或全部完成规则；局部关系标签是否被偷换成高级别完整中枢。

源码v1a只把无效namespace别名展开成全限定名称；v1快照保留，当前锁为v1a。语法依据为本机Lean4.31 `Lean/Parser/Command.lean:317–318`，不是一次数学反驳。

从工作树根目录使用新的输出目录复现：

```bash
LEAN_PATH="$PWD/formal/.lake/build/lib/lean:/Users/silencehan/Documents/Codex/research-evidence/issue1467/lift-boundary-verified-v1/lean-verification-runs/70afcb63cad748cebe219c90e28e4091/lib" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/center-exit-project-v1a --target-file CenterExitProof.lean --declaration CenterExit.exact_root --expected-type 'CenterExit.RELEASE_TARGET ∧ CenterExit.FORK_TARGET ∧ CenterExit.NO_EARLY_CHOICE_TARGET' --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake --direct --build-timeout 60 --strict-exit --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/center-exit-reproduction-1
```

读取现有证据时，以相同LEAN_PATH运行`lean_evidence.py --manifest /Users/silencehan/Documents/Codex/research-evidence/issue1467/center-exit-v1a-checked/run-manifest.json`。任何定义或环境变化都需重新检查，不能沿用旧通过状态。
