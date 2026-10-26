# 收据 workers/p3fix-0077/REPORT.md

- 原件：`workers/p3fix-0077/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`ed4a6b6b33ded4354e3f1804af674f9a3daf668987077be789c5ae2360a23c87`
- 字节数：11903；行数：53

## 关键行（行号: 原文）

```
1: # #1488 0077 P3 本票·修 交付报告
3: - 工位：budget-handoff-fix-v2-checkout，分支 `codex/1488-budget-full-chain-fix-v2-20261018`；起点 HEAD `3ee05a2713`；NC_TARGET=target-p77。
8: | `a041499efc` | 主改动：P08/P12 锁、P13 断言、P15 注释、P50 锁、P55 真值表行（observe_for_test）、P18 锁表登记、3 条新锁登记 |
13: | `06582330d7` | P58：本票新增 P12 锁按 rustfmt 1.10 格式化（仅测试模块） |
18: ## 逐条表
23: | **P12**（R-12）±1~2B 文字不精确；重复 ingest 与 C→D 链式推进无锁 | **修**：订正文字 + 补锁 | 勘误已追加到 `workers/recover-carry-implementation-0037/REPORT.md:76` 和 `…-0041/REPORT.md:79`：已观测值为 {3602192, 3602193, 3602194}，运行间跨度 ≤2B；「±1B」不精确；「+40B」应读作「+38～+40B」。锁 `driver.rs:1810 recover_lock_promotes_after_duplicate_ingest_and_ch
24: | **P13**（R-13）有序锁 material_epoch 递增断言恒真 | **修**：改读真实计数器 | `a041499efc`，`effects.rs:2690` 起：先记 `epoch_before`，再断言 `j.material_epoch - epoch_before == 3`、`keys[0] == epoch_before+1`、`keys[1] == epoch_before+3`；原恒真行已替换 | N13（Noop 不递增 epoch）→ 红于 `effects.rs:2692`（left 2 / right 3）。基线 `3ee05a2713` 上同一变异
25: | **P15**（R-16）篡改锁注释与实际不符；宽对拍过滤器缺 content_manifest | **修** | 注释订正：`effects.rs:2743`「只测越序（future index）；指向别的既有 key 未在本锁单独构造」（`a041499efc`）。过滤器：仓内没有常驻的宽对拍脚本，0038 用的是一次性命令；本票宽对拍脚本 `workers/p3fix-0077/wide.sh` 的过滤器为 `v3:: content_manifest content_store`，下面的收尾宽对拍就用它 | 纯注释与脚本，无需负控 |
26: | **P18**（R-21）第四处裁改断言登记最终形态 | **修**（文档） | `.github/1488-uo-locks.txt:85` write_budget 锁注释追加：「最终形态：`handoff_peak − entry ≤ candidate.limit`（0045 P3-B / R-21 要求登记，P18；f49db3d7e7、cc5e324736）」。措辞与 CLOSE-GATE-DRAFT §4 第 4 处一致（`a041499efc`） | — |
27: | **P20**（R-23）receipt 正控缺独立负控 | **修**：补变异负控，验红后回退 | 不改锁本体（`driver.rs` receipt 锁 `recover_lock_promotes_after_new_request_receipt_rebind`） | NA12_20 → 红于 `driver.rs:1793`「资格所记冻结域须随合法重绑前进」；`git checkout` 回退后 CLEAN_AFTER=yes。`R/p3fix-0077/20261021-095453-98aa65eccd.md`（`2cf884e204` 上首次验红：`R/p3fix-0077
28: | **P24**（0030 P3-ζ）冷根残留报错后 writer 不退役 | **跳过**（按控制文件：交非作者复核，不在本切片改） | 只在此列出。登记表建议：由非作者确认「不退役」在 12635bd142 / 4017955224 之后的现路径下已不可观察，再结案 | — |
29: | **P28**（0054 P2-1）r14「仅改一处须变红」的读法 | **按主控读法验证**：推导式里任一计入项单点改动即变红；不改断言 | 变异表 `workers/p3fix-0077/mut.py` 中的 R14_* | 15 项单点变异全红：c33357、plus_l、c97、nodes_term → 4304；c526876、2l → 4309；schema、policy、domain → 4328；len_sum → 4303；slot_delta、slots、c16640 → 4329；sum_claims、sum_layout → 4330。CLEAN_AFTER=yes
30: | **P30**（0054 P3-2）N2 日志不能以 /tmp 路径作关票证据 | **修** | `4fe8a89b3d`：39 份收据（关键行 + 整文件 sha256）+ `R/INDEX.md` + `R/make_receipts.py`。N2 = `R/20261020-130308-2f8aa72060.md`，sha256 `af31327f634304bcb2bc07aa17e075c6328338e6dfad7cda285a0e140fe82507`。关票证据集 6 份文档已改写为仓内路径，并在文末追加迁移说明（原件备份在 `workers/p3fix-0077/ori
32: | **P55**（0073 G1）真值表缺语义失败行和 `semantic==0` 负控 | **修** | `a041499efc` / `98aa65eccd`：`publication_degrade_verdict_tests` 新增两行，Budget/Allocation 各跑一次：(a) reject 后观察到非 scratch codec 错误、闭包返回 Ok → 须含「工作账损坏」；(b) 闭包返回原错误 → 须原样为 `Err(e)`。测试面 `scratch.rs` 文件末尾 `WorkScope::observe_for_test`（`cfg(feature = s_c
34: | **P58**（fix-0072 已知局限）远端 fmt --check 基线不干净 | **核实：登记表前提不成立**；本票增量已修平；全量 fmt 需主控裁定 | 远端 rustfmt 1.10.0。main `df6e680930`、merge-base `6badaa6049` 的 `fmt --check` 都干净，main CI 有 fmt 门（`ci.yml:231`），所以 15 个文件的差异是 #1488 分支引入的，不是「main 上就有」。0077 原本只在 driver.rs 多出 1 个 hunk（1829，P12 锁，35 对 34），已由 `06582330d
36: ## 收尾三件
38: 1. **全锁**：`98aa65eccd` 上 95 passed、0 failed（`R/p3fix-0077/20261021-095730-98aa65eccd.md`）。`06582330d7` 只改了一条测试的格式，复跑受影响的三锁（P08/P12/receipt），3/3 ok（`…103253-06582330d7.md`）。
40: 3. **生产不变**：release `--features s_session`，去 build-id 并 strip 后的 sha256：
45: ## 未处理的 /tmp 引用（不在关票证据集，P30 只列清单）
49: ## 待主控裁定
53: - **P24**：安排非作者复核。
```
