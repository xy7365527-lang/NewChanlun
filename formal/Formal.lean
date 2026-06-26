/-
  真完全分类形式化（task #27 / 603 范式根模块）

  编排者纲领：把项目里所有"完全分类"claim 在递归数据类型范式下重铸为
  **真完全分类 = 构造子穷尽（内涵式）**，而非轴集合穷尽（外延式）。

  - 走势 = 初代数 μF（三构造子）；initiality 的泛性质 = 结构归纳 ⟹ "无第四构造子"是定理。
  - 级别递归 = 终余代数 νG（无"最后一步"= r* 非特殊，余归纳）。
  - 买卖点 = 端点非空标签集 totality（允许 2B/3B 重合，非互斥 enum）。

  认识论等级：所有机器可检验的 induction/coinduction/totality 命题均为 **L0**
  （定义内蕴，继承缠论已结算定理：走势三分 + 第21课完备性 + 中枢三态穷尽）。
  Lean build 通过 = 逻辑/管线正确（L0），不是实证有效域，不得膨胀（formalization-validity-domain）。

  Phase 1 脊柱：走势三分 + 递归构造 + 中枢三态 + BSP 标签集 totality + r* 非特殊。
  Phase 2：元素构成性阶梯（claim5）+ 背驰区间套（claim7）+ 操作语义（claim6）。
  （claim8 守恒012=watch-item，是 settled 材料的范式判定，非 Lean 模块——见 tmp/formalization-result.md。）
-/

-- Phase 1 脊柱
import Formal.TrendTrichotomy
import Formal.RecursiveConstruction
import Formal.CenterTrichotomy
import Formal.BSPLabels
import Formal.RStarNonSpecial

-- Phase 2
import Formal.ConstitutiveLadder      -- claim5：元素构成性阶梯（K线→分型→笔→线段→走势→中枢）
import Formal.DivergenceNesting       -- claim7：背驰/区间套（L_confirm=构造子字段）
import Formal.OperationalSemantics    -- claim6：操作语义（Σ 4 构造子 + Σ* 自由幺半群）
