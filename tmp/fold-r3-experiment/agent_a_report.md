# Blind Descriptive Analysis: Three Ratio Series (e1, e2, e3)

**Data**: 6391 daily observations, 2000-08-30 to 2026-02-27

## 1. Individual Series Characteristics

### e1

**Level statistics**: mean=13.8729, std=7.9583, range=[2.1783, 55.2389], current=53.5857
**Total change over sample**: +2093.9%
**Level distribution**: skewness=1.07, excess kurtosis=2.99

**Daily return distribution**: mean=0.000483, std=0.0139, skewness=-0.29, excess kurtosis=4.34
**Jarque-Bera**: stat=5088.6, p=0.00e+00 (reject normality)

**ADF test (levels)**: stat=3.733, p=1.0000 → non-stationary
**ADF test (returns)**: stat=-79.620, p=0.000000 → stationary
**Time in uptrend (252d rolling)**: 69.4%

**Volatility regime breaks**: 50 detected
  - 2001-06-01 to 2001-06-11 (z-peak=2.23)
  - 2001-09-10 to 2001-10-02 (z-peak=3.73)
  - 2001-12-17 to 2002-01-25 (z-peak=3.79)
  - 2002-06-20 to 2002-09-03 (z-peak=4.71)
  - 2003-09-25 to 2003-10-02 (z-peak=2.31)
  - 2003-12-31 to 2004-01-14 (z-peak=3.83)
  - 2004-02-20 to 2004-04-28 (z-peak=2.74)
  - 2004-08-09 to 2004-10-11 (z-peak=2.7)
  - 2005-05-25 to 2005-06-27 (z-peak=2.17)
  - 2005-11-04 to 2005-11-08 (z-peak=2.47)

### e2

**Level statistics**: mean=0.0026, std=0.0009, range=[0.0011, 0.0062], current=0.0011
**Total change over sample**: -64.5%
**Level distribution**: skewness=1.54, excess kurtosis=2.36

**Daily return distribution**: mean=-0.000162, std=0.0172, skewness=-0.57, excess kurtosis=10.65
**Jarque-Bera**: stat=30489.6, p=0.00e+00 (reject normality)

**ADF test (levels)**: stat=-2.097, p=0.2457 → non-stationary
**ADF test (returns)**: stat=-19.323, p=0.000000 → stationary
**Time in uptrend (252d rolling)**: 41.7%

**Volatility regime breaks**: 47 detected
  - 2001-06-08 to 2001-07-06 (z-peak=3.37)
  - 2001-08-21 to 2001-09-10 (z-peak=4.07)
  - 2001-12-03 to 2001-12-05 (z-peak=2.31)
  - 2002-01-10 to 2002-02-11 (z-peak=2.35)
  - 2002-05-29 to 2002-05-30 (z-peak=2.06)
  - 2003-01-23 to 2003-04-15 (z-peak=3.92)
  - 2003-12-01 to 2003-12-22 (z-peak=2.34)
  - 2004-02-27 to 2004-05-18 (z-peak=3.42)
  - 2004-10-13 to 2004-11-12 (z-peak=3.23)
  - 2005-01-04 to 2005-01-07 (z-peak=2.16)

### e3

**Level statistics**: mean=0.5918, std=0.2810, range=[0.1821, 1.6806], current=0.7604
**Total change over sample**: +317.1%
**Level distribution**: skewness=1.24, excess kurtosis=0.76

**Daily return distribution**: mean=0.000224, std=0.0165, skewness=0.12, excess kurtosis=6.45
**Jarque-Bera**: stat=11074.2, p=0.00e+00 (reject normality)

**ADF test (levels)**: stat=-1.841, p=0.3605 → non-stationary
**ADF test (returns)**: stat=-20.442, p=0.000000 → stationary
**Time in uptrend (252d rolling)**: 58.3%

**Volatility regime breaks**: 49 detected
  - 2001-07-09 to 2001-08-23 (z-peak=2.55)
  - 2001-09-24 to 2001-09-24 (z-peak=2.0)
  - 2002-06-19 to 2002-09-05 (z-peak=4.46)
  - 2003-01-16 to 2003-02-03 (z-peak=2.85)
  - 2003-07-02 to 2003-07-03 (z-peak=2.11)
  - 2004-01-06 to 2004-01-22 (z-peak=3.67)
  - 2004-06-22 to 2004-06-23 (z-peak=2.29)
  - 2004-11-30 to 2004-12-07 (z-peak=2.67)
  - 2005-06-10 to 2005-08-11 (z-peak=3.09)
  - 2005-10-31 to 2005-11-10 (z-peak=2.33)

## 2. Pairwise Relationships

### Correlation Matrix (full sample, levels)

|     | e1   | e2   | e3   |
|-----|------|------|------|
| e1 | 1.0  | -0.5015 | 0.4009 |
| e2 | -0.5015 | 1.0  | -0.1279 |
| e3 | 0.4009 | -0.1279 | 1.0  |

### Correlation Matrix (full sample, daily returns)

|     | e1   | e2   | e3   |
|-----|------|------|------|
| e1 | 1.0  | -0.2572 | 0.6143 |
| e2 | -0.2572 | 1.0  | -0.4152 |
| e3 | 0.6143 | -0.4152 | 1.0  |

### Pair e1-e2

**Rolling correlation (252d)**: mean=-0.283, std=0.212, range=[-0.715, 0.167]
**Correlation sign changes**: 10 times over sample
**Cointegration (Engle-Granger)**: stat=0.973, p=1.0000 → not cointegrated

### Pair e1-e3

**Rolling correlation (252d)**: mean=0.639, std=0.156, range=[0.211, 0.901]
**Correlation sign changes**: 0 times over sample
**Cointegration (Engle-Granger)**: stat=4.603, p=1.0000 → not cointegrated

### Pair e2-e3

**Rolling correlation (252d)**: mean=-0.418, std=0.167, range=[-0.689, -0.030]
**Correlation sign changes**: 0 times over sample
**Cointegration (Engle-Granger)**: stat=-2.035, p=0.5104 → not cointegrated

### Time-Varying Correlation by 3-Year Epochs

| Period | e1-e2 | e1-e3 | e2-e3 | Top eigenvalue |
|--------|-------|-------|-------|---------------|
| 2001-2004 | -0.571 | +0.692 | -0.557 | 2.215 |
| 2004-2007 | -0.099 | +0.819 | -0.134 | 1.851 |
| 2007-2010 | -0.131 | +0.545 | -0.383 | 1.732 |
| 2010-2013 | -0.107 | +0.472 | -0.535 | 1.768 |
| 2013-2016 | -0.487 | +0.786 | -0.537 | 2.217 |
| 2016-2019 | -0.402 | +0.686 | -0.496 | 2.065 |
| 2019-2022 | -0.442 | +0.516 | -0.508 | 1.978 |
| 2022-2025 | -0.093 | +0.454 | -0.281 | 1.579 |
| 2025-2028 | -0.275 | +0.805 | -0.354 | 2.003 |

### Lead-Lag Analysis

**e1-e2**: max |cross-corr| at lag=-2 (value=-0.2572) → e2 leads e1 by 2 days
  Key lags: lag -5: -0.2564, lag -3: -0.2570, lag -1: -0.2571, lag 0: -0.2572, lag 1: -0.2571, lag 3: -0.2570, lag 5: -0.2564

**e1-e3**: max |cross-corr| at lag=-4 (value=0.6145) → e3 leads e1 by 4 days
  Key lags: lag -5: 0.6143, lag -3: 0.6144, lag -1: 0.6143, lag 0: 0.6143, lag 1: 0.6143, lag 3: 0.6144, lag 5: 0.6143

**e2-e3**: max |cross-corr| at lag=-3 (value=-0.4153) → e3 leads e2 by 3 days
  Key lags: lag -5: -0.4147, lag -3: -0.4153, lag -1: -0.4152, lag 0: -0.4152, lag 1: -0.4152, lag 3: -0.4153, lag 5: -0.4147

## 3. Joint Structure

### Full-Sample PCA (on standardized levels)

**Explained variance**: PC1=56.9%, PC2=29.2%, PC3=13.9%

**Component loadings**:

| Component | e1 | e2 | e3 |
|-----------|------|------|------|
| PC1 | +0.672 | -0.563 | +0.482 |
| PC2 | -0.047 | +0.617 | +0.786 |
| PC3 | +0.739 | +0.551 | -0.387 |

### Full-Sample PCA (on standardized daily returns)

**Explained variance**: PC1=62.5%, PC2=25.5%, PC3=11.9%

**Component loadings**:

| Component | e1 | e2 | e3 |
|-----------|------|------|------|
| PC1 | +0.594 | -0.481 | +0.645 |
| PC2 | +0.510 | +0.845 | +0.160 |
| PC3 | -0.622 | +0.234 | +0.747 |

### Rolling PCA Summary (252-day window, returns)

**PC1 explained variance**: mean=64.3%, std=9.4%, range=[43.7%, 83.8%]
**Effective dimensionality**: mean=2.02, std=0.31

Interpretation: effective dimensionality near 1 means all three series move in lockstep; near 3 means they are essentially independent.

## 4. Structural Breaks (CUSUM)

**e1**: no significant structural break detected
**e2**: no significant structural break detected
**e3**: no significant structural break detected

## 5. Anomalous and Extreme Periods

### Extreme Days (Mahalanobis distance, top 1%)

Total extreme periods (clustered): 24

| Period | Days | Max Mahal. | e1 cum% | e2 cum% | e3 cum% |
|--------|------|-----------|---------|---------|---------|
| 2001-09-17 to 2001-09-17 | 1 | 7.5 | +8.8 | -6.6 | +11.5 |
| 2002-07-24 to 2002-07-24 | 1 | 5.2 | +0.7 | +0.2 | -6.0 |
| 2004-10-13 to 2004-10-13 | 1 | 7.1 | -0.3 | -11.2 | +0.3 |
| 2005-01-04 to 2005-01-04 | 1 | 5.7 | -1.6 | -8.8 | +1.1 |
| 2005-05-27 to 2005-05-27 | 1 | 5.5 | +0.9 | -8.7 | +0.3 |
| 2006-05-23 to 2006-05-23 | 1 | 6.7 | +2.4 | +8.9 | +2.8 |
| 2006-06-13 to 2006-06-13 | 1 | 6.2 | -8.2 | +0.3 | -6.5 |
| 2008-09-17 to 2008-12-16 | 28 | 9.9 | +6.0 | -91.3 | +36.5 |
| 2009-01-20 to 2009-01-21 | 2 | 5.9 | -0.7 | -7.7 | +2.4 |
| 2009-02-06 to 2009-02-06 | 1 | 5.2 | +0.9 | +8.2 | -2.6 |
| 2009-02-17 to 2009-02-17 | 1 | 6.9 | +0.9 | -10.4 | +7.3 |
| 2009-03-03 to 2009-03-23 | 4 | 6.9 | +7.8 | +18.1 | -14.7 |
| 2011-08-08 to 2011-08-08 | 1 | 6.9 | +3.4 | -7.5 | +10.6 |
| 2013-04-15 to 2013-04-15 | 1 | 7.8 | -9.9 | +7.7 | -7.5 |
| 2013-06-20 to 2013-06-20 | 1 | 5.4 | -7.1 | +4.0 | -4.1 |
| 2016-06-24 to 2016-06-24 | 1 | 5.7 | +2.5 | -7.0 | +8.2 |
| 2020-03-09 to 2020-03-26 | 9 | 9.4 | -4.8 | -14.1 | +11.0 |
| 2020-04-06 to 2020-04-06 | 1 | 5.4 | +2.5 | -1.5 | -4.2 |
| 2020-06-11 to 2020-06-11 | 1 | 5.4 | +0.3 | -3.8 | +7.2 |
| 2022-11-10 to 2022-11-10 | 1 | 6.0 | +4.5 | -0.9 | -3.1 |

### Low-Dimensionality Periods (system nearly 1-D)

- 2003-07-25 to 2003-11-17: eff.dim=1.47
- 2013-10-23 to 2014-09-29: eff.dim=1.40

### High-Dimensionality Periods (system nearly 3-D)

- 2010-06-11 to 2010-07-02: eff.dim=2.50
- 2010-08-12 to 2010-08-16: eff.dim=2.48
- 2018-11-14 to 2019-01-03: eff.dim=2.54
- 2022-10-05 to 2023-10-18: eff.dim=2.83

## 6. Summary of Key Findings

- **Stationarity**: e1, e2, e3 are non-stationary in levels (unit root not rejected at 5%). All series have stationary returns.

- **e1 strong trend**: +2094% total change over sample period.

- **e2 strong trend**: -64% total change over sample period.

- **e3 strong trend**: +317% total change over sample period.

- **Dominant common factor**: PC1 explains 62% of return variance, suggesting a strong common driver.

- **No pairwise cointegration detected** at 5% level.

- **Most anomalous periods** (by Mahalanobis distance): 2025-07-31–2025-07-31 (d=15.9); 2008-09-17–2008-12-16 (d=9.9); 2026-01-30–2026-01-30 (d=9.6)

- **Time-varying dimensionality**: effective dimensionality fluctuates (mean=2.02, std=0.31), indicating the system's internal structure is not stable over time.

---
*Analysis performed blind — series identities unknown to analyst.*