# Stage18：九原单元反例不能靠连续重分组修复

日期：2026-10-03，#1467，`DC-NINE-SOURCE-REGROUP-v1`。承接[开工卡](Stage18WorkCard.md)。名分是限定候选类的数学否定，作者Lean机器证据已取得，独立语义审查未完成。

## 本轮结论

Stage14否定了固定三段重切后的自动上送。本轮加强为：**在同一份九原单元上，任何按原顺序、连续、不重复取源、每块至少三原单元的三个子对象，都无法避开那个严格上级核心反例。**即使允许在三块之前、之间、之后保留未消费片段，结论也不变。

因此，调整这类分组的起点、长度或跳过片段，不能使该前缀立即取得严格上级核心。这排除了一个候选族，不仅是一份实现。它没有证明所有订单递归不可能，也没有证明未来增加成员后仍不能形成上级。

## 独立来源关系与一般定理

声明 [NineRegroupSpec.lean](NineRegroupSpec.lean)先冻结，证明位于[NineRegroupProof.lean](NineRegroupProof.lean)。`SourceTriplet xs a b c`独立规定：

```text
存在 pre、gap₁、gap₂、post，使
pre ++ a ++ gap₁ ++ b ++ gap₂ ++ c ++ post = xs，
且 len(a)、len(b)、len(c) 均至少为3。
```

它没有预先指定三块长度恰好为3，也没有要求中间残留为空。对任意元素类型、任意长度为9的列表，已证明：

- 四个残留列表全部为空；
- 三块长度都等于3；
- `a = take 3 xs`、`b = take 3 (drop 3 xs)`、`c = drop 6 xs`；
- `a ++ b ++ c = xs`，保序且完整恢复来源。

证明由长度守恒得到：三块下界已经耗尽九个位置，所有非负残留长度只能为零。再由列表取前缀/后缀规则确定每块的实际内容。这是关系本身的唯一分配结论，不是某个程序确定性，也不是有限枚举。

## 接到真实的旧反例与实际Origin谓词

本轮复用且核验Stage14的28项源码/证据绑定未变。该见证的合法完整DC分解、正量投影、三次回入吸收、九成员待重切、三个局部窗全通过，都直接包含在本轮精确根中，不仅引用报告的状态标签。

三个被唯一确定的块，其自身完整外缘仍为：

| 原单元位置 | 自身完整外缘 |
|---|---|
| 0–2 | [10040,10060] |
| 3–5 | [10030,10055] |
| 6–8 | [10030,10040] |

三者共同交集只有10040单点。`Carries`要求每个上送对象的范围等于该块所有原单元的完整外缘，没有沿用母核心。`GeometryLift`只要求存在上述来源分配、三个携带相应区间的Segment以及实际`Origin.CenterConfirmedComplete`。根证明直接得到`¬ GeometryLift input`。

这里刻意没有预先要求三个容器已经取得完成走势身份。连这些更弱的必要几何条件都不能同时成立，增加完成、方向或时点证书也不能改变这一范围失败。证明调用原`LiftBoundary.origin_rejection`，没有自建一个更窄的替代中枢谓词。三个源块实际存在，旧见证中的子窗全部通过，所以否定不来自空输入或不存在任何可分组对象。

## 候选类边界与下一步

每块至少三原单元是**本次候选族的明示条件**，不是对所有上层连接对象的教义裁定。中枢正本Z-8允许连接对象参与；若连接对象已具备同一级资格，而其来源不能被本关系刻画，本定理不覆盖它。不能为了套定理而抹掉这种类型差异，也不能把单个低级原单元改名成同级连接来规避资格问题。

本轮排除的是在这九原单元上立即形成严格上级核心的该类分组。以下仍开放：

- 保留全部低级来源及未完成状态，等待更多已确认输入后按具名新规则组装；
- 建立有语义依据的同级连接证书及成员归属，再检验中枢、连接和其他片段的组合；
- 研究其他F₁，包括直接订单簿候选。它们不能直接继承此DC见证的失败或通过状态。

本轮没有把“保留为未完成”当作完整F₂的解答。完整组装仍须说明未来的转换、唯一分解、完成证书、同级约束、全部来源去向以及有限输入终止；不能永远不产出而宣称闭合。也没有放宽严格端点、删去相切输入、重复消费成员或修改旧失败路线。

## 机器证据、失败记录与资源偏差

精确根`NineRegroup.exact_root`的类型是：

```text
GAP_TARGET ∧ SLICE_TARGET ∧ NONEMPTY_TARGET ∧
LiftBoundary.WITNESS_TARGET ∧ NO_REGROUP_TARGET
```

最终run为`bee3909396b7486b9744d28953dcee6f`。Lean4.31编译exit0、machine与exact_root通过；传递公理只有`propext`和`Quot.sound`，没有sorryAx、自定公理、不安全或未知依赖。声明锁未变。证据摘要见[stage18-lean-verification-summary.json](stage18-lean-verification-summary.json)，源码与外部绑定见[source-sha256-stage18.json](source-sha256-stage18.json)。

首轮失败是误写不存在的`List.length_eq_zero.mp`，只把证明体修正为库中实际的`List.eq_nil_of_length_eq_zero`；定义和目标未改。两次编译均保存。最终检查使用逐字相同的两份新源，复用已锁定Stage14编译模块；验证器仍检查实际加载的传递依赖和公理。证据复核起初因未提供相同LEAN_PATH被正确拒绝为`runtime_search_path`；恢复原环境后exit0、snapshot_current与exact_root均通过。该环境诊断不是数学反例。

**资源上限未守住。**首次展开证据为6,389,573字节，最终精确检查展开证据为23,822,613字节，均超过开工卡自设的5MiB估计；没有追改原上限或宣称资源检查通过。原因是依赖构建及完整声明/依赖清单的保存量，第二次复用模块减少了重编译，但没有减少全部提取记录。取得结论后停止扩测，两个结果均逐文件SHA核验后压缩归档，保留失败、通过、源码快照及日志；移除的仅为已完整归档的本轮展开副本。当前两归档、归档收据和两份源副本合计4,018,334字节，约3.83MiB，最终占用降低不抹去过程超限。没有付费调用或下载。

通过结果归档：`/Users/silencehan/Documents/Codex/research-evidence/issue1467/nine-regroup-v1-verified.tar.gz`，SHA256为`7fc209812b6ecbf9dc5bdc4f403f4ff4b84e36c54b872973135c40ee9fe7df6f`。归档内49个常规文件逐字校验一致；失败归档202个文件也已逐字核对。复核完整证据前需解压至原父目录，恢复manifest所引用的绝对路径；解压约需23MiB：

```bash
tar -xzf /Users/silencehan/Documents/Codex/research-evidence/issue1467/nine-regroup-v1-verified.tar.gz -C /Users/silencehan/Documents/Codex/research-evidence/issue1467
LEAN_PATH="$PWD/formal/.lake/build/lib/lean:/Users/silencehan/Documents/Codex/research-evidence/issue1467/lift-boundary-verified-v1/lean-verification-runs/70afcb63cad748cebe219c90e28e4091/lib" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/lean_evidence.py --manifest /Users/silencehan/Documents/Codex/research-evidence/issue1467/nine-regroup-v1-warm/run-manifest.json
```

重新运行精确验证时使用新的输出目录，并预留完整提取证据的空间：

```bash
LEAN_PATH="$PWD/formal/.lake/build/lib/lean:/Users/silencehan/Documents/Codex/research-evidence/issue1467/lift-boundary-verified-v1/lean-verification-runs/70afcb63cad748cebe219c90e28e4091/lib" python3 .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py --project /Users/silencehan/Documents/Codex/research-evidence/issue1467/nine-regroup-project-v1 --target-file NineRegroupProof.lean --declaration NineRegroup.exact_root --expected-type 'NineRegroup.GAP_TARGET ∧ NineRegroup.SLICE_TARGET ∧ NineRegroup.NONEMPTY_TARGET ∧ LiftBoundary.WITNESS_TARGET ∧ NineRegroup.NO_REGROUP_TARGET' --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake --direct --build-timeout 60 --strict-exit --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/nine-regroup-reproduction-1
```

底层编译命令保存在入仓摘要的`build_commands`与原manifest中；外部小项目`nine-regroup-project-v1`的两文件与仓内新源已逐字核对。所有旧生产/formal源保持不变，不触发fixture漂移检查。独立审查入口见[待审包](Stage18PendingReview.md)，`semantic=not_reviewed`。研究票和活动goal继续开放。
