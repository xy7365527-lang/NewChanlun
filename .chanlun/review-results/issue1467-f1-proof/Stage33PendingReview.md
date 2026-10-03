# Stage33 独立审查待审包

状态pending，`semantic=not_reviewed`。

盲读材料：[声明](DirectReachSpec.lean)、[fixture](DirectReachFixture.lean)及原MovingQuote/DirectQuote/SourceTriplet/ContainsSpan定义；比较工位另收[开工卡](Stage33WorkCard.md)、[生成器](stage33_fixture.mjs)、[证明](DirectReachProof.lean)、[报告](Stage33DirectReachability.md)、[锁](stage33-declaration-lock.json)与[证据](stage33-evidence.json)。不接作者对话。

重点检查：每次撤减是否有原Step的实际列表见证；全部中间盘口是否有效；8个Flow是否消费38个原事件；期望段是否逐字段等于实际construct输出；独立Completed是否只用有限前缀；eventAt域外默认值是否被任何目标消费；9事件与1确认块是否混淆；Stage32的条件是否确实按本模型块坐标实例化；是否把模型可达扩大成真实交易所/完整Move或全部F₁结论。

工作树根目录，用新的证据目录复现fixture：

```bash
node .chanlun/review-results/issue1467-f1-proof/stage33_fixture.mjs \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/direct-reach-setup-reproduce-1
```

先核生成的DirectReachFixture.lean与仓内锁定文件SHA相同。生成器只出数据和逐事件证明脚手架，完整的逐块Flow及根证明以仓内DirectReachProof.lean为准。

```bash
export LEAN_PATH="$(node -p 'require("./.chanlun/review-results/issue1467-f1-proof/stage33-evidence.json").LEAN_PATH')"
python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/direct-reach-project-v1 \
  --target-file DirectReachProof.lean --declaration DirectReach.exact_root \
  --expected-type 'DirectReach.TRAJECTORY_TARGET ∧ DirectReach.GROUP_TARGET ∧ DirectReach.GEOMETRY_TARGET ∧ DirectReach.OBSTRUCTION_TARGET' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/direct-reach-reproduce-1
```

只复核现存证据时，以相同LEAN_PATH运行 `lean_evidence.py --manifest /Users/silencehan/Documents/Codex/research-evidence/issue1467/direct-reach-v1-checked/run-manifest.json`。源码、fixture、模块解析或环境变化后不能继承通过。预设价位是合成存在性材料，不能拿作在线预测或确认样本。
