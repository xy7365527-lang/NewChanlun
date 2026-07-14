# 冲突记录：两份独立 Codex 全权终裁在 Q2 上结论相反

- **工位**：ws-codexp2
- **日期**：2026-07-02
- **性质**：非新裁定——记录两次独立 Codex `decide` 调用在同一问题上产生的真实矛盾，供 team-lead/编排者裁断，不由本工位单方面覆盖。

## 事实

同一个任务（#92 "codex-q2-d1"）被两个工位几乎同时（时间戳相差约2分钟）独立处理：

| | 工位 ws-codexq2（先完成，23:54） | 工位 ws-codexp2（本工位，23:56） |
|---|---|---|
| 裁定文件 | `.chanlun/review-results/codex-q2-d1-ruling-20260702.md`（已存在，未改动） | 本应写同名文件，因文件已存在而中止覆盖 |
| Codex 交互记录 | `codex-decide-20260702-235449-5f01.md` | `codex-decide-20260702-235608-f2c0.md` |
| Q2 结论 | **删除 `closed_loop/mutex_interp.rs::choose_action`（9谓词），保留 `strategy/mutex.rs::mutex_class`（8谓词）** | **删除 `strategy/mutex.rs`（8谓词），保留 `closed_loop/mutex_interp.rs::choose_action`（9谓词）** |

**下游任务 #94（"p2-impl"，当前 pending，尚未开始实装）已经按 ws-codexq2 的结论设计**："Q2: 删 closed_loop/mutex_interp.rs+test"。

## 两份裁定各自的核心论据（都是独立成立的推理，不是任何一方的失误）

**ws-codexq2（留 mutex.rs / 删 choose_action）**：
- `choose_action` 的 9 谓词/10 类 `ActionClass`（含 `Deleverage`/`ExecutionRepair`/`PhaseThreeAccreteCore` 等）比生产 `interp::interpret` 实际输出的三桶（close/open/record）粒度**更细**，且这些细分类别在生产代码里根本不存在对应实现——用它做 D1 等价测试的 oracle 会重犯"范畴不匹配"的错误（原 D1 被否的同一种问题）。
- `mutex_class` 的 8 谓词/三桶粒度与生产 `interp` 输出**恰好匹配**，是唯一能在同一输入空间上做比较的候选。
- 这是一个**功能适配性**论据：D1 测试需要一个粒度匹配的 oracle，choose_action 的粒度不匹配，无法胜任。

**ws-codexp2（本工位，留 mutex_interp.rs / 删 mutex.rs）**：
- genealogy 619 号（已结算 2026-06-26）裁定"`formal/Origin/` 六层是唯一 canonical base，所有后续形式化向 Origin 对齐实例化，旧零件 port 重锚非独立重证"。
- `mutex_interp.rs::choose_action` 自陈"忠实镜像 Lean 的 9 谓词链（canonical base）"——是合规的 Origin port。
- `mutex.rs` 是直接从 PDF 独立重新推导（不 port 自 Origin），且 git 时间线显示它是在 619 号裁定结算 4 天后才新增的（与 `mutex_interp.rs` 同一 commit）——不能用"历史遗留未及追溯"辩护，是纪律结算后的新增违规。
- 这是一个**治理合规性**论据：619 号已经结算的架构纪律不应被绕过。

## 我的评估（不是替team-lead做最终裁断，只是提供质询后的判断供参考）

两个论据处理的其实是有细微差异的问题：ws-codexq2 回答的是"哪个模块能实际胜任 D1 oracle 这个具体工程需求"，我的 Codex 回答的是"哪个模块在架构治理意义上是合规的"。二者不是同一个问题的两个答案，而是**同一个 Q2 提问下被两次 Codex 调用理解为不同问题**——这本身提示 Q2 的提问方式有歧义，应该拆分成两个独立问题："D1 该用哪个模块做 oracle"（功能问题）vs "两个零消费者模块的治理去留"（合规问题），而不是假设两者答案必然一致。

如果按 619 号纪律严格执行，`mutex_interp.rs` 应该保留作为"Origin canonical port 的本地锚点"，但这不代表它必须同时兼任"D1 测试的 oracle"——这两个角色可以分离：`mutex_interp.rs` 保留但**不参与** D1 桥接（因为粒度确实不匹配，这个技术判断我认同 ws-codexq2 的分析是对的）；`mutex.rs` 若要删除以满足 619 纪律，D1 桥接需要一个新的、专门为此设计的、粒度匹配的小型独立 oracle（不是 `mutex.rs` 也不是 `mutex_interp.rs`，而是本次 D1 裁定里提到的"独立于 interpret 本身的小 oracle"，可以直接内嵌在测试模块里，不需要作为独立的公共谓词链模块存在）。

这样两个论据都不用被牺牲：619 纪律通过删除 `mutex.rs`（作为独立于 Origin 的重复证明）满足；D1 测试的可用性通过写一个新的、范围明确限定为"仅供 D1 property test 使用"的局部 oracle 满足（不需要复用现存的任一个谓词链模块）。

**但这是我的判断，不是终局**——是否接受这个"两个都删，D1 自建小 oracle"的第三条路径，或者干脆认为其中一个论据的权重应该压倒另一个（如认为 619 治理纪律本来就不适用于这种测试辅助模块的范围），需要 team-lead/编排者裁断。**不建议在此冲突解决前推进 #94 的实装**（#94 目前设计基于"留 mutex.rs"，若最终裁断是"两个都删"，#94 的设计也需要相应调整）。

## 建议行动

1. 暂停 #94 的实装启动，直到 Q2 的最终去留有一个团队认可的单一结论。
2. team-lead 可以：(a) 直接采纳 ws-codexq2 的结论（继续 #94 原设计），并把本冲突记录标注为"已知但不影响执行"；(b) 采纳"两个都删 + D1 自建局部 oracle"的第三条路径，相应调整 #94 设计；(c) 走 `/escalate` 交编排者裁断（如果认为 619 纪律的适用范围本身是一个需要编排者价值判断的问题）。

---

```yaml
---conflict-note---
type: dual_codex_ruling_conflict
task_id: "92"
downstream_blocked: "94"
verdicts:
  ws_codexq2: {keep: "strategy/mutex.rs", delete: "closed_loop/mutex_interp.rs"}
  ws_codexp2: {keep: "closed_loop/mutex_interp.rs", delete: "strategy/mutex.rs"}
assessment: "两次调用实际回答了不同问题（功能适配性 vs 治理合规性），非单纯对错"
recommended_third_path: "两个模块都删除，D1 property test 自建范围受限的独立局部 oracle（不复用任一现存谓词链模块）"
action_required: "team-lead/编排者裁断后再启动#94"
---end-conflict-note---
```
