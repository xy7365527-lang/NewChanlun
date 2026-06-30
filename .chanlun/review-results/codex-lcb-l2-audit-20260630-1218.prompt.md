You are an adversarial code reviewer (heterosource negation). Attack the L2 empirical conclusion below. Report in English. Be terse and brutal. For EACH of the 4 attack points: state whether the attack SUCCEEDS (conclusion has a defect) or FAILS (conclusion robust), tag epistemic level (L0 pure-algebra/definition, L1 synthetic/pipeline, L2 real-data falsifiable, L3 cross-validated), then a final verdict (A) robust or (B) defective.

# DOMAIN (Chanlun quant, theta_v0 alpha selector)
A selector chi_t opens a position for class z iff its alpha estimate exceeds threshold theta=0. Two selectors compared, ONLY variable = chi_z_alpha:
- naive mu: chi opens iff mu(z) = sample mean > theta. (z_alpha=0)
- LCB: chi opens iff LCB(z) = mean - 1.645*std/sqrt(n) > theta. (z_alpha=1.645, 95% one-sided lower confidence bound)
mu_lcb code: n<2 -> None (std undefined, single sample); n>=2 -> Some(mean - z*std/sqrt(n)). LCB <= mean always.
A "class z" is a discrete bucket of alpha observations; n = #observations in that bucket. Walk-forward: mu table estimated on train half (first 8000 bars), frozen, applied on test half. 8 symbols, 16K bar test windows (O(n^2) wall prevents longer).

# THE CONCLUSION UNDER AUDIT
Two-dimension split verdict:
- DIM-1 (overfit control): src(b)=39 >> src(a)=9. Claim "LCB has real alpha value (correctly shrinks high-variance classes to rejection)". Evidence: CL/BRN/OKLO chi-open-count crushed to 0; BRN delta-R flips negative->positive (-1.3e-6 -> +1.4e-5); OKLO improves (+1.4e-6 -> +9.6e-6).
- DIM-2 (statistical power): naive n_L3=4 sign-test p=0.3125; LCB n_L3=1 p=0.5. Both INCONCLUSIVE (n_L3<5, even all-positive can't reach p<0.05).
- Core tension: LCB improves DIM-1 at the COST of weaker DIM-2 (degenerate symbols 7>4, L3 pool shrunk 4->1). Conclusion: "LCB alone does NOT resolve inconclusive."

Two-source decomposition of "naive passes AND LCB rejects":
- src(a) n<2 no-LCB-evidence rejected = 9 (sample starvation, NOT overfit control, honest but no alpha value)
- src(b) n>=2 high-variance LCB<theta rejected = 39 (claimed: REAL alpha value)

Per-symbol: BTC/ES/GC/DX/QQQ: src_b=0 (LCB==naive delta-R identical). CL src_b=15, BRN src_b=10, OKLO src_b=14. (src_b concentrates in exactly the 3 symbols whose chi-count went to 0.)

# 4 ATTACK POINTS
1. src(b)=39 real overfit control or artifact? n>=2 high-variance classes rejected by LCB -- is this "LCB correctly IDs overfit" or "LCB over-conservative, rejected classes that HAD signal" (type II error)? Is BRN's flip negative->positive a real improvement or noise from few samples? (Note: BRN n_L3 -- the test does NOT report it. CL/BRN/OKLO all went to chi-count=0, meaning they DEGENERATED to empty position. Their delta-R is computed how, if no orders?)

2. Is the dimension split REAL? Are DIM-1 (overfit) and DIM-2 (power) truly orthogonal, or is "LCB rejects more classes -> fewer samples -> lower power" the SAME phenomenon seen twice (LCB tightening = simultaneously less overfit + less power)? Is the orthogonality claim artificial?

3. Is the two-source (a/b) attribution clean? Boundary src(a) n<2 vs src(b) n>=2. n=2 class variance estimate is extremely unstable (1 dof). Does it count as "src(b) valid evidence" or should it fold into starvation? How many of the 39 src(b) are n=2 or n=3 low-dof classes? (The test code does NOT bucket src_b by n -- it only counts mu_lcb(z)=Some AND <=theta. A class with n=2 and one outlier produces huge std -> LCB crashes below theta -> counted as src_b "alpha value". Is that overfit CONTROL or just variance-estimation noise being relabeled as signal?)

4. Falsifiability / honesty. Conclusion = "(a) LCB improves + inconclusive root cause is power deficit". Is this OVER-OPTIMISTIC? CL/BRN/OKLO had ALL their trades rejected by LCB (chi-count -> 0 = flat = no positions). Flat = no trading = no alpha. Yet this is dressed up as "overfit control". Is "chi crushed to 0" alpha value or degeneracy? A selector that rejects everything has zero overfit BY CONSTRUCTION (trivially: never trades, never overfits). How does the conclusion distinguish "LCB found real overfit and pruned it" from "LCB is so conservative on these 3 symbols it just stopped trading"? The improved delta-R on a flat book -- what does it even measure?

# KEY CODE FACTS (verified by reading the Rust source)
- delta_r_stats compares base equity (chi==1 always-open) vs selector equity. If selector n_orders==0, the selector book is FLAT. base is chi==1 (ALWAYS open / always-long). So flat-selector vs always-open-base: delta-R = nav*(E_flat - E_alwaysopen). On a DOWN window, flat (no position) BEATS always-open-long. So BRN delta-R "negative->positive" may just mean "NOT trading beats trading in a losing window" -- NOT alpha discovery.
- L3 pool admission requires st.chi_changed_trades AND selector.n_orders>0 AND base.n_orders>0. So a DEGENERATE (n_orders==0) symbol is EXCLUDED from L3 pool. BRN/CL/OKLO with chi-count 0 -> excluded from LCB L3 pool -> that's WHY LCB n_L3 dropped 4->1.
- So the SAME symbols (CL/BRN/OKLO) are simultaneously: counted as "src_b alpha value" in DIM-1 AND excluded from L3 pool in DIM-2 because they degenerated. Question: is "alpha value" and "degenerate/excluded" the same fact relabeled with opposite valence?

Attack hard. Genealogy rule 161 forbids dressing up "stopped trading" as "improvement".
