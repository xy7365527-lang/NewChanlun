# Basis Leading Signal Test: 2020-21 Synchronous Compression Regime

Run time: 2026-03-04T12:18:14.952775
Epistemological level: **L2** (real data, single regime event, falsifiable)

## 1. Conclusion

**Regime**: 2020-04-03 to 2021-04-23 (263 sync compression days, single cluster)

### Au Basis (GC=F - GLD*10)

- Entry classification: **LEADING_SIGNAL**
- Lead time at entry: 64 calendar days
- Exit leads eff.dim: False
- Exit lead time: -55 calendar days
- Normal period (2019) extreme z-score frequency: 0.0833
- Pre-entry extreme z-score frequency: 0.0938
- During-regime extreme z-score frequency: 0.0564

### OIL Basis (CL=F - DCOILWTICO)

- Entry classification: **LEADING_SIGNAL**
- Lead time at entry: 73 calendar days
- Exit leads eff.dim: False
- Normal period (2019) extreme z-score frequency: 0.0720
- Pre-entry extreme z-score frequency: 0.1406
- During-regime extreme z-score frequency: 0.0528

### 2020-04-20 Negative Oil Price

- In data: True
- OIL basis value: -0.65
- OIL basis z-score: -0.799
- Note: 2020-04-20: WTI May futures (CL=F) closed at approximately -$37.63, DCOILWTICO spot at approximately -$36.98. This is a real market event (storage cost exceeding commodity value), NOT a data artifact. The extreme basis value reflects genuine market dislocation.

### Combined Assessment

Au basis is a LEADING signal (leads eff.dim sync by 64 calendar days at entry); OIL basis is a LEADING signal (leads eff.dim sync by 73 calendar days at entry)

### Comparison with Previous Experiment

The previous Au basis experiment (`tmp/au-basis-experiment/`) used the old 3-cluster model:
- Cluster 1: 2020-04-22 to 2020-05-05 (6 days)
- Cluster 2: 2020-06-11 (1 day)
- Cluster 3: 2020-11-09 (1 day)

This experiment uses the corrected single-regime model:
- Single regime: 2020-04-03 to 2021-04-23 (263 sync days)

The old model had a narrower cluster start (2020-04-22), which made leading signal detection easier. 
The corrected regime start (2020-04-03) is earlier, which may change the lead time calculation.

## 2. Definition Basis

- **Au basis** = GC=F - GLD*10. GLD (SPDR Gold Trust) tracks gold spot price. 1 share of GLD represents approximately 1/10 troy ounce of gold. Tracking error is small relative to basis deviations.
- **OIL basis** = CL=F - DCOILWTICO. DCOILWTICO is WTI spot price from FRED (Cushing, Oklahoma delivery). CL=F is CME WTI futures continuous contract.
- **Sync compression** = all 4 node eff.dim simultaneously < their respective full-sample 5th percentile (329 settlement).
- **Z-score** = (basis - 20d rolling mean) / 20d rolling std. |z| > 2 = extreme.
- **Leading signal** = basis z-score breaches |z| > 2 before eff.dim first enters sync compression.

## 3. Boundary Conditions

The following changes would alter the conclusion:

1. **Z-score threshold**: Using |z| > 1.5 or |z| > 3 would change the "first extreme" date.
2. **Rolling window**: 20d is arbitrary. 10d (more sensitive) or 60d (more stable) would shift detection dates.
3. **GLD as spot proxy**: GLD has tracking error, creation/redemption costs, and is an ETF (not physical gold). If LBMA London Fix were available, basis computation would differ.
4. **Regime boundary**: 2020-04-03 is the first day ALL 4 nodes are below 5th percentile. A different threshold (e.g., 10th percentile) would shift this date.
5. **Single regime**: This is N=1 analysis. The conclusion does not generalize to other regimes without additional L3-level testing.
6. **Futures roll artifacts**: Both GC=F and CL=F are continuous contracts with roll artifacts. OIL basis partially mitigates this (spot side from FRED), but Au basis (both sides futures-linked) retains this issue.

## 4. Downstream Implications

- If basis is a leading signal: basis monitoring can be added to the K4 eff.dim dashboard as an early warning indicator.
- If basis is concurrent/lagging: basis is a symptom of sync compression, not a predictor. No actionable signal.
- The OIL basis has the 2020-04-20 negative oil price event within the analysis window. If OIL basis leads, the negative oil price is part of the leading signal (market dislocation precedes statistical sync compression).
- This is L2 (single regime). Upgrading to L3 would require finding other sync compression regimes (none in 2000-2025 data) or using different asset classes.

## 5. Genealogy References

- 329: K4 sync compression definition + threshold causality + QE regime dependency
- 330: K4 topology and capital flow mapping
- 231: Formalization validity domain rule (L2 annotation)
- Previous experiment: `tmp/au-basis-experiment/` (3-cluster model, superseded)

## 6. Impact Statement

- No code or definition changes.
- Three new files in `tmp/regime-analysis/basis_leading/`:
  - `basis_leading_analysis.py` (this script)
  - `basis_leading_results.json` (structured results)
  - `basis_leading_report.md` (this report)
- Supersedes `tmp/au-basis-experiment/` conclusions for the 2020-21 regime (corrected from 3-cluster to single-regime model).
- OIL basis analysis is new (not in previous experiment).
