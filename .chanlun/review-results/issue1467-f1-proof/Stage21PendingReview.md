# Stage21 独立审查待审包

状态pending，`semantic=not_reviewed`。父模型切换限制未解除；未绕过运行时限制派发，作者检查不替代独评。

盲读工位先收[当前声明](RetilePacketSpec.lean)和实际导入定义，复述输入、角色、时间与四组目标。比较工位再读[开工卡](Stage21WorkCard.md)、[证明](RetilePacketProof.lean)、[机器摘要](stage21-lean-verification-summary.json)及[源指纹](source-sha256-stage21.json)。

重点攻击：是否真的与原Retile Cell序列逐项相同；源恢复和Cover是否都检查；失败块及非空余段是否保留；角色是否只由Local决定，是否被误称完整走势；余段active是否与未确认DC单元混淆；范围是否直接来自实际成员；时间是否仅保证下界而非真实首次触发；五成员例是否被错误用于证明九段触发；一般记录域与真实合法历史域是否分开。

v1快照包含错误导入别名；有效锁为v1a，二者差异只有删掉无效别名。不要用旧锁检查当前源后把预期不匹配当作未记录修改。

从工作树根目录，用新证据目录复现：

```bash
LEAN_PATH="$PWD/formal/.lake/build/lib/lean:/Users/silencehan/Documents/Codex/research-evidence/issue1467/lift-boundary-verified-v1/lean-verification-runs/70afcb63cad748cebe219c90e28e4091/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/compose-audit-v1-checked/lean-verification-runs/e0fff3aa5da843c3a33719390e0b6d82/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/compose-minimal-v1/lean-verification-runs/6ba44fc8a81949c486cb4c8aab1743a5/lib:/Users/silencehan/Documents/Codex/research-evidence/issue1467/source-hull-v1-checked/lean-verification-runs/b6e0219a0b234ea6a7556148fdedee02/lib" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/retile-packet-project-v1a --target-file RetilePacketProof.lean --declaration RetilePacket.exact_root --expected-type 'RetilePacket.FIDELITY_TARGET ∧ RetilePacket.PACKET_TARGET ∧ RetilePacket.ROLE_TARGET ∧ RetilePacket.WITNESS_TARGET' --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake --direct --build-timeout 60 --strict-exit --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/retile-packet-reproduction-1
```

读取现有证据时用相同LEAN_PATH执行`lean_evidence.py --manifest /Users/silencehan/Documents/Codex/research-evidence/issue1467/retile-packet-v1a-checked/run-manifest.json`。声明、依赖或环境改变后不得复用旧通过状态。
