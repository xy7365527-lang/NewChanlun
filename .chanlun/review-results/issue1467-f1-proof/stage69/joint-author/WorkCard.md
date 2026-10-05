# Stage69：同一 JointCut 关系在既有 H2 上的有限检验

研究票 #1467；作者 A。计算前冻结，唯一写域为本目录。采用 rigorous-open-math-research 技能。只读取既有冻结材料；不读 MEMORY，不改旧冻结、仓库、生产、formal、教义或 tracker。

## 问题与输入

检验 Stage68 Ω68-JointCut-v1 原规则在 Stage64 **H2**（不是 H1）已有的 583 观察、582 事件、144 份 R_W 笔、S0…S27 共 28 个所选已审 raw 段上，是否产生从固定 S1 起点出发的非空三因子候选 F2 必要条件见证。S0 仍为固定前导，state0→1 的 E1 仍为左支持。不能搬用旧 LocalDyn64 从 S0 开始的 U/D/U。零新历史，零 R_W 重跑，零价格、时钟或参数修改；只扩实际窗口上界，不改谓词。

原始 H2 文件取 stage64/dynamic-author/v2/run/{source.json,segments.json,reference.stdout}，依 author run/manifest.json 与 dynamic-review/input-hashes.json、独评证据核实际 SHA256。读取全参考数据并不构成物理未来读取隔离。

## 同一候选关系

每个窗口由任意连续 4k+1 段组成，k≥1：b、三段核、连接段与后续三段核、c。枚举 S1…S27 内所有可容纳窗口，长度 5/9/13/17/21/25，共 78 窗口。每核三段范围正宽严格交、方向交替；Outer0=core；单核类型 P，多核严格同向分离为 U/D 且须与 c 实际方向一致。

围绕最后核，b_ref 为其紧前 raw，c 为最后 raw；同向、核外。Up 入臂 start<ZD≤end、出臂 start≤ZG<end；Down 入臂 start>ZG≥end、出臂 start≥ZD>end。WholeExtreme 用 whole 起点到 c 开始的真实观察，c 末价须严格越过同向极值。L 为实际首末结构笔速度之差，速度按事件时间差，直接带号 L(c)<L(b_ref)。已封 raw 的端点证与上述谓词联合得到 JEnd68 候选完成；不赋原义完成。

原规则的 RootArm68、Outer0=core、CycleDiv68 + raw c 封证到 whole 完成的源义桥保持未证；不是将旧 P 或旧 LocalDyn64 完成字段运输给新 whole。

## 单入口修复与控制

新 check_joint.py 显式输出**全部边路径（含空）**，再分别分类 maximal、非空 NoPP、至少三孩子及候选 F2 必要条件通过集合。空路径不作成功；短 NoPP 路径若可继续或尾内含可选对象，不称完整或唯一分解。NoPP 仅用于此处实际独立构造因子 Adj，不由 MemberNext 泛推。申请多解不自动是原义多解。

先从旧入口原样抽取 force/meet/窗口谓词，保留抽取文本、AST/函数绑定及差异。新实现先以 Stage68 封存的 raw-roots/all-windows（40 窗口）及 all-delta-paths/path-classes（443 前缀）作字段等值控制；控制不重跑 R_W，也不引入第二个主候选。不得导入旧入口触发覆写。

通过等值控制之后，仅对 H2 主运行一次：全 78 窗口、全 583 前缀；边只在全部 raw 证 known_at≤t 时可见。记录真实 Path.cwd()、完整 argv、程序 hash、输入前后 hash、时间和 RSS；输出目录排他创建，禁止覆盖既有输出。

## 失败条件、消费与边界

枚举失败层次为：结构、比较角色、whole 极值、有符号力度、从 S1 接续、NoPP、三孩子、固定 F2 必要关系。三孩子存在时列实际 Own/whole/core/clock，检查前三孩子同级、连续、技术方向交替、严格三交及独立因子 NoPP。此检查不是生产验收，OriginalCompleted 未知，因此原义 F2 仍不认证。

如失败，给全图、每个层次拒绝数、可达最大因子数以及具体阻碍。去 NoPP 为同一边图的必要对照，不改其余规则。27 段下四因子 NoPP 至少要 28 段；三因子可行核数枚举必须覆盖所有长度型，不沿用 20 段只有 5/9/5 的论证。

成功时须指出相对 Stage68 究竟哪个可检验条件改变；失败时定位差异，不改参重试。无论结果如何，LocalDyn64+A64-global 的既有不相容不复活，全目录后继不自动取得原义 Next。归属、生命周期、一般完成、RootArm、初始外缘、source 笔桥、覆盖/唯一性/NE 保持相应未证边界。原候选发布与 knownAt 不回写。

## 资源与交付

唯一新检验输入 1 份；≤600 前缀、≤32 raw 段、RSS 和新增产物各≤96 MiB。一次主运行；程序错误另记，不能隐性重试模型。交付 Report、独立可重算脚本、rule-binding/diff、H2 输入 hash 前后、全窗口/路径/关键前缀、命令回执、manifest/FINAL；作者自检不替代独评。
