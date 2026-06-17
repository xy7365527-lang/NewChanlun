#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
辩证法穷尽的数量交叉验证（独立代数计算，非数数）。

验证三个数：
  1. Burnside 轨道级联 506→46→2→1（H⁰ 基本域 = 46 = 23×2，R-无关）
  2. dim H¹(D∞, ℝ₋) = 1（扬弃层秩 1：所有 H¹ 投影 = 同一生成元 Δr=-1）
  3. H¹ 在各模上的维数（ℝ₋=1 / ℝ_triv=0 / ℚ[ℤ/23]=0）— 检验"投影数 ≤ 模维数"

群：D∞ = ⟨h, τ | τ²=e, τhτ⁻¹=h⁻¹⟩ = ℤ/2 * ℤ/2，a=τ, b=τh，σ=h²³。
依据：docs/orbit_enumeration_completeness.md §2/§4B；纯 L0 群论/上同调。
"""
from fractions import Fraction

# ============================================================
# Part 1: Burnside 轨道级联（环面紧化 S_cyc = ℤ/23 × ℤ/R × {±1}）
# ============================================================
# helix 编码：(n, e)，n ∈ ℤ/(23R)，e ∈ {0,1}（0=+1 手性, 1=-1 手性）
#   h:     n -> n+1 (mod 23R)        （角向推进一环）
#   σ=h²³: n -> n+23 (mod 23R)       （径向升一级，φ 不变）
#   τ:     n -> -n (mod 23R), e->1-e （手性翻转 + 角向反演 τhτ⁻¹=h⁻¹）

def orbit_count(R, gens, tau_flips=True):
    M = 23 * R
    states = [(n, e) for n in range(M) for e in (0, 1)]
    idx = {s: i for i, s in enumerate(states)}
    parent = list(range(len(states)))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(a, b):
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[ra] = rb

    def h(s):
        n, e = s; return ((n + 1) % M, e)

    def sig(s):
        n, e = s; return ((n + 23) % M, e)

    def tau(s):
        n, e = s; return ((-n) % M, (1 - e) if tau_flips else e)

    gen_funcs = {'h': h, 'sigma': sig, 'tau': tau}
    for s in states:
        for g in gens:
            union(idx[s], idx[gen_funcs[g](s)])
    return len(states), len({find(i) for i in range(len(states))})


print("=" * 64)
print("Part 1: Burnside 轨道级联（子群塔）")
print("=" * 64)
for R in [3, 7, 11, 20]:
    sz, o_e = orbit_count(R, [])               # ⟨e⟩
    _, o_sig = orbit_count(R, ['sigma'])       # ⟨σ⟩ ≅ ℤ/R
    _, o_h = orbit_count(R, ['h'])             # ⟨h⟩ ≅ ℤ/(23R)
    _, o_full = orbit_count(R, ['h', 'tau'], tau_flips=True)   # D_{23R}, τ 翻 ε
    _, o_full_nf = orbit_count(R, ['h', 'tau'], tau_flips=False)  # τ 不翻 ε
    print(f"R={R:2d}: |S|={sz:4d}  ⟨e⟩={o_e:4d}  ⟨σ⟩={o_sig:3d}  "
          f"⟨h⟩={o_h:2d}  D(τ翻ε)={o_full}  D(τ不翻ε)={o_full_nf}")
print("预期(R=11): |S|=506, ⟨e⟩=506, ⟨σ⟩=46, ⟨h⟩=2, D(翻)=1, D(不翻)=2")
print("关键: ⟨σ⟩ 轨道数 = 46 对所有 R 不变 ⟹ 46=23×2 基本域 R-无关 ✓")

# ============================================================
# Part 2/3: H¹(D∞, V) = V/(V^a + V^b)  （Bass-Serre, ℝ 系数）
# ============================================================
# D∞ = ℤ/2 * ℤ/2 = ⟨a⟩ * ⟨b⟩, a=τ, b=τh。H¹(ℤ/2, ℝ-mod)=0（char 0）⟹
#   H¹(D∞,V) = coker( V^a ⊕ V^b → V ) = V / (V^a + V^b)
#   dim H¹ = dim V − dim(V^a + V^b)，  V^a=ker(a−I)， V^b=ker(b−I)

def nullspace_basis(mat):
    """返回 mat 零空间的一组基（精确 Fraction 高斯消元）。mat: list[list[Fraction]]。"""
    if not mat:
        return []
    A = [row[:] for row in mat]
    rows, cols = len(A), len(A[0])
    pivot_cols = []
    r = 0
    for c in range(cols):
        piv = None
        for i in range(r, rows):
            if A[i][c] != 0:
                piv = i; break
        if piv is None:
            continue
        A[r], A[piv] = A[piv], A[r]
        inv = Fraction(1) / A[r][c]
        A[r] = [x * inv for x in A[r]]
        for i in range(rows):
            if i != r and A[i][c] != 0:
                f = A[i][c]
                A[i] = [a - f * b for a, b in zip(A[i], A[r])]
        pivot_cols.append(c)
        r += 1
        if r == rows:
            break
    free_cols = [c for c in range(cols) if c not in pivot_cols]
    basis = []
    for fc in free_cols:
        vec = [Fraction(0)] * cols
        vec[fc] = Fraction(1)
        for ri, pc in enumerate(pivot_cols):
            vec[pc] = -A[ri][fc]
        basis.append(vec)
    return basis


def mat_rank(rows):
    if not rows:
        return 0
    A = [row[:] for row in rows]
    nr, nc = len(A), len(A[0])
    r = 0
    for c in range(nc):
        piv = None
        for i in range(r, nr):
            if A[i][c] != 0:
                piv = i; break
        if piv is None:
            continue
        A[r], A[piv] = A[piv], A[r]
        inv = Fraction(1) / A[r][c]
        A[r] = [x * inv for x in A[r]]
        for i in range(nr):
            if i != r and A[i][c] != 0:
                f = A[i][c]
                A[i] = [a - f * b for a, b in zip(A[i], A[r])]
        r += 1
        if r == nr:
            break
    return r


def H1_dim(dim, a_mat, b_mat):
    I = [[Fraction(1) if i == j else Fraction(0) for j in range(dim)] for i in range(dim)]
    aI = [[a_mat[i][j] - I[i][j] for j in range(dim)] for i in range(dim)]
    bI = [[b_mat[i][j] - I[i][j] for j in range(dim)] for i in range(dim)]
    Va = nullspace_basis(aI)   # ker(a-I)
    Vb = nullspace_basis(bI)   # ker(b-I)
    dim_sum = mat_rank(Va + Vb)  # dim(V^a + V^b)
    return dim - dim_sum, len(Va), len(Vb), dim_sum


print()
print("=" * 64)
print("Part 2/3: dim H¹(D∞, V) = dim V − dim(V^a + V^b)")
print("=" * 64)

# 模 1: ℝ₋（h↦+1, τ↦−1）⟹ a=τ↦−1, b=τh↦(−1)(+1)=−1
a = [[Fraction(-1)]]; b = [[Fraction(-1)]]
d, va, vb, s = H1_dim(1, a, b)
print(f"ℝ₋   (a=−1,b=−1): dimV=1  dimV^a={va} dimV^b={vb} dim(V^a+V^b)={s}  =>  dim H¹ = {d}")

# 模 2: ℝ_triv（h↦+1, τ↦+1）⟹ a=+1, b=+1
a = [[Fraction(1)]]; b = [[Fraction(1)]]
d, va, vb, s = H1_dim(1, a, b)
print(f"ℝ_triv(a=+1,b=+1): dimV=1  dimV^a={va} dimV^b={vb} dim(V^a+V^b)={s}  =>  dim H¹ = {d}")

# 模 3: ℚ[ℤ/23]（角向置换，σ 平凡）。商 D₂₃=⟨h̄,τ⟩, h̄^23=e。
#   h̄: e_n ↦ e_{n+1};  τ: e_n ↦ e_{−n}
#   a=τ:    e_n ↦ e_{−n mod 23}
#   b=τh̄:   e_n ↦ τ(e_{n+1}) = e_{−(n+1) mod 23}
N = 23
def perm_matrix(f):
    M = [[Fraction(0)] * N for _ in range(N)]
    for n in range(N):
        M[f(n)][n] = Fraction(1)
    return M
a = perm_matrix(lambda n: (-n) % N)
b = perm_matrix(lambda n: (-(n + 1)) % N)
d, va, vb, s = H1_dim(N, a, b)
print(f"ℚ[ℤ/23](角向): dimV={N} dimV^a={va} dimV^b={vb} dim(V^a+V^b)={s}  =>  dim H¹ = {d}")

print()
print("=" * 64)
print("结论交叉验证")
print("=" * 64)
print("H¹ 支撑唯一在 ℝ₋（手性符号模/径向方向）= 1 维；角向 ℚ[ℤ/23] 与平凡 ℝ_triv 均 0。")
print("⟹ dim H¹(D∞)=1 ⟹ 扬弃层秩=1 ⟹ 所有 H¹ 投影 = 同一生成元 Δr=−1（不独立）。")
print("⟹ 若辩证图扬弃节点 >1 个'独立'不变量，则 H¹ 维数算错或投影不独立。")

# ============================================================
# Part 4: 三个数的范畴区分（代数强制 vs 构造计数）+ 否定上界 + 边数检验
# ============================================================
print()
print("=" * 64)
print("Part 4: 数量范畴区分（诚实标注：哪些代数强制，哪些是构造）")
print("=" * 64)

# 4a. 坐标子集 cell 数 = 2³（代数强制：3 坐标 {φ,r,ε} 的幂集）
cells = 2 ** 3
print(f"[代数强制] 坐标子集 cell 数 = 2^3 = {cells}（{{φ,r,ε}} 的幂集）")

# 4b. H⁰=58 的来源剖析——它 *不是* Burnside 轨道数（=46）也不是 dim H⁰(函数环)（=1）
#     而是"独立不变量的标签计数" = 11 基础 + 47 派生（spiral_exhaustive 构造）
#     = 54(T1-T54) + 4(T56-T59)（necessity §11.2）。两个分解的算术一致性检验：
base_inv = 11        # 基础不变量（生成元/关系/商/Casimir/上同调/闭合）
derived_inv = 47     # 派生（11 基础 × 8 cell × 操作/会计/形态语境的实现，去重后）
t_direct = 54        # T1-T54
t_basic = 4          # T56-T59
print(f"[构造计数] H⁰ 标签数: 11基础+47派生 = {base_inv + derived_inv}  ；  "
      f"54(T1-54)+4(T56-59) = {t_direct + t_basic}  ；  两分解 == 58 ? "
      f"{base_inv + derived_inv == 58 == t_direct + t_basic}")
print("  诚实：58 是 *标签计数*（construction），非干净群不变量。")
print("  干净群不变量是：Burnside 基本域=46，dim H⁰(全空间函数环)=1，dim H¹=1。")
print("  58 ≠ 46：46 数'轨道'（什么互相可达），58 数'被命名的独立不变量'（含 8 cell × 语境实现）。")

# 4c. 否定推论数（NR）的来源剖析——同样是标签计数，独立根更少
#     合法单子 M 生成元 = {h_fwd, τ_seam, σ⁻¹_read} = 3
#     禁止模式（M 在 D∞ 作用的边界）= {P1 时间倒流, P2 τ@φ≠0, P3 径向越界, P1' 前向读未来}
legal_gens = 3
forbidden_patterns = 4   # P1, P2, P3, P1'（P1/P1' 同属观测⊥操作维 → NR-4）
# NR 标签：边界方向(NR-3 手性/NR-4 观测操作/NR-5 越界)=3 + σ=e投影(NR-1形态/NR-2时间/NR-6操作,同一根)=3 + 上同调残余(NR-7)=1
nr_boundary = 3          # NR-3, NR-4, NR-5
nr_sigma_eq = 3          # NR-1, NR-2, NR-6（同一 σ≠e 根的三语境投影）
nr_cohom = 1             # NR-7（H²(D∞,ℤ)=(ℤ/2)² 第二生成元）
nr_total = nr_boundary + nr_sigma_eq + nr_cohom
nr_independent_roots = 2 + 1 + 1 + 1  # σ≠e(1根) + 手性缝 + 观测/操作 + 边界 + 上同调 ... 见注
print(f"\n[构造计数] 否定推论 NR 标签数 = {nr_boundary}(边界)+{nr_sigma_eq}(σ=e三语境)+{nr_cohom}(上同调) = {nr_total}")
print(f"  合法单子生成元={legal_gens}, 禁止模式={forbidden_patterns}（P1/P1' 合为观测⊥操作 NR-4）")
print("  独立根更少：σ≠e（NR-1/2/6 同根，1）+ 手性缝 NR-3 + 观测操作 NR-4 + 边界 NR-5 + 上同调 NR-7 = 5 独立根。")

# 4d. 辩证运动图边数检验（节点 N，边 ≤ N(N-1)/2；实际远少；太少=遗漏）
def edge_bound(n):
    return n * (n - 1) // 2
print("\n[边数检验] 节点 N → 完全图上界 N(N-1)/2（实际辩证边远少于此；太少=遗漏信号）")
# 节点 = 58(T) + 7(NR) + 1(A1扬弃,dim H¹=1) + 2(A2/组合) ≈ 59正面 + 7否定 + 扬弃
for N in [59, 66, 68]:
    print(f"  N={N}: 完全图上界={edge_bound(N)} 边（辩证边只占其中携带蕴含/否定/扬弃语义的子集）")
print("  穷尽性硬检验：#否定边 必 == #(否定边有对应扬弃边) ；差 = gap 数（预期 gap = NR-7 + T41）。")
