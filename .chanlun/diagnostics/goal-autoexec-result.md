# 工位 #26 [goal-autoexec] 结果包

## 结论

`/goal <目标>` 现可自动执行：command 头部 `` !`python scripts/goal_events.py GOAL_SET --description "$ARGUMENTS"` `` 在 skill content 喂模型之前同步跑 shell（蓝色执行反馈 UI），自动写合法 GOAL_SET 到 events.jsonl，stdout 注入聊天框供 Lead 审查。

两处改动：
1. **scripts/goal_events.py** 加 CLI 入口（`if __name__ == "__main__"` + argparse 子命令 `GOAL_SET --description`）：
   - `_slug_goal_id`：中文描述无法 ASCII slug，用 `g-<UTC时间戳>-<描述sha256前8位>`（时间戳保唯一，hash 段锚内容可溯源）。
   - `_cli_goal_set`：`git rev-parse HEAD` 取 base_head + 生成 falsifiable acceptance 骨架（`a0-skeleton`，check 非空，falsifiable=true，结构合法）+ 复用 `append_event`（同一 schema 校验路径，不绕过）。
   - 不破坏 import 用法（`append_event` 签名不变）。
2. **.claude/commands/goal.md** 加 frontmatter（name/description/argument-hint/allowed-tools: Bash(python:*)）+ `!` 自动执行行 + Lead 审查/补全骨架指引。**保留全部运行协议正文**（步骤1-7/终止条件/谱系）。

## 端到端验证（已执行）

用 `--ev-path $(mktemp)` 临时文件（真实 events.jsonl 未被触碰，无需事后清理）：
- `python scripts/goal_events.py GOAL_SET --description "测试目标自动执行验证"` → 输出 goal_id=g-20260630T041525Z-2ce027af / base_head=5b75807 / acceptance 骨架。
- `goal_reducer.reduce_goal` 认出：current_goal 非空、goal_id 以 g- 开头、base_head 对账 HEAD、base_head_stale=False、acceptance[0].falsifiable=True、id=a0-skeleton。
- 空描述被拒：`ValueError: GOAL_SET 缺少必填字段 description`（fail fast，输入验证在边界）。
- 回归：`scripts/tests/` goal_events + goal_reducer + ceremony_scan_goal_driven 共 **71 passed**（含原 65 goal_events，未破坏）。

## 边界条件

- 结论翻转条件：若 `acceptance_vector_hash`/SCHEMA 必填字段表变更使 `a0-skeleton` 骨架不再通过 `_validate_acceptance`，CLI 写入会 raise——需同步更新骨架结构。
- acceptance 骨架是**占位真验收**（"待 Lead 补全"可被"已补全"否证，falsifiable 合法），不是假数据；Lead 必须在 command 后用 GOAL_AMEND 绑具体 acceptance_id 补质量（骨架的 id=a0-skeleton 已预留 amend slot）。
- 新 GOAL_SET 经 reducer 成为最后一个未 SUPERSEDE 的 GOAL_SET = active goal（顶替当前 active）——这是 /goal 设定新目标的预期语义，非 bug。

## 影响声明

- 改动文件：`scripts/goal_events.py`（+CLI 入口，约 60 行，库 API 不变）、`.claude/commands/goal.md`（+frontmatter +自动执行行，正文保留）。
- 影响模块：/goal command 行为（纯 markdown 注入 → 自动写 events.jsonl）。reducer/scan/SCHEMA 未改。
- 不涉及领域概念定义（行动类工具层产出，结果包简化版：结论/边界/影响声明 + 验证证据）。
