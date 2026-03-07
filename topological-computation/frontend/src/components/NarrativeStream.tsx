import { T, FONT, importanceToOpacity } from "../tokens";
import type { NarrativeEvent } from "../types";

interface Props {
  events: NarrativeEvent[];
}

export function NarrativeStream({ events }: Props) {
  if (events.length === 0) {
    return (
      <div style={{
        flex: 1, display: "flex", alignItems: "center", justifyContent: "center",
        fontFamily: FONT.mono, fontSize: 11, color: T.textMuted,
      }}>
        等待 daemon 事件...
      </div>
    );
  }

  return (
    <div style={{
      flex: 1, overflow: "auto", padding: "8px 12px",
      fontFamily: FONT.mono, fontSize: 11, lineHeight: 1.7,
    }}>
      {events.map((ev, i) => (
        <div key={i} style={{
          padding: "6px 0",
          borderBottom: `1px solid ${T.border}22`,
          opacity: importanceToOpacity(ev.importance),
        }}>
          {ev.time && (
            <span style={{ color: T.textMuted, marginRight: 8 }}>{ev.time}</span>
          )}
          <span style={{
            display: "inline-block", width: 6, height: 6, borderRadius: "50%",
            background: ev.level === 3 ? T.fHigh : ev.level === 2 ? T.fMid : T.textDim,
            marginRight: 8, verticalAlign: "middle",
            flexShrink: 0,
          }} />
          <span style={{ color: ev.importance > 0.85 ? T.text : T.textDim }}>
            {ev.text}
          </span>
        </div>
      ))}
    </div>
  );
}
