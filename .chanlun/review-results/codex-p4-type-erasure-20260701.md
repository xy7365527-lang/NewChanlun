# P4 类型信息抹除——代码层异质审查诊断

> **审查模式**：diagnose（trust-but-verify，直接读代码）  
> **时间**：2026-07-01  
> **环境说明**：Codex CLI 二进制丢失（`ENOENT: aarch64-apple-darwin/codex`），以代码层异质审查工位身份手动执行，认识论 L0（代码结构事实）。

---

## 诊断结论

**P4 缺口真实存在，但性质与原审计假说不同。**

原审计假说："`interp.rs:214` 算了 min_class 但**没透传**到台账/信号"。

**trust-but-verify 修正**：
- `interp.rs` 的 `min_class` 确实算了，`Candidate.bsp_class` 也存了
- 但 `econ_positive.rs` **不消费** `Candidate.bsp_class`，改用 `bsp_disc`（6位掩码）独立计算
- `bsp_class` 字段已出现在 `SignalDecomp` 结构体 + CSV 输出列
- **真实缺口**：μ̂ 分桶键是 `(level, δ)`，不含 `bsp_class`——字段存在但分层未做

---

## 四问逐条判定

### Q1：bsp_class 字段存在 + CSV 有输出，算"骨架"还是"完整透传"？

**判定：骨架，非完整。差距精确定位如下：**

| 层级 | 现状 | 完整所需 |
|------|------|----------|
| 数据携带 | SignalDecomp.bsp_class 已存在（`econ_positive.rs:86`） | 已完成 |
| CSV 输出 | bsp_class 列已在 CSV（`:688`） | 已完成 |
| μ̂ 分桶 | 分桶键 `(level, δ)` 不含 bsp_class（`:503`） | **缺失** |
| per-class 检验函数 | `class_actual_pnl` 只按 `(level, delta)` 过滤（`:734`） | **缺失** |

字段透传完成，分层检验未做 = 骨架完成、检验层缺失。

---

### Q2：μ̂ 分桶键不含 bsp_class，是否构成真实的混合池稀释？

**判定：真实存在，且有代码证据。**

代码证据：
```rust
// econ_positive.rs:503-505
type Bucket = (usize, f64, f64, f64, f64, f64, usize, f64, f64, usize);
let mut buckets: BTreeMap<(u32, i8), Bucket> = BTreeMap::new();
for d in &decomps {
    let e = buckets.entry((d.level, d.delta)).or_default();  // 键=(level, δ)，bsp_class 忽略
```

混合效应的具体形式（L0，结构必然性）：

- `(level=0, δ=+1)` 桶里混入：bsp_class=0x01（仅buy1）、0x04（仅buy3）、0x05（buy1+buy3）等所有买侧类型
- 若 buy1 信号有正 α、buy3 信号有负 α，两者在同一桶内互相抵消
- 桶级 μ̂ = 混合平均，单类型 α 不可辨别

---

### Q3：Candidate.bsp_class（min_class）vs SignalDecomp.bsp_class（bsp_disc）——语义不一致是否问题？

**判定：语义分裂，但 econ_positive 路径选对了；问题是 Candidate.bsp_class 是孤儿字段。**

| 字段 | 位置 | 值语义 | 多类同时成立时 |
|------|------|--------|----------------|
| `Candidate.bsp_class` | `interp.rs:79` | `min_class()` = 最小类号（1/2/3/MAX） | **有损**：buy1+buy3 → 返回1，buy3信息丢失 |
| `SignalDecomp.bsp_class` | `econ_positive.rs:86` | `bsp_disc()` = 6位掩码 | **无损**：buy1+buy3 → 0x05，两位都置1 |

`econ_positive.rs` 信号收集路径直接用 `bsp_disc(&p.bits)` 绕过了 `Candidate.bsp_class`（代码证据：`:193`）。

技术上 `econ_positive` 路径的类型信息完整。但 `Candidate.bsp_class` 这个字段存的是有损的 min_class 版本，且在下游任何地方都没被消费（既不被 `econ_positive` 读，也不被其他已知模块读）——这是一个孤儿字段，存在语义误导风险（名字叫 bsp_class 但语义是 min_class）。

---

### Q4：最小改动让每类型 α 可独立检验？

**判定：把分桶键改成 `(level, δ, bsp_class)` 是充分的最小改动，无需拆解 6 个独立类别。**

理由：
- `SignalDecomp.bsp_class` 存的是 6 位掩码（bsp_disc），已完整保留类型组合信息
- 分桶键加入 `bsp_class` 后，同一信号的全部类型组合（如 0x01=仅buy1 vs 0x05=buy1+buy3）自然分开
- 对于"纯类型"分析（只看1买 vs 只看2买），可后处理过滤 bsp_class == 0x01 vs bsp_class == 0x02

需要修改的位置：

1. **`econ_positive.rs:503`**：分桶键从 `(u32, i8)` 改为 `(u32, i8, u8)`
   ```rust
   // 改前
   let mut buckets: BTreeMap<(u32, i8), Bucket> = BTreeMap::new();
   let e = buckets.entry((d.level, d.delta)).or_default();
   
   // 改后
   let mut buckets: BTreeMap<(u32, i8, u8), Bucket> = BTreeMap::new();
   let e = buckets.entry((d.level, d.delta, d.bsp_class)).or_default();
   ```

2. **`econ_positive.rs:734` `class_actual_pnl()`**：加 bsp_class 过滤参数（可选，供 OOS 测试用）

3. **CSV 报告输出**：`bsp_class` 列已存在（`:688`），报告表头/行加 bsp_class 维度即可

---

## P4 现状总结

| 维度 | 状态 |
|------|------|
| bsp_class 字段存在于 SignalDecomp | ✓ 完成 |
| CSV 含 bsp_class 列 | ✓ 完成 |
| 信号收集用 bsp_disc（6位，无损） | ✓ 完成（选对了） |
| μ̂ 分桶按类型分层 | ✗ **缺失**（分桶键仅 (level, δ)） |
| per-class OOS 检验支持 bsp_class | ✗ **缺失** |
| Candidate.bsp_class 孤儿字段 | △ 风险（min_class 有损，但无下游消费者） |

**缺口定性**：实现错误（未完成），非定义冲突。修复不需改规格/定义，只需扩展分桶键。

---

## 认识论标注

**L0**（代码结构事实）：所有判定来自已读代码，不依赖市场数据。  
- 混合池稀释的"真实性"在 L0 层是结构必然（分桶键决定什么会混合）
- α 被稀释的**程度**是 L2（需要真实数据回测，当前无读数）
- 类型分层后 μ̂ 是否分歧是 L2（需要真实数据，当前未跑）

---

## 边界条件（翻转判据）

以下条件成立时判定可翻转：
1. 存在某个已读的消费 `Candidate.bsp_class` 的下游模块（排查需 grep 全库）
2. 某测试断言了 `(level, delta, bsp_class)` 三键分桶的正确性（排查需 grep 测试文件）
3. bsp_class 在信号层有别的分层路径（排查需检查 signal/ 目录）
