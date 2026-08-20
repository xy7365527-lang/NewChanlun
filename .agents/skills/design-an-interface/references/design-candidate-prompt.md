# Design candidate prompt (Prime RLM)

Design one interface candidate for **{MODULE_DESCRIPTION}**.

## Requirements and assigned pressure

- Requirements: {REQUIREMENTS}
- This candidate's distinct design pressure: {DESIGN_PRESSURE}
- Read budget: at most {READ_BUDGET} files
- Tool budget: at most {TOOL_BUDGET} calls
- Time budget: at most {TIME_BUDGET}
- Output budget: at most {OUTPUT_BUDGET}
- Absolute fallback result file: `{RESULT_FILE}`

Stay independent: do not ask other candidates for their designs and do not spawn another orchestrator. Use the caller's needs to choose the seam; do not merely rename a conventional interface.

## Work and stop rules

Use the repository's read/search tools only. Do not modify project files. Stop exploring when any budget is reached, when only the final {RETURN_TOOL_RESERVE} tool calls remain, or at about {CONTEXT_STOP_PERCENT}% context. Preserve those final calls for result return. Put uncertainty under `Open questions`; never invent missing evidence.

## Complete result (usage first)

Return one self-contained result with these sections:

1. **Caller's usage** — README-style usage plus two or three realistic call sites: imports, calls, and returned values.
2. **Interface signature** — types and methods derived from that usage.
3. **What it hides** — policy, representation, sequencing, or infrastructure callers no longer coordinate.
4. **Trade-offs** — strengths, limits, and why this seam differs materially from the alternatives requested by the root.
5. **Red-flag screen** — explicitly check shallow module, information leakage, temporal decomposition, and pass-through method; revise or reject the candidate if one remains.
6. **Open questions** — unresolved facts, or `None`.

## Return protocol

1. Keep the complete result as your final assistant response so the parent can recover it from the final session JSONL.
2. Import/use the available `agent_message` skill and attempt `await agent_message.send(message=<the complete result>, receiver_role='parent')`.
3. If `agent_message` is unavailable or not imported, or sending fails, write the **same complete result** to `{RESULT_FILE}`. This fallback is your only permitted write. Do not write a partial marker.
4. Admission metadata, progress updates, rollout previews, and source-tool output are not the result. Do not finish on a source `toolResult`; finish with the complete result.
