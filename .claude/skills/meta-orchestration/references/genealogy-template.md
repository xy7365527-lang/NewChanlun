# 谱系记录模板

每一次矛盾的发现、概念分离、和处理都使用此模板记录。

```yaml
id: [唯一标识，如 GEN-001]
timestamp: [时间戳]
status: [生成态 | 已结算]
type: [矛盾发现 | 概念分离 | 蜂群内部消化 | 编排者决断 | 回溯结算]
negation_type: [homogeneous | heterogeneous]  # 同质（Claude内部质询）或异质（外部模型质询）
negation_source: [可选，异质否定时填写模型名，如 "gemini-3-pro-preview"]

# 矛盾（type=矛盾发现 时必填）
contradiction:
  description: [什么跟什么冲突]
  layer: [概念 | 代码 | 编排]
  trigger: [什么操作/事件暴露了这个矛盾]

# 概念分离（type=概念分离 时必填）
separation:
  before: [分离前：什么概念被当作同一个]
  after:
    - name: [分离后概念A的名称]
      definition: [定义]
      source: [原文依据]
    - name: [分离后概念B的名称]
      definition: [定义]
      source: [原文依据]
  pending_verification: [分离后需要实证回答的问题]

# 涉及的定义
definitions_involved:
  - name: [定义名]
    version: [版本或引用]
    role: [这个定义在矛盾中扮演的角色]

# 解决方式
resolution:
  type: [概念分离 | 定义修正 | 结构重组 | 未解决]
  description: [具体怎么解决的]
  decided_by: [编排者 | 蜂群内部]

# 被否定的方案
negated:
  description: [之前的定义/设计/方案是什么]
  why_negated: [为什么走不通——逻辑必然性，不是经验总结]

# 新产出
new_output:
  definitions: [分离后的新定义列表]
  code_changes: [相关代码变更摘要]
  orchestration_changes: [编排层面的变更]

# 影响范围
impact:
  affected_modules: [受影响的模块列表]
  affected_definitions: [受影响的其他定义]
  downstream_implications: [对下游的推论影响]

# 回溯结算（如果适用）
retroactive_settlement:
  settled_by: [哪个后来的定义/记录解决了这个矛盾]
  settlement_date: [结算时间]
  settlement_description: [怎么被回溯性地解决的]

# 谱系关联
related_records:
  parent: [如果这个矛盾是旧矛盾的深化，引用父记录]
  children: [如果这个矛盾后来产出了新的矛盾，引用子记录]
```

## 使用说明

- 每次矛盾处理后立即写入，不要事后补。
- `status: 生成态` 表示矛盾已被辨认但尚未完全解决，或其意义尚待后续确定。
- `negated.why_negated` 必须写清楚逻辑上为什么走不通，不能只写"不好用"或"出了bug"。
- `related_records` 维护谱系的树状结构，确保新记录与旧记录的关联不断裂。
