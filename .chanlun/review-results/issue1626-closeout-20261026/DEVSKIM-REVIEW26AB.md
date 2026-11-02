# PR1734 的 Git 提交标识误报登记

DevSkim run [40277078930](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40277078930) 的 job 121421989411 扫描成功，gate 唯一新增键为 `DS173237|.chanlun/review-results/issue1626-closeout-20261026/1693-main-ci-40268271555.json|1`。

## 命中是提交标识

原始 SARIF 的片段为 `"df7beb0c8da5d8f3b6053f85813b86613dcc6849"`，起始列34，字符长度42。该值位于归档 JSON 的 `headSha`，不是认证令牌。`git cat-file -t df7beb0c8da5d8f3b6053f85813b86613dcc6849` 返回 `commit`。实时读取 CI40268271555 也返回此 headSha、completed、success。

DS173237 说明限定为 “If this represents a secret”。本次沿用已合入的 #1725 同类 Git 对象误报登记，主控确认属于既有技术修复范围。

## 精确登记范围

`.github/devskim-reviewed-1488.json` 仅新增本文件的一条结果。既有76项文件登记逐结构比较不变。完整 SARIF result 及原件字节仍分别绑定 SHA256 身份，原件未修改。baseline、扫描范围、规则、gate 代码与workflow均未修改。

本次包含误报登记配置，不再是只有原件复制的纯文书候选。main 合入门应包含该实际差异，仍须用户批准。

## 验证

GitHub原始全仓 SARIF 经现有 gate 回灌。此为已完成扫描的本地重判，不代表新提交已在 GitHub 扫描或重新上传 artifact。结果见外盘 `1734-devskim26ab/gate-replay.log`。

只把该真实 result 的 startColumn 加1，分类器将它保留为未审结果。精确原始 result 排除1条，变更位置后排除0条、保留1条。结果见外盘 `binding-controls.json`。

`python3 -m unittest discover -s tests -p 'test_devskim*py' -v` 的22项测试通过。回灌 gate 退出0，新增未审键为0。原始输出见外盘 `1734-devskim26ab/regression.log` 和 `gate-replay.log`。本次未运行 Rust 编译或重放。

外盘证据目录为 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1734-devskim26ab/`。最终精确head及其CI由交付报告记录，待非作者审查和CI终态，不据此关闭整票或删除旧工位。
