# 影子评审报告：#387 treasury 重验 T1（臂R 逐位对照）

- **评审票**：#401　**被评票**：#387（已关）　**母 SPEC**：#385
- **被评 commit**：`0dc5bc785d`（报告 `chanlun/review-results/treasury-reverify-t1-armR-20260727.md` +
  golden `treasury-reverify-t1-armR-trades-golden-20260727.json` + 校验器 `scripts/check_armR_trades_digest.py`）
- **评审工位**：`/tmp/nc-review-387`（隔离 worktree，detached @ `0dc5bc785d`），只读评审，零 git mutation，
  主仓与 `/tmp/kimi-nest-mainline` 全程未触碰。
- **评审性质**：opus 新上下文独立评审，与交付方零关系，不采信被评报告 §10 自审结论作为证据。

---

## 结论

**打回（MED×4，无 HIGH；LOW×5 附带）**

核心机制与主体证据链是硬的——golden/校验器三态独立实证通过、臂A 零差异声明独立复算全中、
二分终判与 v4 溯源缺陷论证独立复核成立。**打回的理由不是结论错，是三条承重声明与其证据不对齐**：

1. §4.3 的「48 个提交」实为 **37**（口径任取皆非 48）；
2. §4.3 声称被测区间含票面枚举**全部**九项修复链，但 **#246 的四个提交全部早于区间起点**
   （`4cde104869` 即 #246 封口提交，恰是区间起点 `2fd5ee08ba` 的父）——被测区间只覆盖八项；
3. 交付方自己产出的两份文件（`/tmp/m8_report_4cde104869_R.md` / `_2fd5ee08ba_R.md`）**已经证明
   臂R §2.2 读数同样由 #309 翻转**，但报告 §4 未给这条归因，§9.3 反而写成「臂R 侧照实登记为不可二分」——
   这条措辞正是上浮给 Lead 的 SPEC 红线裁定材料。

三条都可用文本订正 + 一次补跑修复，不需重跑主批。修复路径见文末。

---

## 分级发现

### MED-1｜§4.3「48 个提交」与实际不符（37）

- **位置**：报告 §4.3 L190；同一数字复现于 commit message、#387 resolution 评论、#401 票体验收标准。
- **主张**：`2fd5ee08ba`→HEAD 之间的提交数被写为 48，实际为 37（至 `0dc5bc785d`）/ 36（至报告自述 HEAD
  `c126d4cf69`）。任何合理口径都到不了 48（见下表）。承重性：该数字是「复合 bit-exact 成立」声明的
  样本规模表述，虚高约 30%。
- **证据**：

  | 口径 | 计数 |
  |---|---|
  | `2fd5ee08ba..HEAD`（HEAD=`0dc5bc785d`） | **37** |
  | `2fd5ee08ba..c126d4cf69`（报告自述 HEAD） | 36 |
  | `2fd5ee08ba~1..HEAD`（含起点） | 38 |
  | `4cde104869..HEAD`（#309 之父起） | 43 |

- **最小复现**：
  ```bash
  cd /tmp/nc-review-387 && git rev-list --count 2fd5ee08ba..HEAD   # → 37
  ```

### MED-2｜§4.3「含票面枚举全部九项」不成立——#246 在被测区间之外

- **位置**：报告 §4.3 L190-191（「含票面枚举的**全部** #351/#356/#363/#365/#376/#360/#374/#246/#361」）；
  结论摘要 §0.1 与 resolution 评论「整条修复链…零影响」同承此误。
- **主张**：#246（相切=重合封口）的全部在册提交（`e79b12e10f` / `491f638a8c` / `77040317f2` /
  `4cde104869`）**都是 `2fd5ee08ba` 的祖先**——其中 `4cde104869` 就是 #246 的封口提交，恰恰是
  §4.1 用作 #309 之父的那个 GOOD 锚点。因此「2fd5ee08ba→HEAD 区间」对 #246 **零覆盖**，
  九项里只被测了八项。
  - 补充：#246 的臂A bit-exact **另有**证据——二分链上 `12aa929d44` / `e79b12e10f` / `8dce346588` /
    `381ae777ae` / `57a50d939f` / `bbb130c215` / `4cde104869` 逐个 `execR=+923705`（GOOD），
    所以结论本身很可能站得住；**但那不是 §4.3 所引的证据**。臂R 侧则确实无覆盖（臂R 只在
    `4cde104869` / `2fd5ee08ba` / HEAD / v4head 四点跑过，`4cde104869` 已是 #246 之后）。
- **证据**：
  ```
  git log --oneline --grep="#246"  → e79b12e10f / 491f638a8c / 77040317f2 / 4cde104869
  git merge-base --is-ancestor <每一个> 2fd5ee08ba  → 全部 YES
  git log --oneline 2fd5ee08ba..HEAD | grep '#246'  → 空
  ```
- **最小复现**：
  ```bash
  cd /tmp/nc-review-387
  for c in e79b12e10f 491f638a8c 77040317f2 4cde104869; do
    git merge-base --is-ancestor $c 2fd5ee08ba && echo "$c 在区间之前"; done
  ```

### MED-3｜臂R §2.2 的 #309 归因缺席，且 §9.3 措辞比证据宽

- **位置**：报告 §4（缺一条）、§0.2、§9.3；影响 resolution 评论的 SPEC 红线上浮措辞。
- **主张**：报告 §4.1 引用了 `/tmp/m8_report_4cde104869_A.md` 与 `_2fd5ee08ba_A.md` 做臂A 归因，
  同一批跑批还产出了 **`_R.md` 两份**（臂R）。这两份文件直接证明 **臂R §2.2 三窗读数同样在 #309 处翻转**，
  且翻转后即为 HEAD 值：

  | 窗 | 臂R @`4cde104869`（#309 之父） | 臂R @`2fd5ee08ba`（#309） | HEAD |
  |---|---|---|---|
  | p3fold | execR=**-1031612** MaxDD=0.1351 R=-962558 | **-1268551** 0.1475 -1191916 | 同 #309 |
  | wf7 | execR=**-2036084** MaxDD=0.1028 R=-1948592 | **-5427281** 0.2601 -5147413 | 同 #309 |
  | wf8 | execR=**+4580249** MaxDD=0.0464 R=+4881460 | **+6346975** 0.0803 +6851062 | 同 #309 |

  ⟹ 臂R §2.2 的发散**可分解**为「pre-#309 的不可二分部分（v4 -36842 → -1031612）」+
  「#309 部分（-1031612 → -1268551）」。报告只登记了前者的不可归因，未登记后者的**可归因**。
  §9.3 写「『逐条归因到具体开关/commit』只对臂A 可完成；臂R 侧照实登记为不可二分」——**过宽**。
  §0.2 只列 §2.1/§2.3/§2.4（技术上精确地回避了 §2.2），但通篇未在任何地方补上 §2.2 臂R 的归因。
- **为什么是 MED 而非 LOW**：SPEC #385 的「红」定义含「臂R 发散无法归因」，本票据此上浮给 Lead 裁定。
  裁定材料若写成「臂R 侧不可归因」vs「臂R 侧部分归因 #309、余下不可二分」，是两种不同的裁决输入。
  这不是抹平（数据没被藏，文件都在案），是**归因不完整 + 措辞外扩**。
- **最小复现**：
  ```bash
  diff /tmp/m8_report_4cde104869_R.md /tmp/m8_report_2fd5ee08ba_R.md   # 三行数据行全变
  diff /tmp/m8_report_2fd5ee08ba_R.md /tmp/m8_armR_report.md           # 仅口径标签行差，数据行全同
  ```

### MED-4｜§4.2 的两个承重锚点无留存证据

- **位置**：报告 §4.2 L175-177（「臂R 的 `NEST_GATE_STATS` 在 `4cde104869` 与 `12aa929d44` 处**已经是
  今日形态**（p3fold 1159/1633、wf7 1116/1724、wf8 1074/1518，逐字段相同）」）。
- **主张**：这句是「门控侧发散不由 #309 引起，早于它」的唯一直接支撑，也是红线上浮论证的承重梁，
  但**证据链里没有任何文件承载它**：
  - 全 `/tmp` 检索 `1159/1633`，只命中报告自身及其源工作区副本，**无任何 `.out` / `.log` 命中**；
  - `4cde104869` / `2fd5ee08ba` 的臂R 跑批只留了四层报告 `.md`（`grep -c NEST_GATE_STATS` = **0**，
    该报告格式不含门控行）与 dump 目录，**stdout 未落盘**；
  - `12aa929d44` 更彻底——**没有臂R dump 目录**（`/tmp/` 下只有 `m8_4cde104869_R` / `m8_2fd5ee08ba_R` /
    `m8_v4head_R`），唯一的 `bisect_log/12aa929d44.out` 是**臂A**（`bisect_armA.sh` 不设
    `THETA_NEST_CERT_GATE`），`grep NEST_GATE_STATS` 零命中。
  - 间接旁证只到这一层：`/tmp/m8_4cde104869_R/trades.jsonl` 笔数 168 = HEAD wf8 的 168（≠ v4 的 449），
    支持「trades 侧在 #309 之父处已是今日形态」；对 `12aa929d44` 则**一无所有**。
  ⟹ 报告写的是「逐字段相同」这种逐位强度的断言，实际不可复核。这与 090「声明与实际一致」冲突。
- **最小复现**：
  ```bash
  grep -rl "1159/1633" /tmp --include="*.out" --include="*.log"      # 空
  grep -c NEST_GATE_STATS /tmp/m8_report_4cde104869_R.md             # 0
  grep -c NEST_GATE_STATS /tmp/bisect_log/12aa929d44.out             # 0
  ls -d /tmp/m8_12aa929d44_R                                          # 不存在
  ```

### LOW-1｜回归锁未接 CI，且输入在 /tmp，缺产物不判红

`scripts/check_fixture_drift.py` 有 `.github/workflows/ci.yml:137` 的 `fixture-drift` job；
本校验器**无任何 CI 接线**（`grep -rn check_armR_trades_digest .github/` 空）。加上默认输入是
`/tmp/m8_win_gate`（易失）且缺产物返回 `EXIT_MISSING=4`（不判红），「防静默漂移」实际只在
「有人手动重跑三窗跑批 + 手动跑校验器」时生效。报告 §5 的诚实边界只声明了「后向、不追认 v4」，
未点出触发条件这一层。（票体只要求 golden 落地作后向锁，故不升 MED。）

### LOW-2｜`EXIT_MISSING=4` 双语义

`check_armR_trades_digest.py:128` 用 4 表示「跑批产物缺失」，`:152` 同样用 4 表示「golden 文件不存在」。
docstring（`:27`）只写前者。姊妹件 `check_fixture_drift.py` 的 3/4/5 分层把不同环境问题分开码，
此处两种不同状况共码，机器消费方无法区分。

### LOW-3｜`--regen` 无任何门

`:119` 的 `--regen` 直接覆写 golden，唯一约束是 docstring 里的「改动经审后才允许」——一句自然语言。
误跑即静默重落基线，回归锁自毁。建议加 `--force` 二次确认或落盘前打印 diff。

### LOW-4｜`readings_of` 边界校验弱

`:71-77` 对 `certificate` 缺失取 `<none>`、`pnl_raw_unlevered` 缺失静默取 `0.0`，
与 `.claude/rules/common/coding-style.md`「Never trust external data / Fail fast」相左。
digest 逐字节比对能兜住真漂移，故仅 LOW；但字段级 diff 报告会因此给出误导性的「pnl 相同」。

### LOW-5｜`WINDOWS` 三窗硬编码，多余窗静默忽略

`:41` 固定三窗；若 dump 目录出现第四个窗（未来 T2/T3 扩窗），校验器既不校验也不告警，
「全绿」的语义悄悄从「全部产物无漂移」退化为「这三个窗无漂移」。

---

## 两轴分述

### Standards 轴

| 项 | 结论 |
|---|---|
| **090 严格纪律（声明=实际）** | **不通过**——MED-1（48↔37）、MED-2（#246 覆盖）、MED-4（无证据的「逐字段相同」）三处声明超出实际。 |
| **090 无简化实装 / 无模糊地带** | 通过。校验器不是玩具：FNV-1a64 常数与 `extract_signals_bit_exact_digest_guard` 同源，逐字节喂原始流，禁容差；红/绿/缺失三态本评审独立实证（见下）。 |
| **no-patch-mentality（不打补丁掩盖）** | 通过。§4.2/§4.4 把不可归因与 v4 溯源缺陷照实写出、不伪造归因，这一点是这份交付最扎实的地方。MED-3 是归因不完整，不是掩盖。 |
| **v3 硬禁令（无 alpha / 无策略择优）** | 通过。全文 `[L1机制/费率未标定]` 标签在案，§8 明文「不作 alpha 论据、不作策略择优输入」；未见任何优劣判断或参数取舍表述。 |
| **formalization-validity-domain（231号）** | 通过且做得好。§8 显式标 L2、显式收窄「§4.3 复合 bit-exact」的有效域到读数集 × 三窗 × 双臂，明文「声明不抬格为全定义域 bit-exact」，并声明 trades 逐字节层未对照。 |
| **result-package 六要素** | 通过（结论 §0 / 依据 §2-4 / 边界 §8 / 下游 §9 / 谱系引用 §8 引 231号 / 影响 §10）。 |
| **Python 仓内标准** | 基本通过。类型注解齐、退出码分层沿用姊妹件先例、`print` 惯例与 `check_fixture_drift.py` 一致（仓内既有标准覆盖 `python/hooks.md` 的 logging 提示）。缺陷见 LOW-2/3/4/5。 |
| **零 git mutation / 只读承诺** | 通过。`rust/src` 零改动，diff 仅三个新增文件（`git show --stat` = 359+69+170 行，全新增）。 |

### Spec 轴

**#387 票体验收标准逐条**

| # | 标准 | 结论 |
|---|---|---|
| 1 | 臂A/臂B 按 v4 命令重建、标「重建基线」、与 v4 发表表逐项比对落盘 | **PASS**。§1 命令表与 v4 报告 §1 逐字对齐；「重建基线」在抬头三处；§2 对照表本评审独立复算全中。 |
| 2 | 臂R 三窗逐位对照表落盘；零差异则声明成立，有差异则逐条归因 | **PARTIAL**。对照表齐、数值真；归因缺臂R §2.2 一条（MED-3）。 |
| 3 | 臂R trades digest golden 落地（含生成命令）作后向回归锁 | **PASS**。golden 三窗齐，`_regen_command` / `_check_command` 元字段在案；本评审独立复算三窗 digest 全中。 |
| 4 | 不变量清单逐窗断言全绿（读数落盘） | **PASS**（书面）。§6.1 跑批内两条 + §6.3 机器件清单 + §7 全量 `--lib` 日志在案。注：§6.3 是「机器件存在且在全量测试里绿」的清单，不是逐窗读数——报告未混淆此点。 |
| 5 | 全部跑批命令/env/落盘路径记录 | **PASS**。§1 + §5 齐；dump 布局与 v4 的差异照实登记并给了 `M8_WIN_FILTER` 的中性实证。 |
| 6 | 基线复跑 1928 passed / 1 failed 未劣化 | **PASS**。`/tmp/387_full_lib_test.log` 尾部逐字复核：`1928 passed; 1 failed`，唯一失败即 `extract_signals_bit_exact_digest_guard`（#115 线）。 |

**与母 SPEC #385 的对齐**

- **四臂设计**：本票是 T1，只承臂R（+ 臂A/B 基线重建），臂D/臂C 归 T2/T3——分工与 #385 Further Notes
  的 T1/T2/T3 拆分建议一致，未越界。✓
- **回归锁**：#385 Testing Decisions 要求「臂R 的 trades digest 与关键读数落成机器可复核的对照文件」——
  golden 同时冻结 digest + n_trades + per_dir_n/pnl + nest_depth_hist，超额满足「关键读数」。✓
- **报告口径**：#385 要求格式对齐 v4（同表同口径）。§2 四张表逐一对应 v4 §2.1/2.2/2.3/2.4，可并排阅读。✓
- **红线**：#385 的「红」= 不变量破 / 臂R 发散无法归因 / 标定臂标签缺失。第一、三条不触发；
  第二条触发并照实上浮——**上浮动作本身忠实，但上浮材料受 MED-3 影响（见重点核查⑤）**。
- **偏离登记**：§9 三条冲突登记（票体「均 Off 应零差异」前提被推翻、#385 修复链枚举缺 #308/#309、
  臂R 不可二分）措辞诚实，未把前提失败包装成跑批异常。✓

---

## 重点核查五条

### ① 「复合 bit-exact 成立」的证据强度

**方法核验通过，规模表述有两处错。**

- **真的地方**：`diff /tmp/m8_report_2fd5ee08ba_A.md /tmp/m8_armA_report.md` 与
  `diff /tmp/m8_report_2fd5ee08ba_R.md /tmp/m8_armR_report.md` **仅差一行口径标签文案**
  （#374 报告口径改动），19 列数据行**逐字段全同**——§4.3 的核心事实成立，双臂三窗都成立。
  臂R 的 `[m8]` 行 -1268551 / -5427281 / +6346975 与 `/tmp/m8_win_gate.out` 原始行逐位一致。
- **错的地方**：提交数 48 → 实为 37（MED-1）；「含全部九项」→ 实为八项，#246 在区间外（MED-2）。
- **有效域**：报告 §8 已自行把该声明收窄到「本报告读数集 × 三窗 × 双臂」，并声明 trades 逐字节层
  未对照 `2fd5ee08ba`↔HEAD——这个收窄是对的，没有抬格。
- **本评审独立复算**：三窗臂A trades 逐笔聚合（479/504/518，Long/Short n 与 pnl，nest_depth 直方图）
  与 §2.1/§2.4「本批」列全中；三窗臂R 与 golden 全中。

### ② 臂R 发散「不可归因」登记是否照实

**「9/10 不可独立构建」成立且抽查通过；但登记范围比证据宽一格。**

- 独立复核 `git rev-list --count 640609071..12aa929d44` = **10**，与报告一致。
- 抽查（实际逐份扫了全部 9 份）：`ec728bf6cf` / `a080e97b1c` 各 15×`E0432`+4×`E0433`；
  `3a0dc77963` 12×`E0425`；`bb01eeb655` 8×`E0063`；`bbbd8f89fa` 33×`E0063`+30×`E0308`；
  `d2312352f9` / `997c920b7d` / `65292c7e44` / `9a48915d72` 同类——**9/10 全部 `error: could not compile`**。
  第 10 个 `12aa929d44` 是首个可构建态且臂A `execR=+923705`（= v4 锚值）。断言成立。
- **没有把无法归因包装成已归因**——这一点明确通过，§4.2/§8 两处都写了「不是『未做』，是在册提交序列上
  不可做」「不以『大致归到某票』冒充归因」。
- **但**：臂R §2.2 那部分是**可以**归因的，证据在交付方自己手里（MED-3）；且「4cde104869/12aa929d44 处
  已是今日形态」这两个支撑点无留存证据（MED-4）。所以「不可归因」的**登记态度**照实，
  **登记边界**画错了。

### ③ v4 溯源缺陷论证

**独立复核完全成立，这是本交付最硬的一条。**

| 论点 | 独立复核 |
|---|---|
| `640609071` 上门控代码不存在 | `git grep -c THETA_NEST_CERT_GATE 640609071 -- rust/src` **零命中**；`NEST_GATE_STATS` 同。对照：`ec728bf6cf` 起 2/3 命中。✓ |
| 门控代码入库于 v4 报告次日 | `ec728bf6cf` 日期 **2026-07-21**，v4 报告日 2026-07-20。✓ |
| 臂A ≡ 臂B（门 env 无效） | `cmp /tmp/m8_v4head_A/trades.jsonl /tmp/m8_v4head_R/trades.jsonl` → **逐字节相同**（698127 bytes）。✓ |
| p3fold execR = -15383474 ≠ 发表表 +923705 | 数值本身只在工作日志 `/tmp/p387.log` 与报告中留存，**无原始跑批 stdout**。前三条已足以支撑「v4 HEAD 标注对读数无效」的结论，故此条证据薄弱不单独升级；但与 MED-4 同源，属同一类证据留存缺口。 |
| v4 臂A 发表读数的真实代码态 = `4cde104869` | `/tmp/bisect_log/4cde104869.out` 三行 `[m8]` 与 v4 §2.2 臂A 三行**逐位相同**（+923705/0.0409/+1005147/-827536 等）。✓ |

结论：**v4 报告的 HEAD 标注对其读数无效**——论证独立成立，登记方式（标「非本票修复范围」）恰当。

### ④ golden/校验器防静默漂移的能力

**机制过关，覆盖面够，触发条件是弱项。**

- **独立重跑**：`python3 scripts/check_armR_trades_digest.py` → exit **0**，三窗 digest 全中。
- **独立复算**：自写 FNV-1a64（不复用被评脚本）对三份 `trades.jsonl` 复算：
  `0x6bf47daf0aa737cd` / `0x282c28ca8ca65e16` / `0xcafa7c4846c762cd`，与 golden **全中**；
  bytes / n_trades / per_dir_n / per_dir_pnl / nest_depth_hist 五项亦全中。
- **红绿切片本评审独立实证**（非采信交付方自述）：
  | 切片 | 操作 | 结果 |
  |---|---|---|
  | 绿 | 复制三窗产物到临时目录 | exit **0**，三行 ✓ |
  | 红 | wf7 尾部追加**一个空格**（1 字节） | exit **1**，打出 `digest: golden=0x282c… actual=0xe967862cfeb215c2` + `bytes: 270684→270685` |
  | 缺失 | 指向不存在的 dump 根 | exit **4**，三条缺失路径 + 再生成提示 |
  ⟹ 单字节漂移可被抓住，字段级差异可读，退出码分层可被机器消费。
- **覆盖字段**：digest（逐字节，兜底一切）+ bytes + n_trades + 分向笔数 + 分向 pnl（`repr` 存，禁容差）
  + nest_depth 直方图。对本票的对照维度（§2.1/§2.4）**完全覆盖**；§2.2/§2.3（execR/门控统计）
  不在 golden 内，但那些在 stdout 层，报告未声称 golden 覆盖它们。
- **regen 可复现性**：`_regen_command` 与报告 §5、脚本 docstring 三处命令**逐字一致**；
  `M8_WIN_FILTER` 的 bit-exact 中性本评审给出了**比报告更强**的复核——全窗跑批根目录 dump 与
  wf8 分窗 dump **逐字节相同**（双臂各一次 `cmp`），报告只声称了 `[m8]`/`NEST_GATE_STATS` 行相同。
- **确定性实证复核**：`cmp /tmp/m8_determinism_check/trades.jsonl /tmp/m8_win_gate/p3fold/trades.jsonl`
  → 逐字节相同。✓
- **弱项**：LOW-1（未接 CI + 输入在 /tmp + 缺产物不判红）、LOW-3（regen 无门）。
  即：**能抓住漂移，但不会自己去抓**。

### ⑤ SPEC 红线「臂R 发散无法归因」的登记忠实度

**上浮动作忠实，上浮材料不完整。**

- 报告 §0.2 / §4.2 / §8 / §9.3 与 resolution 评论四处措辞**一致**，无一处淡化：
  resolution 明写「SPEC 红线触发（照实登记，待 Lead 裁定）……是判『红』打回，还是接受解释……提请 Lead 裁定」，
  并把编排者倾向（后者）**标为建议而非结论**。没有把红线偷偷降级，也没有用「已归因」措辞掩盖。这一点通过。
- **但材料有两处缺口**，都直接作用于 Lead 的裁定输入：
  - MED-3：臂R §2.2 那部分**可归因到 #309**，材料里没有。裁定语境应是「臂R 发散 = #309 部分（可归因）
    + pre-#309 部分（不可二分）」，而非现在的「臂R 侧不可归因」。
  - MED-4：「门控侧发散早于 #309」这个把发散推出声明链之外的关键论点，其两个锚点无留存证据。
- 另：§9.2 建议 T3 采「枚举链复合 bit-exact 成立；发散归 #309 + 不可二分区间」的分离口径——
  这个口径本身是对的，但因 MED-2，「枚举链」在 T3 引用时须改述为八项（#246 另引二分链证据）。

---

## 最小修复路径（全部为文本订正 + 一次补跑，不需重跑主批）

1. **MED-1**：报告 §4.3、commit message、#387 resolution 三处「48 个提交」→ **37**（或写明口径）。
   复核命令：`git rev-list --count 2fd5ee08ba..0dc5bc785d`。
2. **MED-2**：§4.3 把区间覆盖改述为
   「含 #351/#356/#360/#361/#363/#365/#374/#376 **八项**；#246 全部提交早于区间起点
   （`4cde104869` 即 #246 封口提交、亦即 `2fd5ee08ba` 之父），其臂A bit-exact 由二分 GOOD 链
   （`12aa929d44`..`4cde104869` 逐点 `execR=+923705`）间接覆盖，**臂R 侧无覆盖**」。
   §0.1「整条修复链」同步改述。
3. **MED-3**：§4 新增「4.2b 臂R §2.2 归因」，落 `4cde104869_R` ↔ `2fd5ee08ba_R` ↔ HEAD 三点表
   （数值见本报告 MED-3），写明分解式；同步订正 §0.2、§9.3 与 resolution 评论中
   「臂R 侧不可归因」→「臂R 侧 §2.2 归 #309、§2.1/§2.3/§2.4 不可二分」。
4. **MED-4**：二选一——
   (a) 补跑臂R @`12aa929d44` 与 @`4cde104869`，stdout 落 `/tmp/bisect_log/<sha>_R.out`（含
       `NEST_GATE_STATS` 行）并在 §4.2 逐行引用；或
   (b) 把这两句降级为「未留存原始输出的过程观察」，并从「门控侧发散早于 #309」的论证中移除，
       改以 `/tmp/m8_4cde104869_R/trades.jsonl` 笔数 168（= HEAD wf8，≠ v4 449）作为可复核的弱化支撑，
       同时在 §8 登记该论点的证据等级。
   （(a) 恢复原声明强度，(b) 保持 090 一致但削弱红线论证；建议 (a)。）
5. **LOW 批**（可随 T2/T3 批处理）：LOW-1 把校验器接进 CI 或在 §5 写明触发条件；
   LOW-2 拆 `EXIT_MISSING` 双语义；LOW-3 `--regen` 加门；LOW-4 缺字段 fail-loud；
   LOW-5 对 golden 未覆盖的窗告警。

---

## 附：本评审执行的独立验证命令清单

```bash
# 工位：/tmp/nc-review-387（detached @ 0dc5bc785d），全程只读
python3 scripts/check_armR_trades_digest.py                          # exit 0
python3 scripts/check_armR_trades_digest.py --dump-dir <复制目录>      # 绿 exit 0
printf ' ' >> <复制目录>/wf7/trades.jsonl && python3 …                # 红 exit 1
python3 scripts/check_armR_trades_digest.py --dump-dir <不存在>        # 缺失 exit 4
# 自写 FNV-1a64 + 逐笔聚合，复算臂A/臂R 三窗（未复用被评脚本）
git grep -c THETA_NEST_CERT_GATE 640609071 -- rust/src                # 零命中
git rev-list --count 640609071..12aa929d44                            # 10
git rev-list --count 2fd5ee08ba..HEAD                                 # 37
git merge-base --is-ancestor 4cde104869 2fd5ee08ba                    # YES
cmp /tmp/m8_v4head_A/trades.jsonl /tmp/m8_v4head_R/trades.jsonl       # 相同
cmp /tmp/m8_win_gate/trades.jsonl /tmp/m8_win_gate/wf8/trades.jsonl   # 相同
cmp /tmp/m8_determinism_check/trades.jsonl /tmp/m8_win_gate/p3fold/trades.jsonl  # 相同
diff /tmp/m8_report_{2fd5ee08ba,arm}A*.md / _R*.md                    # 仅标签行差
grep -oE "error: could not compile" /tmp/bisect_log/*.out             # 9/10 命中
```

**未能独立复核的项（登记）**：臂R @`12aa929d44` / @`4cde104869` 的 `NEST_GATE_STATS`（MED-4）、
v4head 的 `execR=-15383474`——两者均因原始 stdout 未留存，且重建需在 `/tmp/bisect387`
（已删）重新 clone + 全量 release 构建，超出本评审工位的只读边界。
