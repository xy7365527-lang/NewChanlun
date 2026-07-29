# #475 复审：#495 核销（`103dbfb355`）

## 结论

**维持打回。最高等级：HIGH。**

#495 已逐项核销首轮 `HIGH-1/S1`、`HIGH-2`、`HIGH-3`、`HIGH-4`，归因清扫也通过；Spec 轴没有残留。  
但 T3 §3 对“历史证词 / 当前口径”的处理仍形成一组同根 HIGH：`103dbfb355` 把
`/tmp/423_fee_decomp.md` 当时真实的 `fee_quoter[当前:124起] / assert![当前:128]` 直接倒写成
`:175`，同时仍无订正地称脚本侧 tax “不是手抄”，与本提交已经承认的“手工镜像”相反。
这没有按“旧证词保留 + 显式订正”区分历史与当前，违反 090，故总体不能 PASS。

## 审计边界

- 工作区：`/tmp/kimi-nest-mainline`
- 核得 HEAD：`103dbfb355 docs(chanlun): #495 修复 #475 打回——随机对照三值落盘 + 订正块 ×8 + 行号/脚本声明/指纹订正`
- 父提交：`f695e6a8c81273a90b0cd5644a5b907a4d69de61`
- 被审 diff：5 文件，`+121/-14`
- 全程未做 git mutation，未改仓内文件。
- 共享工位既有修改 `chanlun/agent-roster-2026-07-21.md`、
  `rust/src/theta_v0/classifier/level_view.rs`、
  `rust/src/theta_v0/classifier/nest_lifecycle.rs` 及既有未跟踪文件均未触碰、未归因。

## 逐项核销

| 项 | 结论 | 独立证据 |
|---|---|---|
| HIGH-1/S1：九格三值、LCB/三态、订正块 | **PASS** | T3 §14.8 九格与 `/tmp/49x_arm{R,D,C}_report_{p3fold,wf7,wf8}.md` 逐格一致；LCB/三态与 T3 §14.4 为 9/9 一致，T2 §13.8 与 §13.3 为 3/3 一致。T3 §4.5、§14.7 与 T2 §3、§13.6 的订正块均明确旧声明在当时代码态/19 列产物下为真、收尾轮 F 后失效，且保留旧证词、不倒写当前能力。 |
| HIGH-1/S1：独立 wf8 抽跑 | **PASS** | 隔离命令 exit `0`；`/tmp/rev475_armR_wf8.md:23` 为 `否 / 0.6753 / 0.4436`，同时 `R=+6851062`、`LCB=-2105181`、`INCONCLUSIVE`，与 T3 §14.8 臂R/wf8 逐位一致。 |
| HIGH-1/S1：dump 中性 | **PASS** | `/tmp/rev475_opsem_wf8/{trades,tower_events}.jsonl` 对 `/tmp/m8_win_gate/wf8/` 均 `cmp SAME`。SHA-256 分别为 `0a744e228ce37640f3681c3f4dc494618f0c475c8bd3f1f0092c91d5d6455b87`、`1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f`，两侧相同。 |
| HIGH-2：行号与函数锚 | **PASS** | 最终 blob 中 `fee_quoter` 在 `treasury.rs:175`，`assert!` 在 `:179`。源码自述在 `:158` 写成“当前 `:179` + `fn fee_quoter`”；脚本 `:62`、`:118`、`:185-200` 均使用 `:175/:179` 并保留函数锚。T3 另两处订正的 `wverify_run.rs:1845-1849` 与 `:2273-2276` 也和实际代码一致。 |
| HIGH-3：手工镜像声明 | **PASS** | `ARM_CALIBRATED_TAX_BPS = 0.0` 在脚本 `:64`，并在 `:148-150` 传给 D/C、`:162` 传入 `decompose`。脚本输出文字 `:196-200` 明说“手工镜像、上游改变须人工同步、不会自动感知”，与实现一致。 |
| HIGH-4：指纹口径清扫 | **PASS** | 旧 `1966/1976/1979/1980` 只作为历史运行/共享工位读数保留。T3 `:977-983`、T2 `:596-602`、OKLO `:257-264` 均紧邻订正为干净快照 debug `1957/4/135`、release `1960/1/135`、相对 1944 净增 `+16`；明确禁止用共享工位读数计算本票净增。 |
| 归因准确性 | **PASS** | #475 订正共 8 处：T3 4、T2 3、OKLO 1，均标 #475/#495。`订正（#481` 唯一命中为 OKLO §2.9 `:247`，属于 #484 回归计数的正当归因。 |

### 九格随机对照三值对拍摘要

| 窗 | R | D | C |
|---|---|---|---|
| p3fold | 否 / 0.9181 / 0.8372 | 否 / 0.9191 / 0.7882 | 否 / 0.9151 / 0.8561 |
| wf7 | 否 / 0.8032 / 0.9920 | 否 / 0.8122 / 0.9870 | 否 / 0.8082 / 0.9760 |
| wf8 | 否 / 0.6753 / 0.4436 | 否 / 0.6733 / 0.4236 | 否 / 0.6733 / 0.4216 |

九格 `theta_beats_random` 全“否”；九格 LCB 全 `≤0`。p3fold/wf7 为 `无(R≤0)`，
wf8 为 `INCONCLUSIVE`，与 T3 §14.4、T2 §13.3 一致，没有把读数冒充 alpha 论据。

## 新发现

### HIGH-N1：T3 §3 未按“保留旧证词 + 显式订正”处理历史与当前口径

以下是同一根因的两项证据，合并计一个 HIGH。

#### A. 历史跑批表头的旧行号被静默改成当前行号

`103dbfb355` 直接把
`chanlun/review-results/treasury-reverify-20260727.md:245` 从：

```text
treasury.rs fn fee_quoter[当前:124起]
```

改为：

```text
treasury.rs fn fee_quoter[当前:175起]
```

但该句的主语不是“当前脚本/当前源码”，而是 `:238` 明确命名的历史命令产物
`/tmp/423_fee_decomp.md`，并在 `:244` 明说“本次复跑的脚本表头里”。只读产物
`/tmp/423_fee_decomp.md:1` 实际仍逐字为：

```text
treasury.rs fn fee_quoter[当前:124起]内 assert![当前:128]
```

因此当前报告把历史产物描述成了它从未输出过的 `:175`。这不是后续 HEAD 漂移问题，而是被审
commit 对历史证词的直接倒写；同段 `:243` 还明列“原表不删、不改（090）”。按首轮已经采用的
“数字/行号事实错误 = HIGH”标准，本项为阻断项。

#### B. 同段仍称 D/C tax“不是手抄”，与当前脚本及本提交自己的订正相反

T3 `:241-243` 仍无订正地写：

```text
臂D/C 是标定档下 treasury::fee_quoter 内 assert! 强制的推论，不是手抄
```

但脚本 `:64` 明确写死 `ARM_CALIBRATED_TAX_BPS = 0.0`，并在 `:149-150` 把这个 Python 常量传给
D/C。本提交修改后的脚本 `:186`、`:197-200` 也已经正确承认它是 Rust 约束的“手工镜像”，上游改变
须人工同步，脚本不会自动感知。因此 T3 的“不是手抄”并非“当时为真、后来失效”，而是一直不符合
实现；可以保留为历史错误原句，但必须追加撤销该说法的订正块。当前没有。

建议修复口径（本次只读，未实施）：保留历史句的 `:124/:128`，随后追加明确订正块：
“`/tmp/423_fee_decomp.md` 生成时表头为 `:124/:128`；当前 `103dbfb355` 的真实位置为
`fn fee_quoter :175 / assert! :179`；脚本 tax 常量是 Rust 约束的手工镜像，须人工同步、不会自动感知；
旧产物与旧误称保留为历史证词，不静默倒写。”

复现：

```bash
nl -ba chanlun/review-results/treasury-reverify-20260727.md | sed -n '234,248p'
nl -ba /tmp/423_fee_decomp.md | sed -n '1p'
git diff 103dbfb355^ 103dbfb355 -- \
  chanlun/review-results/treasury-reverify-20260727.md
```

## 两轴结论

### Standards

**打回 / HIGH。** 首轮标准项本身均已核销，但 HIGH-N1 违反 090 的历史证词纪律。
`git diff --check 103dbfb355^ 103dbfb355` 为 exit `0`，Python 脚本 AST 解析通过；格式/语法无新增问题。

### Spec

**PASS。** 九格三值、LCB/三态、订正块、行号锚、脚本镜像声明、指纹口径与归因清扫均满足本轮核销清单；
未发现缺失、部分实现、错误实现或范围蔓延。

## 实跑与退出码

1. 臂R/wf8 隔离跑批：
   - 命令：题面给定命令，`M8_REPORT_PATH=/tmp/rev475_armR_wf8.md`，
     `OPSEM_DUMP_DIR=/tmp/rev475_opsem_wf8`
   - exit：`0`
   - 结果：`1 passed / 0 failed`；三值 `否 / 0.6753 / 0.4436`
2. 臂R digest 回归门：
   - `python3 scripts/check_armR_trades_digest.py`
   - exit：`0`
   - p3fold `149 / 0x6bf47daf0aa737cd`
   - wf7 `196 / 0x282c28ca8ca65e16`
   - wf8 `168 / 0xcafa7c4846c762cd`
3. 当前共享工位 `cargo test --lib`：
   - exit：`101`
   - 指纹：`1980 passed / 4 failed / 135 ignored`
   - 失败集：
     - `theta_v0::backtest::runner::tests::lee_m3_attribution_dimension_is_readable_and_not_residual_only`
     - `theta_v0::backtest::runner::tests::lee_m4_cap_on_sparsity_has_no_unexplained_violation`
     - `theta_v0::backtest::runner::tests::lee_m4_level_cap_narrows_position_when_enabled`
     - `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`
   - 与题面预期指纹相同；这是共享工位现状，只作回归边界，不归因给 `103dbfb355`。

## 未覆盖声明

- 独立重跑仅覆盖臂R/wf8；D/C 与其余窗口采用现存九份 `/tmp/49x_*` 报告、对应九份 exit-0 日志及
  T3/T2 表格做逐项对拍，没有重新跑九臂窗。
- 没有重复制作 `c0c1a74b16` 的干净 archive 并重跑 debug/release 全量；本轮只核查三份订正是否统一采用
  首轮独立快照的 `1957/4/135`、`1960/1/135`、`+16` 口径。当前共享工位的 `1980/4/135`
  未用于净增计算。
- 未重跑 release 全量 `cargo test --release --lib`；题面要求的当前 `cargo test --lib` 已执行并照实登记
  exit `101`。
- 未验证未来代码移动后数字行号仍有效；本轮按修复 commit 的固定 blob 核验，并要求函数锚作为稳定语义定位。
