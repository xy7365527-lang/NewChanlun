"""Call Gemini 2.5 Pro with thinking mode for 254 ontology challenge."""
import json
import os
import urllib.request
import urllib.error

API_KEY = os.environ.get("GOOGLE_API_KEY", "")
if not API_KEY:
    raise RuntimeError("GOOGLE_API_KEY not set")

PROMPT = r"""你是一个异质质疑者（heterogeneous challenger）。你的任务是对下面给出的 254 号谱系进行严格的否定性审计。

## 你的角色

- 只制造裂缝，不给答案
- 每个否定必须包含：定义回溯（引用具体定义）、反例构造、推论检验
- 不允许"建议改进"——只允许"证明断裂"
- 如果找不到真正的裂缝，说"未发现可成立的否定"，不要强行制造

## 被审计目标：254号谱系全文

---START 254---
# 254号：多经济体资本流转本体论

## 1. 结论

将 K4 完全图从单一经济体（美元计价）展开为多经济体资本流转结构。核心操作：$ 顶点展开为结算尺空间 Σ，拉动 K4 整体共展开，产生三层递归结构。

### 公理 0（历史前提）

结算尺空间 Σ 是历史给定的经验前提，不是逻辑必然。
当前实例：Σ = {法定货币群（USD, CNY, EUR, JPY...）∪ 黄金}
公理 0 有有效域，有效域边界 = 当前结算尺结晶开始松动处。认识论等级：L2（当前历史阶段经验确认）。

### 定义 1（结算尺空间拓扑）

Σ 上的拓扑由两种结构构成：
- 节点：结算尺（法定货币 + 黄金）
- 边权重：结算通道（全球大宗商品贸易流量，石油为主要实例）
- 黄金的特殊性：同时属于 Σ（结算尺）和 K4 的 C 顶点，构成拓扑折叠点

### 定义 2（K4 共展开）

$ 顶点展开为 Σ 时，拉动 K4 整体展开：
- 原 K4 = (E, C, R, $) 四个点
- 展开后：四簇点 (E_i, C_i, R_i, $_i)，每簇由 Σ 索引
- 截面 K4_i = (E_i, C_i, R_i, $_i) = 单一结算尺下的局部 K4
- 截面间耦合通过 $_i-$_j 汇率关系 + 结算通道权重传导

### 定义 3（三层递归结构）

- 层 0：结算尺空间 Σ。对 $_i-$_j 边 + 结算通道做三态分类（吸收/保留/放大，235号）。分类结果给出活跃资本通道。
- 层 1：截面内 K4_j。232号纤维丛在此运行。联络系数为截面局部值。双管道置信度在此判断。
- 层 2：截面内缠论递归。选定矩阵后，1min 递归产出买卖点。赋格状态机在此运行。

### 定理 1（区间套语法——待完备化）

层间过渡由三态分类给出：
- 放大态 = 缠论辨识力最强处 = 区间套优先入口
- 保留态 = 第二优先
- 吸收态 = 对缠论透明，跳过或最后处理

递归顺序 = 三态逆序：先放大、后保留、最后吸收。

### 定理 2（美元霸权的形式化）

E-$_USD ∈ ker(D) = 美元霸权的形式表达。
美元与全球权益的耦合为纯幅度性质，对缠论透明。有效域边界 = E-$_USD 移出 ker(D) 之时。234号数据为当前实例（E-$：pearson=-0.183，β=-0.006，吸收率 96.7%）。

### 定理 3（黄金折叠）

黄金 ∈ Σ ∩ C，$ 和 C 在黄金处拓扑折叠。
- C-$ 边在黄金处退化为自指
- 金/油比 = 结算尺（黄金）与结算通道（石油）之间的张力指标
- 影子金价 = 美元结算尺与黄金结算尺之间的裂缝

### 公理 0 的向上展开（渐近结构，非不动点）

结算尺空间自身是资本流的历史产物（马克思 M-C-M' 螺旋）。向上展开无不动点，但有渐近序列：每层结算尺 = 上一轮资本积累的结晶。系统不求永恒公理，标注当前结晶层并监控其有效域边界。

### 开放问题 1（语法闭合的关键）

235号三态分类中放大效应（C-R: D1→D3 放大 76 倍）是否方向对称？即 C→R 和 R→C 的放大率是否相同？
- 若不对称 → 层 0 到层 1 有内在方向判据 → 语法闭合
- 若对称 → 需要另外的方向确定机制

### 开放问题 2（向下展开）

- 截面内 E_i 本身可展开（行业轮动结构是否具有 K4 类拓扑？）
- 层 2 内部递归的终止条件是什么？

### 开放问题 3（三个独立程序的统一性）

旧缠论三个独立程序（缠论走势、比价关系、基本面）在新体系中统一为资本流在不同尺度上的读数。统一后交叉验证力减弱（循环验证风险）。安全机制 = 让系统持续暴露在可否定的经验位置上（230号模式）。

## 2. 定义依据

### 结构来源

| 概念 | 来源 | 关键表述 |
|------|------|---------|
| 纤维丛联络 | 232号 | π: E → B，底空间 B = sigma_e × sigma_c，联络 ω 参数化为 softmax |
| ker(D) = 纯幅度耦合 | 233号 | D = D3 ∘ D2 ∘ D1，ker(D) = {纯幅度耦合模式} |
| 六条边全覆盖验证 | 234号 | 4 ∈ ker(D)，1 direction，1 independent，无反例 |
| 三态分类 | 235号 | 吸收/保留/放大，C-R 放大比≈76 |
| 认识论等级标注 | 231号 | L0/L1/L2/L3 四级，有效域 ≠ 定义域 |

### 数据特征满足理论条件

- 定理 2 的经验基础：234号数据 E-$ (SPY-UUP) pearson=-0.183，β=-0.006，吸收率 96.7%——E-$_USD 位于 ker(D) 核心区域，不是边界成员
- 定理 3 的经验基础：234号数据 C-$ (GLD-UUP) pearson=-0.414，β=-0.030，吸收率 92.8%——黄金作为 C 顶点的同时作为结算尺，其与美元的耦合是纯幅度性质
- 三态分类的经验基础：235号数据 C-R 放大比≈76，表明层 0→层 1 的过渡中方向信号被系统性放大

## 3. 边界条件

### 3.1 公理 0 的有效域

结算尺空间 Σ = {法定货币群 ∪ 黄金} 是当前历史阶段的经验前提。结论翻转条件：
- 新结算尺出现（如数字货币获得大宗商品定价权）→ Σ 扩展
- 现有结算尺丧失结算功能（如某法定货币退出全球贸易）→ Σ 收缩
- 黄金丧失 C 顶点属性（不再充当避险资产）→ 定理 3 的折叠结构解体

### 3.2 定理 2 的有效域

E-$_USD ∈ ker(D) 成立的条件：美元-权益耦合保持纯幅度性质。结论翻转条件：
- E-$_USD 移出 ker(D)：美元涨跌开始系统性决定权益方向（而非仅调制幅度）→ 美元霸权形式化失效
- 监控指标：E-$ 吸收率跌破 80%（当前 96.7%），或 β 显著偏离 0

### 3.3 K4 共展开的适用域

截面 K4_i 内部的纤维丛结构（232号）需要逐截面校准。当前仅在 USD 截面有 L2 验证。其他截面（CNY、EUR、JPY）的联络参数可能不同。推广到非 USD 截面需要新的 L2 验证。

### 3.4 层间传导的方向性（开放问题 1 相关）

定理 1 的递归顺序（先放大、后保留、最后吸收）依赖于放大效应的方向性。如果放大效应方向对称，区间套入口的优先级排序仍然成立，但内在方向判据缺失，需要外部输入。

## 4-6. （略）
---END 254---

## 前置谱系核心定义

### 232号（纤维丛重构）核心定义
- 底空间 B = {-1,0,+1}^2（sigma_e × sigma_c 的直积，因 E-C 偏相关 ≈ 0）
- 纤维 F = {-1,0,+1}（sigma_r）
- 联络 ω: B → Δ(F)，参数化为 softmax：P(sigma_r | sigma_e, sigma_c) ∝ exp(beta_er * sigma_e * sigma_r + beta_cr * sigma_c * sigma_r)
- beta_er ≈ -0.017（E-R 在方向层面几乎为零），beta_cr ≈ 0.688（C-R 在方向层面显著）
- 关键发现：连续-离散耦合强度排序反转——偏相关 |E-R|=0.336 > |C-R|=0.153，但 β 值 |C-R|=0.688 >> |E-R|=0.017

### 233号（ker(D) 核结构）核心定义
- D = D3 ∘ D2 ∘ D1，三层离散化算子
- D1（bar→笔）：包含处理+分型，丢弃幅度，只编码方向
- D2（笔→线段）：特征序列分型，短促振荡合并
- D3（线段→中枢→走势方向）：中枢[ZD,ZG]吸收区间内振荡
- ker(D) = {纯幅度耦合}：一个耦合模式属于 ker(D)，当且仅当它仅调制共同方向内的收益率大小，不调制方向本身
- 数据验证：E-R 连续偏相关-0.336，离散β≈-0.017 → ∈ ker(D)；C-R 连续偏相关0.153，离散β≈0.688 → ∉ ker(D)

### 235号（三态分类）核心定义
- 吸收（ker(D)）：纯幅度耦合 → 0。代表边：E-R, E-$, C-$。吸收率>92%
- 保留：底空间独立 → 不变。代表边：E-C。偏相关≈0
- 放大：方向耦合被噪声滤除相对放大。代表边：C-R。β_d1=0.009 → β_d3=0.688，放大比≈76倍
- 234号数据：E-$（SPY-UUP）pearson=-0.183, β=-0.006, 吸收率96.7%；C-$（GLD-UUP）pearson=-0.414, β=-0.030, 吸收率92.8%

## 三个审计重点（你必须逐一深入回应）

### 审计点 1：三态逆序作为递归顺序

254号定理1声称"递归顺序 = 三态逆序：先放大、后保留、最后吸收"。

质疑：这个推导是否真的从 ker(D) 的结构中严格导出，还是"先看信噪比最高的地方"这个朴素直觉的形式化包装？

具体地：
- ker(D) 和三态分类描述的是 D 对耦合的作用（观察属性），不是操作指令
- 从"D 在 C-R 通道放大方向信号"到"应当先处理 C-R 通道"，这个蕴涵关系的严格形式是什么？
- 缠论区间套的入口由级别结构完成度（如背驰）决定，不是由信噪比决定。跨品种耦合信噪比如何嫁接到单品种跨级别递归？

### 审计点 2：黄金折叠的拓扑后果

254号定理3声称"黄金 ∈ Σ ∩ C，$ 和 C 在黄金处拓扑折叠"。

质疑：C 和 $ 在黄金处重叠后，234号/235号的六边三态分类是否需要修正？黄金截面下 K4→K3 退化的严格推导。

具体地：
- 在黄金结算尺截面，C 顶点（避险资产 = 黄金）和 $ 顶点（结算尺 = 黄金）坍缩为同一节点
- K4 = (E, C, R, $) 退化为 K3 = (E, R, 黄金)
- 235号三态分类中：E-$ ∈ ker(D)（吸收态），E-C 是保留态。但在黄金截面 E-$ = E-C（同一条边），一条边不能同时是吸收态和保留态
- 这个矛盾对254号整体结构意味着什么？是否意味着 K4 共展开（定义2）在黄金截面不成立？

### 审计点 3：E-$_USD 移出 ker(D) 的可检测性

254号定理2声称 E-$_USD ∈ ker(D) 是美元霸权的形式表达，有效域边界 = E-$_USD 移出 ker(D)。

质疑：这个判据在操作上怎么检测？UUP 作为代理变量是否存在范畴错误？

具体地：
- UUP 是美元兑一篮子法定货币的相对汇率指数。美元霸权可能动摇（如丧失石油定价权）但 UUP 保持稳定——代理变量在此场景下产生假阴性
- β 从 -0.006 缓慢漂移时，何时判定拓扑相变发生？80%吸收率阈值的来源是什么——是来自 ker(D) 结构的严格推导，还是经验直觉？
- 如果检测判据本身不严格，"移出 ker(D)"就只是一个理论标记，不是操作上可执行的边界守卫
- ker(D) 的成员判定在操作中需要什么数据和什么计算？实时监控是否可行？

## 回复要求

对每个审计点，你必须：
1. 先展开完整的推理链（思考过程），确认每一步逻辑是否成立
2. 给出明确的否定判定：成立 / 不成立 / 部分成立
3. 如果否定成立，精确指出断裂发生在推导链的哪一步
4. 使用中文回复
5. 每个审计点的回复必须超过800字，确保推理链完整
"""

# Use gemini-2.5-pro (confirmed available) and gemini-3.1-pro-preview
models = [
    "gemini-2.5-pro",
    "gemini-3.1-pro-preview",
]

for model in models:
    print(f"\n{'='*60}")
    print(f"Calling {model}...")
    print(f"{'='*60}")

    url = f"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={API_KEY}"

    payload = {
        "contents": [{"parts": [{"text": PROMPT}]}],
        "generationConfig": {
            "temperature": 0.7,
            "maxOutputTokens": 16384,
            "thinkingConfig": {"thinkingBudget": 24576}
        }
    }

    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json"})

    try:
        with urllib.request.urlopen(req, timeout=180) as resp:
            result = json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8")
        print(f"HTTP Error {e.code}: {body[:500]}")
        continue

    # Save raw
    outfile = f"tmp/gemini_thinking_254_{model.replace('-', '_')}_raw.json"
    with open(outfile, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=2)
    print(f"Saved to {outfile}")

    # Parse
    candidates = result.get("candidates", [])
    if not candidates:
        print("No candidates!")
        continue

    parts = candidates[0].get("content", {}).get("parts", [])
    thinking_text = ""
    reply_text = ""

    for part in parts:
        if part.get("thought"):
            thinking_text += part.get("text", "")
        else:
            reply_text += part.get("text", "")

    usage = result.get("usageMetadata", {})
    print(f"Prompt tokens: {usage.get('promptTokenCount', 'N/A')}")
    print(f"Thinking tokens: {usage.get('thoughtsTokenCount', 'N/A')}")
    print(f"Output tokens: {usage.get('candidatesTokenCount', 'N/A')}")
    print(f"Thinking text length: {len(thinking_text)} chars")
    print(f"Reply text length: {len(reply_text)} chars")
    print(f"Finish reason: {candidates[0].get('finishReason', 'N/A')}")

    # Save parsed
    parsed_file = f"tmp/gemini_thinking_254_{model.replace('-', '_')}_parsed.json"
    with open(parsed_file, "w", encoding="utf-8") as f:
        json.dump({
            "model": model,
            "thinking_text": thinking_text,
            "reply_text": reply_text,
            "usage": usage,
            "finish_reason": candidates[0].get("finishReason", "N/A")
        }, f, ensure_ascii=False, indent=2)
    print(f"Parsed saved to {parsed_file}")

print("\nDone.")
