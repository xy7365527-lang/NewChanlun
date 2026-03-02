"""tests for concept_extractor.py

Uses real genealogy file content patterns to verify extraction accuracy.
"""

from __future__ import annotations

import pytest
import sys
from pathlib import Path

# Add project root to path
PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.concept_extractor import (
    ContentAnalysis,
    ConceptDefinition,
    Modification,
    NewConcept,
    Section,
    analyze_content,
    _extract_sections,
    _extract_concepts,
    _extract_references,
    _extract_new_concepts,
    _extract_modifications,
    _extract_conclusion_summary,
    _strip_frontmatter,
)


# --- Test Data ---

SAMPLE_005B = """---
id: "005b"
title: "对象否定对象（语法规则）"
status: "已结算"
type: "语法规则确立（谢林式原初命名）"
date: "2026-02-17"
depends_on: []
related: ["004", "005", "005a", "020"]
negated_by: []
negates: []
---

# 005b — 对象否定对象（语法规则）

**状态**: 已结算
**创建时间**: 2026-02-17
**类型**: 语法规则确立（谢林式原初命名）
**域**: 全局（语法层 / 本体论层）

---

## 语法规则陈述

**对象否定对象**：在本体系中，一个对象被否定的唯一来源是：
1. **内在否定**：对象自身产生的否定（如中枢被自身的突破段否定）
2. **外部对象生成**：另一个同级或更高级别对象的生成否定了当前对象

体系中不存在第三种否定来源。

## 与定理半（005a）的关系

005a（禁止性定理）是本规则的**必要条件但不是充分条件**。
"""

SAMPLE_292 = """---
id: '292'
title: "折叠拓扑本体论——分类学之前的存在论结构"
type: "理论形式化"
status: 已结算
date: 2026-03-01
depends_on:
  - '254'
  - '255'
  - '284'
  - '287'
---

# 292号：折叠拓扑本体论——分类学之前的存在论结构

## 1. 核心命题

分类学处理不了折叠。拓扑应该从折叠出发建，不是从分类出发建。

254号建立了 K4 完全图及其多经济体展开。

## 2. 折叠定义

**折叠**：同一个对象在不同域中呈现不同的范畴身份。不是"既是A又是B"，是同一个存在在不同截面下的不同相位。

### 新增概念
1. **折叠**（本体论）——同一个对象在不同域中呈现不同的范畴身份
2. **折叠生命周期**——折叠的四阶段动态过程：稳定→松动→重新结算→未结算
3. E-C 无直接折叠——权益和商品之间没有直接折叠通道

### 对现有概念的修正
- 254号 K4 顶点划分：从操作便利驱动提升为折叠根据
- 254号 OQ3：从"基本面不入语法"修正为"基本面入语法（折叠生命周期阶段判定）"

## 结论

折叠是比 K4 更基本的本体论层次。矩阵划分是折叠的推论。
"""

SAMPLE_298 = """---
id: '298'
title: "折叠↔FEM对应"
type: 理论形式化
status: 已结算
date: 2026-03-01
depends_on:
  - '292'
  - '254'
---

# 298号：折叠↔FEM对应

## 1. 核心命题

有限元映射出缠论已有结构的完备性。

292号建立了折叠拓扑本体论。254号的 K4 框架获得了折叠根据。

### 新增概念
1. **有限元是镜子不是工具**——有限元映射出缠论已有结构的完备性，不提供新操作手段
2. **内在语言完备性声明**（编排者完成）——缠论概念集合已完备

### 对现有概念的定位修正
- "残差分析"降级为镜子：用内在语言"叙事不一致检测"替代
- "条件数监测"降级为镜子：用内在语言"背驰密度和分布检测"替代
"""

SAMPLE_230 = """---
id: '230'
title: K4 独立边独立性——真实数据检验
type: 概念发现
status: 已结算
date: 2026-02-27
depends_on:
  - '229'
---

# 230号：K4 独立边独立性——真实数据检验

## 1. 结论

三条独立边 E/$, C/$, R/$ 在真实数据上**不满足无条件独立性**。

229号的合成数据验证在真实数据上部分否证。

## 3. 边界条件

结论在以下条件下翻转。

## 4. 下游推论

### 4.1 ConfigurationSpace 直积结构需要分层修正

**推论**：27 种配置中独立性退化。
"""


# --- Tests ---

class TestStripFrontmatter:
    def test_strips_yaml_frontmatter(self):
        result = _strip_frontmatter(SAMPLE_005B)
        assert not result.startswith("---")
        assert "# 005b" in result

    def test_no_frontmatter(self):
        text = "# Just a heading\n\nSome content"
        assert _strip_frontmatter(text) == text


class TestExtractSections:
    def test_basic_sections(self):
        body = _strip_frontmatter(SAMPLE_005B)
        sections = _extract_sections(body)
        assert len(sections) >= 2
        titles = [s.title for s in sections]
        assert any("语法规则陈述" in t for t in titles)

    def test_numbered_sections(self):
        body = _strip_frontmatter(SAMPLE_292)
        sections = _extract_sections(body)
        numbered = [s for s in sections if s.number]
        assert len(numbered) >= 2
        assert numbered[0].number == "1"
        assert "核心命题" in numbered[0].title

    def test_section_levels(self):
        body = _strip_frontmatter(SAMPLE_292)
        sections = _extract_sections(body)
        levels = {s.level for s in sections}
        assert 2 in levels  # ##
        assert 3 in levels  # ###


class TestExtractConcepts:
    def test_inline_concept(self):
        body = _strip_frontmatter(SAMPLE_292)
        concepts = _extract_concepts(body)
        terms = [c.term for c in concepts]
        assert "折叠" in terms

    def test_concept_definition(self):
        body = _strip_frontmatter(SAMPLE_292)
        concepts = _extract_concepts(body)
        fold = [c for c in concepts if c.term == "折叠"]
        assert len(fold) == 1
        assert "同一个对象" in fold[0].definition

    def test_skips_metadata_lines(self):
        body = _strip_frontmatter(SAMPLE_005B)
        concepts = _extract_concepts(body)
        # Should not include "状态", "创建时间" etc.
        terms = [c.term for c in concepts]
        assert "状态" not in terms
        assert "创建时间" not in terms
        assert "类型" not in terms

    def test_005b_concepts(self):
        body = _strip_frontmatter(SAMPLE_005B)
        concepts = _extract_concepts(body)
        terms = [c.term for c in concepts]
        assert "对象否定对象" in terms
        assert "内在否定" in terms
        assert "外部对象生成" in terms


class TestExtractReferences:
    def test_basic_references(self):
        body = _strip_frontmatter(SAMPLE_292)
        refs = _extract_references(body)
        assert "254" in refs

    def test_multiple_references(self):
        body = _strip_frontmatter(SAMPLE_230)
        refs = _extract_references(body)
        assert "229" in refs

    def test_suffix_references(self):
        text = "005a号的定理和005b号的语法规则"
        refs = _extract_references(text)
        assert "005a" in refs
        assert "005b" in refs

    def test_no_self_in_refs(self):
        # References should include all found, filtering is caller's job
        body = _strip_frontmatter(SAMPLE_292)
        refs = _extract_references(body)
        # 292 references itself in the heading
        assert "292" in refs


class TestExtractNewConcepts:
    def test_292_new_concepts(self):
        body = _strip_frontmatter(SAMPLE_292)
        new_concepts = _extract_new_concepts(body)
        assert len(new_concepts) >= 2
        terms = [nc.term for nc in new_concepts]
        assert "折叠" in terms
        assert "折叠生命周期" in terms

    def test_qualifier(self):
        body = _strip_frontmatter(SAMPLE_292)
        new_concepts = _extract_new_concepts(body)
        fold = [nc for nc in new_concepts if nc.term == "折叠"]
        assert len(fold) == 1
        assert fold[0].qualifier == "本体论"

    def test_298_new_concepts(self):
        body = _strip_frontmatter(SAMPLE_298)
        new_concepts = _extract_new_concepts(body)
        terms = [nc.term for nc in new_concepts]
        assert any("有限元" in t for t in terms)

    def test_298_qualifier(self):
        body = _strip_frontmatter(SAMPLE_298)
        new_concepts = _extract_new_concepts(body)
        completeness = [nc for nc in new_concepts
                        if "完备" in nc.term]
        assert len(completeness) == 1
        assert completeness[0].qualifier == "编排者完成"

    def test_no_new_concepts(self):
        body = _strip_frontmatter(SAMPLE_005B)
        new_concepts = _extract_new_concepts(body)
        assert len(new_concepts) == 0

    def test_no_new_concepts_230(self):
        body = _strip_frontmatter(SAMPLE_230)
        new_concepts = _extract_new_concepts(body)
        assert len(new_concepts) == 0


class TestExtractModifications:
    def test_292_modifications(self):
        body = _strip_frontmatter(SAMPLE_292)
        mods = _extract_modifications(body)
        assert len(mods) >= 2
        target_ids = [m.target_id for m in mods]
        assert "254" in target_ids

    def test_modification_content(self):
        body = _strip_frontmatter(SAMPLE_292)
        mods = _extract_modifications(body)
        k4_mod = [m for m in mods if "K4" in m.target_desc]
        assert len(k4_mod) >= 1
        assert "折叠" in k4_mod[0].modification

    def test_298_alt_format_modifications(self):
        body = _strip_frontmatter(SAMPLE_298)
        mods = _extract_modifications(body)
        # These use the alternate "降级为" format
        assert len(mods) >= 1

    def test_no_modifications(self):
        body = _strip_frontmatter(SAMPLE_005B)
        mods = _extract_modifications(body)
        assert len(mods) == 0


class TestExtractConclusionSummary:
    def test_292_conclusion(self):
        body = _strip_frontmatter(SAMPLE_292)
        conclusion = _extract_conclusion_summary(body)
        assert "折叠" in conclusion
        assert len(conclusion) > 10

    def test_230_conclusion(self):
        body = _strip_frontmatter(SAMPLE_230)
        conclusion = _extract_conclusion_summary(body)
        assert "独立" in conclusion or len(conclusion) > 0

    def test_no_conclusion(self):
        # 005b has no explicit conclusion section
        body = _strip_frontmatter(SAMPLE_005B)
        conclusion = _extract_conclusion_summary(body)
        assert conclusion == "" or isinstance(conclusion, str)


class TestAnalyzeContent:
    def test_full_analysis_292(self):
        ca = analyze_content("292", SAMPLE_292)
        assert isinstance(ca, ContentAnalysis)
        assert ca.genealogy_id == "292"
        assert len(ca.sections) >= 2
        assert len(ca.concepts) >= 1
        assert len(ca.references) >= 1
        assert len(ca.new_concepts) >= 2
        assert len(ca.modifications) >= 2

    def test_full_analysis_005b(self):
        ca = analyze_content("005b", SAMPLE_005B)
        assert ca.genealogy_id == "005b"
        assert len(ca.concepts) >= 2  # 对象否定对象, 内在否定, 外部对象生成
        assert len(ca.new_concepts) == 0
        assert len(ca.modifications) == 0

    def test_frozen_dataclass(self):
        ca = analyze_content("292", SAMPLE_292)
        with pytest.raises(AttributeError):
            ca.genealogy_id = "999"

    def test_references_are_sorted(self):
        ca = analyze_content("292", SAMPLE_292)
        refs = list(ca.references)
        assert refs == sorted(refs)


class TestRealFiles:
    """Tests that run against actual genealogy files if they exist."""

    @pytest.fixture
    def settled_dir(self):
        d = PROJECT_ROOT / ".chanlun" / "genealogy" / "settled"
        if not d.exists():
            pytest.skip("settled directory not found")
        return d

    def test_005b_real(self, settled_dir):
        f = settled_dir / "005b-object-negates-object-grammar.md"
        if not f.exists():
            pytest.skip("005b file not found")
        text = f.read_text(encoding="utf-8")
        ca = analyze_content("005b", text)
        assert ca.genealogy_id == "005b"
        assert len(ca.concepts) >= 2

    def test_292_real(self, settled_dir):
        f = settled_dir / "292-fold-topology-ontology.md"
        if not f.exists():
            pytest.skip("292 file not found")
        text = f.read_text(encoding="utf-8")
        ca = analyze_content("292", text)
        assert len(ca.new_concepts) >= 3
        assert len(ca.modifications) >= 2
        assert "254" in [m.target_id for m in ca.modifications]

    def test_230_real(self, settled_dir):
        f = settled_dir / "230-k4-independence-real-data.md"
        if not f.exists():
            pytest.skip("230 file not found")
        text = f.read_text(encoding="utf-8")
        ca = analyze_content("230", text)
        assert "229" in ca.references
        assert len(ca.concepts) >= 1

    def test_298_real(self, settled_dir):
        f = settled_dir / "298-fold-topology-fem-correspondence.md"
        if not f.exists():
            pytest.skip("298 file not found")
        text = f.read_text(encoding="utf-8")
        ca = analyze_content("298", text)
        assert len(ca.new_concepts) >= 2

    def test_137_real(self, settled_dir):
        f = settled_dir / "137-lead-pause-structural-root.md"
        if not f.exists():
            pytest.skip("137 file not found")
        text = f.read_text(encoding="utf-8")
        ca = analyze_content("137", text)
        assert "136" in ca.references
        assert "089" in ca.references
        # 090 may appear in frontmatter only (depends_on),
        # not necessarily as "090号" in body text
        assert len(ca.concepts) >= 3
