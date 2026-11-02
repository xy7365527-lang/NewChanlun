# #1488 DevSkim 精确误报豁免

主CI 40201355404 在 761ba618 候选成功，15个执行job成功、macos-session按配置跳过。DevSkim 40201355388 原始扫描成功，baseline比较报1541个新增键；不得称全CI通过。

## 已审分类

1504个DS173237键是Git身份、文件/数据库/双跑摘要、序列指纹与hex编码比较值；36个DS162092键为隔离验收的127.0.0.1；1个DS440010键为CI脚本EXPECTED_SHA标识符（不是SSL调用）。74文件：73份验收材料加run-1488-driver-ci.py；对应2799个完整SARIF result。

全量扫描、原始SARIF上传、baseline均保持。经主控批准，新增可选误报清单参数；清单绑定文件SHA256与完整result的规范化JSON SHA256，包括规则、全部位置、片段、消息与属性。身份以32个数字字节保存，避免将审计清单的摘要再次误判为硬编码秘密；不改变摘要算法。仅完全匹配的result可过滤，然后才归并旧三元键。任何文件漂移、路径越界、缺失或坏清单均失败。

清单：.github/devskim-reviewed-1488.json；schema devskim-reviewed-false-positives/v1；归属1488。清单变更必须独审。原件字节不改，无目录豁免、无全局禁规则、无baseline增项。未配置参数的既有调用不要求Python或清单。

## 验证与适用性

靶向测试覆盖既有门禁、精确结果、同旧键不同列/片段的新结果、新规则、原件漂移、缺失、越界/符号链接、坏schema与身份、scanner/upload/tool失败、无参数兼容。

真实原始SARIF回灌：94257个results中精确豁免2799，剩余与未修改baseline比较PASS；24个历史baseline键已消失。此结果验证gate处理，不冒充新候选GitHub扫描。新代码及清单仍待独立安全审查和GitHub CI。

原代码c5f49adc97的Rust验证与独审仍适用；本补丁不改Rust，不重跑Rust。新gate/helper/清单的独审单独进行。

仓外原件与日志：/Volumes/AgentStorage/issue1488-ticket-manager-20261018/workers/resume-20261025/（DEVSKIM-TRIAGE.md、devskim-new-findings.json、devskim-artifact/devskim-results.sarif、devskim-targeted-tests.log、devskim-real-sarif-gate.log）。
