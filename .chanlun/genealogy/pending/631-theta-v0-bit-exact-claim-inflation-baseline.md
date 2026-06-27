---
id: "631"
title: "theta_v0 bit-exact 声明膨胀基线对照表 = 『镜像 Lean spec 输出对齐』措辞在零 Lean 数值 parity 测试下的有效域膨胀（231号活实例）：全部 `_bit_exact` 测试是 Rust 内部硬编码 golden（L1），无任何代码运行 Lean 并逐 bit 比对输出（L0↔L1 交叉缺失）——『bit-exact 镜像 Lean』裸声明 vs 已诚实标注的 conformance 协议契约的分裂"
status: "生成态"
type: bias-correction
date: "2026-06-27"
domain: "认识论等级（L0↔L1 交叉验证）/ theta_v0 实装"
depends_on: ["625", "627", "623"]
related: ["607", "600"]
negated_by: []
negates: []
source: "质量守卫工位 bit-exact 声明膨胀基线扫描（Lead 本轮 #1 任务）+ gap-audit #4 gap（theta_v0 零 Lean parity 测试）独立佐证"
---

# bias-correction 631：theta_v0 bit-exact 声明膨胀基线对照表

**类型**: bias-correction（声明膨胀纠正）
**状态**: 生成态
**日期**: 2026-06-27
**规则依据**: `formalization-validity-domain.md`（231号——有效域 ≠ 定义域）+ `no-patch-mentality.md` 禁止模式5（声明膨胀：在注释/文档中声明代码不具备的能力）
**域**: 认识论等级标注（L0/L1/L2）/ theta_v0 Rust 实装的 Lean 对齐声明

---

## 核心发现（一句话）

theta_v0 模块头与多处子模块注释用 **「bit-exact 镜像 Lean spec 输出对齐」** 的措辞声明 Rust↔Lean 一致性，但 **代码库中不存在任何运行 Lean 并对其输出逐 bit 比对的测试**：全部命名含 `_bit_exact` 的测试都是 **Rust 内部硬编码 golden 断言**（L1，零信息增量），无 FFI 调 Lean、无加载 Lean 导出的向量文件、无 `include_str!`/`read_to_string` 引入 Lean fixture。

这是 231号（形式化有效域 ≠ 定义域）的活实例：声明的有效域是「Rust 与 Lean spec 输出对齐」（暗示 L0↔L1 交叉），实际有效域仅「Rust 实装内部自洽 + 对手工硬编码期望值一致」（L1）。

**关键澄清**：231号 L1 表的「合成数据验证」指**经验假设的合成验证**。本对照表里的 `_bit_exact` 测试性质更弱——它们是 **Rust 实装 vs Rust 作者手填的期望常量**的一致性检验（golden test），连「合成数据跑 Lean 对比 Rust」都不是。所谓 bit-exact 的对象（Lean 输出）从未进入测试。

---

## 声明 vs 验证对照表

### A. 裸声明（措辞暗示 Lean parity，但无 Lean parity 测试支撑）——膨胀点

| file:line | 声明内容（节选） | 实际验证 | 判定 |
|-----------|----------------|---------|------|
| `rust/src/theta_v0/mod.rs:1` | 模块标题「**bit-exact 缠论可执行系统引擎**」 | 无 Lean parity 测试 | 膨胀（措辞）|
| `rust/src/theta_v0/mod.rs:4` | 「Θ v0 规格的 **bit-exact Rust 实装**」 | 无 Lean 输出比对 | 膨胀（措辞）|
| `rust/src/theta_v0/mod.rs:27` | 「实装本身 = L1（bit-exact 一致性：**Rust 与 Lean spec 输出对齐** = 验证管线正确）」 | 无任何代码运行 Lean spec 取输出 | **核心膨胀**——L1 定义被偷换：「Rust 与 Lean spec 输出对齐」需 L0↔L1 交叉（跑 Lean 比 Rust），实测只有 Rust 内部 golden |
| `rust/src/theta_v0/mod.rs:29` | 「**对齐 Lean fixture 的 conformance test** = L1（合成/golden 一致性）」 | 不存在 Lean fixture 文件；全部 golden 是 Rust 手填常量 | **膨胀**——「Lean fixture」不存在 |
| `rust/src/theta_v0/classifier/mod.rs:5` | 「递归级别构造…**bit-exact** 计算」 | 见 signal.rs，硬编码 golden | 膨胀（措辞）|
| `rust/src/theta_v0/classifier/mod.rs:35` | 「实装本身 = L1（bit-exact 一致性：**Rust 与 Lean spec 输出对齐**）…各子模块的判定函数是 **Lean 纯函数的镜像** | 镜像 = 人工移植，无机器验证的输出一致性 | **核心膨胀**（同 mod.rs:27）|
| `rust/src/theta_v0/classifier/signal.rs:7` | 「由走势结构相对中枢的位置…**bit-exact 计算**（非臆造）」 | 测试 `third_buy_extracted_bit_exact` 等是手填期望值 golden | 膨胀（措辞 + 测试名误用 bit_exact）|
| `rust/src/theta_v0/classifier/signal.rs:14` | 「`Origin.CenterStates.classifyPosition`，**已 bit-exact 形式化**」 | Lean 侧存在该 def ≠ Rust 与之 bit-exact；无比对测试 | 膨胀（把「Lean 有定义」当「Rust 与 Lean bit-exact」）|

### B. 测试名误用 `bit_exact`（实为 Rust 内部 golden，名字制造 parity 假象）

以下测试函数名含 `_bit_exact`，但**断言对象全是 Rust 实装 vs 作者手填的期望常量**，与 Lean 无关。名字本身是膨胀（暗示 bit-exact parity，实为 golden）：

| file | 测试函数（节选） | 实际断言 |
|------|----------------|---------|
| `classifier/signal.rs:156,198` | `third_buy_extracted_bit_exact` / `third_sell_extracted_bit_exact` | `assert_eq!(points[0].pivot_low, 210)` 等手填常量 |
| `classifier/level.rs:107-162` | `*_bit_exact`（7 个） | Rust `classify_move` 输出 vs 手填 MoveKind |
| `classifier/center.rs:235-375` | `*_bit_exact` / `*_bit_exact_origin`（5 个） | Rust 中枢判据 vs 手填 Center 字段 |
| `classifier/bsp.rs:140-178` | `*_bit_exact`（5 个） | Rust BSP bit-vector vs 手填 bool |
| `classifier/nest.rs:147-219` | `*_bit_exact`（5 个） | Rust sel_order vs 手填序 |
| `classifier/level_state.rs:114-139` | `rlevel_*_bit_exact`（4 个） | Rust R6 态 vs 手填态 |
| `parser/segment.rs:410,421` | `*_bit_exact_claim10` | Rust overlaps/contains vs 手填 bool |
| `closed_loop/sell.rs:314` | `sell_ledger_delta_bit_exact` | Rust ledger delta vs 手填 i64 |

**注**：B 类测试作为 L1 golden 是**合法且有价值的**（验证 Rust 实装确定性 + 回归保护）。膨胀仅在于**命名**——`bit_exact` 应指「与某外部参考逐 bit 相等」，这些是「与作者期望逐字段相等」，应命名为 `*_golden`。

### C. 已诚实标注（承认无 Lean 数值 parity）——非膨胀，作为正面对照与守卫模板

这些文件**明确声明**了 bit-exact 对齐的实际有效域，是工位有能力诚实标注的证据，也是 SG-1/SG-2 应遵循的模板：

| file:line | 诚实声明内容 |
|-----------|------------|
| `closed_loop/conformance.rs:42-47` | 「Origin `hybridStep` 是抽象多态函数，**无具体数值轨迹可逐 bit 比**…**不**冒充『跑出 Origin Lean 数值逐 bit 比对』…本模块诚实声明 conformance 对象是协议契约，非 Lean 数值轨迹」。其测试实为 Rust **运行两次自身比对确定性**（`s1=step;s2=step;assert_eq!(s1,s2)`），非 Rust↔Lean |
| `classifier/force_conformance.rs:25-27` | 「**Rust↔Lean bit-exact 对齐仍是 L2 未验证假设**：`MacdConformsTo` 在 Lean 中是 Prop（未证定理）…当前未验证」 |
| `closed_loop/transition.rs:47-48` | 「**bit-exact 逐态对齐证（U5）诚实延后**——本文件不冒充已 conformant」 |
| `parser/second_kind.rs:5` | 「本文件 = L1（对 frozen 定义忠实 + 对 Python 参考交叉验证），**非对 Lean bit-exact**」 |
| `parser/segment.rs:202,226,308` | 「★L1…**非对 Origin Lean bit-exact**——动态状态机有意不形式化」 |
| `parser/feature_seq.rs:14` | 对 **Python** parity 的诚实实测差距：「前 237 段仅 2 段 bit-exact 一致」——这是真正做了 parity 比对（对 Python，非 Lean）并诚实报告否定性结果的范例 |

---

## 矛盾的精确形式（231号实例）

**定义域声明**（mod.rs:27 / classifier/mod.rs:35）：「实装本身 = L1（bit-exact 一致性：Rust 与 Lean spec 输出对齐）」。

**「Rust 与 Lean spec 输出对齐」要成立，必须存在**：跑 Lean spec 取输出 → 跑 Rust 取输出 → 逐 bit 比对的测试（L0↔L1 交叉）。

**实际有效域**：零此类测试。全部支撑证据是 B 类 Rust 内部 golden（Rust vs 作者手填常量）。

**∴ 声明的有效域（Rust↔Lean 输出对齐）⊋ 实际有效域（Rust 内部自洽）**——有效域膨胀。

这与 625号（L2 真实数据揭示 L1 合成 GREEN 掩盖引擎 bug）同构：**L1 GREEN ≠ 上层声明成立**。631号是该模式在「L0↔L1 交叉」维度的实例——L1 golden GREEN 被措辞包装为「与 Lean 对齐」。

---

## 边界条件（结论翻转条件）

本基线对照表的「膨胀」判定在以下任一条件下翻转为「已支撑」：

1. **出现真正的 Lean parity 测试**：Rust 测试加载 Lean 导出的输出向量（fixture 文件 / FFI）并对 theta_v0 对应函数逐 bit 比对 ⟹ A 类「核心膨胀」消解，对应措辞获 L0↔L1 支撑。
2. **措辞降级为诚实标注**：若 mod.rs:27/29、classifier/mod.rs:35 的措辞改为 C 类风格（如「Rust 实装 L1 内部 golden 一致；Rust↔Lean 输出 parity 未验证，待 fixture」）⟹ 膨胀消解（声明与实际一致，符合 no-patch-mentality 要求模式4「诚实」）。
3. **`_bit_exact` 测试改名为 `_golden`**：B 类命名膨胀消解。

注：本对照表**不主张** B 类 golden 测试无价值——它们是合法 L1 回归保护。翻转的只是「bit-exact / 与 Lean 对齐」的措辞，不是测试本身。

---

## 下游推论

1. **#4 gap 独立佐证**：gap-audit 报「theta_v0 零 Lean parity 测试」——本扫描从声明侧独立证实：不仅零 parity 测试，且存在用 bit-exact/镜像 Lean 措辞**掩盖**该缺口的注释。两路证据交汇。
2. **SG-1/SG-2 守卫提醒**（本 goal 将产 parity 测试）：
   - parity 测试必须**真正引入 Lean 输出**（fixture 文件或 FFI），不得用 Rust 手填常量冒充 Lean 期望——否则只是又一个 B 类 golden 戴 parity 帽子。
   - 测试必须能**否证**（L2 价值）：构造 Lean 与 Rust 可能分叉的输入，而非只喂双方都能过的平凡 case（合成确认偏差，231号禁止模式2）。
   - 产出 L 级标注必须诚实：Lean 导出向量 = L0 侧；Rust 跑出 = L1 侧；二者比对 = L0↔L1 交叉。不得标为 L2（L2 是真实市场数据有效性，非 spec 一致性）。
   - feature_seq.rs:14 是模板：做了 parity 比对、诚实报告「237 段仅 2 段一致」的否定性结果——SG 工位的 parity 测试应同样敢于暴露分叉，而非追求绿。
3. **认识论分类层影响**：mod.rs:27 的 L1 定义「bit-exact 一致性：Rust 与 Lean spec 输出对齐」需修正——当前实装的 L1 实质是「Rust 内部 golden 一致性」，二者是不同的 L1（前者需 L0↔L1 交叉，后者纯 L1）。建议 SG 工位/仪式区分这两个 L1 子级。

---

## 谱系引用

- **231号**（formalization-validity-domain）：本对照表是其在「L0↔L1 交叉」维度的活实例——有效域（Rust↔Lean 输出对齐）⊋ 实际有效域（Rust 内部 golden）。
- **625号**：L2 真实数据揭示 L1 合成 GREEN 掩盖引擎 bug——631号是同构模式（L1 GREEN 被措辞包装为更高声明）在 spec 一致性维度的实例。
- **627号**（双引擎线有效域外推风险）+ **623号**（蜂群自身工具/spec 层的 231号连续实例化）：631号是 spec 层声明膨胀的又一实例，可与 623号二阶模式并轨。
- **090号**（严格性语法规则）：声明膨胀禁止的来源——「代码能做什么就声明什么」。
- **600号**（异质审计否定价值）：本扫描的价值在于暴露蜂群自己没标红的措辞膨胀，否定性结果缩小有效域边界。

---

## 影响声明

- **改动**：仅写入本谱系记录（pending/631）。**未改任何 rust/lean 代码**（质量守卫只读 + 写谱系）。
- **影响模块**：标记 theta_v0 `mod.rs`、`classifier/mod.rs`、`classifier/signal.rs` 的 bit-exact/镜像 Lean 措辞为待修正声明膨胀点（A 类核心膨胀 2 处：mod.rs:27、classifier/mod.rs:35；措辞膨胀 6 处；命名膨胀 8 组测试）。
- **影响定义**：触及 `formalization-validity-domain.md` 的 L1 等级定义在「spec 一致性」语境下的细分需求（L0↔L1 交叉 vs 纯 L1 golden）。
- **不改**：B 类 golden 测试逻辑（合法 L1）、C 类已诚实标注文件。
- **升级判断**：本记录暂为 bias-correction（声明与实际不一致的纠正），**未升级为概念分离**。若 SG 工位实装 parity 测试时发现 Lean spec 与 Rust 实装存在**不可调和的输出定义矛盾**（非措辞问题，而是两侧定义本身冲突），则升级为 domain 类概念分离并走 /escalate。
