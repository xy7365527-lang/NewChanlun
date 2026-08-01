# 纵向 / 横向 Resources

## Knowledge

- [`docs/chanlun/text/blog/038-第38课.md`](../../chanlun/text/blog/038-第38课.md) ——
  缠中说禅第38课《走势类型连接的同级别分解》。
  **本课（第 0001 课）的最高权威一手源，主要依据。** Use for：同级别分解的定义、
  "不需要中枢延伸/扩展"的规则原文、"某级别不延伸/以下全延伸/以上不考虑"三段式规则、
  分区卷钱的机械（多操作级别并行）的原文出处。重点段落：18、20、22、24、26、28、30、36。
- [`docs/chanlun/text/blog/`](../../chanlun/text/blog/) —— 缠中说禅全部 110 课语料。
  Use for：任何需要溯源到"原文明文"而不是"推断"的场合；本课引用之外的其他概念（延伸/扩展/升级
  的精确数学定义、区间套的正式定义等）大概率在这批语料的更早课号里，需要时逐课检索。
- [`docs/adr/0010-chong-persistent-operating-unit.md`](../../adr/0010-chong-persistent-operating-unit.md) ——
  ADR 0010「重」的名分裁定。Use for：「重」的三要件定义（〈标的、操作级别、专属筹码〉）、
  「重」与「声部」的错位登记、「重」和「买卖操作」的区别。
- [`.chanlun/definitions/level_recursion.md`](../../../.chanlun/definitions/level_recursion.md) ——
  级别递归教义正本。Use for：级别 = 递归层级（`level_id`）这条基础定义的形式化表达，
  是"纵向递归造塔"这个比喻背后的正式定义来源。
- [GitHub issue #787](https://github.com/xy7365527-lang/NewChanlun/issues/787) —— 架构正本地图。
  Use for：这门课存在的最终目的——给这张 50 张子票的地图画出站得住脚的模块边界。
- [GitHub issue #827](https://github.com/xy7365527-lang/NewChanlun/issues/827) ——
  本课对应的架构裁定票。Use for：追踪"纵向造级别 / 横向读级别"这条结论在架构地图上落地成了
  哪条具体裁定。
- [GitHub issue #842](https://github.com/xy7365527-lang/NewChanlun/issues/842) —— 分区卷钱的机械
  v0 原型票。Use for："多重赋格"落地为工程原型的第一手记录，也是 ADR 0010 的直接触发点。
- [GitHub issue #840](https://github.com/xy7365527-lang/NewChanlun/issues/840) —— 三轴分离。
  Use for：「重」的判据如何解开"主从 vs 并列"两幅图从未对齐的历史遗留矛盾。

## Wisdom (Communities)

本仓语境高度自有（架构裁定、术语判据都是本仓编排者与蜂群协作产出，通用缠论社群不共享这套上下文），
暂不推荐外部社群。待学习者提出需求再补。

## Gaps

- 延伸 / 扩展 / 升级 / 区间套 的精确数学定义课号：本课引用范围内只确认了它们在 038 课的使用场景，
  没有确认定义它们的具体课号。下次需要精确定义时，从 `docs/chanlun/text/blog/` 里往第38课之前找。
