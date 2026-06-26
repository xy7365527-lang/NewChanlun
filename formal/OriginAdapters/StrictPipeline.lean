import Origin.ChanlunElements
import Origin.TrendCompleteClassification
import Origin.FullDefinitionStrategy
import Origin.TraceProjection

/-! # OriginAdapters/StrictPipeline.lean

A′ 方向（task #96/#97）的 legacy→Origin 适配器骨架。

## 存在论位置

Origin 六层 standalone 闭包是**唯一 canonical base**（formal/Origin/）。现有 Strict/Tlayers/
Foundation/theta_v0 成果降为「待重锚 legacy reference」。本文件是 Phase2 各 port 在 Origin
canonical 接口上的**挂接锚点**：

- legacy pipeline（Strict.Parse / Strict.Trend / Strict.HybridAssembly / theta_v0 contract）
  将实现这里声明的 Origin 接口别名。
- Phase2 worker（#99 RecursiveLevelSystem 重锚 / #100 分类极限 guardrail / #101 双账本桥 +
  HybridAssembly / #102 theta_v0 contract）import 本文件 + Origin canonical，把各自的 legacy
  实现填入对应接口别名。

## 严格性声明（no-patch / no-声明膨胀）

本文件**零 sorry**：只声明真能编译的对象——
- 接口别名：`abbrev` 指向 Origin 已证 canonical 结构（真能立，类型层等价）。
- 适配器入口：`def` 形式的恒等/构造函数，把 Origin 结构本身作为「Origin 接口的 canonical 见证」
  暴露给下游 port（真能编译，不假定任何 legacy 已满足契约）。

**尚不能无 sorry 实现的适配器不在此声明**（诚实标 TODO-接口）——具体的 legacy→Origin
实现（如「Strict.HybridAssembly.AssemblyState 满足 Origin.RustEngineContract」这类桥接定理）
依赖 Phase2 重锚 HybridAssembly + 拷入 EngineBridge，本 Phase1 门**不声明、不冒充实现**。

## 认识论等级

L0（类型/接口层）。本文件不验证任何 legacy 引擎「做到了」Origin 契约——那是 Phase2 port 的
L0 桥接定理（结构层）+ 后续 L2 的事。本骨架只暴露 Origin canonical 接口供下游 import。
-/

namespace OriginAdapters

open NewChanlun.Origin

/-! ## 接口别名（真能编译的 Origin canonical 接口）

下游 legacy port 的目标类型——每个别名直指 Origin 已证结构，是类型层 canonical 见证。 -/

/-- 元素解析流水线接口（笔/线段/中枢/走势/买卖点的 total-deterministic 装配）。
    legacy 实现：`Strict.Parse` 的 `Θ_parse`（Phase2 #102 theta_v0 contract 重锚此接口）。 -/
abbrev ElementPipelineIface := ElementPipeline

/-- 走势分类器接口（`{unfinished, consolidation, trendUp, trendDown}` 四类穷尽）。
    legacy 实现：`Strict.Trend` 走势分类器（Phase2 #100 分类极限 guardrail 重锚此接口）。 -/
abbrev TrendClassifierIface := TrendClassifier

/-- 递归级别系统接口（`State : Nat → Type` + 逐级 `lift`，自相似递归）。
    `State : Nat → Type` 字段令本结构落在 `Type 1`，故别名不固定宇宙。
    legacy 实现：`Strict.Recursive` 走势递归（Phase2 #99 RecursiveLevelSystem 重锚此接口）。 -/
abbrev RecursiveLevelIface := RecursiveLevelSystem

/-- 全定义策略闭环接口（`Rec→Class→Intent→Risk→Schedule→T` 单步，账本 `R=Π-A-W`）。
    含 `Event/Intent/Control/Order : Type` 字段，落在 `Type 1`，故别名不固定宇宙。
    legacy 实现：`Strict.HybridAssembly` 单一闭环（Phase2 #101 双账本桥 + HybridAssembly 重锚）。 -/
abbrev FullDefinitionIface := FullDefinitionSystem

/-! ## 适配器入口（Origin canonical 见证暴露）

下游 port 接收一个具体 legacy 实现（已构造为对应 Origin 结构），本入口把它作为 Origin
接口的 canonical 见证返回——恒等 adapter，类型层真能编译，不假定 legacy 自动满足契约。 -/

/-- 把一个 Origin 元素流水线暴露为 Origin 接口见证（恒等 adapter）。 -/
def asElementPipeline (P : ElementPipeline) : ElementPipelineIface := P

/-- 把一个 Origin 走势分类器暴露为 Origin 接口见证（恒等 adapter）。 -/
def asTrendClassifier (C : TrendClassifier) : TrendClassifierIface := C

/-- 把一个 Origin 递归级别系统暴露为 Origin 接口见证（恒等 adapter）。 -/
def asRecursiveLevel (R : RecursiveLevelSystem) : RecursiveLevelIface := R

/-- 把一个 Origin 全定义策略系统暴露为 Origin 接口见证（恒等 adapter）。 -/
def asFullDefinition (S : FullDefinitionSystem) : FullDefinitionIface := S

/-- L0 锚点见证：`asElementPipeline` 是恒等 adapter（解析结果不被 adapter 改写）。
    这是真能编译的定理，证明 adapter 不引入语义漂移。 -/
theorem asElementPipeline_parse_id (P : ElementPipeline) (bars : List Bar) :
    (asElementPipeline P).parse bars = P.parse bars :=
  rfl

/-! ## TODO-接口（Phase2 承载，本 Phase1 门不声明实现）

以下桥接尚不能在 Phase1 无 sorry 实现，**故不声明定义/定理**，仅在此登记承载位：

- **theta_v0 → ElementPipelineIface bit-exact 桥**（Phase2 #102）：需 Rust `parser.rs` 的
  step 输出与 `Origin.ElementPipeline.parse` 字段级对齐的 L0 桥接定理。依赖 theta_v0 contract 重锚。
- **Strict.HybridAssembly → Origin.RustEngineContract 桥**（Phase2 #101）：需先拷入 EngineBridge
  （现 import Strict.HybridAssembly 的 legacy），并证 `assemblyStep` 满足 `RustEngineContract.StepSpec`。
  本 Phase1 门已明确延后 EngineBridge，故此桥不在此声明。
- **分类极限 guardrail → TrendClassifierIface 完备性兑现**（Phase2 #100）：需把
  `CompleteClassifier` 行为极小性对具体缠论分类器的兑现（共有缺口 #91）重锚到 Origin。

诚实声明：上述三项是 Phase2 的 L0 桥接定理，Phase1 门**不假装已实现**。
-/

end OriginAdapters
