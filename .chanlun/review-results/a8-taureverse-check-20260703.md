# A8 τ^reverse 残留复核（提前销项，双侧验证）

## 结论

**非零命中（当前分支侧）**——A8 不满足"两侧均零命中"提前销项条件。但 12 处命中经逐条核查，
全部落在**注释/文档字符串**（`///`、`//!`、`//`），无一处是活代码标识符（无 `tau_reverse`/`TauReverse`
命名的变量/函数/类型）。这些注释是对 τ^reverse 口径**已废弃历史**的记录性描述（G4 #134/#135 终裁
+ #137 口径降级标注），不是残留的旧口径实现。

**处置建议**：不阻塞 merge。这些注释是谱系记录（describing what was replaced），删除会丢失
"typed exit 替换 τ^reverse" 的因果说明，保留即可。main 分支侧零命中是因为 main 尚未拥有这些文件
的这批内容——merge 后 main 会继承这 12 处注释，而不是清零，这与最初"merge 后自然零命中"的预期
不符，需订正预期：**A8 的销项条件不是"文本命中归零"，而是"τ^reverse 是否仍作为生产路径的活代码
参与 alpha 结论"**——经查不是（见下）。

## 逐条残留点

| # | 文件:行 | 性质 | 内容摘要 |
|---|---------|------|---------|
| 1 | `runner.rs:1073` | 注释 | doc comment，说明 `build_mu_from_bars` 替换 τ^reverse |
| 2 | `runner.rs:1189` | 注释 | doc comment，点名废弃 τ^reverse 平行简化状态机（G4 终裁引用） |
| 3 | `runner.rs:2041` | 注释 | 行内注释，PDF §9 typed exit 替换 τ^reverse |
| 4 | `runner.rs:2081` | 注释 | doc comment，对比 τ^typed vs τ^reverse 的可观测差异 |
| 5 | `l3_delta_r_alpha.rs:41` | 注释 | 模块级 `//!` doc，PDF §9 点名废弃 τ^reverse |
| 6 | `l3_delta_r_alpha.rs:119` | 注释 | doc comment，"替换"废弃的 τ^reverse |
| 7 | `l3_delta_r_alpha.rs:163` | 注释 | 行内注释，"τ^reverse 平行简化状态机已删除" |
| 8 | `l3_delta_r_alpha.rs:1887` | 注释 | doc comment，τ^reverse → typed exit 语义变更量级证据 |
| 9 | `econ_positive.rs:34` | 注释 | 模块级 `//!` doc，τ^reverse 定义（历史口径说明） |
| 10 | `econ_positive.rs:36` | 注释 | 模块级 `//!` doc，"整体删除 τ^reverse"（生产路径） |
| 11 | `econ_positive.rs:221` | 注释 | doc comment，L2 诊断分解引用 `legacy_reverse_exit_diagnostic` 标注 |
| 12 | `econ_positive.rs:498` | 注释 | 行内注释，"τ^reverse 口径（PDF §9 点名废弃）" |

## 延伸核查：τ^reverse 语义是否仍以活代码存在

`legacy_reverse_exit_diagnostic` 不是函数名（无 `fn legacy_reverse_exit_diagnostic` 定义），
是模块头注释里的**口径降级标注**（#137，G4 #134 移交处置）。实现 τ^reverse 语义（"持有到下一
反向新确认信号出场"）的活代码是 `fn pair_signals`（`econ_positive.rs:492`），有两处调用：

- `econ_positive.rs:236`（`decompose_capturable_spread`，L2 逐信号可捕获价差分解——诊断路径）
- `econ_positive.rs:4280`（测试）

两处调用均在**已标注为 L2 诊断/非生产 alpha** 的路径内（模块头 `//!` 明确声明生产路径已整体
删除 τ^reverse，替换为 typed exit）。即 `pair_signals` 是**活代码**，但其作用域是诊断可观测性
（对比新旧口径差异），不参与生产 π ledger / alpha 结论——与 [[project_g4_typed_exit_landed]]
记录的"build_mu 重接生产 π ledger"一致，无需清理。

## 边界条件

若后续发现 `pair_signals` 的输出被任何生产 alpha 计算路径（非诊断）消费，则本判定翻转——
需要把 `pair_signals` 也计入 A8 残留清理范围。当前复核未发现此类消费路径。

## 可复算命令

```bash
# 当前分支侧
grep -rn "tau_reverse\|τ\^reverse\|tau\^reverse\|TauReverse" rust/src/

# main 分支侧
git grep -n -E "tau_reverse|τ\^reverse|tau\^reverse|TauReverse" main -- rust/src/
```

## 影响声明

纯复核产出，未改动任何代码。仅新增本复核记录文件。
