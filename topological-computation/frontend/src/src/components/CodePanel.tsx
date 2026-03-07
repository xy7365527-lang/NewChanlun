import { T, FONT } from "../tokens";

export function CodePanel() {
  return (
    <div style={{
      flex: 1, padding: 16, fontFamily: FONT.mono, fontSize: 11,
      color: T.textDim, overflow: "auto",
    }}>
      <div style={{ color: T.text, marginBottom: 12, fontSize: 12 }}>Code Topology</div>
      <div style={{ color: T.textMuted, marginBottom: 16, lineHeight: 1.6 }}>
        daemon 的代码拓扑视图。显示 AST 结构、f 值标注、重构候选、承重函数。
      </div>
      <div style={{
        background: T.bgCard, border: `1px solid ${T.border}`,
        borderRadius: 6, padding: 12, marginBottom: 8,
      }}>
        <span style={{ color: T.fLow }}>■</span> engine.py —{" "}
        <span style={{ color: T.textDim }}>14 functions, 3 承重 (settled)</span>
      </div>
      <div style={{
        background: T.bgCard, border: `1px solid ${T.border}`,
        borderRadius: 6, padding: 12, marginBottom: 8,
      }}>
        <span style={{ color: T.fMid }}>■</span> traversal.py —{" "}
        <span style={{ color: T.textDim }}>8 functions, f(traverse_step, engine_step)=2.1 → fold候选</span>
      </div>
      <div style={{
        background: T.bgCard, border: `1px solid ${T.border}`,
        borderRadius: 6, padding: 12, marginBottom: 8,
      }}>
        <span style={{ color: T.fHigh }}>■</span> auto_feed.py —{" "}
        <span style={{ color: T.textDim }}>negation detected: connection_check vs quality_check (redundant logic)</span>
      </div>
      <div style={{
        background: T.bg, border: `1px solid ${T.border}`,
        borderRadius: 6, padding: 12, marginTop: 20,
        fontFamily: FONT.mono, fontSize: 10, color: T.textMuted,
        height: 120, display: "flex", alignItems: "center", justifyContent: "center",
      }}>
        $ terminal — execution callback output
      </div>
    </div>
  );
}
