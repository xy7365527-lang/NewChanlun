# 旧 `tb03b-finance/2` 九字段冻结 manifest：TestOnly 实验件，不保留旧读法

> 名分：#1394 交付报告的登记件（工作草稿，活期绑定 [#1394](https://github.com/xy7365527-lang/NewChanlun/issues/1394)，世代宪法 §5）。
> 依据：根裁定（2026-10-14，经 /root 转 /root/tb1414_p2_fix）；APPROVED-SOURCE §5.7 修订记录 A6 r2（同目录 `A6-5.7-REVISION.r2.md`，SHA-256 `0e62036346ac11e637a9662da77419bb13f0c1013e789e03466f049fc1b46476`）§2.4、第 5 节与第 9 节「待 A4 核」。

## 登记的事实

A4（整合分支 `0c657f4492`、`c1b7d3cb5a`）之后，`finance.structure` 的 exact 字段由九个变为十个（增 `reader_session`），`schema_revision` 仍为 `tb03b-finance/2`（A6-1）。新 E/B 代码对九字段 `/2` manifest 一律拒绝，离线恢复入口 `verify_evidence` 也一样：它先经 `SourcePins::from_structure`，`content_store::verify_image` 又经它核每条证据。所以下列旧件在新代码下不能离线审计。

本机扫描到 44 份九字段 `tb03b-finance/2` 冻结 manifest，`clock.kind` 全为 `TestOnly`，不含 `reader_session`：

| 位置 | 份数 |
|---|---:|
| `/Users/silencehan/Projects/NewChanlun-1394-content-v3/.scratch` | 37 |
| `/private/tmp/goal1323-native-20261012/finance/cap80-driver-draft` | 4 |
| `/private/tmp/goal1323-native-20261012/finance/run-1` | 1 |
| `/private/tmp/goal1323-native-20261012/finance/reusable-inputs` | 1 |
| `/private/tmp/goal1323-native-20261012/finance/archive-readonly` | 1 |

逐份的路径、SHA-256、字节数、structure 字段数、是否含 `reader_session`、clock kind 与来源见同目录 `old-tb03b-finance-v2-manifests.json`（SHA-256 `3edf09c26b71865cfdbe568e5b9b59f235c9b84106e6d215f8e4f7052bc5b531`，由 /root/tb1394_c2_review 产出，扫描口径在文件的 `scan` 字段）。同一清单另列 4 份十字段件（`r3/p8/dry`，P-8 干冻结产物），不是旧库，不在本登记范围。

## 根裁定

这 44 份是实验件，不属任何生产或验收证据，没有在新代码下审计的义务；不为它们保留旧读法（#799：同一判断不留第二套读法）。

## 与 A6 r2 的关系

A6 r2 §2.4 与第 9 节把「九字段与 `null` 读法的旧 `/2` 库在新代码下不能离线审计」与 §5.7:364（历史恢复与本地审计不依赖 S）的关系列为待 A4 核对的唯一一项，核对办法是确认现存这类库全为 TestOnly 且无新代码下的审计义务，否则回根。上表与根裁定结清该项：现存九字段库全为 TestOnly，根已裁无审计义务。第 5 节退场前提 2 中关于 `null` 读法经济库的部分，随逐片读法在合入 #1394 的 PR 中删除时另行核对，不在本登记内。
