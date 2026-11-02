# #1778 LOW 修订 28J

依据：`1778-INDEPENDENT-REVIEW-28I.md`（PASS，三项 LOW）。只改措辞与验收细节，裁定语义不变；未改代码、ADR 0051、GitHub 票或 main。

改动文件：`/Volumes/AgentStorage/decisions/1778/{draft.md,archaeology.md,archaeology.json}`（改前备份在 `/tmp/{draft1778,arch1778,arch1778json}.bak`）。

1. **#1776 笔误改为过去时**：三份材料均写明票面第 4 项曾写「#1776 blocks #1768」，主控已于 2026-10-27T23:42:02Z 更正；删除「票面由主控更正」。`forbidden_add` 保留。
2. **裁定 5、6 及第三节、考古 Q5 措辞**：「本域已绑定」统一改为「本会话已选择，且本域 meta 已在 init 绑定」；`service.rs` 分派条件同步。
3. **验收细节**：
   - 第 1 项改为 `cat rust/src/economic_session/*.rs | grep -c '"domain-event-withdrawal/1"'` ≥ 1（合并后单一计数）。
   - 第 5 项点名命令：`python -m pytest -q s_session/tests/test_*.py s_session/tests/issue1527_recovery/test_python_r4.py` 与 CI 同口径 `pytest -m "not slow" -q`，均退出码 0。路径按 origin/main 实测：上述文件含 `test_` 函数且导入九个读者模块之一；CI `test` job 只跑 `tests/`。
   - 第 7 项增加 `git diff --numstat origin/main -- CONTEXT.md` 删除列为 0。

检查：`archaeology.json` 通过 `python3 -m json.tool`；三份文件已无「本域已绑定」「由主控更正」；`23:42:02Z` 在 draft 3 处、archaeology.md 2 处、json 3 处。
