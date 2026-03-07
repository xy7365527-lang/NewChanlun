# 多模态解析器架构设计文档

**状态**：设计态（尚未实现）
**作者**：Gemini 异质质询代理（030a号谱系位置）
**日期**：2026-03-07
**目标系统**：topological-computation / parsers/

---

## 0. 前提：引擎是什么

在写任何规格之前，必须先确认引擎的实际操作单元。

引擎（`engine.py`）操作的是：
- `Vertex`：带 `id`、`status`、`content`、`created_at` 的不可变节点
- `Edge`：带 `source`、`target`、`edge_type`（DEPENDENCY/NEGATION/SUBLATION/REFERENCE/FOLD）、`created_at` 的有向边
- `Graph`：持有 K_full（所有顶点+边）和 K_active（非 FOLDED 的子集）
- 操作：`fold`、`negate`、`sublate`
- 指标：`beta_1`（第一 Betti 数，衡量独立环路数），Morse 地形的 `f` 值（共同邻居数）

这意味着：**所有解析器的最终产出必须是 Vertex 列表 + Edge 列表**。持久同调特征不是目的，是中间产物——最终要映射回 Vertex+Edge。

---

## 1. `ph_injector.py` 精确映射规则

### 1.1 输入：持久同调特征的数学形式

持久同调给出一组 **birth-death 对**（持久对），称为 persistence diagram：

```
PH_k = {(b_i, d_i) | b_i < d_i, i = 1..n}
```

其中 k 是同调维度：
- k=0：连通分量的生灭（数据聚类结构）
- k=1：独立环路的生灭（数据中的"洞"）
- k=2：空腔的生灭（3D 数据专用）

每个特征的 **lifetime = d_i - b_i**（持久性/显著性）

### 1.2 顶点 ID 生成规则

```
vertex_id = "{source_label}:ph{k}_{i}_b{birth_int}_d{death_int}"
```

其中：
- `source_label`：来自 `ParserResult.source_label`（如 `"paper_2024_attention_fig3"`）
- `k`：同调维度（0/1/2）
- `i`：在同维度特征中的序号（按 lifetime 降序排列后的序号）
- `birth_int`：birth 值乘以 1000 取整（避免浮点数在 ID 中的不稳定性）
- `death_int`：death 值乘以 1000 取整

示例：
```
"deepseek_v3_fig2:ph1_0_b124_d891"
```
含义：DeepSeek V3 论文图2中第一个最显著的 k=1 环路特征，birth=0.124，death=0.891。

### 1.3 顶点 content 生成规则

content 字段是引擎用于语义比较和 LLM encounter 的字符串。对于持久同调顶点：

```
content = "ph{k} feature: lifetime={lifetime:.3f} birth={birth:.3f} death={death:.3f} | source={source_label} | rank={rank_in_dimension}"
```

示例：
```
"ph1 feature: lifetime=0.767 birth=0.124 death=0.891 | source=deepseek_v3_fig2 | rank=0"
```

**为什么这样编码**：content 不是给人看的，是给引擎的 f 值计算（共同邻居）和 LLM encounter（语义理解）用的。LLM 能理解 `lifetime=0.767` 代表显著的拓扑特征，并能与文本中的"prominent cycle"等概念建立连接。

### 1.4 同一数据源内特征间的边类型

同一个数据源的多个持久同调特征之间，添加 **DEPENDENCY** 边，方向从低 rank 到高 rank（显著性降序）：

```
ph1_0（最显著）<-- ph1_1 <-- ph1_2 ...
```

语义：较不显著的特征"依赖"更显著的特征存在（拓扑上，高 lifetime 特征构成骨架）。

跨维度边（k=0 特征与 k=1 特征之间）：如果存在几何上的关联（如环路包含于连通分量内），添加 REFERENCE 边。实现方式：若 ph0 特征的 birth-death 区间与 ph1 特征的 birth-death 区间有交叉，添加 REFERENCE 边。

```python
def _intervals_overlap(b1, d1, b2, d2) -> bool:
    return b1 < d2 and b2 < d1
```

### 1.5 lifetime 作为顶点属性对引擎的影响

当前 `Vertex` 结构没有 `weight` 字段。lifetime 通过以下方式间接影响引擎：

**gap 检测**（`traversal.py` 中的 f 值计算）：f 值 = 共同邻居数。lifetime 高的顶点有更多邻居（因为显著特征倾向于连接更多其他特征），所以 f 值自然更高，引擎在穿越中会优先选择它们。

**sublation 触发**：当两个持久同调顶点都被 negate 时，如果它们有 DEPENDENCY 边，引擎可能触发 sublate 操作，产生新的合成顶点。这在拓扑上对应：两个独立检测到的特征被认识为同一个结构的两个面。

**重要限制**：不建议把 lifetime 编码为 `created_at` 字段（该字段是时间戳语义，不是权重）。lifetime 只存在于 content 字符串中。这是当前 `Vertex` 数据类的约束，未来可考虑添加 `metadata: dict` 字段扩展，但那是对引擎的修改，不是解析器的职责。

---

## 2. 跨模态连接机制

### 2.1 四个方案的分析

**方案 A：共同出现（同一文档内）**

实现最简单：同一篇论文的文本顶点和图像拓扑特征顶点之间，统一加 REFERENCE 边。

问题：连接密度过高。一篇 40 页的论文可能有 500 个文本顶点和 20 个图像特征，产生 10000 条 REFERENCE 边。这会使 f 值（共同邻居数）对所有顶点都极高，消除了地形的信息梯度。引擎的穿越将退化为随机游走。

**方案 B：token 相似度（cross_domain.py 现有机制）**

图像持久同调顶点的 content 是数值描述，Jaccard 相似度对 `"ph1 feature: lifetime=0.767"` 这样的字符串几乎无效——不同图像的特征描述在词汇层面几乎相同，只有数值不同。

结论：现有 cross_domain.py 对图像特征无效，需要专门的相似度函数。

**方案 C：显式标注（Figure 引用解析）**

论文结构中，`citation_parser.py` 提取文献引用，同样的逻辑可以提取图注引用（"see Figure 3"、"as shown in Fig. 2b"）。

这是**最精确的连接**：文本概念 → 图像的连接有明确的语义来源（作者明确声明了关联）。

**方案 D：拓扑积累（引擎自主发现）**

不显式连接，让引擎在穿越中发现跨模态对。这依赖 f 值计算能跨模态工作——但 f 值 = 共同邻居数，如果文本顶点和图像顶点之间没有任何连接，f 值永远为 -1，引擎永远不会穿越过去。

结论：方案 D 单独使用时等价于"不做跨模态连接"。

### 2.2 推荐方案：C + 数值相似度补充

**主要机制（方案 C 变体）**：

在 `router.py` 的文档级处理中，解析 Figure 引用：

```python
FIGURE_REF_PATTERN = re.compile(
    r'\b[Ff]ig(?:ure|\.?)\.?\s*(\d+[a-z]?)\b'
)
```

将文本中引用 "Figure 3" 的段落内的顶点与 `figure_3` 解析结果的顶点之间添加 REFERENCE 边。

连接精度：高（显式语义）；连接召回：中（只有明确引用的图会被连接）。

**补充机制（持久同调相似度）**：

对于没有显式引用的图像特征，引入跨模态 PH 相似度：Wasserstein 距离（或更廉价的 bottleneck 距离）衡量两个 persistence diagram 的相似性。

阈值：如果两个图像的 k=1 持久图的 bottleneck 距离 < θ（默认 0.1），添加 REFERENCE 边。

实现位置：`ph_injector.py` 中的 `inject_cross_modal_ph_edges()` 函数。

**不建议使用方案 A**（共现）和方案 B（token Jaccard）用于图像模态。

### 2.3 跨模态连接总结

```
文本顶点 -- [REFERENCE] --> 图像PH顶点    (Figure引用，方案C)
图像PH顶点 -- [REFERENCE] --> 图像PH顶点  (PH相似度，补充机制)
代码顶点 -- [REFERENCE] --> 文本顶点      (cross_domain.py现有机制)
```

这三类边覆盖了"同一论文内的文本-图、不同论文的图-图、代码-文本"三种跨模态关系。

---

## 3. 统一解析器接口

### 3.1 数据类定义

```python
# parsers/base.py

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Optional

from engine import Vertex, Edge


@dataclass(frozen=True, slots=True)
class PHFeature:
    """单个持久同调特征（birth-death 对）。"""
    dimension: int          # 同调维度 k (0, 1, 2)
    birth: float
    death: float

    @property
    def lifetime(self) -> float:
        return self.death - self.birth


@dataclass
class ParserResult:
    """所有解析器的统一输出格式。

    vertices + edges 直接可插入 Graph。
    ph_features 供 ph_injector.py 做二次映射（可选）。
    """
    vertices: list[Vertex]
    edges: list[Edge]
    ph_features: list[PHFeature] = field(default_factory=list)
    source_modality: str = "unknown"   # "text"|"code"|"image"|"audio"|"video"|"3d"|"formula"|"table"|"diagram"|"chart"
    source_label: str = ""             # 来源标识，如 "deepseek_v3_paper" 或 "paper_fig3"
    parse_warnings: list[str] = field(default_factory=list)  # 非致命警告
```

### 3.2 解析函数签名约定

每个解析器模块必须导出以下函数：

```python
def parse(input_data: <模态特定类型>, source_label: str) -> ParserResult:
    """将输入数据解析为 K_active 顶点+边。

    Args:
        input_data: 模态特定的输入（见各解析器文档）
        source_label: 来源标识符，用于 vertex ID 前缀

    Returns:
        ParserResult，vertices+edges 可直接插入 Graph

    Raises:
        ValueError: 输入数据格式不合法
        ImportError: 缺少可选依赖（见各解析器）
    """
```

**不允许**解析器直接修改传入的 `Graph` 对象。解析器只产生 `ParserResult`，由调用方决定如何将其合并到 `Graph`。

### 3.3 各模态的 input_data 类型

| 解析器 | input_data 类型 | 可选依赖 |
|--------|----------------|---------|
| formula_parser | `str`（LaTeX 字符串）| sympy |
| citation_parser | `str`（论文文本）| 无（正则）|
| table_parser | `str`（CSV/Markdown 表格）| 无（正则）|
| diagram_parser | `bytes`（PNG/JPG 字节流）| opencv-python |
| chart_parser | `bytes`（PNG/JPG 字节流）| opencv-python, scikit-learn |
| image_parser | `bytes`（任意图像字节流）| ripser, scikit-tda |
| audio_parser | `bytes`（WAV/MP3 字节流）| librosa, ripser |
| video_parser | `bytes`（视频字节流）OR `list[bytes]`（帧序列）| opencv-python, ripser |
| mesh_parser | `bytes`（OBJ/PLY 字节流）| trimesh, ripser |
| router | `bytes` OR `str`（任意输入）| 所有上述 |

### 3.4 可选依赖的处理规范

每个依赖可选依赖的解析器必须在模块顶部做延迟导入：

```python
def parse(input_data: bytes, source_label: str) -> ParserResult:
    try:
        import ripser
        import numpy as np
    except ImportError as e:
        raise ImportError(
            f"image_parser requires ripser and numpy. "
            f"Install with: pip install ripser numpy. Original error: {e}"
        ) from e
    # ... 实现
```

这保证没有安装可选依赖的用户仍然可以 `import parsers.router` 而不报错——只有实际调用 `parse()` 时才会触发 ImportError。

### 3.5 router.py 的分发逻辑

```python
def parse(input_data: bytes | str, source_label: str) -> ParserResult:
    """内容类型检测 → 分发到具体解析器。"""
    if isinstance(input_data, str):
        # 文本类：检测是否含 LaTeX 公式、表格标记等
        if _looks_like_latex(input_data):
            return formula_parser.parse(input_data, source_label)
        if _looks_like_table(input_data):
            return table_parser.parse(input_data, source_label)
        # 默认：作为文本（phi_L 路径）
        return _parse_text(input_data, source_label)
    else:
        # 二进制类：检测 magic bytes
        mime = _detect_mime(input_data)
        if mime.startswith("image/"):
            # 进一步区分：diagram vs chart vs 普通图像
            return _route_image(input_data, source_label)
        if mime.startswith("audio/"):
            return audio_parser.parse(input_data, source_label)
        if mime.startswith("video/"):
            return video_parser.parse(input_data, source_label)
        if _looks_like_mesh(input_data):
            return mesh_parser.parse(input_data, source_label)
        raise ValueError(f"Unsupported content type: {mime}")
```

### 3.6 未来模态添加的步骤

添加一个新模态只需：

1. 创建 `parsers/{modality}_parser.py`
2. 实现 `parse(input_data, source_label) -> ParserResult`
3. 在 `parsers/base.py` 的 `source_modality` 文档注释中添加新值
4. 在 `router.py` 的 MIME/格式检测中添加一个 `elif` 分支

不需要修改 `engine.py`、`traversal.py`、`persistence.py` 中的任何代码。

---

## 4. 各解析器的具体映射规则

### 4.1 audio_parser（提纲）

**输入** → WAV/MP3 字节流

**处理流程**：
1. 用 librosa 提取音频特征矩阵（MFCC + chromagram，维度 = 时间帧 × 特征数）
2. 将特征矩阵作为点云，对每个时间窗口（如 1 秒）计算局部 Rips 复形
3. 用 ripser 计算持久同调 PH_0（音色聚类）和 PH_1（旋律环路）
4. `ph_injector.py` 将 birth-death 对映射为顶点

**语义解读**：
- PH_0 特征：音色/音段聚类（如"静音段"、"打击乐段"、"旋律段"）
- PH_1 特征：重复旋律模式（环路 = 旋律返回）

**顶点 ID 格式**：`"{source_label}:audio_ph{k}_{i}_t{start_ms}"`

### 4.2 video_parser（提纲）

**输入** → 视频字节流或帧序列

**处理流程**：
1. 以固定帧率（如 1fps）采样关键帧
2. 每帧作为静态图像走 `image_parser` 路径
3. 帧间持久同调变化：相邻帧 PH 特征之间添加 DEPENDENCY 边
4. 场景切换检测（PH 特征突变）：添加 NEGATION 边标记场景边界

**特殊边类型**：
- 同一场景内相邻帧顶点：DEPENDENCY（时序依赖）
- 场景切换：NEGATION（语义不连续）
- 重复出现的相似帧：SUBLATION（场景回归）

### 4.3 mesh_parser（提纲）

**输入** → OBJ/PLY 格式的 3D 网格字节流

**处理流程**：
1. 用 trimesh 解析顶点坐标和面片
2. 计算网格上的持久同调（PH_0：连通分量，PH_1：把手/洞，PH_2：空腔）
3. 几何特征顶点：连通分量、把手数（拓扑亏格）、空腔

**关键语义**：PH_2 空腔在 3D 结构中有直接语义（如蛋白质结合口袋、建筑内部空间）。

---

## 5. ph_injector.py 的完整接口

```python
# parsers/ph_injector.py

from __future__ import annotations
from engine import Vertex, Edge, EdgeType, VertexStatus
from parsers.base import ParserResult, PHFeature


def inject(result: ParserResult, threshold: float = 0.01) -> ParserResult:
    """将 ph_features 映射为 vertices + edges，追加到 result 中。

    Args:
        result: 已有顶点+边的 ParserResult（来自解析器）
        threshold: lifetime 低于此值的特征被过滤（噪声去除）

    Returns:
        新的 ParserResult，追加了 PH 顶点和 PH 边
    """


def inject_cross_modal_ph_edges(
    results: list[ParserResult],
    bottleneck_threshold: float = 0.1,
) -> list[Edge]:
    """在多个 ParserResult 的 PH 特征之间检测相似性并生成跨模态边。

    仅对 ph_features 非空且 source_modality in ("image", "audio", "video", "3d") 的结果操作。

    Args:
        results: 多个来源的 ParserResult
        bottleneck_threshold: bottleneck 距离阈值，低于此值添加 REFERENCE 边

    Returns:
        跨模态 REFERENCE 边列表（不修改输入的 ParserResult）
    """


def _ph_vertex_id(feature: PHFeature, source_label: str, rank: int) -> str:
    """生成确定性的顶点 ID。"""
    return (
        f"{source_label}:ph{feature.dimension}_{rank}"
        f"_b{int(feature.birth * 1000)}"
        f"_d{int(feature.death * 1000)}"
    )


def _ph_vertex_content(feature: PHFeature, source_label: str, rank: int) -> str:
    """生成供引擎和 LLM 使用的 content 字符串。"""
    return (
        f"ph{feature.dimension} feature: "
        f"lifetime={feature.lifetime:.3f} "
        f"birth={feature.birth:.3f} "
        f"death={feature.death:.3f} | "
        f"source={source_label} | "
        f"rank={rank}"
    )
```

---

## 6. 诚实的限制分析

### 6.1 纯拓扑图像理解 vs CNN/ViT 的差距

**拓扑方式擅长的**：
- 全局结构：图像中"有几个环状结构"（PH_1）、"有几个孤立团"（PH_0）
- 跨尺度不变性：持久同调对旋转、尺度变化、噪声具有稳定性
- 定性区分：两个图像的 PH 特征差距大 = 结构根本不同

**拓扑方式无法做到的**：
- 语义识别：无法区分"猫"和"狗"——两者的 PH 特征可能相近
- 纹理理解：PH 只看拓扑形状，不看像素颜色或纹理
- 精确位置：持久同调是全局指标，不保留局部位置信息
- 细粒度分类：2000 个物体类别的分类任务，拓扑方式几乎无效

**量化差距**：在 ImageNet 分类任务上，ViT-Large 达到 88% top-1 准确率；纯拓扑方式在同任务上预计 < 5%（接近随机），因为 1000 类物体之间的 PH 特征几乎无法区分。

### 6.2 系统的多模态"理解"和 VLM 的区别

这个系统**不做语义理解**。它做的是**结构关系提取**。

| 维度 | 本系统 | VLM（如 GPT-4V）|
|------|--------|----------------|
| 语义理解 | 无——不知道"这是一只猫" | 有——识别物体、场景、关系 |
| 结构关系 | 强——拓扑连接关系明确 | 弱——隐含在 attention 中，不可解释 |
| 跨文档推理 | 强——K_active 持久，跨文档图可直接遍历 | 弱——每次推理独立，无持久知识图 |
| 可解释性 | 完全——每条边都有明确来源 | 低——黑箱 |
| 适合任务 | 结构相似性检测、论文架构分析、代码-论文对应 | 图像问答、视觉推理、多模态生成 |

核心区别：VLM 把图像翻译成语言，然后在语言空间推理。本系统把图像翻译成拓扑，然后在同调空间推理。两者处理的是不同层次的信息——不是竞争，是互补。

### 6.3 什么任务适合拓扑方式

**适合**：
- 论文架构分析：检测两篇论文的神经网络架构是否在拓扑上同构
- 代码结构比较：函数调用图的拓扑等价性
- 信号异常检测：时间序列的 PH_1 特征突变 = 系统状态变化
- 分子结构比较：药物分子的拓扑特征相似性
- 科学图像中的结构识别：MRI 图像中的脑连接环路计数

**不适合**：
- 物体识别（猫/狗）
- 人脸识别
- OCR（文字识别）
- 任何需要像素级精度的任务
- 情感/风格理解（"这张图很压抑"）

### 6.4 对当前实现的诚实评估

`diagram_parser.py` 的 OpenCV 边缘检测 + 连通分量方法对**结构清晰的流程图/架构图**有效，对**自然图像**几乎无效。

`chart_parser.py` 的 DBSCAN + Delaunay 三角剖分对**散点图、折线图**有效，对**饼图、热力图**等需要颜色语义的图表失效。

`image_parser.py` 的像素 filtration 方法（将像素灰度值作为 filtration 函数）是当前最粗糙的实现——它把颜色梯度当作拓扑结构。更好的方法是先做边缘检测，再在边缘图上计算 Rips 复形，但这增加了 OpenCV 依赖。

---

## 7. 实现优先级建议

基于系统当前的主要用途（论文阅读 + 代码分析），建议按以下顺序实现：

1. **formula_parser**（最高优先级）：论文中数学公式的变量依赖关系，直接丰富现有文本图
2. **citation_parser**（高优先级）：论文引用网络，与现有文本顶点连接
3. **diagram_parser**（中优先级）：架构图解析，论文图-文本连接
4. **ph_injector**（与 image_parser 配套）：完成上述设计中的映射规则
5. **table_parser**（中优先级）：表格中实体-属性关系
6. **chart_parser**（低优先级）：数值图表拓扑，仅在量化分析时有用
7. **image_parser**（低优先级）：通用图像，语义最弱
8. **audio/video/mesh**（最低优先级）：当前使用场景中不出现

---

## 8. 边界条件声明（结果包六要素之边界条件）

以下条件下本设计文档的建议会失效或需要修订：

1. **引擎添加 `weight` 字段**：如果 `Vertex` 获得权重字段，lifetime 编码方式应从 content 字符串改为 weight 值，本文 1.5 节的 f 值影响分析相应改变。

2. **引擎添加高阶单纯形**（三角形、四面体）：当前设计只产生顶点和边（0-单形和 1-单形）。如果引擎扩展到三角形填充，PH_2 特征应映射为三角形而非顶点。

3. **cross_domain.py 实现语义向量**：如果 cross_domain.py 从 token Jaccard 升级到向量相似度，跨模态连接的补充机制（2.2 节）可以统一化，不再需要单独的 PH bottleneck 距离。

4. **单次论文含 > 1000 个 PH 特征**：当前 vertex ID 的序号 `i` 没有上限，但 K_active 超过 5000 个顶点时 beta_1 计算的时间复杂度（O(V^2.37)）会成为瓶颈。需要在 ph_injector 中添加 lifetime 阈值过滤（已在接口设计中预留 `threshold` 参数）。

---

**文档版本**：v1.0-设计态
**下一步**：CC 实现各解析器后，本文档升级为执行态（增加"实际行为与设计的差异"节）
