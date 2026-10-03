# Stage26 独立审查待审包

状态 pending，`semantic=not_reviewed`。作者自检及机器回放已完成，但不替代独立审查。

盲读工位仅收冻结的模型源 DirectQuoteSpec、MovingQuoteSpec、MovingQuoteWireSpec、LocatedSpec、原生消息契约、[运行实现](native_quote_blocks.rs)和[源码指纹](source-sha256-stage26.json)，不接作者对话或结论。比较工位另收[开工卡](Stage26WorkCard.md)、[报告](Stage26NativeQuote.md)、[机器摘要](stage26-evidence.json)及两份验证脚本。此前 Lean 根只按各自原声明使用，不能由此假定真实消息精化已经成立。

重点审查：净消息变化是否恰对应模型动作；匿名片段是否保留合法删除见证；no-op 是否遗漏相关变化；模型域外更新有没有跨接；原生发生、捕获、应用和确认时刻有没有混淆；补应用有没有回填过去；确认端点有没有错误取成反号后的报价；无报价是否被当成未知，或恢复后回填；来源账与 pending 恢复是否一致；快照重申、前移与断连是否保留未完成尾部；校验摘要是否被夸大为防恶意篡改认证；有限样本有没有被扩成一般定理或完整 Move。

当前证据无需重新跑矩阵即可核摘要与归档。所有原始日志、检查点及展开输出均留仓外，不向 Git 或评论粘贴订单身份。归档 SHA 与解压 SHA 见各 `rows-archive.json`；独立检查器只接受展开 rows。若要对原基线重查，解压到新目录后执行：

```bash
evidence_dir=$(mktemp -d /Users/silencehan/Documents/Codex/research-evidence/issue1467/native-quote-inspect.XXXXXX)
gzip -dc /Users/silencehan/Documents/Codex/research-evidence/issue1467/native-quote-v1/baseline/rows.jsonl.gz > "$evidence_dir/rows.jsonl"
node .chanlun/review-results/issue1467-f1-proof/stage26_inspect.mjs \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson \
  "$evidence_dir/rows.jsonl" \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/native-quote-v1/baseline/report.json \
  "$evidence_dir/inspection.json"
```

需要完整复现时，在工作树根执行以下命令，使用新 evidence_dir，避免覆盖既有证据。构建目录位于仓外。检查点绑定输入和可执行文件；不同构建的字节哈希可能不同，应重建该轮基线，不强行复用旧检查点。

```bash
cargo build --offline --locked \
  --manifest-path .chanlun/review-results/issue1467-f1-proof/stage26-native/Cargo.toml \
  --target-dir /tmp/nc1467-cargo-target
evidence_dir=$(mktemp -d /Users/silencehan/Documents/Codex/research-evidence/issue1467/native-quote-reproduce.XXXXXX)
node .chanlun/review-results/issue1467-f1-proof/stage26_run.mjs \
  /tmp/nc1467-cargo-target/debug/order-event-native-quote-blocks \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson \
  "$evidence_dir/baseline" 339,340,10000
node .chanlun/review-results/issue1467-f1-proof/stage26_inspect.mjs \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson \
  "$evidence_dir/baseline/rows.jsonl" "$evidence_dir/baseline/report.json" \
  "$evidence_dir/baseline-inspection.json"
node .chanlun/review-results/issue1467-f1-proof/stage26_verify.mjs \
  /tmp/nc1467-cargo-target/debug/order-event-native-quote-blocks \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-v1/gap-recovery-1/delayed-across-checkpoint.ndjson \
  "$evidence_dir" "$evidence_dir/verification-1"
```

如果只发现一个反例或一个恢复案例失败，保留已有收据，另建该案例目录修复和复验，不从头反复重跑。源码或模型定义更改后须重新判断哪些旧证据仍可用，不能继承整批通过状态。
