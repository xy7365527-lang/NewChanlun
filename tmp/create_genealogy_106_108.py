"""Atomically create 106/107/108 genealogy files and update dag.yaml."""
import yaml
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SETTLED = ROOT / ".chanlun" / "genealogy" / "settled"
DAG_PATH = ROOT / ".chanlun" / "genealogy" / "dag.yaml"


# ── Genealogy file content ──

FILE_106 = """---
id: '106'
title: "中枢定义选择谱系——定义到实现的溯源追踪"
type: 语法记录
status: 已结算
date: 2026-02-22
depends_on: ['004', '001', '002']
related: ['006', '010', '068']
negation_source: "定义选择过程缺乏谱系追踪（暗选择）"
negation_form: "语法记录（将隐性选择写进谱系）"
negates: []
negated_by: []
provenance: "[旧缠论] + [旧缠论:选择]"
---

# 106 — 中枢定义选择谱系

**类型**: 语法记录（已在运作但未显式化的定义选择）
**状态**: 已结算
**日期**: 2026-02-22
**域**: 中枢（Zhongshu / Center）
**定义文件**: `.chanlun/definitions/zhongshu.md` v1.3
**代码文件**: `src/newchan/a_zhongshu_v1.py`

---

## 核心结论

中枢的定义和实现中包含 12 个关键选择点。7 个直接遵循原文 [旧缠论]，5 个涉及选择 [旧缠论:选择]。

## 定义选择列表

### [旧缠论] 直接遵循（7个）

| # | 选择 | 原文依据 | 代码位置 |
|---|------|---------|---------|
| 1 | 核心定义：至少三个连续次级别走势类型重叠 | 第17课 | `a_zhongshu_v1.py:114` |
| 2 | ZG > ZD 严格不等式 | 第17课答疑 | `a_zhongshu_v1.py:117` |
| 3 | 中枢区间固定（前三段确定后不变） | 第20课 ZG/ZD 公式 | `a_zhongshu_v1.py:8` |
| 4 | 延伸判定使用弱不等式 | 第20课中心定理一（突破严格→延伸弱） | `a_zhongshu_v1.py:80` |
| 5 | 突破方向用严格不等式 | 第20课 "dn>ZG", "gn<ZD" | `a_zhongshu_v1.py:92-95` |
| 8 | 波动区间 GG/DD 随延伸扩大 | 第20课 "GG=max(gn)" | `a_zhongshu_v1.py:74-75` |
| 10 | 笔不可作为中枢组件（笔不裁决） | 第17课 "次级别走势类型" | 入口为 `list[Segment]` |

### [旧缠论:选择] 实现者选择（5个）

| # | 选择 | 理由 | 代码位置 |
|---|------|------|---------|
| 6 | 中枢组件必须 confirmed=True | 防止中枢"闪烁" | `a_zhongshu_v1.py:104` |
| 7 | 续进策略 break_seg_idx - 2 | 保证中枢间衔接连贯 | `a_zhongshu_v1.py:135` |
| 9 | v1 省略中枢扩展逻辑 | 单级别管线定位，扩展留待递归引擎 | 未实现 |
| 11 | 确定性 ID（身份键） | 事件引擎 diff 需要 | `seg_start+zd+zg` 组合 |
| 12 | settled 由突破段决定 | "中枢终结"→布尔映射 | `a_zhongshu_v1.py:122` |

## 边界条件

- 若扩展逻辑（#9）被实现，下游模块（背驰、买卖点）可能需适配
- 若 confirmed 过滤（#6）改变，中枢会"闪烁"
- 续进策略（#7）的 -2 是经验值

## 影响声明

- **创建**: `.chanlun/genealogy/settled/106-zhongshu-definition-choices.md`
- **不修改**任何定义文件或代码文件
- **追踪对象**: `zhongshu.md` v1.3 + `a_zhongshu_v1.py` 中的 12 个定义选择

## 谱系引用

- 004号溯源框架、001号退化段、002号源文件不完整
"""

FILE_107 = """---
id: '107'
title: "背驰定义选择谱系——定义到实现的溯源追踪"
type: 语法记录
status: 已结算
date: 2026-02-22
depends_on: ['004', '106']
related: ['005b', '068']
negation_source: "定义选择过程缺乏谱系追踪（暗选择）"
negation_form: "语法记录（将隐性选择写进谱系）"
negates: []
negated_by: []
provenance: "[旧缠论] + [旧缠论:选择] + [旧缠论:隐含]"
---

# 107 — 背驰定义选择谱系

**类型**: 语法记录（已在运作但未显式化的定义选择）
**状态**: 已结算
**日期**: 2026-02-22
**域**: 背驰（Beichi / Divergence）
**定义文件**: `.chanlun/definitions/beichi.md` v1.1
**代码文件**: `src/newchan/a_divergence_v1.py`

---

## 核心结论

背驰的定义和实现中包含 14 个关键选择点。6 个直接遵循原文 [旧缠论]，2 个为 [旧缠论:隐含]，6 个涉及选择 [旧缠论:选择]。

## 定义选择列表

### [旧缠论] 直接遵循（6个）

| # | 选择 | 原文依据 | 代码位置 |
|---|------|---------|---------|
| 1 | 趋势背驰：C段力度 < A段力度 | 第24课 | `a_divergence_v1.py:342-343` |
| 2 | 盘整背驰：同向离开段力度比较 | 第24课 | `a_divergence_v1.py:469-473` |
| 3 | 趋势前提：至少2个同向中枢 | 第24课 "没有趋势就没有背驰" | `a_divergence_v1.py:388` |
| 4 | T4: B段MACD穿越0轴（无阈值） | 第25课 | `a_divergence_v1.py:120-121` |
| 6 | T6: DIF峰值比较 | 第25课 "黄白线不能创新高" | `a_divergence_v1.py:129-164` |
| 7 | T7: HIST峰值比较 | 第25课 "柱子伸长高度" | `a_divergence_v1.py:170-205` |

### [旧缠论:隐含]（2个）

| # | 选择 | 理由 | 代码位置 |
|---|------|------|---------|
| 8 | 无MACD时fallback：振幅x时间 | 原文指出MACD是辅助 | `a_divergence_v1.py:79-82` |
| 13 | 区间套级别=递归层级 | CLAUDE.md 级别口径 | `a_nested_divergence.py` |

### [旧缠论:选择]（6个）

| # | 选择 | 理由 | 代码位置 |
|---|------|------|---------|
| 5 | 三维度 OR 关系 | 原文用"或者"连接 | `a_divergence_v1.py:252-255` |
| 9 | A段=前中枢结束→后中枢开始 | 两中枢间连接段 | `a_divergence_v1.py:299-306` |
| 10 | C段=最后中枢结束→Move终点 | 最后离开段 | `a_divergence_v1.py:399-400` |
| 11 | 盘整离开段=超出[ZD,ZG] | 第33课确认 | `a_divergence_v1.py:424` |
| 12 | 每个Move最多一个背驰 | 趋势优先于盘整 | `a_divergence_v1.py:527` |
| 14 | Level2+力度=价格振幅 | MACD仅raw bar有意义 | `a_nested_divergence.py` |

## 边界条件

- 若 OR→AND（#5），检出率大幅下降
- T4 仅对趋势背驰检查，盘整背驰不检查

## 影响声明

- **创建**: `.chanlun/genealogy/settled/107-beichi-definition-choices.md`
- **不修改**任何定义文件或代码文件

## 谱系引用

- 004号溯源框架、005b号对象否定对象、106号中枢定义选择、068号范式转换
"""

FILE_108 = """---
id: '108'
title: "买卖点定义选择谱系——定义到实现的溯源追踪"
type: 语法记录
status: 已结算
date: 2026-02-22
depends_on: ['004', '106', '107']
related: ['005b', '068']
negation_source: "定义选择过程缺乏谱系追踪（暗选择）"
negation_form: "语法记录（将隐性选择写进谱系）"
negates: []
negated_by: []
provenance: "[旧缠论] + [旧缠论:选择] + [旧缠论:隐含]"
---

# 108 — 买卖点定义选择谱系

**类型**: 语法记录（已在运作但未显式化的定义选择）
**状态**: 已结算
**日期**: 2026-02-22
**域**: 买卖点（MaiMai / Buy-Sell Points）
**定义文件**: `.chanlun/definitions/maimai.md` v1.0
**代码文件**: `src/newchan/a_buysellpoint_v1.py`

---

## 核心结论

买卖点的定义和实现中包含 15 个关键选择点。8 个直接遵循原文 [旧缠论]，2 个为 [旧缠论:隐含]，5 个涉及选择 [旧缠论:选择]。

## 定义选择列表

### [旧缠论] 直接遵循（8个）

| # | 选择 | 原文依据 | 代码位置 |
|---|------|---------|---------|
| 1 | 第一类买点=下跌趋势背驰点 | 第17课+第24课 | `a_buysellpoint_v1.py:103-104` |
| 2 | 第一类卖点=上涨趋势背驰点 | 编纂版 | `a_buysellpoint_v1.py:112-113` |
| 3 | 1B仅限下跌确立后（>=2中枢） | 第21课L40-43 | 只接收 kind="trend" |
| 4 | 2B=1B后首个回调结束点 | 第17课/第21课 | `a_buysellpoint_v1.py:188-197` |
| 6 | 3B=离开后回试不跌破ZG | 第20课 | `a_buysellpoint_v1.py:261` |
| 7 | 3B使用[ZD,ZG]非[DD,GG] | 第20课明确用ZG/ZD | `a_buysellpoint_v1.py:261,263` |
| 9 | 第三类仅取第一次离开后回试 | 第20课 "必须是第一次" | `a_buysellpoint_v1.py:246-250` |
| 10 | 盘整背驰不产生买卖点 | 第21课 | `a_buysellpoint_v1.py:104` |

### [旧缠论:隐含]（2个）

| # | 选择 | 说明 | 代码位置 |
|---|------|------|---------|
| 8 | "不跌破"用严格大于(>) | 可能需审查：`>=`更符合字面 | `a_buysellpoint_v1.py:261` |
| 13 | 2B+3B重合检测 | 第21课V型反转 | `a_buysellpoint_v1.py:271-297` |

### [旧缠论:选择]（5个）

| # | 选择 | 理由 | 代码位置 |
|---|------|------|---------|
| 5 | 2B方向搜索逻辑 | 用段方向匹配代替走势计数 | `a_buysellpoint_v1.py:189-194` |
| 11 | confirmed=Move.settled | 统一确认语义 | `a_buysellpoint_v1.py:173` |
| 12 | confirmed不可逆 | frozen dataclass | `BuySellPoint` |
| 14 | 价格取段端点(buy=low,sell=high) | 工程惯例 | `a_buysellpoint_v1.py:121` |
| 15 | 纯函数全量计算 | 幂等性 | `buysellpoints_from_level()` |

## 跨谱系张力点

108号#8（"不跌破"的 `>` vs `>=`）与 106号#4（延伸弱不等式）形成一组相关选择。若遵循同样逻辑（"不跌破"="不低于"=`>=`），则 #8 应改为 `>=`。值得在下一轮源头审计中解决。

## 边界条件

- #8 "不跌破"严格性影响第三类买卖点检出率
- 若允许盘整背驰产生买卖点（否定#10），需重定义安全性

## 影响声明

- **创建**: `.chanlun/genealogy/settled/108-maimai-definition-choices.md`
- **不修改**任何定义文件或代码文件

## 谱系引用

- 004号溯源框架、005b号对象否定对象、106号中枢、107号背驰
"""


def main():
    # Step 1: Write files
    SETTLED.mkdir(parents=True, exist_ok=True)
    (SETTLED / "106-zhongshu-definition-choices.md").write_text(FILE_106.strip() + "\n", encoding="utf-8")
    (SETTLED / "107-beichi-definition-choices.md").write_text(FILE_107.strip() + "\n", encoding="utf-8")
    (SETTLED / "108-maimai-definition-choices.md").write_text(FILE_108.strip() + "\n", encoding="utf-8")
    print("3 genealogy files written.")

    # Step 2: Update dag.yaml
    with open(DAG_PATH, encoding="utf-8") as f:
        dag = yaml.safe_load(f)

    existing_ids = {str(n["id"]) for n in dag["nodes"]}

    new_nodes = [
        {"id": "106", "title": "中枢定义选择谱系——定义到实现的溯源追踪", "status": "已结算", "type": "语法记录", "file": "settled/106-zhongshu-definition-choices.md"},
        {"id": "107", "title": "背驰定义选择谱系——定义到实现的溯源追踪", "status": "已结算", "type": "语法记录", "file": "settled/107-beichi-definition-choices.md"},
        {"id": "108", "title": "买卖点定义选择谱系——定义到实现的溯源追踪", "status": "已结算", "type": "语法记录", "file": "settled/108-maimai-definition-choices.md"},
    ]

    for n in new_nodes:
        if n["id"] not in existing_ids:
            dag["nodes"].append(n)
            print(f"Added node {n['id']}")

    # Add edges
    edges = dag.setdefault("edges", {})
    deps = edges.setdefault("depends_on", [])
    rels = edges.setdefault("related", [])

    new_deps = [
        {"from": "106", "to": "004"},
        {"from": "106", "to": "001"},
        {"from": "106", "to": "002"},
        {"from": "107", "to": "004"},
        {"from": "107", "to": "106"},
        {"from": "108", "to": "004"},
        {"from": "108", "to": "106"},
        {"from": "108", "to": "107"},
    ]

    new_rels = [
        {"between": ["106", "006"]},
        {"between": ["106", "010"]},
        {"between": ["106", "068"]},
        {"between": ["107", "005b"]},
        {"between": ["107", "068"]},
        {"between": ["108", "005b"]},
        {"between": ["108", "068"]},
    ]

    existing_deps = {(str(e.get("from", "")), str(e.get("to", ""))) for e in deps}
    for d in new_deps:
        if (str(d["from"]), str(d["to"])) not in existing_deps:
            deps.append(d)

    existing_rels = set()
    for r in rels:
        if "between" in r:
            existing_rels.add(tuple(sorted(str(x) for x in r["between"])))
    for r in new_rels:
        key = tuple(sorted(str(x) for x in r["between"]))
        if key not in existing_rels:
            rels.append(r)

    with open(DAG_PATH, "w", encoding="utf-8") as f:
        yaml.dump(dag, f, allow_unicode=True, default_flow_style=False, sort_keys=False, width=200)

    print("dag.yaml updated.")

    # Step 3: Validate
    md_files = set()
    for subdir in ("settled", "pending"):
        d = ROOT / ".chanlun" / "genealogy" / subdir
        if d.exists():
            for ff in d.glob("*.md"):
                md_files.add(f"{subdir}/{ff.name}")

    print(f"Final: DAG nodes={len(dag['nodes'])}, Files={len(md_files)}")
    assert len(dag["nodes"]) == len(md_files), f"MISMATCH: {len(dag['nodes'])} != {len(md_files)}"
    print("BALANCED - validation passed!")


if __name__ == "__main__":
    main()
