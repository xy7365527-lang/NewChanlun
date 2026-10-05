# Stage67 闭包声明独立盲读回

本读回仅依据 `readback-inputs.json` 所列的 `ClosureRoots.lean`、`direct.stdout`、`direct-receipt.json`、`lean-toolchain`，以及允许读取的 `lean-verify/SKILL.md`。未读取普通证明文档、契约 expected_type、MEMORY、ResearchProgress、市场来源或其他代理结论。以下判断只说明形式声明实际表达的内容；未取得的领域意图不在判断范围内，也不声称已与之匹配。

## 1. 实际声明与参数

源文件只导入 `Std`，定义位于 `Stage67` 命名空间。独立运行原文件，并另以 `pp.universes = true`、`pp.explicit = true` 打印定义和检查根声明，均成功退出。源文件 SHA-256 为 `e1c96a05d4f2f5450f9056cafe54254e8a97413083927ba2949ae4d27794c6c4`，与冻结清单一致。

第一个定理的实际类型是：

```lean
∀ (α : Type) (rank : α → Nat) (closed : α → Prop),
  Stage67.NeedsEarlierClosed α rank closed → ∀ (x : α), ¬ closed x
```

`α`、`rank`、`closed`、前提证明 `h` 均为显式参数；`∀ x` 中省写的类型由 `α` 推断。这里 `Type` 为 `Type 0`（即 `Sort 1`），声明没有自由宇宙参数，未写成对任意 `Type u` 的宇宙多态版本。没有隐藏的 `Inhabited`、有限性、可判定性、序结构或额外类型类前提。`rank` 的值域固定为 `Nat`；`<` 是自然数的严格小于。

第二个定理没有参数或前提，其实际类型是：

```lean
Stage67.MovementClosed 1 ∧
  ∀ (x : Nat), Stage67.MovementClosed x →
    ∃ b : Nat, Stage67.RawClosed b ∧ b < x ∧ ¬ Stage67.MovementClosed b
```

源代码中的省略绑定类型均被推断为 `Nat`；后半段的合取右结合，实际为 `RawClosed b ∧ (b < x ∧ ¬ MovementClosed b)`。

## 2. 核心定理的数学读回

源文件第 5–6 行定义：

```lean
def NeedsEarlierClosed (α : Type) (rank : α → Nat) (closed : α → Prop) : Prop :=
  ∀ x, closed x → ∃ b, closed b ∧ rank b < rank x
```

令 A 为任意处于 `Type 0` 的对象类型，r : A → ℕ 为任意函数，C 为 A 上的任意一元谓词。若对每个 x ∈ A，只要 C(x) 成立，就存在同一对象域中的 b ∈ A，使 C(b) 成立且 r(b) < r(x)，则对所有 x ∈ A，C(x) 都不成立。

量词次序是“对所有 A、r、C，若整个前提成立，则对所有 x 否定 C(x)”。前提中的 b 可以依赖 x 及其 C(x) 证明；不要求唯一，也没有预先指定的前驱函数。

这里的 `closed` 是一元性质，并不是已给定的二元依赖关系，也没有数学上的闭包算子公理。源代码只以名称称其为 closed。x 和 b 使用完全同一个 C；这正是归纳步骤能把更低秩对象再次纳入同一假设的原因。不能把 b 满足另一个谓词当作满足此前提。

自然数秩提供良基的严格下降：每次见证必须满足 r(b) < r(x)，同秩不够，小于等于不够。秩不需要单射，不要求覆盖所有自然数，也不要求 A 有限；秩可以无界。b 与 x 必然不同，但这种不同由严格秩下降推出，声明没有另加不等式。定理没有提供真实时间、价格、区间、走势、构成或因果依赖的定义；它只使用自然数秩。

证明对秩 n 作 `Nat.strongRecOn` 强归纳。归纳命题为“对每个秩等于 n 的 x，¬ C(x)”。若 C(x) 成立，前提给出更低秩且仍满足 C 的 b；归纳假设立即否定 C(b)，得出矛盾。这不需要选择公理来构造无限序列，也没有把某条具体领域依赖规则作为结论证明。

## 3. 空域、空谓词与条件空真

- A 可以为空，因为没有非空前提。此时 `NeedsEarlierClosed` 与结论 `∀ x, ¬ C(x)` 都由空域上的全称量词平凡成立。
- A 可以非空而 C 的满足者集合为空。此时前提中每个 `C(x) → ...` 都条件空真，结论也成立；因此定理没有证明“A 本身为空”。
- 一旦存在某个 C(x)，前提就不可能满足。特别是 r(x) = 0 时，前提要求一个自然数秩严格小于 0，已直接矛盾；较高秩由强归纳同样排除。
- 前提本身没有预先假定矛盾：取任意 A 和 r，并令 C 恒假，就可满足它。它与“存在满足 C 的对象”联合时才不相容。
- 从声明和基本逻辑可进一步读出：对于给定的 A、r、C，`NeedsEarlierClosed A r C` 与 `∀ x, ¬ C(x)` 等价。正向是已证明的根定理；反向由每个 C(x) 的矛盾消去得到。这个反向等价没有在原文件中另立具名定理，此处是对前提强度的数学分析。

因此，第一个声明是一个有明确前提的不可存在结论。它不自行确立任何给定谓词满足此前提，也没有无条件否定任意 closed 对象的存在。

## 4. 边界模型的数学读回

源文件第 20–21 行精确定义：

```lean
def RawClosed (n : Nat) : Prop := n = 0
def MovementClosed (n : Nat) : Prop := n = 1
```

`RawClosed` 的满足者集合恰为 {0}，`MovementClosed` 的满足者集合恰为 {1}，二者在自然数模型中互斥。名称没有额外附带任何领域结构。

`raw_root_boundary_model` 证明两个事实同时成立：

1. 1 满足 MovementClosed，所以 MovementClosed 确实非空。
2. 对每个自然数 x，只要 x 满足 MovementClosed，就存在一个自然数 b，满足 RawClosed、b < x，且不满足 MovementClosed。

在唯一满足前提的 x = 1 情形中，构造的见证为 b = 0：0 = 0、0 < 1、0 ≠ 1。对 x ≠ 1 的情形，后半段蕴含式条件空真；但整个定理不是靠 MovementClosed 为空成立，因为首个合取项给出了明确非空见证。

这个模型表达的是“非空的 MovementClosed 对象可有一个更小的 RawClosed 对象，而且该对象不属于 MovementClosed”。后半段没有一般的 `rank` 参数，直接使用自然数 b < x；若与核心定理对照，可取恒等秩 r(n) = n。

它不满足 `NeedsEarlierClosed Nat id MovementClosed`：若满足，x = 1 所需的 b 也必须满足 MovementClosed，因此 b = 1，却又需要 1 < 1。它也不满足 `NeedsEarlierClosed Nat id RawClosed`：x = 0 无法找到小于 0 的 RawClosed 见证。原声明没有提出这两种前提，所以没有与核心定理发生冲突。

边界声明仅建立这个具体模型。它没有对任意 RawClosed/MovementClosed 谓词、任意秩、多个根、分支结构或任意领域对象作全称推广，也没有证明现实对象等同于这两个单点集合。其“根”只体现在具体自然数最小元素 0；模型没有额外的根节点关系。

## 5. 根依赖闭包与公理边界

独立复跑的 `#print axioms` 结果为：

| 根声明 | 实际传递公理依赖 |
|---|---|
| `Stage67.no_closed_if_all_need_earlier_closed` | 无 |
| `Stage67.raw_root_boundary_model` | `[propext]` |

核心根使用自然数强归纳与等式/逻辑消去，Lean 报告不依赖公理。边界根当前证明通过 `simp [MovementClosed]` 的简化路径间接使用命题外延性 `propext`，即把逻辑等价的命题识别为相等的命题。`propext` 是这个已提交证明项的依赖；这不表明边界命题在数学上必须依赖该公理。

两个根的打印结果均不含 `sorryAx`、`Classical.choice`、`Quot.sound` 或自定义公理。这里的结论针对根声明的实际传递依赖闭包，不是只扫描源文本，也不声称 `Std` 中所有声明都无公理。编译成功与这些依赖结果不替代形式声明和未提供意图之间的独立比较。

## 6. 可复跑证据与审阅边界

独立执行使用固定绝对路径 `/Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean`，未调用 elan 默认入口、未下载依赖；运行时报告 Lean 4.31.0。两个运行的 `LEAN_PATH` 均显式置空，exit code 均为 0，stderr 均为空。

- `input-hashes.json`：四个冻结输入的预期哈希、实测哈希及一致性。
- `direct.stdout`、`direct.stderr`、`direct-receipt.json`：直接检查原始源文件，保留完整根声明和公理闭包输出。
- `InspectDeclarations.lean`、`inspect.stdout`、`inspect.stderr`、`inspect-receipt.json`：原始源的逐字副本后追加显式参数/宇宙打印命令，保留展开定义与实际绑定类型。
- 回执记录准确 argv、cwd、时间、Lean 版本、可执行文件哈希、源哈希与输出哈希。依赖环境来自该固定工具链的 `Std`；本次未逐文件哈希整个工具链库目录。

独立读回已完成，未发现声明被隐藏参数扩大前提或把非空见证省掉的情况。第一个根的实质限制是“同一谓词下的自然数严格下降前提”；第二个根的实质范围是具体自然数双单点模型。后续比较者仍需依据另行提供的意图判断是否匹配，本读回不对此作结论。
