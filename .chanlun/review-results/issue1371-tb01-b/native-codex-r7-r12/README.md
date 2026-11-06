# #1371 原生 Codex 交付记录

本目录是绑定 #1371 的报告工作草稿。入口依次为 `ROOT-ACCEPTANCE.md`、`IMPLEMENTATION.md`、`r12/REVIEW.md`；原候选的完整失败独评在 `r7/REVIEW.md`。各报告保留原候选 SHA、原结论与实际原生 Codex 身份，底层未暴露的模型回执没有补造。

`COMMIT-DECLARATIONS.md/.json` 逐项说明历史和新增产品提交的可编译性及证据复用。此目录后续只有报告/附件提交，按交付纪律纯文档豁免；最终 Git 提交与产品等同性由仓外 `FINAL-DELIVERY.json` 固定，避免把包含自身 SHA 的文件放入自身提交。

`EVIDENCE-INDEX.json` 将独评引用中的 `inputs/...` 或旧绝对路径映射至指定 Git 对象或 `EVIDENCE-ATTACHMENTS.json` 的原字节附件。附件以 SHA256 为键，先 base64 解码，再 xz 解压，必须复核字节数及 SHA256；JSON 内的原来源路径和时间保持不变。附件只是具名报告引用、配对载荷与 R12 执行/页面证据，完整阅读清单的其余原件仍在原冻结包，不称为全部输入副本。没有把会话导出、完整数据库、二进制或构建备份加入仓库。

R12 回归入口为 `node s_session/tests/r12_consumer.cjs`（在仓根运行）。它使用实际 HTML 函数与已捕获 HTTP 原字节，不需要在线服务；函数级检查、真实页面、真实 native 及三次进程终止分别计数。

历史 `VALIDATION.json`、旧矩阵或准备文案中 GUI_PENDING、Prime 待批准及宽泛的 R5 原件缺失表示各自写入时的状态；以本目录根验收与新原生复审明确更新，冻结原件不回写。

#1371 的 B 子域结论不能替代 TB-01、17 个完整来源义务或 #1323 六个 Destination。经济域和后续 C 义务仍开放；#1448 同 ID 迁移验收未满足。发布、合入与关票只按实际授权和后续 main/CI/工位事实执行。
