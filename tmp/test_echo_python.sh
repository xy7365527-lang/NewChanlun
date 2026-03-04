#!/bin/bash
# 最小化测试：echo + python 在 Git Bash 中是否能解析 JSON
INPUT='{"tool_name":"Write","cwd":"C:/test"}'
R1=$(echo "$INPUT" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('tool_name','EMPTY'))" 2>/dev/null || echo "FAIL")
echo "test1_simple: $R1"

# 带嵌套的 JSON
INPUT2='{"tool_name":"Write","tool_input":{"file_path":"C:/Users/hanju/NewChanlun/.chanlun/test.md","content":"hello"},"cwd":"C:/Users/hanju/NewChanlun"}'
R2=$(echo "$INPUT2" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('tool_name','EMPTY'))" 2>/dev/null || echo "FAIL")
echo "test2_nested: $R2"

# 从 stdin 读取（模拟 Claude Code 框架）
R3=$(echo "$INPUT2" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('tool_name','EMPTY'))" 2>/dev/null || echo "FAIL")
echo "test3_stdin: $R3"
