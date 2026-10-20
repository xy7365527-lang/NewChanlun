# #1406 C6a 原资源与 guardian 合同恢复查核

名分：绑定 #1406 / #1323 的恢复工作草稿。核查日期 2026-10-17。本次仅恢复证据，不改源码、不派发、不执行产品、不访问外盘；未重复远端 CI 状态查询。

**结论：在下列已查范围没有找到 C6a 旧 CI v3 原包，不能恢复其 RSS、磁盘、输出容量、监测周期的确切数值及完整 guardian 合同。现存评论能证明原限额必须保留，不能给出缺失限额；不允许据 C7a 或当前重建件推理填值。**

## 搜索范围和结果

1. 读取持久副本 `issue1406-economic.json` 的正文与全部 14 条评论，提取 C6a、guardian、RSS、审包摘要。文件 SHA256：`3187e2bdc4ff8bca111a550368e8562d6ae1a5978327c12685fbfc97b47780bb`。与临时目录原 JSON 摘要相同。
2. 本地 Git 760 个 refs：执行 `git rev-list --objects --all`，只对返回路径按 `(^|[/_-])(?:c6a|run-c6a|test-run-c6a)(?=[/_.-]|$)` 忽略大小写筛选；exit 0，候选路径数 0。边界避免把内容寻址文件名随机含 `c6a` 当合同。该结论仅覆盖当前本地 refs 可达对象的具名路径，不包含未提交或不可达对象，不是整个历史内容不存在的证明。
3. 精确历史路径及 `.github/`、`s_session/` 的 c6a 文件 glob：`.github/scripts/run-c6a-remote.py`、`.github/scripts/test-run-c6a-remote.py`、`.github/workflows/ci-c6a.yml` 的 `git log --all` 无提交。旧 `codex/1406-c6a-ci-prep-20261016` 仍指 `4d9b072e5b671fc1a84f7d75b3976471b93d2afb`；其 `.github/scripts/` 只有 C7a 及 cargo 通用脚本，没有 C6a 原件。
4. 主仓 `.scratch/`、`.chanlun/review-results/` 的 Markdown、JSON、Python、YAML 文本，关键词 `\bC6a\b`、`6199094453`、`b693f936`、`run-c6a`；有效窄查限制 4 MiB 以下，排除 session/rollout/thread/conversation 路径与 JSONL 会话导出。只命中本轮恢复目录中的图快照、issue JSON、RECOVERY、economic-next-step 和重建报告，没有旧资源合同。文件名 `*1406*/*c6a*/*C6a*` 扫描另命中旧 #1404 HTTP 序号1406文件；这是请求序号，不是 C6a，未作为证据。
5. 阅读作者 `c6a-ci-rebuild.md`，其当前持久副本 SHA256：`3a9af2e7980fd130f6483ac80570498f437bf00cb590c44798a2b42cbd51dd39`。该文件明确新准备件 `CONTRACT_READY=False`，不是完整 runner；其新增代码与12项测试不能恢复旧合同来源。

没有扫描凭据目录、全盘、外盘；没有以会话密文或无关历史任务内容作为证据。未查询远端所有未获取 refs，未尝试恢复 Git 不可达对象。

## 已恢复的原文与出处

来源均为上述 SHA256 固定的 issue JSON；comments 可按 URL 定位，评论正文不是已恢复的原审包。

[2026-10-15 17:37 评论](https://github.com/xy7365527-lang/NewChanlun/issues/1406#issuecomment-6198366223)：

> 外层同t0端点1200/1830/1920/2040/2070秒、内层600/630/300/10/30及TERM3/KILL3、资金/隔离/恢复门均不变。

> CI v1独审 **2 HIGH + 1 MEDIUM**：oracle与公共图像缺逐值绑定；exchange按全case汇总未绑具名步骤/角色/请求内容；init/source CLI误接受无TERM回执的-15退出。

该评论记录 CI v1 审包 MANIFEST：`b693f936b024c1fabd6d30548ff71a491be0cf98dd7cdc7f5b2d2442eed93861`，9件；**只有摘要引用，没有原件路径或包字节**。

[2026-10-15 18:15 评论](https://github.com/xy7365527-lang/NewChanlun/issues/1406#issuecomment-6199094453)：

> 只有连接阶段 ECONNREFUSED、未发送、零响应、原绝对启动截止内才重试。

> CI v2 的原 2 HIGH / 1 MEDIUM 及六反例已闭合，但独审发现新 H3：Capture 原件/事实及 Recorded 回执 cut 未与 after owner 绑定。两个 hash 自洽反例被误接纳，组合继续阻断。

> 原内外同原点截止、取消/回收、保留原件及资源门不变。PENDING/CONTRACT_READY=False，尚未 push 或实际运行 C6a。

该评论记录 CI v2 审包 MANIFEST：`d5b5c3e9f16111f4c49630f967cc2d88dc03141282633b83c14884c5b377f0b2`，8件。driver v4 包 MANIFEST：`b6273d2e1662d16822d7ce7cc994019a442383089cb592c137f9e8de2b771369`，41件。两者同样只有引用，不能把摘要当恢复出的原件。

评论6191944368中的 guardian/35分钟job/33分钟step/180秒测试/60秒工具属于 **C5** 历史修复；不把它提升为 C6a 资源及 guardian 原合同。

## 准确缺失项

| 条款 | 当前能恢复的边界 | 尚缺原始事实 |
|---|---|---|
| RSS | 资源门必须保留 | 数值、单位、逐进程/进程组/全任务聚合口径、峰值/采样判定、越界动作 |
| 磁盘 | 资源门必须保留 | 启动最低可用空间、运行中低水位或增长上限、统计路径/文件系统、越界封存与回收条件 |
| 输出容量 | 保留原件 | stdout/stderr/证据包分别或合并上限、单位、截断/拒绝/停止规则、清单封包预留 |
| 监测周期 | 同一 t0 时间端点存在 | 采样周期、单次工具观测超时、监测失败如何处理、超限发现延迟允许范围 |
| guardian | 取消/回收必须保留，TERM3/KILL3已明写 | 原已审实现/合同；进程组身份保有及leader未收割规则、后代发现/退出收割、guardian自身监督/超时、取消后失败封包与终态确认的完整顺序 |
| 时间端点语义 | 原数列已恢复 | 原 CI v3 如何把每一端点绑定具体阶段的已审实现；不能仅凭常量推断监督正确 |
| 原反例 | 六反例和 H3 两反例的存在、问题类别、包摘要已知 | 可运行原件、准确字段/输入/预期结果、每项编号及其与新准备件的对应关系 |

## 可继续与不可签署的界线

可以继续完成不依赖缺失数值的绑定、transport、startup、封包与受控反例工作，并维持 `CONTRACT_READY=False`。原审包如另有已知保存位置，应按已记录 MANIFEST 恢复及逐项验哈希；不存在可读原包时，重订资源合同属于明确的新取舍，不能冒充“恢复原值”。本报告不指定任何替代数值，也不批准真实运行。
