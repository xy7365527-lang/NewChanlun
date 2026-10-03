# Stage20 独立审查待审包

状态pending，`semantic=not_reviewed`。当前无法按用户规则切换父模型后派发，继续保留独立审查缺口；作者检查不替代独评。

盲读材料：[SourceHullSpec.lean](SourceHullSpec.lean)及实际导入定义。先复述Cover、ExactHull、三个根目标的量词和隐含假设；比较工位再收[开工卡](Stage20WorkCard.md)、[证明](SourceHullProof.lean)、[机器摘要](stage20-lean-verification-summary.json)与[源绑定](source-sha256-stage20.json)。

重点审查：Cover是否只检查给定分配，是否被误称分解唯一；角色标签是否被偷换为走势完成；源位置是否与端点或只读证明依赖混淆；一般使用是否还需实际列表/版本绑定；ExactHull是否同时要求包含和极值可取得；空列表和任意整数是否正确处理；给定错误子区间时证书不能提供什么；旧六单元与严格单点反例是否确实进入根。NoCore示例是已知标签的记账例，未声明任意Retile到角色的自动语义桥。

工作树根目录复现，用新的输出目录：

```bash
LEAN_PATH="$PWD/formal/.lake/build/lib/lean:/Users/silencehan/Documents/Codex/research-evidence/issue1467/lift-boundary-verified-v1/lean-verification-runs/70afcb63cad748cebe219c90e28e4091/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/compose-audit-v1-checked/lean-verification-runs/e0fff3aa5da843c3a33719390e0b6d82/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/compose-minimal-v1/lean-verification-runs/6ba44fc8a81949c486cb4c8aab1743a5/lib" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/source-hull-project-v1 --target-file SourceHullProof.lean --declaration SourceHull.exact_root --expected-type 'SourceHull.COVER_TARGET ∧ SourceHull.HULL_TARGET ∧ SourceHull.WITNESS_TARGET' --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake --direct --build-timeout 60 --strict-exit --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/source-hull-reproduction-1
```

只读取现有证据时使用相同LEAN_PATH调用`lean_evidence.py --manifest /Users/silencehan/Documents/Codex/research-evidence/issue1467/source-hull-v1-checked/run-manifest.json`。依赖的旧研究模块仍由各自源码/运行指纹限定；改动模块后不得沿用旧通过状态。
