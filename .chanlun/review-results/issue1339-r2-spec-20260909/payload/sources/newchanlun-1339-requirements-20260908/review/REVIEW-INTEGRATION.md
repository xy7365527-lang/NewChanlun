# #1339 需求汇合独立复核

结论：作者9件需求汇合文档通过本次有界复核。初审 **2 M** 已修复并独立复核关闭，当前 **0 H / 0 M / 0 L**。整体需求范围仍待用户确认，程序验收仍未运行；这不是 #1323 整图 goal 完成。

## 两项修复

| 发现 | 初稿问题 | 修订复核 |
|---|---|
| INT-M01 · M · 已关闭 | RF-01验收把共同约束收窄成共同上限，漏掉上界可信但其他既有共同约束不可信的暂停分支。 | 当前四场景分别验证条件齐全必须新增、责任上界未知暂停、另一项既有共同约束未知暂停，以及恢复后不覆盖其他重动作并保留原尾账。 |
| INT-M02 · M · 已关闭 | 35个FG的对象/轴栏填入了领域义务和裁定状态。 | 已逐项改为实际对象及级别、数量、时间、版本等观察维度；名分单列，原判据与边界未变。 |

精确原指针、当前关闭依据和80行检查结果见[同名机器表](/tmp/newchanlun-1339-requirements-20260908/review/REVIEW-INTEGRATION.json)。初稿问题按首次实际读到的文本、稳定ID和指针保留；本报告仅签下列当前字节。

## 贯通核验

- 80行恰为44个ST、35个FG和RF-01，没有撤下的K4条目或额外重数量门。
- 对照已签分表核562个字段；FG-029/032/033的空来源在总表补用户票面正文，属于有效工程需求出处，未赋予实现信用。
- 1,334条OB引用及载荷摘要、411个AC引用、71个FE场景引用均能回到对应条目；具体输出保留，通用字段按语义适用，不预定冗余存储。
- 217个EV展开与已签原摘要、限制完全一致，全部仅历史静态导航，当前源码未逐项复验、靶向执行和端到端验收均未运行。
- 当前OB20、AC15、RF四场景及COVERAGE80行的MD/JSON内容一致；主读入口链接明细并准确区分当前需求票与整图后续阶段。

固定Q、净省禁复用、毛净权利及#937的条件式净账→毛账验收边界都保留。未定量、费率、费用归因、本金/配对、资金政策、阶段衔接与交易权限没有由表格默认值裁定。RF-01来自用户已确认条件，不由局部可观察性推导新经济权限。

## 本次明确签署的作者文件

| 文件 | SHA256 |
|---|---|
| [REQUIREMENTS.md](/tmp/newchanlun-1339-requirements-20260908/author/REQUIREMENTS.md) | `690dcd65e029e5069c46d5ade40684a831405d9e5af214624136e7fd52702b19` |
| [COVERAGE.md](/tmp/newchanlun-1339-requirements-20260908/author/COVERAGE.md) | `f937b8659546be750d28f2d82038f2f64820f4e1d75054f7fe121a3766c847d2` |
| [COVERAGE.json](/tmp/newchanlun-1339-requirements-20260908/author/COVERAGE.json) | `0eeedeb138add9138a2d904bb7fb69884876d85ae085f192d54ab32c8566f970` |
| [USER-DECISIONS.md](/tmp/newchanlun-1339-requirements-20260908/author/USER-DECISIONS.md) | `21d8b29ba2b00c6dfc50acd9ab02040848df56a8c9601a0406ae376d134fbb35` |
| [USER-DECISIONS.json](/tmp/newchanlun-1339-requirements-20260908/author/USER-DECISIONS.json) | `21543f6b61df3e1c94f2b96b5b153e2f492fb3e9799b5d6af9c403ac65770541` |
| [ACCEPTANCE-PLAN.md](/tmp/newchanlun-1339-requirements-20260908/author/ACCEPTANCE-PLAN.md) | `b6f695cbfc1f8f34567c11476d66a3d3773bd747a1cc73898d6f8f1a4e9b825f` |
| [ACCEPTANCE-PLAN.json](/tmp/newchanlun-1339-requirements-20260908/author/ACCEPTANCE-PLAN.json) | `9fc12ed938b2a4949f90b265d6148208b21738f6655a6f9b9e436cc5da767b2c` |
| [OBSERVATION-CONTRACT.md](/tmp/newchanlun-1339-requirements-20260908/author/OBSERVATION-CONTRACT.md) | `d7e9a66eb78434eea22bdec1eb4724c729c5726f3e5103c3e375a30260b20a2b` |
| [OBSERVATION-CONTRACT.json](/tmp/newchanlun-1339-requirements-20260908/author/OBSERVATION-CONTRACT.json) | `65ff5481cab5caf18134d23287e34fa697a1d771b544acb0813aad380d6c9c7d` |

ST、FG、EV分表的当前哈希均与各自已有专项评审签署输入一致，详见机器表的`reused_signed_part_bindings`。旧OBS评审不自动覆盖当前OB字节，本轮已重新核当前20条。

## 射程

本评审只签上述9件。root后续编写的交付索引、manifest不在本签名内；任一作者文本之后再改，须按变更重新复核并绑定。没有改author、仓内代码、Git或GitHub，没有读取禁用旧方案，也没有选择软件组织。

本次检查证明这份需求稿的汇合一致性与已识别缺陷闭合，不能证明程序已实现、生产链已接通、交易能执行或经济指标通过。用户确认范围、架构采纳、SPEC、Sandcastle实施与harvest验收仍按整图后续流程办理。
