# #1406 离线验收 loopback 的精确误报登记

## 改动与用途

只向 `.github/devskim-reviewed-1488.json` 新增一个文件条目和一条 DS162092 完整结果身份。受检文件是 `.github/scripts/run-1406-c6a.py`，原始命中在第 788 行、第 77677 列。该值用于核验 Q 启动端口原件的 loopback 合同，非生产调试遗留。

完整结果身份和文件字节绑定由现有 `scripts/devskim_reviewed_false_positives.py` 与原始 SARIF 重新计算。其他 76 个文件条目逐项相同。受检脚本、baseline、gate 和扫描器均未修改，同行另外三处位置没有预登记。

本工作承接 #1406 与 #1488 的既有机制，解除 #1740 暴露的安全检查阻塞。独立语义裁决原件为 `1406-LOOPBACK-REVIEW26BP.md`，本报告不代表本候选已完成独审、CI 或 main 合入。

## 实际验证

使用 `scripts/devskim_sarif_gate.sh` 回灌两个真实扫描原件，保留工具和 artifact 合同参数。

| 输入 | 原清单 | 本候选完整清单 |
| --- | --- | --- |
| #1740 失败扫描 run 40343213245 | FAIL，排除 2835 条 | PASS，排除 2836 条 |
| main 成功扫描 run 40330532339 | 既有成功原件 | PASS，排除 2835 条 |

在同一完整清单上修改失败原件中的目标结果。列改到 61283、77905、78628，各自不被排除且 gate 退出 1。路径变化或规则变化，各自不被排除且 gate 退出 1。受检脚本的临时副本末尾增加一个换行后，完整清单加载报原件身份变化。临时副本不修改工位源码。

`git diff --check` 通过。未运行 Rust、业务验收或新的扫描器实例，没有靠重复扫描取得偶然成功。

## 复跑与后续

原件、回灌日志及可复跑脚本保存在外盘 `issue1709-pstack-orchestrate-20261025/1406-exact26bp-evidence/`。运行 `python3 verify-negative-controls.py` 可复跑负控，脚本内保存受检工位和原件路径。真实 SARIF 原件来自同级 `1740-devskim26bp/`。

本候选须经非作者独审、CI 和用户 main 门。合入后由 #1740 owner 更新其 CI 的受检合并树，不对旧 run 原样重试。未来其他位置产生的新身份仍会报红，须按实际 SARIF 精确核验。

Prove It Works 使验收使用原始失败和主线 SARIF，并用位置、路径、规则和源字节变异证明条目不会扩大匹配范围。
