# Code Review

Comprehensive security and quality review of uncommitted changes:

1. Get changed files: git diff --name-only HEAD

2. For each changed file, check for:

**Security Issues (CRITICAL):**
- Hardcoded credentials, API keys, tokens
- SQL injection vulnerabilities
- XSS vulnerabilities  
- Missing input validation
- Insecure dependencies
- Path traversal risks

**Code Quality (HIGH):**
- Functions > 50 lines
- Files > 800 lines
- Nesting depth > 4 levels
- Missing error handling
- console.log statements
- TODO/FIXME comments
- Missing JSDoc for public APIs

**Best Practices (MEDIUM):**
- Mutation patterns (use immutable instead)
- Emoji usage in code/comments
- Missing tests for new code
- Accessibility issues (a11y)

3. Generate report with:
   - Severity: CRITICAL, HIGH, MEDIUM, LOW
   - File location and line numbers
   - Issue description
   - Suggested fix

4. Block commit if CRITICAL or HIGH issues found

5. **Codex 异质审查**（155号谱系：代码层异质否定源）:
   - 如果有 Python 代码变更，触发 codex-challenger 的 review 模式
   - 执行：spawn codex-challenger agent（subagent_type="codex-challenger"），传入变更文件列表
   - Codex 审查结果中 CRITICAL/HIGH 问题同样阻塞 commit
   - **触发方式**：执行 /code-review 的 agent 必须主动 spawn codex-challenger（353号标注：无 hook 自动触发——Claude Code 平台不支持语义级事件，此步为 D策略手动认领）

Never approve code with security vulnerabilities!
