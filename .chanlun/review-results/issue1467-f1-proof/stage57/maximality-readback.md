# Stage57 实际声明独立盲读

本报告只解释实际 Lean 根 `Stage57.terminal_root`。独立重新编译、打印与提取成功；根闭包状态为 `declaration_closed_uncompared`。没有提供 expected type，也未核对作者意图，所以 `exact_root_passed=false`。本报告不作原义、P 或 B56 认证，不宣告任何被隔离知识可以恢复使用。

复核者：新上下文子代理 `/root/readback_stage57_maximality`。仅读取指定最小入口、锁、其指向的实际 Lean 源、工具链和本树验证技能/工具；没有读取 extension-maximality.md、contract 意图、Progress、Intent、其他代理会话或原 verdict。独立性由实际会话隔离提供，本报告文本本身不是平台签名。

## 输入与执行

- 最小入口 SHA-256：`03a6708bd7af2598e3a52e3cb10282e90542c5c28ee913333fc1016fcc69b04f`。
- 锁 SHA-256：`60ec70f9fe086a0d30a312f295c6d17af691b346e22af807cac71a2becb62b07`。
- `RawMaximality.lean` SHA-256：`8abac6db0faf02ea4cb0efcca7ab3d00ce0a19ee731f666e753a5e944cae65f7`。入口、锁及实际源均吻合。
- 工具链：Lean 4.31.0，arm64-apple-darwin24.6.0，commit `68218e876d2a38b1985b8590fff244a83c321783`；direct 模式，无继承 LEAN_PATH。lean、lake、共享库 SHA 与最小入口一致。
- 从源码重新输出 `.olean` 到复核者证据目录，未覆写作者产物；随后编译检查器、提取当前根。源码自带的 `#print terminal_root` 与 `#print axioms terminal_root` 已实际执行。
- 包装器退出码：0；五个 Lean 子命令（导入解析、依赖路径解析、目标编译、检查器编译、根提取）退出码各为 0。未发生超时。包装器默认 0 仅表示写出报告；这里编译成功的依据是子命令及 manifest。
- 独立 run ID：`ead2b4bb330b49638ca610f113927c1d`。`machine.status=passed`、`machine_verification_passed=true`、`declaration_closed=true`；比较状态 `not_requested`，机器 semantic 状态 `not_reviewed`。本盲读文本没有被冒充为作者合同的 faithful 审计。
- 实际类型与最小入口中的 `actual_type` 逐字相同。完整指令、日志、产物、模块清单及输入绑定在证据目录；检查器记录加载模块数 2260。

初次按技能目录查找脚本得到“文件不存在”；随后定位到该技能所链接的同树插件 `scripts/verify_lean_project.py` 才运行上述独立检查。前者不是 Lean 编译失败，也未以其退出码替代实际运行结果。

## 参数、载体与输入中已有的性质

根有八个显式参数：`α : Type`、`le : α → α → Prop`、`h : Tape α le`、`K : Seed α le`、`j r τ : Nat`、`ht : RawTerminal h K j r τ`。八个 binder 均为 `Lean.BinderInfo.default`；无根级隐式参数、类型类前件或 universe 参数。`Type` 在这里固定为 `Type 0`，并非对所有 universe level 多态。

`le` 仅是任意二元命题关系，没有自反、传递、反对称、全序或可判定性假设。以下自然数的 `<`、`≤`、加法用 Nat 的标准实例，与传入的 `le` 无关。不能仅因字段名而将 `α` 当成实数、将 `Closed` 当成通常的实数闭区间。

`Closed α le` 含 `low high : α` 和输入证据 `valid : le low high`。`Seed` 含 `identity : Nat`、一个 `Closed` 核心、`start : Nat`；没有核心如何由数据构造的约束，`identity` 不参与根谓词。`Tape` 是定义在所有自然数上的总函数 `range`、`sealAt`、`endAt`，并携带两项已给定证明：封存时刻严格递增 `∀ {i j}, i < j → sealAt i < sealAt j`，以及 `∀ i, endAt i < sealAt i`。前一字段的 i、j 为隐式绑定；它们属于结构输入，不是根额外推导的全局规律。

`Touch a b` 精确为 `le a.low b.high ∧ le b.low a.high`。在任意关系下这只是两项端点关系的合取；没有定义点集或证明它等价于几何非空交集。

设 s = K.start，C = K.core。`TouchSpan h K k` 精确为：`s+2 ≤ k`，且每个自然数 i 若 `s ≤ i ≤ k`，则 `Touch (h.range i) C`。它始终固定同一核心和起点；要求至少三个索引，却不要求三个不同值、方向交替或任意两项互相 Touch。

输入 ht 已直接给出四件事：

1. `TouchSpan h K j`；
2. `r = j+1`；
3. `h.sealAt r ≤ τ`；
4. `¬Touch (h.range r) C`。

因此已有一段连续 Touch 的区间及其紧邻反例，不是从任意 Tape 搜索或构造出它们。根不证明存在某个这样的 j、r、τ。

## 六项精确结论

在上述输入全部给定时，根同时给出：

1. 对所有自然数 k，若 j<k，则 `¬TouchSpan h K k`。结合输入已有的 `TouchSpan h K j`，j 是此固定 h、K 的可行终点集合的最大元。证明原因是任何更长前缀都含 r=j+1，违背已给定的不 Touch。
2. 对所有自然数 q、r′、τ′，若 `RawTerminal h K q r′ τ′`，则 q=j。原声明只写终点 q 的等式；结合定义可再推出 r′=r，但不要求 τ′=τ。唯一性固定同一 h 与完整 K，不比较不同核心、起点或 Seed。
3. 对每个 `h′ : Tape α le`，若 `PreservesThrough h h′ r`，则原 `RawTerminal h′ K j r τ` 仍成立，且所有 k>j 均不满足 `TouchSpan h′ K k`。
4. `RawTerminal h K j r (h.sealAt r)` 成立。
5. 对所有自然数 t，若 t<h.sealAt r，则 `¬RawTerminal h K j r t`。
6. `h.endAt j < h.sealAt r`。

第 6 项由输入的 `endAt j < sealAt j`、严格递增性和 j<r 推出；第 4、5 项来自 RawTerminal 自身的阈值条件。没有独立的观测、通知、可计算检测或算法执行语义。

## 全称域、时间与扩张边界

这不是有限样本枚举：k、q、r′、τ′、t 的量词均遍及 Nat；h、h′ 遍及满足结构字段的全部无限总 Tape。每一个 TouchSpan 的内部检查区间 [s,k] 以及保留前缀 [0,r] 是有限的。不存在“只验证到某个经验上界”的隐含截断。

`PreservesThrough` 精确要求所有 i≤r 的 `range`、`sealAt`、`endAt` 分别相等，包括 r 本身与 s 之前的索引。其后的字段可以变化，但 h′ 仍必须满足 Tape 的全局严格递增和结束先于封存。这里的“扩张”是两个总 Tape 的前缀相等关系；声明没有有限列表 append、逐步生长或真实在线历史。没有允许改写核心、起点或紧邻反例，亦没有证明任意后缀都能补成合法 Tape。

“最大”是固定起点与核心下的连续前缀最大终点；根并不声称 r 之后每一项都不 Touch。后缀可以重新 Touch，只是包含已有反例 r 的整段前缀仍不满足 TouchSpan。换起点或核心的另一个区间不在这一最大性结论内。

τ、t、sealAt、endAt 全为自然数标签。第 4、5 项说明给定同一 h、K、j、r 的 RawTerminal 谓词，在 sealAt r 达到阈值；没有历史信息可得性定义，所以不推出主体在此前不能通过其他信息知道终点。它也没有将索引 j、r 等同为时刻。

条件域未被证明普遍非空：若 α 为空，无法提供 Closed/Seed/Tape；若 le 恒真，则不 Touch 的 ht 无法提供。结构与前件的这种限制必须保留，不能由全称量词误读成任意输入必有终点。根也未要求 Decidable，因此这是一条命题证明，不附带可执行搜索器。

## 实际闭包与证据范围

Lean 的实际传递公理集合为 `[propext, Quot.sound]`；无 `sorryAx`、无额外公理、无 unsafe 或 unknown dependency。此集合来自新运行的根检查，不是源文件文字扫描结论。`Classical.choice` 不在该实际集合内；导入环境存在相关库不等于根依赖它。

输入提供的 valid、seal_strict、end_before_seal 和 ht 是定理条件，不是额外全局公理，也不是被这条定理从外界证实的事实。整个检查适用于上述锁定源码与工具链；不认证缠论原文、生产实现或其他未导入理论。独立盲读完成；作者意图对照留待另一个隔离比较步骤。

实际完整 elaborated type 如下（来自本次提取）：

```lean
∀ (α : Type) (le : α → α → Prop) (h : Stage57.Tape α le) (K : Stage57.Seed α le) (j r τ : Nat)
  (ht : @Stage57.RawTerminal α le h K j r τ),
  And (∀ (k : Nat), @LT.lt.{0} Nat instLTNat j k → Not (@Stage57.TouchSpan α le h K k))
    (And (∀ (q r' τ' : Nat), @Stage57.RawTerminal α le h K q r' τ' → @Eq.{1} Nat q j)
      (And
        (∀ (h' : Stage57.Tape α le),
          @Stage57.PreservesThrough α le h h' r →
            And (@Stage57.RawTerminal α le h' K j r τ)
              (∀ (k : Nat), @LT.lt.{0} Nat instLTNat j k → Not (@Stage57.TouchSpan α le h' K k)))
        (And (@Stage57.RawTerminal α le h K j r (@Stage57.Tape.sealAt α le h r))
          (And
            (∀ (t : Nat),
              @LT.lt.{0} Nat instLTNat t (@Stage57.Tape.sealAt α le h r) → Not (@Stage57.RawTerminal α le h K j r t))
            (@LT.lt.{0} Nat instLTNat (@Stage57.Tape.endAt α le h j) (@Stage57.Tape.sealAt α le h r))))))
```

证据目录：`/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage57/maximality-readback`

独立 manifest：`/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage57/maximality-readback/verification/run-manifest.json`

manifest 文件 SHA-256：`e1c3e14def88938db46fd9074757d46bfd9ab63b4881e1624b4b28966e7d2b17`

实际声明摘录：`/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage57/maximality-readback/actual-declaration.json`。包装器回执：`/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage57/maximality-readback/wrapper-receipt.json`。本报告的文件 SHA 和绑定回执另存 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage57/maximality-readback/readback-receipt.json`，避免文件自哈希。
