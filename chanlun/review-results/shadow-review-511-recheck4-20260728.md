# #511 四轮复审：#515 核销（commit `9928948aa3`）

日期：2026-07-28  
工作区：`/tmp/kimi-nest-mainline`  
固定点：`9928948aa364c5d57c4a5fa07d4997e6e4fdb7be`  
直接父提交：`f6639a2a1bb5674ecd735a953ce964f4b205e6e9`  
评审方式：只读；未改仓内文件、未做 git mutation。独立负例只使用自动清理的临时目录；
全量测试日志写 `/tmp/rev511_recheck4_cargo_test_lib.out`。

## 结论

**PASS。**

`9928948aa3` 已完整核销三轮复审发现的“可选 lineage 遮蔽必填 base”残留：

- `source_base_head → final_verification_head` 现在是无条件恒验边；
- `source_lineage_start` 存在时，另验更强的
  `source_lineage_start → source_base_head`；
- 三轮原组合负例已独立通过真实 `main()` 复现，门由修前 exit 0 翻为当前 exit 1；
- Python 单测 12/12、当前 golden、无参门和全量 Rust 基线指纹均符合票面。

| 轴 | 结论 | HIGH | MED | LOW |
|---|---|---:|---:|---:|
| Standards | **PASS** | 0 | 0 | 2（既知、非阻塞） |
| Spec | **PASS** | 0 | 0 | 0 |
| 总结 | **PASS** | **0** | **0** | **2（不构成假绿）** |

本轮没有新阻塞 finding、没有 scope creep，也没有理由围绕 #511 再开第六轮。

## 1. 边界与提交固定

- `git rev-parse HEAD` 与 `git rev-parse 9928948aa3` 均为
  `9928948aa364c5d57c4a5fa07d4997e6e4fdb7be`。
- `git show 9928948aa3` 全 diff 恰为：
  - `scripts/check_armR_trades_digest.py`
  - `scripts/tests/test_check_armR_trades_digest.py`
- 统计精确为 **2 files changed, +43/-10**；`git diff 9928948aa3^ 9928948aa3 --check`
  exit 0。
- `4b2b69afc6..9928948aa3` 中间有并行 #421 提交 `f6639a2a1b`，只改
  `issue421-acceptance-selfcheck-20260727.md`、`p123_fast_replay.rs`、
  `nest_lifecycle.rs`，不归本票。
- 对 #512 已核销的 Rust、T1/T2/T3/OKLO 报告及 golden 面执行
  `git diff --exit-code 8d8895c652..9928948aa3 -- <相关文件>`，exit 0；
  后续 #513/#514/#515 未回退这些已闭合面。
- 共享工位既有：
  - `M chanlun/agent-roster-2026-07-21.md`
  - `?? chanlun/review-results/shadow-429-issue421-20260727.md`
  - `?? chanlun/review-results/shadow-430-issue421-20260727.md`
  
  全程未触碰、未归因。

## 2. #515 核销清单

| 核销项 | 结论 | 独立证据 |
|---|---|---|
| base→final 恒验 | **PASS** | `check_armR_trades_digest.py:215-223` 总是把 `("source_base_head", "final_verification_head")` 加入 `ancestry_edges`；`:224-238` 逐边执行 Git 祖先检查并落字段级问题。 |
| lineage→base 附加边 | **PASS** | 同文件 `:215-219` 在 lineage 存在时增加 `("source_lineage_start", "source_base_head")`；不再与 base→final 二选一。 |
| Git fail-closed / 错误归属 | **PASS** | `_git_ancestry_problem`（`:155-172`）对 Git 不可执行、exit 1 和其他非零分别报错；新增 `descendant_field` 使错误能区分 `source_base_head` 与 `final_verification_head`。 |
| 三轮组合负例独立复现 | **PASS** | 临时 golden：lineage=`a12a…`、base=`6e15…`、final=`a12a…`。真实 `main()` 返回 `negative_gate_exit=1`，stderr 为 `source_base_head: 不是 final_verification_head 的祖先`。临时目录自动清理。 |
| 回归测试咬住旧 bug | **PASS** | 新测 `test_check_rejects_reversed_base_ancestry_when_lineage_is_valid`（测试文件 `:133-154`）精确构造上述组合并断言字段及错误边；旧二选一实现会假绿，使该测试失败。 |
| Python 单测 | **PASS** | `python3 -m unittest scripts.tests.test_check_armR_trades_digest`：`Ran 12 tests in 0.583s`，`OK`。 |
| Golden 合法性 | **PASS** | 见 §3。 |
| 无参门 | **PASS** | `python3 scripts/check_armR_trades_digest.py` exit 0；三窗逐位命中，见 §3。 |
| 两文件完整 diff | **PASS** | 已逐行审阅 `git show 9928948aa3`；改动仅为祖先边枚举、错误目标字段和一条精确回归测试，无无关能力、无 golden 改写。 |
| 全量 Rust 基线 | **PASS（预期指纹）** | `cargo test --lib`：`1992 passed; 1 failed; 135 ignored`；进程 exit 101 仅因在案 #115×1，见 §4。 |
| 订正块归因终扫 | **PASS** | 见 §5。 |

## 3. Golden 四层合法性与无参门

当前 committed golden：

```text
schema  = armR-trades-digest/v2
base    = a12a1022d9ddd8d1cae867a107a3a33c359358cf
final   = 6e15ceffeeb8259c065bf7c0ec9ec7c65935737c
lineage = <absent>
```

| 层 | 结论 | 证据 |
|---|---|---|
| 格式 / schema / 字段类型 | **PASS** | 对当前 JSON 调用 `provenance_problems` 返回 `[]`；HEAD、日期、字符串、digest、正整数窗计数均通过。 |
| Commit 可解析 | **PASS** | 两个 HEAD 的 `git cat-file -e <head>^{commit}` 均 exit 0。 |
| base→final | **PASS** | `git merge-base --is-ancestor a12a… 6e15…` exit 0；反向命令 exit 1。 |
| lineage→base | **N/A（合法）** | 当前锚没有可选 lineage，按 schema 不要求伪造该边。只读克隆探针加入 `lineage=base` 后 `provenance_problems=[]`；加入倒挂 `lineage=final` 后明确报 `source_lineage_start: 不是 source_base_head 的祖先`。 |

无参门输出：

```text
p3fold: n_trades=141 digest=0xac5952cfcd8b8746 bytes=195220 ✓
wf7:    n_trades=184 digest=0x981a0560b8db0b40 bytes=253658 ✓
wf8:    n_trades=159 digest=0x18291f8ba8f1d40f bytes=216129 ✓
臂R trades 逐位无漂移。
```

门 exit 0。没有对仓内 golden 执行 `--regen`。

## 4. `cargo test --lib` 全链终查

命令在 `rust/` 目录亲跑，完整输出保存于
`/tmp/rev511_recheck4_cargo_test_lib.out`：

```text
running 2128 tests
test result: FAILED. 1992 passed; 1 failed; 135 ignored; 0 measured; 0 filtered out
```

唯一失败：

```text
theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard
```

仍属既线 #115，失败摘要与前三轮一致，不是 #515 新回归。

`lee_m` 定点抽取共 **8/8 绿**：

- `lee_m3_*`：6/6；
- `lee_m4_*`：2/2。

因此票面 `1992/1/135、#115×1、lee_m 8/8` 全部核中。

## 5. 订正块归因终扫

| 票号 / 面 | 终扫结论 |
|---|---|
| #446 | T1 §14、T3 §15 仍只归活动集同 `ElementId` 双 idx 修复；没有被 #515 重写。 |
| #475 / #495 / #496 | T2/T3/OKLO 的历史 finding、首轮回修和二轮补订正继续逐项配对。 |
| #481 / #484 / #490 / #492 | OKLO §2.9 及其后续展示/列名修复保持本线归因，没有串到 #511/#515。 |
| #511 / #512 | T3 §15.6/§15.7、T2 `:622-634` 仍归首轮复审与 #512 回修；T2 旧臂 D 数仍显式标 `superseded`。 |
| 历史消费族 | T3 §15.7 仍保持三态：九窗“已重验”、T2 指定旧表 `superseded`、其余 v4/w2/dual-open/L3/incremental/run-theta 为“未复核”；未把未知状态洗成已污染或未污染。 |
| #493 | T3 §15.8 仍明确登记首轮 LOW×2：`wverify_run.rs` 文件体量、长测试/重复夹具。 |
| #513 / #514 / #515 | 三票都只改两份 Python 文件，没有伪造仓内报告订正块或占用其他票号。 |

归因终扫 PASS。

## 6. 首轮 5 项 + 二轮 2 项 + 三轮 1 项 + 四轮 1 项

| 轮次 | 历史项 | 最终判定 |
|---|---|---|
| 首轮 1/5 | Standards MED-1：release fail-loud | ✅ **闭合**；#512 已将硬防线前移到 coverage 生产边界 panic，后续相关 Rust 面未改。 |
| 首轮 2/5 | Standards MED-2：canonical 当前态 | ✅ **闭合**；默认根仍为 `/tmp/m8_win_gate`，T3 §15.6 只追加订正仍在，无参门 exit 0。 |
| 首轮 3/5 | Standards MED-3：golden / provenance | ✅ **闭合**；v2 追加迁移、格式、类型、日期、计数、commit 可解析与完整祖先边均已闭环。 |
| 首轮 4/5 | Spec MED-1：逐笔 diff 旧侧路径 | ✅ **闭合**；旧根仍是 `/tmp/423_backup_m8_win_gate`，新根 `/tmp/446_armR_dump`。 |
| 首轮 5/5 | Spec MED-2：污染面登记 | ✅ **闭合**；“已重验 / superseded / 未复核”三态及 T2 订正块保持。 |
| 二轮 1/2 | provenance schema 假绿 | ✅ **闭合**；HEAD/digest/count/date/string 等负例与正常门均被测试覆盖。 |
| 二轮 2/2 | docstring 跨票前缀 / canonical | ✅ **闭合**；当前命令使用 `/tmp/m8_win_gate` 与中性前缀，`/tmp/446` 只作历史锚。 |
| 三轮 1/1 | commit 可解析 + 祖先 + 字符串类型 | ✅ **闭合**；#514 已补可解析/fail-closed/类型；#515 补齐该项最后被遮蔽的 base 祖先边。 |
| 四轮 1/1 | lineage 合法时遮蔽 base→final 倒挂 | ✅ **闭合**；独立真实门 exit 1，新单测锁定，base→final 已成为恒验边。 |

九项全部打勾。

## 7. 收敛判定

### 阻塞残余

**全部闭合。** 当前不存在 provenance 假绿、golden 非法、测试回归或归因串票；Standards 与
Spec 均没有阻塞 finding。

### 已登记债务

首轮 LOW×2 已明确登记到 #493，不需要借 #511 第六轮处理。

### 既知非阻塞维护项

没有新发现；但前轮已记录、当前 diff 仍触达的两个 Python LOW 继续存在：

1. `provenance_problems` 当前约 89 行，超过仓库 `<50` 建议；#515 为修正完整边枚举又扩展该函数。
2. 测试文件 `:121` 的既有 `test_check_rejects_reversed_lineage_ancestry` 本轮改了断言，
   但签名仍缺 `-> None`；新加的 `:133` 测试已带返回类型。

另有 possible Duplicated Code 判断项：两个 Git helper 的
`subprocess.run + OSError fail-closed` 骨架相似。以上都不导致 false green，不属于 #515
语义残留。现有本地文档没有证明这两个 Python LOW 已登记到 #493/其他票；若要求“所有维护债务
也必须有票”，应将它们并入 #493 或单列维护票，但不应为此重开 #511 第六轮。

**最终收敛判定：#511 / #515 正确性链收敛，PASS；无理由再开第六轮。**

## 8. 新发现

**无新阻塞或新回归。** §7 的两个 LOW 与 Duplicated Code 均为前轮已知维护观察，不冒充本轮
新 finding。

## 9. 未覆盖与不能外推

1. 无外网、未使用 `gh`；未核在线 #511/#512/#513/#514/#515/#493 票体。票号登记判断只依据
   用户硬清单、四份 `/tmp` 历史报告及仓内追加订正。
2. 为保持只读，没有执行会改仓内 golden 的 `--regen`；regen 的追加/迁移能力沿用 12/12
   单测覆盖。
3. 没有重跑 BTC release wf8、其余八窗、L3、incremental、dual-open 或历史消费族；本轮按硬
   清单亲跑 Python 单测、无参 digest 门与 `cargo test --lib`。
4. `cargo test --lib` 是当前共享工位指纹，不是 target commit 的隔离干净快照；但 HEAD 精确为
   `9928948aa3`，共享脏文件不在 Rust 测试面，实测数字与票面完全一致。
5. commit 可解析及祖先关系不证明 dump 确由该 commit 生成；现有 docstring 已明确没有
   dump↔HEAD sidecar/manifest，本轮不作超能力外推。
6. 未修改发现的问题；唯一正式评审产物为
   `/tmp/shadow-review-511-recheck4-20260728.md`，另有 `/tmp` 测试日志。
