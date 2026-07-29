# #425 Standards 轴评审（commit 639773fe01）

**结论：PASS-with-findings**。无正确性缺陷、无恒真断言、无旁路写入；发现集中在文档陈旧、护栏的 release 生效面、以及两条明文行数上限。

## HIGH
无。

## MED

**M1 公共 API 文档已被本次改动作废，未同步。** `LifecycleRevisionKind::StructureCompleted`(:231) 仍写「完成但不背驰滞留 Provisional **也留此痕**」——改动后第 7 步一旦推出 StructureCompleted，同 prefix 必入终态（Confirmed / NeverConstituted），该状态组合不可达；`NestEventState::Invalidated`(:159)「力度反超或身份消失」仍只列两成因。建议同批订正。

**M2 新增不变量在 release 下不自动执行。** 两条新断言（:864-878）写在 `pub fn assert_invariants` 内（真 `assert!`，被调用即在 release 生效），但 `advance` 内唯一自动调用点带 `#[cfg(debug_assertions)]`(:786)。模块头称「`assert_invariants` 逐条钉死」——实际是 debug-only 自动 + release 手动。`#[cfg]` 是既有的，新断言继承了它；#426 接线时若无人显式调用，这层等于无护栏。建议措辞改为「debug 自动、release 需显式调用」。

**M3 coding-style 两条硬上限被破且被加深。** `.claude/rules/common/coding-style.md`：「Files are focused (<800 lines)」「Functions are small (<50 lines)」。实测文件 2212 行（票体写 2153，数字不符），去掉 `mod tests`(:980+) 的生产段仍 979 行；`advance` 194 行（:596-789）。实装工位登记「不改」——不同意全盘登记：第 7 步（:728-766）可整体平移为私有 `settle_on_completion`，风险极低，至少应挂债。

## LOW

- **L1 Duplicated Code 搬了家**：T16/T19/T17 三处手搭同形状力度序列，T19 注释自承「同 T16 夹具」。正是本次在生产侧修掉的那条臭味。
- **L2 非对称**：Invalidated 抽了 `invalidate`，Confirmed 仍在 :738-743 内联写状态/钟/修订。
- **L3 教义绑定停在契约层**：创新高预滤只在 provider 侧（level_view.rs:754-766/831-836、nest_lifecycle.rs:948/956），`advance` 不校验输入，新测试手搭 window 绕过它。模块头已如实登记该口径差，属登记非违规；提示 #426 须保证喂入来自产窗侧。
- **L4** `assert!(invalidated <= last_as_of)`(:875) 近乎构造性为真（同 prefix 刚置 `last_as_of = as_of`），信息量低。

## 票体点名四项

1. **测试名副其实**：T16/T17/T18/T19 全部名实相符。T16 带反例臂（c 窗真弱⟹Confirmed），(a) 非构造性恒真；T17 三项两两相反 + 分桶 1:1:0，非同一断言蒙混；T18 靠 `book.get(&bridged).is_none()` + 注记里 `last_as_of=100`（既有身份的）把分支钉死在桥匹配上，成立；T19 二段式（先不判负、补数据后才结算）。唯一瑕疵：T16 把反例塞进同一 test。testing.md 的 RED→GREEN 顺序无法从单 commit 核验，登记为不可核验。
2. **注释 vs 实装**：新增声明未超出能力——模块头把「本模块判据 = 力度谓词从未可证，非逐字未创新高」显式登记，`NeverConstituted` 变体文档同时给出教义锚与真实判据。反方向的陈旧见 M1。
3. **release 构建**：见 M2。本次未新增 `debug_assert!`，未新增 `#[cfg(debug_assertions)]`。
4. **`invalidate` 是否唯一写入点**：**是**。全文件 `state = NestEventState::Invalidated` / `invalidated_at =` / `invalidated_reason =` / `force_evidence =` 各仅一次，均在 :322-325；三调用点（:701 / :757 / :783）无旁路。Confirmed 终态不走它，但 docstring 只声明 Invalidated，未越声明。

## 对实装工位自评的复核

1. 090 声明超出能力已改 —— **同意**（登记 4 由 `invalidate` 结构性兑现）。
2. Duplicated Code 已改 —— **部分同意**：生产侧已消，测试夹具复现（L1），不算清零。
3. 文件超长登记不改 —— **不同意口径**：实测 2212 行，且非测试段已超 800，明文上限，登记 ≠ 豁免。
4. `advance` ~210 行登记不改 —— **部分不同意**：实测 194 行，第 7 步可低风险抽出。

未运行 cargo（并行工位注入中，工作树已见 M1 变异 `else if false`）；以上结论均为静态。
