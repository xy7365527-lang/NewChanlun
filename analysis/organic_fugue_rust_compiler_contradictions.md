# 有机赋格 v2 Rust 实装 — 编译器暴露的矛盾记录（"逼迫澄清"实例集）

> 编排者要求（2026-06-10 追加）："每个设计决策在 Rust 中遇到编译器质疑时，完整
> 记录编译器暴露的矛盾和你的解决方案。这些编译错误就是概念层矛盾的显现。"
>
> **诚实声明（090号）**：下表区分两类质疑——(A) `cargo build/test` 实际报错；
> (B) 写作过程中被借用检查器/类型系统规则**预先逼改**（错误形态可精确预言，
> 故在落键前重构）。两类的纪律来源同一（编译器的拒绝语义），但 (B) 没有留下
> 报错原文。每条标注类别。认识论等级：本文档为实装过程的一手记录（L0 工程史）。

---

## C-1. 借用检查器 vs "读锚点的同时改账本"（类别 B，E0502 形态）

**位点**：`level_operating_unit.rs` osc 子循环 / `runner.rs` main 腿闭腿。

**矛盾的显现**：Python 原型写法直译为
```rust
match ledger.open_slot(okey) {            // ← &OpenLeg 不可变借用存活
    Some(leg) => {
        if c <= leg.anchor.boundary {
            ledger.close_diff(okey, c, bar);   // ← E0502：&mut 与 & 共存
        }
    }
}
```
借用检查器拒绝：**闭腿决策正在读取的锚点，会被闭腿动作本身销毁**。Python 里
`rec = pos.active.get(key)` 之后 `del pos.active[key]`，`rec` 变成悬挂语义的
活引用——恰好没出事是因为后续不再读 `rec`，这是"约定的正确"而非"结构的正确"。

**概念层翻译**：决策的输入必须是**快照**，不是会被决策结果改写的活状态。
这正是 v1 §5.4 已声明却未机械化的纪律："开腿时读取当时的 frac 快照，腿存续期
内不变"——编译器把同一原则推广到了锚点读取。

**解决方案**：`LegAnchor` 是 `Copy`——`ledger.open_slot(okey).map(|l| l.anchor)`
先复制快照，借用立即结束，决策与执行分离。零运行时成本（16 字节复制）。

---

## C-2. 多字段 `&mut` 共存 vs "谁拥有钱、谁拥有决策"（类别 B）

**位点**：`runner.rs` voice 循环。

**矛盾的显现**：voice 循环同时需要：`&mut pos`（账本）、`&mut run.voices[vi]`
（FSM）、`&mut run.counters`、`&run.book`/`&run.gate`（市场性质只读）、以及
闭包 `frac_of` 捕获 `&run.alloc`。若 `frac_of` 写成
`|k| if structure { run.alloc.frac(k) } else { … }`，闭包捕获 `run` 的字段借用
与同语句内 `run.voices[vi].step(&mut run.counters, …)` 的可变借用是否冲突，
取决于闭包精确捕获规则的边缘行为——结构脆弱。

**概念层翻译**：这是所有权链问题（v1R §4.2）："LOU 不拥有钱"，同理
**LOU 也不拥有规模公式**——它只消费某 bar 时刻的规模读数。让闭包持有分配器
引用 = 让声部在步进中途能看到分配器，越权。

**解决方案**：每 bar 把 `frac` 快照为 `[f64; 11]` 数组（`move` 闭包零借用）。
这与 K6 的"中枢生死事件门控重算、腿 open 读快照"语义完全一致——编译器逼出的
形态恰好是设计文档已有的纪律。11×f64 复制/bar，成本可忽略。

---

## C-3. `parse_direction` 的 `?` 类型错误 vs 两种错误存在论（类别 A，实际报错）

**报错**：`run_organic_rust` 边界解析中 `parse_direction(&direction)?` →
`the ? operator can only be applied to values that implement Try`（该函数返回
`Direction` 并对非法值 `panic!`）。

**概念层翻译**：引擎内部的 `parse_direction` 把非法输入当**契约违例**
（"输入契约由调用方保证"→ panic）；交易层磁带边界把非法输入当**外部数据错误**
（用户可触发 → `PyValueError` fail-fast）。一个函数不能同时是两种错误存在论——
编译器在 `?` 上拒绝，暴露了"复用引擎解析器"这个便利想法混淆了契约层与数据层。

**解决方案**：磁带边界自带 match → `PyValueError`；引擎内部 panic 语义不动。
两种错误处理各守其域（输入验证在系统边界——coding-style 既有要求的实例化）。

---

## C-4. 测试断言失败 vs 十进制直觉（类别 A，实际报错）

**报错**：`assert_eq!(py_round(1.00005, 4), 1.0)` →
`left: 1.0001, right: 1.0`。

**概念层翻译**：我（与多数人的十进制直觉）以为 round-half-even 会把 1.00005
舍向 1.0000；但 IEEE-754 中 1.00005 的最近双精度是 1.0000500000000000038…，
**严格大于**十进制 .00005——正确舍入向上。CPython `round` 给出同样的 1.0001。
测试失败暴露的不是实现 bug，是**断言作者的十进制本体论错误**：f64 世界里
不存在"恰好 .00005"这个数。这正是 bit-exact 纪律为什么必须以"二进制事实"
而非"十进制直觉"为准的微型实证。

**解决方案**：修正测试期望为 1.0001（Rust `{:.4}` 与 CPython 在该值上逐位一致
——py_round 实现本身无误）。

---

## C-5. dead_code 警告群 vs 声明膨胀检测器（类别 A，18 条警告）

**报错**（编译器警告，no-patch 纪律下视同质疑）：`since_bar is never read` /
`entry_price is never read` / `RevLeg.home never read` / `Cand/Nested never
constructed` / `market_mode never read` 等 18 条。

**概念层翻译与分类处置**（每条警告 = 一次"这个声明有消费者吗"的质询）：

| 警告 | 矛盾 | 处置 |
|------|------|------|
| `FatigueState::Fatigued{since_bar}` 未读 | v2 设计稿规定了该字段，但三路清空都不消费它——**设计稿自身含声明膨胀** | 删除字段，doc 注明"需要衰竭时长报告时随消费者一起恢复"（设计稿与代码的分歧显式落盘） |
| `OrganicLedger.entry_price` 未读 | Python 原型"顺手存着"的字段在 Rust 里是非法的——存而不读 = 双真相源候选（runner 已自持 entry_price） | 删除字段；`new()` 参数保留（cost_basis 初值仍需要它） |
| `RevClose::Cand/Nested` 不可构造 | 消融轴 R 在 config 里存在但变体表无入口——**不可达的配置轴 = 假自由度** | 变体表补 VRc/VRn/VR2 入口（轴可达才是轴） |
| `market_mode` 未读 | `MarketMode::Stock` 单变体无 match 点 → F1 期货实装时编译器无处强制表态 | runner 入口放穷举 match（"F1 新增变体时此处必须表态"的锚点） |
| `RevLeg.home` / `RevTranche.open_bar` 未读 | 冗余坐标（home ≡ VoiceUnit.ladder；open_bar 只在腿级有意义） | 删除（单一真相源） |
| `BspEvent.price/seg_idx`、`DivEvent.force_a/c` 等未读 | **不是膨胀**：设计 K3/G-2 明文"force 字段已透传，是力度收敛门控的数据基础" | 保留 + `#[allow(dead_code)]` + 引用设计条款（传输义务 ≠ 死代码，区别在于有预注册的未来消费者） |

**方法论结论**：dead_code 警告是 090号"声明=能力"的机械检测器——但它不区分
"膨胀"与"预注册传输"，区分是概念判断（有无在册消费契约），必须逐条裁决而非
统一 `#[allow]` 压制。

---

## C-6. `&mut` 不可 Copy vs 单写者保证（类别 B）

**位点**：`runner.rs` voice 循环逐声部调用 `step(…, pos, …)`。

**矛盾的显现**：`let pos = run.pos.as_mut()` 得到的 `&mut OrganicLedger` 若按值
传入 `step` 会被 move，第二个声部无账本可用——编译器只允许**隐式再借用**
（函数调用位置自动 reborrow，调用结束归还）。

**概念层翻译**：同一 bar 内账本的写权是**串行令牌**：每个声部步进时独占账本、
结束即归还，不存在两个声部同时改写账本的执行路径。Python 里"恰好没写出并发"
是事实；Rust 里**不可能写出**是定理。40课"每一层次的操作都是独立又在一个整体
的操作中"——"独立"由声部状态隔离保证，"一个整体"由账本写令牌串行化保证。

**解决方案**：无需改动——隐式 reborrow 即正确语义；记录于此因为它是
"借用检查器恰好实现了多声部记账的正确并发语义"的实例。

---

## C-7. PyO3 元组 ≤12 元限制 vs trace 行的范畴结构（类别 B）

**位点**：`run_organic_rust` 返回 diag trace（LegTrace 13 字段）。

**矛盾的显现**：PyO3 的 `IntoPyObject` 只为 ≤12 元组实现——13 字段平铺元组
编译不过。

**概念层翻译**：被迫分组时发现 trace 行本就有两个范畴：**身份坐标**
（槽键/腿类/卖买 bar/价，6 元）与**守恒律算术**（股数/价差/利润/相变前后，
7 元）。平铺是 Python dict 的无结构遗产；类型系统的算术限制逼出了语义分组。

**解决方案**：`((身份6元), (算术7元))` 嵌套——对账脚本按组解包。

---

## C-8. `Option<CenterRef>` vs Python 元组的 None 语义（类别 B，设计偏差）

**位点**：`types.rs` BspEvent。

**矛盾的显现**：v2 设计稿给的是 `center: Option<CenterRef>`（cs/zd/zg 三合一）。
但 Python oracle 的事件是 8 元组，`e[4]`（cs）可独立为 None，且主腿闭腿谓词
`e[4] == anchor[0]` 依赖 **None == None → True** 的 Python 语义（center_gate=False
时主腿锚可携带 None cs）。`Option<CenterRef>` 无法表达"cs 为 None 而比较仍须
逐字段进行"——设计稿的"干净类型"比 oracle 的实际语义**少表达了一个状态**。

**概念层翻译**：bit-exact 对账迫使类型忠于 oracle 的真实语义而非设计的理想形态
——"严格"在这里站在丑的一边。`cs: Option<i64>, zd: Option<f64>, zg: Option<f64>`
三个独立 Option + `Option<i64>` 的 `==`（None==None ⇒ true）恰好逐位复刻
Python 元组比较。

**解决方案**：独立 Option 字段 + 磁带构造期不变量"cs.is_some() ⇒ zd/zg 皆
Some"（fail-fast）——把"实际数据从不出现的组合"显式排除在边界，而非假装
类型已排除。

---

## C-9. `unreachable!` arm vs 类型完备性义务（类别 B）

**位点**：osc 子循环 `match anchor { LegAnchor::SegmentScale => unreachable!(…) }`。

**矛盾的显现**：osc 槽内的腿按构造只能持 `Center` 锚，但 `LegAnchor` 有两个
变体——编译器强制 match 写全。"不可能的分支"必须显式表态。

**概念层翻译**："osc 腿必有中枢锚"是**构造保证**（open 路径唯一），不是
**类型保证**（SlotKey 与 LegAnchor 是独立维度）。编译器暴露了这两种保证的
差距。理想形态是让槽类型携带锚类型（GADT 风格），但代价是账本泛型化——
v1R §4.4 反推测性抽象裁决下不做。

**解决方案**：`unreachable!` + "到达即 bug" 注释——把构造保证的边界标记为
显式的响亮失败点（fail loud > silent wrong），而非静默 no-op。

---

## C-10. 追加轮：借用检查器 vs D3 滚动状态（类别 B）

**位点**：追加要求轮，`runner.rs` D3 稀疏翻转行。

**矛盾的显现**：设计 §7 把 dir_row 声明为每 bar 磁带行（`[Option<Direction>; 11]`
per bar）。直译 = BRN 2.4M bar × 11 × (1+8+8)B ≈ 450MB。所有权视角下这是
**把"状态"物化成了"历史"**：方向行是一个随时间演化的状态（翻转才变），
密集行存储的是它在每个时刻的快照副本。

**概念层翻译**：磁带应记录**事件**（翻转），消费者重建**状态**（滚动数组）——
与中枢账本（事件 ingest → last/dead 状态）同构。run_anchor 随之免费获得：
锚 = 最近翻转 bar，无需独立行。

**解决方案**：`dir_flips: Vec<(bar, ladder, Direction)>` 稀疏行（OKLO 实测
24,417 行 vs 447K×11 密集槽）+ runner 滚动 `dir_state/anchor_state`。
逐 bar 视图与密集行逐位等价。

---

## 结果包（简化版：纯工程史记录）

**结论**：10 组编译器质疑，其中 3 组实际报错（C-3/C-4/C-5）、7 组借用/类型
规则预先逼改；2 组暴露设计稿自身的声明膨胀或表达不足（C-5 since_bar、C-8
CenterRef）；2 组的解决方案与设计文档已有纪律完全重合（C-1/C-2 ←
v1 §5.4 快照纪律）——编译器没有制造新约束，它在拒绝设计自己没遵守的承诺。

**边界条件**：类别 B 条目无报错原文，复现方式 = 还原直译形态后 `cargo build`；
若有人质疑某条"预先逼改"的真实性，按此复现。

**影响声明**：本文档为新增记录文件，不改动任何代码/定义；C-5 的处置已在
`rust/src/trading/` 各文件落地（字段删除/变体补全/allow 标注均带引用）。
