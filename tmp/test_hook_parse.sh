#!/bin/bash
INPUT=$(cat)
# Method 1: echo + python
M1=$(echo "$INPUT" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('tool_name',''))" 2>/dev/null || echo "FAIL1")
# Method 2: printf + python
M2=$(printf '%s' "$INPUT" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('tool_name',''))" 2>/dev/null || echo "FAIL2")
# Method 3: grep + sed (like hub-node)
M3=$(echo "$INPUT" | grep -o '"tool_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | sed 's/.*"tool_name"[[:space:]]*:[[:space:]]*"//;s/"$//')
echo "echo+python: [$M1]"
echo "printf+python: [$M2]"
echo "grep+sed: [$M3]"
