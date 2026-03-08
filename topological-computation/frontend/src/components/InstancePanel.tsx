import { T, FONT } from "../tokens";
import type { InstanceConfig, InstanceState } from "../types";

interface Props {
  instances: InstanceConfig[];
  states: Map<string, InstanceState>;
  onToggleVisibility: (instanceId: string) => void;
}

export function InstancePanel({ instances, states, onToggleVisibility }: Props) {
  return (
    <div style={{
      display: "flex",
      gap: 8,
      padding: "6px 12px",
      borderTop: `1px solid ${T.border}`,
      background: T.bgPanel,
      fontFamily: FONT.mono,
      fontSize: 10,
      flexShrink: 0,
      overflowX: "auto",
    }}>
      {instances.map((inst) => {
        const state = states.get(inst.id);
        const connected = state?.connected ?? false;
        const visible = state?.visible ?? true;

        return (
          <button
            key={inst.id}
            onClick={() => onToggleVisibility(inst.id)}
            title={`${inst.name} (${inst.wsUrl})\n${connected ? "在线" : "离线"} | ${visible ? "显示中" : "已隐藏"}`}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              padding: "4px 10px",
              background: visible ? T.bgCard : "transparent",
              border: `1px solid ${visible ? inst.color + "66" : T.border}`,
              borderRadius: 4,
              cursor: "pointer",
              opacity: visible ? 1 : 0.5,
              transition: "all 0.15s",
            }}
          >
            {/* Connection indicator */}
            <span style={{
              width: 6,
              height: 6,
              borderRadius: "50%",
              background: connected ? inst.color : T.textMuted,
              boxShadow: connected ? `0 0 6px ${inst.color}66` : "none",
              flexShrink: 0,
            }} />

            {/* Instance name */}
            <span style={{
              color: connected ? T.text : T.textMuted,
              whiteSpace: "nowrap",
            }}>
              {inst.name}
            </span>

            {/* Stats */}
            {connected && state && (
              <span style={{ color: T.textDim, whiteSpace: "nowrap" }}>
                S{state.steps} / {state.settled}s
              </span>
            )}

            {!connected && (
              <span style={{ color: T.textMuted, whiteSpace: "nowrap" }}>
                offline
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}
