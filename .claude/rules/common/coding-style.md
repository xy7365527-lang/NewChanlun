# Coding Style

## Immutability (CRITICAL)

ALWAYS create new objects, NEVER mutate existing ones:

```
// Pseudocode
WRONG:  modify(original, field, value) → changes original in-place
CORRECT: update(original, field, value) → returns new copy with change
```

Rationale: Immutable data prevents hidden side effects, makes debugging easier, and enables safe concurrency.

### 适用范围注记（2026-07-27，编排者裁定）

回放驱动（如 `rust/src/bin/p123_fast_replay.rs`）内的**性能缓存**——驻留游标、memo 等摊销 O(1) 增量累加器——不适用本条：其设计本体即就地更新的可变累加器，改不可变等于把其要消除的超线性重算请回。锚：R2/R3 裁定（`chanlun/escalate/r1-r3-ruling-confirmation-checklist-20260720.md`）+ #69 实装（影子评审 #494 翻转条件①，编排者 2026-07-27 批）。账本/谱系等「不可篡改」域对象不受此注记影响，仍从严。

## File Organization

MANY SMALL FILES > FEW LARGE FILES:
- High cohesion, low coupling
- 200-400 lines typical, 800 max
- Extract utilities from large modules
- Organize by feature/domain, not by type

## Error Handling

ALWAYS handle errors comprehensively:
- Handle errors explicitly at every level
- Provide user-friendly error messages in UI-facing code
- Log detailed error context on the server side
- Never silently swallow errors

## Input Validation

ALWAYS validate at system boundaries:
- Validate all user input before processing
- Use schema-based validation where available
- Fail fast with clear error messages
- Never trust external data (API responses, user input, file content)

## Code Quality Checklist

Before marking work complete:
- [ ] Code is readable and well-named
- [ ] Functions are small (<50 lines)
- [ ] Files are focused (<800 lines)
- [ ] No deep nesting (>4 levels)
- [ ] Proper error handling
- [ ] No hardcoded values (use constants or config)
- [ ] No mutation (immutable patterns used)
