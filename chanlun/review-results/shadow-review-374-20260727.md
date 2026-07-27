# 影子评审：#374 fee_rate 口径对称化（issue #377）

- **评审对象**：commit `c19a6b5f75`（9 文件 +474/−212），分支 `kimi-nest-mainline-20260717`
- **评审面**：worktree `/tmp/nc-review-374`（detached @ `8de61b94e1`，`git status` 干净）
- **模式**：独立影子评审（新上下文，禁自评；快速模式，票面范围内）
- **裁决**：**PASS**（Spec ①–⑥ 全部坐实；Standards 三面成立；另记 3 条 LOW 措辞/覆盖尾巴，均不阻塞）

## 0. 复跑基线（实测，非采信自述）

```
cargo test --release --lib
→ 1928 passed; 1 failed; 134 ignored
唯一失败 = theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard
          （bit-exact 摘要 0xe6a2… vs oracle，#115 线在案，与本 commit 无关面）
```

定向：`venue_fee` / `datum` / `scalar_cost_rate` 31 passed / 0 failed；
`per_share_cash_rejected_open_underestimates_fee` 单点 1 passed。
datum 哈希独立复核（不经本仓代码）：`shasum -a 256 -c *.sha256` → 两份 OK。

## 1. Spec 轴

### ① 否 A 的四条理由是否成立 —— **成立**

读消费面坐实（非采信 resolution 文字）：

- `metrics.rs:456 random_entry_controls`：δ 位移入场（`entry_bar+δ`，`metrics.rs:501-516`），
  `t.qty` 不变、`prices[e]` 变，费用按 `qty·px·(1+fee_rate)`（:481-482、:494）线性施加。
  per-share 档费率含最低佣金托底 / 名义额上限 / 卖出监管费（`datum_io.rs:121-160` 字段表
  + `venue_fee.rs` 解析），确为 (qty, px, side) 的非线性函数 ⟹ 任何取自**实际路径**的标量
  （含 Σfee/Σnotional）在反事实臂上仍错。**理由①成立**，A 方案确属 090 声明膨胀。
- `l3_delta_r_alpha.rs:860+ rebuild_cost_series`：返回长度 `n_bars` 的**逐 bar** 成本序列，
  标量只能保总额 ⟹ **理由②成立**。
- 铺 (qty, px, side) 缝的规模：`fill.rs` 费率解析点 4 类（`order_fee_rate` ×3、
  `leg_fee_rate` ×4、`forced_flatten_fee_rate` ×2）+ 双腿账本 + overlay ⟹ **理由③成立**（超小修批）。
- 先例 `tax_bps` 双计 `assert!`（`treasury::fee_quoter`）⟹ **理由④成立**（同级处置，090 一致性）。

### ② `scalar_cost_rate` 的 fail-loud 覆盖四处产出点 —— **无旁路**

`RunResult::fee_rate` 的**全部**赋值点 = `runner.rs:246 / 323 / 548 / 738`（分别对应
`run_theta_v0` / `run_theta_v0_dual` / `run_theta_v0_pi_inner` / `run_theta_v0_pi_overlay`），
四处均改经 `treasury::scalar_cost_rate`。grep 全仓 `RunResult {` 构造点仅这 4 处（+`OverlayRunResult`
复用 `net_result`），无第五处、无手抄三常数残留（`commission_bps + slippage_bps` 在 `theta_v0/`
生产路径已清零，剩余命中均为 bin/研究跑批/`exec.rs` 与本票无关面）。
`assert!` 消息命中 `should_panic(expected = "无良定义")`，测试实跑通过。

### ③ 消费面有效域收窄与实际能力一致（090） —— **一致**（附 LOW-1）

`RunResult::fee_rate`（runner.rs:136-140）、`metrics::significance`（metrics.rs:317-320）、
`rebuild_cost_series`（l3:859-863）、`config.rs:285-288` 四处登记均为**收窄**（"仅 None 档"
+ "标定档构造期 panic" + "解锁路径 = 接缝改用 `fee_quoter`"），无一处声明代码不具备的能力。

### ④ 双标签与 `wverify_run` 同法 —— **同法**

`runner.rs:5428-5434`：格式串槽位一为持有成本、槽位二为成交费率；实参顺序
`RATE_UNCALIBRATED_LABEL` → `rate_calibration_label(&config.exec)`，**槽位与实参对齐正确**。
与 `wverify_run.rs:1136-1137 / 1260-1261` 完全同法（成交费率走构造子、持有成本三项恒 L1），
未跳级、未硬编。runner 内已无第二处未升级的费率标签点（grep `RATE_UNCALIBRATED_LABEL`
在 runner.rs 仅 :5433 一处）。

### ⑤ per-share 拒单测试三锁 —— **方向锁死，反证锁死，量级为单侧下界**（附 LOW-2）

`fill.rs:3656-3705`：方向 `fee_close_quoted < fee_close_correct`（低估）✓；
反证 = 同 orders 走未标定档 `approx(paid_flat, 2·10·PX·3e-4)`（费率与询价量无关）✓；
实付恒等式 `paid == fee_open + fee_close_quoted` 把"按整单量询价"这一机制本身钉死 ✓。
量级仅 `> 5.0×` 单侧下界（见 LOW-2）。认识论标注 L1（合成 bar 费用算术，不主张 alpha）✓。

### ⑥ provenance.md 与 datum 内容一致 —— **逐项一致**

| 核对项 | 结果 |
|---|---|
| venue / 取数日期 / source_url ×2 | 与两份 JSON 字段逐字一致 |
| sha256 ×2 | 与 `.sha256` sidecar 逐字一致，且独立 `shasum -c` OK |
| `per_share` 必填 10 字段表 | 与 `datum_io.rs:117-162` `need()` 调用集合一一对应 |
| 一手数字（0.1%/0.075%、$0.0035/股、min $0.35、1% 上限、SEC 0.0000206、TAF $0.000195/上限 $9.79） | 与 JSON `note` 及条目数值一致 |
| "不改动 datum 与 sidecar 任何字节" | commit stat 仅新增 `.md`，datum 未入 diff ✓ |
| 有效域四条（slippage 未标定 / spot 口径 / 未列品种 fail-loud / maker 档不用 / 注入通道未落） | 与代码现状一致（`ExecConfig::default()` 恒 `None`，CLI 无注入参数） |

## 2. Standards 轴

- **datum_io 逐字搬迁声明** —— **坐实**。把 diff 两侧行集去空白排序后比对：venue_fee.rs 删除侧
  199 行**全部**出现在 datum_io.rs 新增侧，唯一"删除侧独有"是留在宿主文件里被改写的那行
  mod 文档（加了 "实现在 `venue_fee/datum_io.rs`" 指针）。新增侧独有 = 模块头 + LOW-G 的 4 条
  Err 测试，属票体要求的新增，非搬迁中夹带的语义改动。
- **模块内聚** —— `venue_fee.rs` 610 行（< 800 硬上限），`datum_io.rs` 274 行；切面沿既有
  `#[cfg(any(test, feature = "backtest_bin"))]` 边界，职责单一（读文件 + sha256 + JSON→Book），
  `pub use datum_io::{datum_dir, load_datum}` 保持对外路径不变（下游零改）。
- **provenance 入 `-f` 的登记照实** —— `analysis/data_cache/` 被 gitignore，commit 用 `-f` 强制入库；
  文件确在 HEAD 树内且工作区可读；模块头（venue_fee.rs:28-30）加了溯源件指针。登记与实际一致。

## 3. LOW 尾巴（不阻塞，建议并入下一小修批）

1. **LOW-1（090 论证完备性）**：`metrics.rs:320` 称"上游 `RunResult::fee_rate` 已 fail-loud 挡住，
   本函数不会收到标定档的成本口径"。但 `metrics::significance` 另有**不经 `RunResult`** 的调用方
   ——`src/bin/pi_bsp_timing.rs:621`（费率由 :519-520 手抄三常数就地算出），
   `l3_pi_falsify.rs:154`、`runner.rs:6954/7031/7183` 则走 `res.fee_rate`（已被挡）。
   结论**当下仍为真**（bin 侧只能是 `None`：`ExecConfig::default()` 恒 `None`、CLI 无注入参数），
   但保护链的**依据**不完备：注入通道一旦落地，bin 侧手抄路径不受 assert 覆盖。
   建议措辞改为"经 `RunResult` 的路径已挡住；bin 侧手抄费率路径的口径由 `None`-only 现状保证"，
   或把 bin 的两处手抄改调 `scalar_cost_rate`（后者顺带消掉手抄源）。
2. **LOW-2（测试声明 vs 断言强度）**：`fill.rs` 该测试注释称锁"量级（总费率 ≈5.1×）"，实际断言只有
   单侧下界 `fee_close_correct > fee_close_quoted * 5.0`；上界漂移（如重构后变 50×）不会被捕获。
   建议补上界（如 `< quoted * 6.0`）或把注释改为"下界 ≥5×"。
3. **LOW-3（Err 分支登记不完备）**：`datum_io.rs:250-256` 把 `build_book` 的构造期校验枚举为
   4 条（未知 unit / 重复 / 空簿 / `need()` 两支），但同函数还有一条独立 Err
   ——`max_commission_frac_of_notional <= 0`（:128-130），既未入枚举也无测试。
   票体只要求"4 条"，故不算未完成项；但"直击 `build_book` 内的构造期校验"这句略强于实际覆盖（090）。

## 4. 结果包六要素

1. **结论**：#374（`c19a6b5f75`）两轴 PASS；MED-A 取 B-强化的裁定在消费面上可坐实，非事后合理化；
   MED-B / LOW-C…G 全部落地且与代码现状相符。另记 3 条 LOW 措辞/覆盖尾巴。
2. **定义依据**：231号（有效域 ⊊ 定义域）——单标量费率仅在 `fee_schedule = None` 上良定义，
   per-share 档 (qty,px,side) 非线性 ⟹ 不存在使"随机对照含同等成本"成真的常数；
   090（声明膨胀禁止）——四处消费面登记均收窄而非膨胀；`.claude/rules/common/coding-style.md`
   800 行上限（610 + 274）。
3. **边界条件**（结论翻转条件）：① 若 datum 注入通道落地（CLI/配置面接 `fee_schedule = Some`），
   则 bin 侧手抄费率路径（`pi_bsp_timing.rs:519`、`pure_bsp_timing.rs:83`）绕过 assert，
   LOW-1 升级为 MED；② 若 `RunResult` 出现第五处构造点而不经 `scalar_cost_rate`，② 项判定翻转；
   ③ 若 `#115` 线那条 bit-exact guard 的失败被证明与本 commit 相关（现读：signal 分类器摘要，
   与费率面无数据依赖），基线判定翻转。
4. **下游推论**：标定档下 `l3_delta_r_alpha` 鞅守卫与 `metrics` 随机对照**双双不可用**
   （构造期 panic）——解锁前提是这两个消费面先接 (qty, px, side) 缝、改调 `fee_quoter` 逐笔重算；
   在此之前任何"标定档 alpha 读数"都不可产出，属另票。
5. **谱系引用**：231号（有效域/认识论等级）、090号（严格性/声明膨胀）、#303（venue = spot）、
   #360（单源门面 `fee_quoter`）、#370（本批来源评审）。未发现本票触及生成态定义分歧。
6. **影响声明**：本评审只读不写；唯一落盘 = 本报告。评审面 worktree `/tmp/nc-review-374` 未做任何
   git mutation，工作区仍干净；`/tmp/kimi-nest-mainline` 未进入。
