# Stage69 文档采用有限差异复审 v2

2026-10-06，研究票 #1467 与实验票 #1468。结论为 Accept。冻结 v1 的 R1、R2 均已关闭，主摘要、交付说明 v2、进度补充 v2 和发布草稿四字段的文字口径可以采用。没有必须修订项。

本次只判断文字采用与发布草稿口径，不确认发布动作已经执行，不审尚待生成的最终证据总表，也不判定研究总目标完成。

## 输入与差异范围

实际读回 `adoption-inputs-v2.json` 指定的 8 项输入，全部与指定 SHA256 匹配，结束时再次核对，8 项字节均未变。前后指纹见 `inputs-before-sha256.json`、`inputs-after-sha256.json`。

主摘要与 `stage69/adoption-freeze-v1/Stage69LongerHistoryAndActualLearner.md` 逐字比较，确实只把采用复审链接和交付说明链接升到 v2，数值、语义和限定没有变化。交付说明仅改版本标题、采用复审链接及 R2 责任表述；进度补充仅改 R1 的出边表述。

v1 主摘要和进度补充的保存副本、原位 `DeliveryStatus-v1.md` 均匹配 v1 输入 SHA。v1 复审清单覆盖的六个产物字节未变。四份作者与独评文字的仓内副本及外部原物均保持 v1 指纹。核查记录见 `diff-audit.json`。此次没有重新运行数学窗口、模型训练或推理，也没有改动被审文件、仓库或 tracker。

## R1、R2 关闭

| 项目 | v2 定位与修订 | 核对依据与结论 |
|---|---|---|
| R1，五条边与固定 S1 的关系 | [progress-addition-v2.md 第 1 行](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/progress-addition-v2.md:1) 改为“5条边的起点均非S1，固定S1无非空根路径”。 | 与数学独评第 49–57 行的五条边 4→9、9→18、13→18、18→27、22→27 一致，排除了将“离开 S1”读成 S1 出边的歧义。R1 关闭。 |
| R2，标签可知性的责任 | [DeliveryStatus-v2.md 第 11 行](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/stage69/DeliveryStatus-v2.md:11) 明写 label_known 仅由可信 host 筛选，学习器只复核 decision 早于 cutoff，不独立验证上游标签可知时刻。 | 与学习器独评第 17–23、62 行及 v1 已核成功 fit 载荷一致。没有把 host 资格控制升格为 learner 的独立时钟证明，OS 权限边界也保留。R2 关闭。 |

主摘要原先已经准确表达这两点；本次只升链接没有削弱它们。v1 报告所接受的其余内容及限制继续适用。

## 发布草稿四字段

审查对象为 [publication-draft.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/publication-draft.json)，SHA256 为 `f3496a1a37c9d49d9f7b41c3c7fab4fed7257d18136afdcf0f006c50227be89a`。四字段齐全，没有额外发布正文。

| 字段 | 采用判断 |
|---|---|
| `research`，第 2 行 | 正确保留固定 S1、78→23→7→7→5、583 前缀空根路径、内部/固定根/原义获证 1/0/0，以及最后的零仅表示未获证。H2 非追加后缀、442/443 价格差、27 段的 13 种长度型与六首窗拒绝依据均准确；RootArm、外缘、一般完成、Owner/Next、NE 和 A64-global 的限定没有被改写。 |
| `experiment`，第 3 行 | 正确区分作者两次成功 fit 与独评各臂一次复算，每臂 169 训练行、172 未拟合记录、400 个共同主决策、800 主推理及 1002 总推理。保留 B13 非强 B、两后段已探索、label_known 由 host 承担、模型 SHA 不同但浮点容差通过、进程无 OS 沙盒、RSS 非硬上限和耗时非生产延迟。没有据 B 的诊断表现推出盘口无效、因果增益或收益。 |
| `arch`，第 4 行 | 明确只是后续设计的条件性接口要求，没有把本轮脚本批准为生产解析器或生产学习系统。返回全部路径或 maximal 路径须声明语义，三类计数和未覆盖材料须分别输出；采用责任与权限边界保持原独评限定。没有形成生产换装、吞吐、成本或递归增量结论。 |
| `mapNote`，第 5 行 | 短摘要保留 H2 非追加后缀、三元组 1/0/0、每臂一次成功 fit、169 训练行和 400 配对主决策，且明写强 B、完整 F₁/F₂及同信息递归增量仍未证。没有将 400 配对决策扩成 800 个独立市场样本。 |

## 最终封存与发布检查范围

`__COMMIT__`、`__COMMENT_URL__`、`__EXPERIMENT_URL__` 是本次草稿中仅有的待填标记。文字采用通过后，发布者仍须填入实际提交及评论链接，并检查引用指向正确产物。本复审没有把待填链接当作已存在的发布证据，也没有执行提交、推送、评论或图正文更新。

`stage69-evidence.json` 要在取得本报告哈希后生成，故不在本次已审对象内。采用复审仓内副本、最终证据总表及其链接和哈希仍须在封存环节核验。这些是实际交付的后续检查，不是本轮已关闭的文字缺陷；不得用本 Accept 声称最终证据总表已经独评。

本 v2 取代 v1 对采用文字的 Revise 结论，保留 v1 报告与原始指纹作为修订依据。原 goal、P1–P4、R_W/R_D 及研究价值判断不由本次文字采用复审关闭。
