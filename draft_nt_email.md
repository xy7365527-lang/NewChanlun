Subject: NautilusTrader Pro — Early Access & IBKR Integration

Hi,

I'm an individual quantitative trader interested in NautilusTrader Pro early access.

I'm building a systematic futures + crypto trading system (CL, BRN, ES, BTC, GC, DX) with a custom Rust signal engine interfaced via PyO3. I'm running streaming backtests on 1-minute bars (~4.6M bars per asset) and plan to go live via Interactive Brokers. NautilusTrader's research-to-live parity and Rust-native architecture are exactly what I need.

I see Pro is listed as "coming soon" on the website. A few questions:

1. Is there a timeline or waitlist for Pro early access? I'd like to sign up.
2. For the IBKR adapter — is it production-ready in the current open-source release, or is it part of the Pro/Institutional tier?
3. My signal engine is a standalone Rust crate that emits buy/sell signals per bar. What's the recommended integration pattern — wrapping it as a custom Strategy in Python via PyO3, or is there a Rust-native strategy interface?
4. For the Pro Execution Engine (VWAP/Iceberg/POV) — does it support futures on IBKR, or is it currently crypto-only?

I'm already using the open-source version for research. Happy to share more about my use case if helpful.

Best regards,
Junyu
maiaedneide133@gmail.com
