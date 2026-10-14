# #1424（TB-02-F）实施工作草稿

> 名分：工作草稿，绑定 [#1424](https://github.com/xy7365527-lang/NewChanlun/issues/1424)，票关后随归档出仓。分支 `codex/1424-center-pair-20261014`，基线 #1419 分支头 `07479fd501`（在前置未合入的分支上预实施，根并行规则第 3 条；#1419 合入 main 后 rebase）。根裁定见 #1424 issuecomment-6160553615（六问）。备料在仓外 `/private/tmp/goal1323-native-20261012/tb1424-prep-r2/`。

| 文件 | 内容 |
|---|---|
| `oracle_pairs.py` | 独立 oracle（纯标准库，不导入项目代码）：W0–W12 工作例的中枢、CC-018 位置、CC-020 五叶与 16 格矩阵、塔子中枢对照。输出即 `rust/tests/fixtures/center_pair_1424_worked_examples.json` |
| `receipts/s1-*.txt`、`s1-mutants.json` | S1（内核）先红后绿、lib 全量与 7 个变异体收据 |

工作例编号（W12 为三切面修订流）：W0 成对最小流；W1/W2 向下/向上扩展；W3/W4 R26 判别例（Zn 外缘判新生、全单元外缘判扩展）；W5 外缘恰等归扩展；W6 核心相切归首叶（两单元连接段）；W7 外缘包含与 #1478 端点条款连接段；W8 #1419 E4 两单元连接段；W9 ≥9 窗口一端待定；W10 CC-018 相切位置；W11 核心重叠；W12 B 开放期 DOWN_NEWBIRTH→DOWN_EXPANSION。
