## DevSkim security gate

- **GitHub Security tab:** unavailable under the current repository entitlement. The file below is an ordinary Actions artifact, **not** GitHub Code scanning.
- **Artifact:** `devskim-sarif` (upload outcome: `success`; retention: 14 days)
- **Scanner outcome:** `success`
- **DevSkim tool name:** `devskim` (expected: `devskim`)
- **DevSkim tool version:** `1.0.90+fb2d676ce4` (expected: `1.0.90+fb2d676ce4`)
- **SARIF findings:** `94377`
- **Reviewed false-positive results:** `2910` (exact identity and immutable file binding)
- **Baseline:** `.github/devskim-baseline.json`
- **Fixed since baseline: 34 finding(s).**
- **Gate conclusion:** PASS — scanner and artifact upload succeeded, SARIF/tool contract matched, and no findings beyond the baseline.
