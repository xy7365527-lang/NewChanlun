# Stage27 独立审查待审包

状态 pending，`semantic=not_reviewed`。作者推导与机器证据不能充当独立语义审查。

盲读材料：[ForceTransportSpec.lean](ForceTransportSpec.lean)、实际 `Origin/ForceVelocity.lean` 的 Stroke.WellFormed/velocity/lastStroke/impulse、实际 Stroke 数据类型。比较工位另收 [开工卡](Stage27WorkCard.md)、[证明](ForceTransportProof.lean)、[报告](Stage27ForceTransport.md)、[声明锁](stage27-declaration-lock.json) 和 [证据摘要](stage27-lean-verification-summary.json)。不接作者对话或预设结论。

重点检查：拼接是否混淆共享笔与共享端点；一般定理是否真的覆盖任意分块含空块；摘要是否只保存值却冒充身份；scalar_insufficient是否证明任意只读两个L值的合成函数，而不只是加法失败；ValidChain是否被扩大成原文合法笔/合法背驰比较对；是否把数值传递自动升级为完成或F₂；Rat除零总化是否被当成有效速度；是否保留原子、时钟与端点选择的外部义务。

工作树根目录使用新输出目录复现，旧证据不覆盖：

```bash
LEAN_PATH="$PWD/formal/.lake/build/lib/lean" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/force-transport-project-v1 \
  --target-file ForceTransportProof.lean --declaration ForceTransport.exact_root \
  --expected-type 'ForceTransport.COMPOSITION_TARGET ∧ ForceTransport.SEAM_TARGET ∧ ForceTransport.LOSS_TARGET ∧ ForceTransport.WITNESS_TARGET' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/force-transport-reproduce-1
```

只复核现存证据时，以同一 LEAN_PATH 运行 `lean_evidence.py --manifest /Users/silencehan/Documents/Codex/research-evidence/issue1467/force-transport-v1-check3/run-manifest.json`。冷环境先在 formal 目录执行 `lake build Origin.ForceVelocity`，不是全仓构建。外部项目中的两份源必须与仓内 SHA 一致；源码、模块解析或工具链变化后不能直接继承旧通过。
