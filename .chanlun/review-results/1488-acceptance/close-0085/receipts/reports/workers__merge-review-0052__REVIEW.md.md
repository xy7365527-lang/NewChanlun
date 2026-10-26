# 收据 workers/merge-review-0052/REVIEW.md

- 原件：`workers/merge-review-0052/REVIEW.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`33ef178d4bb32c0ebe1b11adb09bfd982aba8719d4947a97479bb621836d62f8`
- 字节数：13280；行数：97

## 关键行（行号: 原文）

```
1: # #1488 合并面非作者独审（0052）
3: - 评审对象：merge `57f9680c4ed9256108da4d1369734fbc0fe195cd`（第一父冻结源 `fff166540e`，第二父 main `6badaa6049`，merge-base `b46ba29699`）
6: - 主控裁定 D8（issuecomment-6283355919）：group_reasons 去 equal_extreme 以 main 上 #1405 语义为准（同价只作证据、不作选根门）
8: ## 结论
10: **合并解法：通过。合入门：阻塞（P1-1：锁 r14 红，需主控/非作者裁定重锚）。**
12: 冲突解法没有静默丢掉任何一侧：冻结源已裁定的行为（预算 v4 / CodecVec 准入、slots 抽取与原生贡献路径、Recover 受控迁移、journal、originals 增量许可、锁断言）和 main 的 #1405/#1394 新行为都保留了。D8 逐字一致。oracle 同步没有削弱独立性。r14 失败的归因属实；但作者给出的重锚方案 (a) 数字有误（P2-1）。
14: ## 1. 冲突解法逐块（remerge-diff 9 文件）
16: | 块 | 核对结论 |
23: | A2/P1 pending 调用与头部 | 两侧参数和两行都保留。✔ |
24: | P2/P3 step_record / request_record | main 的 `waiting_count` 版本化逻辑原样移进 `step_record`（source_pending.rs:337-355），`request_reasons` 调用参数与 main 同序同值；`StepView::waiting_count` 两种实现都有；原生路径（source_contributions.rs:1110/1342）按 `self.rule` 计算 `equal_is_evidence`，并有 `state.rule != rule` 拒绝守护（1800 行）。✔ |
33: ## 2. D8：group_reasons 与 main #1405
39: ## 3. 自动合并区域的隐性语义冲突
48: ## 4. oracle 独立性
54: ## 5. bsp 锚
59: ## 6. 测试证据核验
63: | 锁 79/80（`20261020-123407-57f9680c4e.log`） | 命令行的 80 个测试名与 `.github/1488-uo-locks.txt` 80 条集合相同；合并没改锁清单；唯一失败是 r14（audit_owner.rs:4266，left 111361 / right 111334）。属实 |
64: | 冻结基线锁 80/80（`20261020-112049-fff166540e.log`） | 属实 |
67: | CI 39370285846 | headSha = 57f9680c4e；组0 `uo (s_session)` success（1 PASS + 79 NOT_LISTED）；组1 失败 = 79 PASS + r14 FAIL(101)，左右值与快通道相同。属实 |
70: ## 7. 非测试 cargo check
74: ## Findings
76: ### P1-1｜锁 r14 在合并面红，合入门阻塞（不是解法错误）
78: - 反例：合并面上 `claims.1 = 111361 ≠ 111334`，CI 组1 FAIL(101)，快通道 79/80。原因是 main #1405 的 `CURRENT_CONTRACT` 字符串净增 27 B，进了 node_body，预算逻辑没有回归（对照实验已证实）。
79: - 处置：需要主控或非作者裁定 r14 怎么改（字面重锚，或改成按合同长度推导），改完后在终版重跑锁清单和 CI 两组。在此之前不得合入 main。
81: ### P2-1｜REPORT §4-1 重锚方案 (a) 算错：527,048 应对应 +54，不是 +27
82: - 位置：`audit_owner.rs:4255`（`SEALED_BEFORE_P2A`）、4266-4271；`workers/merge-0047/REPORT.md` §4-1(a)
83: - 反例：合并面 sealed = 399,101（基线 399,074，+27），①+② = 111,361 + 16,640 = 128,001。按 (a) 字面只改 111,334→111,361，再把 527,048 加 27 得 527,075：527,075 − 399,101 = 127,974 ≠ 128,001，第三条断言仍然失败。原因是 body 多出的 27 B 在 P2-A 之前的世界里会算两次：正文一次，同额物理来源许可一次。要让恒等式成立，必须用 527,102（+54）。注释里的 `node_body 33,443B` 也应改成 33,470。
84: - 另外，527,048 是在 b6969af4cc 上实测的历史值，改成 527,102 就成了反事实值，不再是测量值。建议裁定时优先选 (b)：从合同长度或 node_body 推导；如果选 (a)，要把常量注明为“按 /2 合同折算”。
86: ### P3-1｜冻结基线的 bsp 白名单 lib 锁本来就是红的
90: ### P3-2｜A1 用 `producer.rule_revision()` 代替 main 的 `rule` 参数，目前恒等，属于隐性耦合
95: ### P3-3｜作者的冲突相关测试证据只覆盖 native 特性组，而且跑在非终版上（评审已补齐）
```
