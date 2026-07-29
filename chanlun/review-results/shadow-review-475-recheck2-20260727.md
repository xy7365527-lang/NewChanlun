# #475 二轮复审：#496 核销（`386a5bdabd`）

## 结论

**维持打回。原 HIGH 已核销；新增最高等级：MED（归因漏标）。**

`386a5bdabd` 已正确恢复 T3 §3 的历史快照行号，并以订正块消除了“推论 / 不是手抄”与当前脚本“手工镜像、须人工同步”之间的语义相违。提交范围、当前共享工位测试指纹和臂 R digest 回归门也均符合题面预期。

但本轮硬清单要求订正块归因中 `#496` “各归其票”。新块标题只写
“`#475 复审 HIGH / 编排者亲修`”，三份指定报告内 `#496` 为零命中。该问题是**漏标**，不是交叉误标，也不重开原 HIGH；但它直接使第 4 项验收不能全绿，故总体不能判 PASS。

## 审计边界

- 工作区：`/tmp/kimi-nest-mainline`
- HEAD：`386a5bdabd9077dff8b25343201e1b8ccd42d017`
- 父提交：`103dbfb355c59c0e2b87f241322b70f40f321d9a`
- 被审 diff：`chanlun/review-results/treasury-reverify-20260727.md`，`+8/-1`
- 全程未做 git mutation，未改仓内文件。
- 共享工位既有修改：
  - `chanlun/agent-roster-2026-07-21.md`
  - `rust/src/theta_v0/classifier/level_view.rs`
  - `rust/src/theta_v0/classifier/nest_lifecycle.rs`
  - 既有未跟踪报告与 `rust/src/bin/p409_pan_live_probe.rs`
  
  均未触碰、未归因给 `386a5bdabd`。本报告是唯一写入件。

## 核销证据

### 1. 恢复正确性：PASS

- 固定 blob 的 T3 §3：
  - `treasury-reverify-20260727.md:245` 已恢复为
    `treasury.rs fn fee_quoter[当前:124起]`。
  - `:249-252` 的订正明确区分历史态 `:124/:128` 与 #495 后当前态 `:175/:179`。
- 物证 `/tmp/423_fee_decomp.md`：
  - 首行逐字记录 `fee_quoter[当前:124起]内 assert![当前:128]`；
  - 后文再次记录 `fn fee_quoter（当前 :124 起）`、`assert!（当前 :128）`。
- 对固定 blob 在该段扫描 `当前:1` 系列，只有历史正文的 `:124`，以及订正块用于区分历史/当前的
  `:124/:128/:175/:179`；未发现另一条历史快照被倒写。
- `git show 386a5bdabd` 的唯一删除行是 `当前:175起`，唯一替换行是 `当前:124起`；没有改动别的历史数字。

### 2. 订正块语义：PASS

新块 `treasury-reverify-20260727.md:249-254` 的语义拆分正确：

- `tax=0` 为什么成立：来自 Rust 标定档约束的**推论**；
- Python 中 `ARM_CALIBRATED_TAX_BPS = 0.0` 的取值来源：是该约束的**手工镜像**。

与固定 blob 的脚本现状逐项一致：

- `scripts/fee_account_decomposition.py:64`：常量硬编码为 `0.0`；
- `:149-150`：D/C 两臂均显式传入该 Python 常量；
- `:185-186`：表头称其为 Rust 约束的“手工镜像”；
- `:196-200`：明确“须人工同步”“不会自动感知 Rust 侧变化”。

当前 Rust 固定 blob 中 `fee_quoter` 位于 `treasury.rs:175`，约束 `assert!` 位于 `:179`，也与订正块登记的当前态一致。未发现“自动纳入”“不是手抄”等当前态残留承诺。

### 3. 无新引入：代码/文档范围 PASS，归因标签 FAIL

提交全 diff：

```text
386a5bdabd docs(chanlun): #496 修复 #475 复审 HIGH——T3 §3 历史快照行号恢复 +「手工镜像」相违订正
 chanlun/review-results/treasury-reverify-20260727.md | 9 ++++++++-
 1 file changed, 8 insertions(+), 1 deletion(-)
```

`git diff --numstat 386a5bdabd^ 386a5bdabd` 为 `8  1`；`git diff --name-only` 仅该 T3 报告。
除 `:175 → :124` 的机械恢复和紧随其后的 6 行订正块外无任何改动。
`git diff --check 386a5bdabd^ 386a5bdabd` exit `0`。

#### 当前共享工位 `cargo test --lib`

- exit code：`101`
- 指纹：`1980 passed / 4 failed / 135 ignored`
- 失败集：
  - `theta_v0::backtest::runner::tests::lee_m3_attribution_dimension_is_readable_and_not_residual_only`
  - `theta_v0::backtest::runner::tests::lee_m4_cap_on_sparsity_has_no_unexplained_violation`
  - `theta_v0::backtest::runner::tests::lee_m4_level_cap_narrows_position_when_enabled`
  - `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`

与前轮登记的 `1980/4/135` 及四项失败集一致，未观察到回归指纹扩大。该结果属于含既有 Rust 工作区修改的**共享工位现状**，只作回归边界，不归因给纯文档提交 `386a5bdabd`。

#### 臂 R digest 回归门

命令：`python3 scripts/check_armR_trades_digest.py`

- exit code：`0`
- p3fold：`149 / 0x6bf47daf0aa737cd`
- wf7：`196 / 0x282c28ca8ca65e16`
- wf8：`168 / 0xcafa7c4846c762cd`

三窗均报告逐位无漂移。

### 4. 收尾普查：存在 1 个归因漏标

已照题面执行：

```bash
grep -n "订正（#" \
  chanlun/review-results/treasury-reverify*.md \
  chanlun/review-results/oklo-treasury-three-stage-20260727.md
```

普查结果：

- `#422/#423`：T2/T3 的措辞收窄、列数订正、机制归因均仍归原票；未与读数恢复混标。
- `#475/#495`：T3 四块、T2 三块、OKLO 一块均保持“发现票 / 修复票”双归因；未发现交叉误标。
- `#481/#484`：OKLO `:247` 标 `#481 回归计数`，`:249` 照实登记 `#484` 开工与修复后读数，归因正确。
- `#490/#492`：只作为后续审计节、动态 header、列名域修复的代码态 provenance 出现；未被误标成 #475 的修复票。
- `#496`：三份指定报告中零命中；新块 `treasury-reverify-20260727.md:249` 只写
  `订正（#475 复审 HIGH / 编排者亲修）`。

## 新发现

### MED-N1：新订正块遗漏修复票 `#496` 的显式标签

用户把检查面明确限定为仓内三份报告的订正块，并点名要求 `#496` “各归其票”。commit message 中虽有
`#496`，但不能替代报告块自身的可检索归因；“编排者亲修”是角色说明，不是稳定票号。

这是机械追溯缺口，不是语义错误或交叉误标。最小修复只需把 T3 `:249` 标题收窄为类似：

```text
订正（#475 复审 HIGH / #496 编排者亲修，2026-07-27）
```

Standards 轴按影响定为 LOW；Spec 轴因直接违反本轮显式验收项定为 MED，并作为本轮 acceptance blocker。

## 两轴结论

### Standards

原 HIGH **PASS**；新增 `LOW-N1 ×1`。历史证词、当前态语义、diff 范围和格式均通过；仅缺
`#496` 的机器可追溯标签。

### Spec

**打回 / MED ×1**。核销清单第 1、2、3 项通过，第 4 项因 `#496` 漏标未完全满足。没有发现别的缺失、
错误实现、范围蔓延或交叉误标。

汇总：Standards 1 个 LOW；Spec 1 个 MED。两轴最坏项均为同一处 `#496` 归因漏标。

## 未覆盖声明

- 未重跑 m8 九臂窗；本提交只改文档，题面本轮要求的回归门采用臂 R digest 脚本。
- 未重跑 `cargo test --release --lib`；已按题面运行当前共享工位 `cargo test --lib`。
- 未制作 `386a5bdabd` 的干净 archive 重跑全量；因此 `1980/4/135` 只描述共享工位，不是该纯文档 commit 的独立快照指纹。
- 未用外网或 `gh`，未读取远端 issue 当前态；票号归因依据为固定 commit、仓内三份报告、两轮 `/tmp` 报告及本轮题面。
- 未评审、修改或归因共享工位的既有脏文件和未跟踪文件。
