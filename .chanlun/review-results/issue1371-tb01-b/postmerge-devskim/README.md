# #1371 / PR #1449 固定 256 扫描的合入后尾归档（草稿）

工作草稿，绑定 #1371（TB-01-B），父图仍为 #1323。建议仓内目录为 `.chanlun/review-results/issue1371-tb01-b/postmerge-devskim/`。本包保存已经完成的安全分诊与修复可行性报告原件，不扩大产品验收范围。

扫描只绑定 PR head `256cd822e0d872f1fb65d433ca0b15b01fa621a2` 和 DevSkim run `34709866745`；实际 checkout 合并 tree 与该 head 相同，证据见 `evidence/devskim/CHECKOUT-BINDING.json`。根报告该版本已按用户条件后备授权合入 `main=d2219e8e014a5d55c091f8eda62ffb9dd8154953`。这是对固定版本 DevSkim **FAIL** 的本次例外，不能称扫描已绿、实际风险清零或后续文档版本已经扫描。

主线 CI、最终清理和交付状态由根代理另补；本工位未核这些结果。尾归档本身若产生新文档提交，也属于之后的版本，应按实际检查状态单独陈述。

## 四份报告原件

`reports/` 下的两份 MD 和小型可行性 JSON 保留可直接阅读的原件；大型 `PR1449-DEVSKIM-R14-TRIAGE.json` 通过内容寻址附件保留完整原字节。分诊结论仍为 9,233 个基线外键，其中 9,232 个有具体反证，1 个旧 entry 映射待办为 needs_review；不能虚填实际安全缺陷数为零。可行性结论是保持冻结字节、scanner/ignore 与现行 baseline 语义时没有范围内真实修绿方案。

## 读取与校验

`EVIDENCE-INDEX.json` 的每项保留源绝对路径、建议仓内/还原路径、原字节数及 SHA256。按 `artifact_key` 查 `EVIDENCE-ATTACHMENTS.json`，base64 解码后再 xz 解压，核 `bytes`、SHA256，并应与源原字节相等。25 项归档原件均有附件；其中 23 项还保存原件副本。原 SARIF 另列为外部原件引用，保留在票属 D，不放入附件容器。标记为 `attachment_only` 的建议路径是逻辑还原路径，不是假称那里已有独立文件；完整分诊 JSON 和完整 DevSkim 日志仅存附件。

`PACKAGE-VALIDATION.json` 记录逐项完整还原验证和包内文件清单。README、SARIF 来源说明、索引、附件容器及验证文件不进入附件索引，验证文件也不包含自己的摘要，避免递归自包含。验证对象是原件归档完整性，不是重新执行扫描器或产品。

## 保留的机器证据与不重复收录项

`evidence/devskim/` 保存对应 REPORT-FREEZE、artifact/run/job 与 checkout/configuration 绑定、容器跨度和原门禁日志。原 PR SARIF 的绝对路径、109,483,815 字节长度、SHA256、run/artifact ID 与下载信息保存在 `SARIF-REFERENCE.json`。原件已核字节并继续保存在票属 D；它单独压缩仍为 13,741,752 字节、经 base64 后约占 18.3 MB，会重复带入已有附件，故按最新限定排除。Actions artifact 有保留期限，固定身份不意味着永久可下载。`evidence/rules/` 保存可行性 freeze、固定上游 commit 的规则原件与来源摘要。`evidence/repair-probe/` 保存反事实诊断 RESULT 及 stdout/stderr，其中 stderr 原件为 0 字节。

反事实 baseline 和门禁投影未重复收录。需要复核其逻辑时，可由 `SARIF-REFERENCE.json` 所指并留在票属 D 的原 SARIF、固定 head 的 `.github/devskim-baseline.json` 与 `scripts/devskim_sarif_gate.sh` 重建等价输入：保留 SARIF version、各 run 的 driver name/version、全部 results 的 ruleId 和首位置 URI/startLine；按原 gate 规则取唯一键 K。令 B 为固定提交的 baseline 键，p 为 `DS176209|rust/src/theta_v0/nautilus/strategy.rs|314`，临时反事实集合为 `B ∪ ((K \ B) \ {p})`。它含 17,433 键，原 gate 仍剩 p 一个外键而失败。这里说明的是门禁语义等价重建，不保证临时 JSON 排版或原投影字节一致，也没有在本归档过程中重跑此诊断；不得把它写入生产 baseline。

未打包旧报告/整个工作目录、原 SARIF 容器、重复 ZIP、巨型反事实文件、数据库、工具程序、会话导出或备份。未改现有仓库与已冻结报告，未提交、推送、新建票或启动 Prime。
