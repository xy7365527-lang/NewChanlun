# #1419 收尾第 4 项：真实 BTC 尾 5 万根上的中枢证据（L3 塔、S 写者、oracle 三方对拍）

> 本地证据。组合树 `/private/tmp/goal1323-native-20261012/tb1419-x1405-composite` 由两个基点组成：
> **#1419 `48e2386463`**（远端 `codex/1419-center-seed-20261013` 头；组合树代码取 `9cd64da5cd` 加 `9cd64da5cd..ecc9b196d3` 的补丁，`ecc9b196d3..48e2386463` 只改文档和一行生成器注释，已同步）与 **#1405 未合入头 `199b820c9d`**。
> 合并未提交、不推送，也不进任何共享分支。上一版（v1.2 口径、裁定前）存为 `REALDATA-L3-v1.2.md`（sha256 `8f266f6d…`）。

## 结论：根裁定 L3 解除

按根裁定 L3（`tb1419-impl/ROOT-RULING-1419-L3.md` sha256 `cce5a83f…`），L0 单元区间改取成员笔方向极值（schema v1.3 §1）之后，9 个切点全部通过：

- S 输入合同 9/9 PASS（v1.2 时是 9/9 拒）；
- S 单元与塔单元逐位相同，全量 cut 72/72，零不符；
- S 写者对塔逐位相同：比较首末成员、ZD、ZG、DD、GG，零差异；
- S 对独立 oracle 零差异：首末、结算、突破单元与方向、续扫锚、ZD/ZG/DD/GG/D/G 的值与贡献者，以及 seed 尝试序列；
- 塔对 oracle 零差异；
- Q 合同 `validate_center_cut`（Python）9/9 ACCEPT，浏览器合同 `validateCenterCut`（JS，node 中运行）9/9 ACCEPT。

**全量 cut（49,995 根）** 上，塔 L0 发布 16 个中枢，S 写者发布 10 个中枢窗口，oracle 扫出 10 个窗口。三者的对应关系是：

- 8 个成员少于 9 的窗口（7 个已结算，[66,71] 开放、游标 EXTENDING）：S、塔、oracle 三方逐位相同；
- 2 个成员不少于 9 的窗口（[38,53] 16 段、[55,64] 10 段）：S 标 `UPGRADE_PENDING_TB02H`，塔按 §9.5 R22 过渡态在其中各切出 5 个、3 个子中枢。这 8 个子中枢全部落在 S 的 UPGRADE 窗口内，没有落在窗口外的塔中枢。子中枢语义归 #1429。

合计 8 + 5 + 3 = 16，与塔一致。seed 尝试 S 与 oracle 各 11 次，其中 1 次排除。

**比较器不是空过。** `mutate_l3_compare.py` 在全量 cut 上做了四个变异：塔中枢 ZG+1、S 中枢 DD−1、删去一个塔中枢，三者都在对应比较里各报出 1 处差异；把 S 对象里一个判别单元的区间改回几何端点，Q 以 `TB02E_UNIT_SOURCE` 拒绝。

## 读数（切点阶梯，start_offset=5，`ThetaConfig::default()`）

| bars | 笔 | 段 | 塔 L0 中枢 | oracle 窗口 | S 中枢 | 几何端点≠成员极值的段 | S 输入合同 | S~塔 | S~oracle | 塔~oracle | Q(Py) | Q(JS) |
|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|---|---|
| 600 | 51 | 6 | 1 | 1 | 1 | 4 | PASS | 0 | 0 | 0 | ACCEPT | ACCEPT |
| 1,000 | 80 | 11 | 2 | 2 | 2 | 5 | PASS | 0 | 0 | 0 | ACCEPT | ACCEPT |
| 2,000 | 173 | 17 | 3 | 3 | 3 | 7 | PASS | 0 | 0 | 0 | ACCEPT | ACCEPT |
| 5,000 | 424 | 24 | 4 | 4 | 4 | 11 | PASS | 0 | 0 | 0 | ACCEPT | ACCEPT |
| 10,000 | 773 | 24 | 4 | 4 | 4 | 11 | PASS | 0 | 0 | 0 | ACCEPT | ACCEPT |
| 20,000 | 1,583 | 24 | 4 | 4 | 4 | 11 | PASS | 0 | 0 | 0 | ACCEPT | ACCEPT |
| 30,000 | 2,391 | 24 | 4 | 4 | 4 | 11 | PASS | 0 | 0 | 0 | ACCEPT | ACCEPT |
| 40,000 | 3,221 | 30 | 5 | 5 | 5 | 14 | PASS | 0 | 0 | 0 | ACCEPT | ACCEPT |
| 49,995 | 4,016 | 72 | 16 | 10（8 个 <9、2 个 ≥9） | 10（8 + 2 UPGRADE） | 35 | PASS | 0 | 0 | 0 | ACCEPT | ACCEPT |

「几何端点≠成员极值的段」一列就是 v1.2 口径会拒掉的单元数。它在每个切点上都非零，所以这 9 个 cut 都是判别例。全量 cut 上按方向计，32 段的起点、16 段的终点不是段内极值。

**全量 cut 的 10 个窗口**（成员位置）：[0,4]、[6,11]、[13,16]、[18,20]、[22,27]、[29,31]、[33,35]、[38,53]（16 段，≥9）、[55,64]（10 段，≥9）、[66,71]（开放）。

**段数在 5k–30k 之间停在 24，未解决，留待 #1420。** 这 25,000 根上没有新的已确认段，推断是某个第二种候选一直没有等到分型。组合树不含 #1420 的 078:52 失效语义，这一点只作观察，要等 #1420 合入后复测。

**#1405 读数在组合树上复现。** 笔 4,016、段 72，与 #1405 收据相同。

## 订正：此前报的 39/22

上一版说「72 段中有 39 段起点、22 段终点不是段内极值」，这个数是拿全部 `stroke_refs` 的包络算的。全部 `stroke_refs` 一直延伸到 observed_through，会越过段端，所以包络偏大。按成员笔（`construction.stroke_indices`）的方向极值计，应为 32 段起点、16 段终点；几何端点区间与成员笔极值区间不同的段为 35 段，这个数与 v1.2 拒绝收据里涉及的 35 个单元一致。正本 `xianduan.md` §8、schema v1.3 差异头和语料生成器注释已在 `48e2386463` 里订正。根裁定文件里的旧数我没有改动。

## v1.2 读数（裁定前，留作依据）

schema v1.2 §1 的单元区间取 CC-013 几何端点 `sort(stroke_refs[0].start_price, geometric_end.price)`，塔经 `make_segment`（`rust/src/theta_v0/parser/segment.rs:687-726`）取成员笔方向极值。合成流和 oracle 流上二者恒等，真实数据上不等。结果是 S 写者在 9 个 cut 上全部被输入合同整 cut 拒绝，全量 cut 的拒绝收据列出 48 处不符、涉及 35/72 个单元；塔与 oracle 在塔单元上 9 个 cut 逐位一致。用几何端点单元喂 oracle，同样得到 10 个窗口，但从第 3 个窗口起边界就和塔不同（[14,18] 对 [13,16]），这是同一判定的输入取了两种值（#804）。产物 `l3-dump-v1.2.json`（sha256 `9c1b6169…`）、`l3-readout-v1.2.json`（`51d3a99f…`），由 `analyze_l3_v12.py` 复算。

## 方法与复现

**数据**：`tb1419-impl/data/btc_tail50k.json`（sha256 `4c005de4…`），按 lib `load_symbol` 同一量化导出成整数 K 线，存为 `realdata/btc_tail50k_int_bars.json`（sha256 `979eefd2…`）。

**组合树合并**：`git merge --no-commit --no-ff 199b820c9d`，冲突解法如下。

- `s_structure_session.rs`：`RULE_REVISION` 合成为 `s2-axis-quantifiers+new-bi/2+segment-second/1+center-seed/1`。
- `s_tb02_contract.py`、`tb01c-client.js`：`CENTER_RULE` 取上述合成值；#1405 的 `RULE_CONTRACTS` 增加该行，取值为 (s-new-bi/2, s-segment/2, 知识域 V2)。cut 校验并用 #1405 的代际绑定和 #1419 的 RULE_MIX 门。
- `s_query_integrity._published_axes`：两条 segment-second 规则取 `SEGMENT_SECOND_AXES`，中枢规则取 `CENTER_AXES`。
- `index.html` logicalSlot：同时认 s-new-bi/2 与 s-center/1。
- JS 导出：并入 `validateBiPayload`。

**L3 修复引入**：`git diff 9cd64da5cd ecc9b196d3` 存为 `realdata/l3-port.patch`（sha256 `231c83f0…`），用工作区 `git apply` 套上。tb02e.rs 测试区那一块因组合树在同处插了临时测试而冲突，按补丁原文手工插入；套完后组合树 tb02e.rs 与 #1419 头只差 87 行临时测试。`s_tb02_contract.py`、`tb01c-client.js` 干净套上（偏移 63、45 行）；语料、真实判别夹具、`test_tb02e_real_units.py` 与 #1419 头逐字节相同。修复前的组合树 tb02e.rs 备份为 `realdata/composite-tb02e.pre-l3.rs`。

**两条临时测试**（只在组合树，不提交）

- `stroke_tb02v_tests.rs::tmp_tb1419_dump_int_bars`：lib 侧导出整数 K 线。
- `tb02e.rs::tmp_tb1419_realdata_l3_dump`：S 二进制测试，`--release`，本次用时 851.70 s（收据 `l3-dump-run-release-v1.3.txt`，sha256 `0fd08c99…`）。每个 cut 依次执行生产 parser `parse_layer_with_inclusion_facts`、生产塔 `classify(..).classification.levels[0].centers`、S 写者 `tb02c::build → units_from_segments（输入合同）→ tb02e::build`，输出 `realdata/l3-dump.json`（sha256 `eca24ad0…`）。

**比较**

- `realdata/analyze_l3.py`（sha256 `2447c77c…`）：S 侧单元直接调用 Q 合同的 `_member_extremes`；导入 S0 oracle（`.chanlun/review-results/issue1419-tb02e-20261013/oracle_check.py`，只用标准库，不读项目实现）。做 S~塔、S~oracle、塔~oracle 三组比较，并逐 cut 跑 Q `validate_center_cut`。输出 `realdata/l3-readout.json`（`5bb3d7e4…`），控制台收据 `analyze-l3-v1.3.txt`（`090d2e11…`），末行 `ALL_CUTS_PASS True`。
- `realdata/mutate_l3_compare.py`（`17704d46…`）：比较器变异检查，收据 `mutate-l3-compare.txt`，末行 `ALL_MUTANTS_CAUGHT True`。
- `realdata/js_validate_l3.cjs`：组合树浏览器合同逐 cut 复核，收据 `js-validate-l3.txt`，末行 `JS_ALL_ACCEPT true`。

**未覆盖**

- 浏览器页面渲染（JS 合同只在 node 里跑，没有起页面）。
- 成员不少于 9 的窗口的子中枢语义：归 #1429。
- 段数停在 24：归 #1420 合入后复测。
