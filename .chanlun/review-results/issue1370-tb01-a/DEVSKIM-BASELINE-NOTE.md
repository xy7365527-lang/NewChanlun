# #1370 验收档案与本地观察入口的 DevSkim 误报登记

名分：绑定开放票 #1370 的工作草稿，记录 PR #1447 的配套检查；不改变已签规范、生产源码或原验收载荷。

固定提交 `17e9afb6fbad` 的[真实扫描](https://github.com/xy7365527-lang/NewChanlun/actions/runs/34334331420)完成扫描器构建、扫描和 SARIF 上传，gate 因基线外告警失败。固定扫描器身份为 `1.0.90+fb2d676ce4`。SARIF 有 8,231 条结果、8,200 个精确键；与归档分支 `38a6b3f6f7d6` 的实际 SARIF 相比，新增恰为 2,614 键、删除 0 键。

本次仅向既有 5,587 键末尾追加这 2,614 个逐规则、文件、行号核实的误报键，共 8,201 键。旧键顺序、历史元数据和基线文件第 860 行均保留；来源单独记入 `updates`。原 8 个基线外告警继续保留。扫描器、规则、DevSkim workflow、gate 和目录扫描范围均不变。

| 新增规则 | 键数 | 核实依据 |
|---|---:|---|
| DS173237 | 2,564 | 验收记录的源码、构建、载荷摘要和 Git 标识；生产目录的 1 键为已签来源文件摘要。源码清单中的摘要与固定提交字节重算相等，目录来源摘要也单独重算。 |
| DS117838 | 26 | 验收记录中的业务身份、测试收据、结果字样，以及名称含 key/pass 的源文件摘要字段；匹配内容无认证用途。 |
| DS162092 | 24 | 19 项验收记录中的本地访问地址，加 launcher 的 4 项地址展示和只读服务的 1 项默认回环监听。读取入口以服务参数绑定本地观察用途。 |

2,608 个新增键位于本票验收档案；另外 6 键分别位于 `s_session/launch_s.sh:234–237`、`s_session/s_readonly_server.py:351`、`s_session/catalog/signed-catalog.json:4`。逐键核过固定提交的文件摘要、源行及 SARIF 匹配区间，未因所在目录或规则名称直接放行。独立候选复核另核新增集合、原键顺序、旧告警保留和误报语义，并用原 gate 对真实 SARIF 重算；实际仅旧 8 键返回 FAIL、退出码 1。此结果不能当作新扫描或整个 PR 全绿。

旧 8 键及其处置边界：

- `analysis/p3_random_gate_control.py` 的 141、197、251、344 行为旧研究脚本的说明和非密码学随机对照；[#1316](https://github.com/xy7365527-lang/NewChanlun/issues/1316)已将该研究线冻结退役，本片不重启它。
- `rust/src/theta_v0/nautilus/strategy.rs` 的 19、104、113 行为既有说明，314 行对应 `entry_tick_for` 尚无完整限价腿映射。旧入口处置由 [TB-10-B #1385](https://github.com/xy7365527-lang/NewChanlun/issues/1385) 的 OE09 义务承接，属于 [TB-10 #1368](https://github.com/xy7365527-lang/NewChanlun/issues/1368)；当前没有据此宣称该功能已修复。新会话首片不扩建该旧入口。

新提交的最终结果须以 PR #1447 当前 head 的实际 CI 为准；原 SARIF 和本地重算都不能替代重扫。main 合入仍待明确批准，#1370、TB-01、SPEC 和整图均保持开放。

[逐键主审](DEVSKIM-CI-REVIEW.md)与[独立候选复核](DEVSKIM-CANDIDATE-REVIEW.md)保留签署 Markdown 原件。独立复核为 PASS、0H/0M/0L；报告所引完整 JSON、逐键明细和本地 gate 输出按其中的原始路径及指纹保留，未重复复制入此目录。实际 SARIF 可由上文 Actions run 的产物取得。
