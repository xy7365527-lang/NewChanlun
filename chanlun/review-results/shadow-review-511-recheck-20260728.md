# #511 复审：#512 核销（commit `8d8895c652`）

日期：2026-07-28  
工作区：`/tmp/kimi-nest-mainline`  
固定点：`8d8895c65287150499fb698bc8566dee644bbcbb`  
评审方式：只读；未改仓内文件、未做 git mutation；独立跑批产物仅写 `/tmp/rev512_*`

## 结论

**维持打回。**

`8d8895c652` 已完整核销 MED-1、MED-2、Spec MED-1、Spec MED-2，且 BTC 臂 R / wf8 独立跑批、默认 digest 门、脚本单测和全量 lib 指纹均符合票面。但首轮 **MED-3 只完成了一半**：v2 的追加式 regen、legacy 迁移和“字段存在”检查已经落地，正常门仍不校验 provenance 字段的类型、commit 格式/可解析性或谱系语义；任意非空伪值仍可绿。因此首轮要求的“provenance 漂移使门失败”尚未兑现。

| 轴 | 结论 | HIGH | MED | LOW |
|---|---|---:|---:|---:|
| Standards | **打回** | 0 | 1 | 1 |
| Spec | **打回** | 0 | 1 | 0 |
| 合计（同一 MED 去重） | **打回** | **0** | **1** | **1** |

首轮 LOW×2 没有在 #512 内结构修复；T3 §15.8 已明确归入 #493。本轮不把它们误算成 `8d8895c652` 新回归。

## 1. 逐项核销

| 首轮项 | 结论 | 独立证据 |
|---|---|---|
| Standards MED-1：release 防线 | **PASS** | `step.rs:295-323` 在 `strategy_target_legs`（`:326`）前扫描重复 ID、先 bump probe 后无条件 `panic!`；release 点名测 1/1 绿。新增槽的三个物化来源均显式登记：held `:119-120/:151-152`、registry restore `:140-142/:254-256`、boundary root `:163-175`；fallback 只报 `post-overlay-unregistered`，不再把未知来源误标成 held。m8 后置 assert 已删，但 `wverify_run.rs:1776-1784` 的 snapshot/eprintln/表行仍在，独立报告 `:38-40` 可见 `duplicate_id_violations=0`。 |
| Standards MED-2：canonical | **PASS** | T3 §15.6（`:1090-1101`）以只追加订正明确覆盖 §15.5（`:1070-1085`）旧声明，没有静默改写；脚本默认根仍是 `/tmp/m8_win_gate`（`check_armR_trades_digest.py:46`）。无参门独立 exit 0，三窗 digest 全中。 |
| Standards MED-3：golden v2 | **FAIL（部分核销）** | regen 会保留 v2 历史锚再 append（脚本 `:213-227`），legacy 字段迁移也保留（`:67-86`），3 条 unittest 全绿；但正常门的 `provenance_problems`（`:116-132`）只验 `_schema`、锚数组非空和 required 字段 truthy，不验字段类型、SHA/commit、日期、命令或谱系。把 `final_verification_head` 改成 `definitely-not-a-commit` 返回 `[]`；六个 required 字段全部填整数 `1` 也返回 `[]`。测试 `scripts/tests/test_check_armR_trades_digest.py:60-72` 只覆盖整块缺失，未锁伪值假绿。 |
| Spec MED-1：逐笔 diff 旧侧 | **PASS** | `/tmp/446_armR_trades_diff.md:3` 与 JSON `:2` 均已指向 `/tmp/423_backup_m8_win_gate`。抽核 p3fold：旧/新实物 149/141 行，SHA-256 分别为 `e0573a…3196` / `65621e…c21b`，逐位命中 JSON summary；current `/tmp/m8_win_gate` 与 `/tmp/446_armR_dump` 为 SAME。 |
| Spec MED-2：污染面登记 | **PASS** | T3 §15.7（`:1103-1129`）明确三态：§15 九窗“已重验”、T2 指定旧表 `superseded`、其余消费族“未复核”；`:1111-1112` 明写未复核禁止改写成“已污染”或“未污染”。表中覆盖 v4×2、w2、dual-open、L3、incremental/run-theta。T2 订正块（`:622-634`）的 p3fold/wf7/wf8 新 D 值与 T3 §15.2（`:1025-1035`）逐位一致。 |
| 新引入排查 | **FAIL（由上述残留 MED-3 触发）** | 8 文件 `+369/-65` 全 diff 已按 Standards/Spec 两轴审阅；核心 Rust 语义、报告订正和跑批未见新 HIGH。另有一个新 LOW，见 §2.2。 |

## 2. Findings

### MED-1（首轮 MED-3 残留）：v2 provenance 门仍可接受任意非空伪值

**位置**

- `scripts/check_armR_trades_digest.py:49-56`：只列 required 字段名。
- `scripts/check_armR_trades_digest.py:116-132`：逐锚仅执行 `anchor.get(field)` truthy 检查。
- `scripts/check_armR_trades_digest.py:237-244`：正常门虽调用该函数，但没有更深校验。
- `scripts/tests/test_check_armR_trades_digest.py:60-72`：只测没有 schema/provenance 的整块缺失。

**反例**

```text
final_verification_head="definitely-not-a-commit" -> provenance_problems == []
六个 required 字段全部为整数 1                 -> provenance_problems == []
```

因此 windows/digest 正确时，伪 commit、错误类型或被篡改锚值仍不会令正常门失败。`_current_regen_anchor` 还会把“执行 regen 时的当前 HEAD”直接盖到所给 dump 上，并没有机器证明该 dump 确由该 HEAD 产生；这使字段存在性尤其不能替代真实性校验。

**影响**

首轮 MED-3 的核心不是“有 provenance 字段即可”，而是 provenance 漂移应令正常门失败。当前实现仍允许 provenance 假绿，违反 `.claude/rules/common/coding-style.md:23-37` 的系统边界 schema 校验/fail-fast 要求，也不满足 090 的声明—能力一致纪律。

**最小回修门**

至少为 required 字段做严格类型和格式校验：两个 HEAD 必须是 40 位十六进制并可解析为 commit，且 `source_base_head` 是 `final_verification_head` 祖先或相等；日期须为 ISO 日期，命令/工位描述须为非空字符串。再补“字段存在但值/类型非法”的正常门红测。若要把 HEAD 与 dump 真正绑定，应再落机器可验的跑批 sidecar/manifest，而不是在 regen 时仅采当前 HEAD。

### LOW-N1（新引入）：再生成命令的临时证据前缀跨票跳到 `/tmp/484_*`

`scripts/check_armR_trades_digest.py:13-20` 在本提交把原 `/tmp/446_armR_*` 改成 `/tmp/484_armR_*`，但脚本头仍定义为 #387/#446 回归锁，本次修复票是 #512，T3 §15.6 登记的是 `/tmp/512_*` 验收证据。该路径不改变摘要算法，但会让照 docstring 重建出的产物带无说明的跨票前缀，并且不能由紧邻的默认校验命令直接检查。

建议改成中性的 `/tmp/armR_regen_*`，或明确使用本票 `/tmp/512_*` 并在随后示例中传同一个 `--dump-dir`。

### 未采纳的测试疑点

新增 panic 测 `step_tests.rs:247-290` 虽未用 `#[cfg(not(debug_assertions))]`，但它不仅 catch panic，还逐字断言 `boundary-root-retain`。修前 debug 的旧 fallback 会报 `held-reregister`，故旧 debug 也会红；修前 release 则会走到双计返回并在 `expect_err` 处红。该测试确能咬住修复，不列 finding。

## 3. MED-1 反例构造复核

新测试用两个完全相同的 `ActiveLeg`：

- `aleg(...)` 默认 `is_boundary_root=true`、`parent_id=None`；
- 空 tree、空 registry、空 buckets 使两腿在修前分别经 boundary-root 直推进入 `raw/next_idx`；
- `VoiceConfig::default()` 的 depth-0 权重为 `0.60`，`base_units=1000`；
- 修前 release 在较晚 debug-only 守卫前已经为两条同向根腿计算 `600 + 600 = 1200`，随后继续返回；
- 新实现先在 `next_idx` 上发现同 ID，于 sizing 前 panic，禁止构造 `p_tilde/sep_legs`。

独立 release 点名运行实际打印：

```text
idx 0(boundary-root-retain) 与 idx 1(boundary-root-retain)
strategy_target_legs 将双计 p̃
test result: ok. 1 passed
```

## 4. 独立跑批与测试

### 4.1 BTC 臂 R / wf8

隔离环境：

```text
M8_WIN_FILTER=wf8
VOICE_EXEC=1
THETA_NEST_CERT_GATE=1
M8_REPORT_PATH=/tmp/rev512_armR_wf8.md
OPSEM_DUMP_DIR=/tmp/rev512_armR_wf8_dump
```

结果：

- `cargo test --release --lib ...m8_e2e_all_systems_oos -- --ignored --nocapture`：**1 passed**。
- `/tmp/rev512_armR_wf8.out:446`：`duplicate_id_violations=0`。
- `/tmp/rev512_armR_wf8.out:447`：`execR=+6281284`、`R=+6782644`、`LCB=-2427669`、`INCONCLUSIVE`。
- `/tmp/rev512_armR_wf8.md:38-40`：报告侧唯一性读数仍存在。
- `trades.jsonl` 与 `/tmp/m8_win_gate/wf8/trades.jsonl`：`cmp -s` exit 0。
- `tower_events.jsonl` 与 `/tmp/m8_win_gate/wf8/tower_events.jsonl`：`cmp -s` exit 0。

### 4.2 digest / Python 单测

- `python3 scripts/check_armR_trades_digest.py`：exit 0；p3fold/wf7/wf8 为 141/184/159，digest 分别 `0xac5952cfcd8b8746`、`0x981a0560b8db0b40`、`0x18291f8ba8f1d40f`。
- `python3 -m unittest scripts.tests.test_check_armR_trades_digest`：**3 tests, OK**。

### 4.3 全量 lib

`cargo test --lib`：

```text
1991 passed; 1 failed; 135 ignored
```

唯一失败为既线 #115：

```text
theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard
```

新 panic 测在全量 debug 中为 green；没有新增失败。

## 5. 订正块归因普查

对 T1/T2/T3/OKLO 四份本链报告及本提交新增行定点穷查：

- #446：T1 §14、T3 §15 归活动集唯一性修复；
- #511/#512：T3 §15.6、T2 末尾订正块归本轮复审/修复；
- #475：T2/T3/OKLO 的历史复审订正继续与对应 #495/#496 配对；
- #481：只在 OKLO §2.9 作为其回归计数订正；
- 本提交新增订正块没有 `#481 HIGH-*` 残留，也没有把 #475/#481 的 finding 交叉归到 #511/#512。

归因普查 PASS；LOW-N1 是临时路径前缀问题，不是订正块票号误标。

## 6. Standards / Spec 两轴摘要

### Standards

生产 panic 点位、来源登记、m8 可观测性、文档只追加订正、格式检查均通过。阻塞项仅 v2 provenance 边界验证仍是 truthy 检查；另有 `/tmp/484_*` 证据前缀 LOW。Python 三项测试重复 temp/golden/argv/mock 脚手架属于 Duplicated Code 判断项，不单独升级 finding。

### Spec

MED-1、MED-2、Spec MED-1、Spec MED-2 均实质满足；MED-3 的 regen/迁移满足，但“正常门校验关键 provenance”只部分满足，因此 Spec 仍不通过。

## 7. 未覆盖与不能外推

1. 独立真实跑批只重跑用户点名的 BTC 臂 R / wf8；其余 R/D/C × 三窗中的八窗未再次 cargo 跑批。
2. 没有实际对仓内 golden 执行 `--regen`，因为本轮是只读评审；追加/迁移通过代码、临时文件单测和只读反例验证。
3. 没有重跑 L3、incremental、dual-open、v4 或其他历史消费族；只核登记表是否覆盖、状态措辞是否诚实。
4. 无外网、未使用 `gh`；没有核在线 #511/#512 票体，只使用用户硬清单、首轮报告、提交 diff 和仓内文档。
5. 没有触碰或归因共享工位既有 `agent-roster` 修改及两份未跟踪评审文件；评审结束 HEAD 仍为 `8d8895c65287150499fb698bc8566dee644bbcbb`。

