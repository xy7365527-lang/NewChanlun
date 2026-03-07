import { T, FONT } from "../tokens";

interface Tab {
  id: string;
  label: string;
}

interface Props {
  tabs: Tab[];
  active: string;
  onChange: (id: string) => void;
}

export function TabSwitcher({ tabs, active, onChange }: Props) {
  return (
    <div style={{
      display: "flex", gap: 0,
      borderBottom: `1px solid ${T.border}`,
      background: T.bgPanel,
      flexShrink: 0,
    }}>
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onChange(tab.id)}
          style={{
            background: "transparent", border: "none",
            padding: "10px 20px",
            fontFamily: FONT.mono, fontSize: 11, cursor: "pointer",
            color: active === tab.id ? T.text : T.textDim,
            borderBottom: active === tab.id
              ? `2px solid ${T.accent}`
              : "2px solid transparent",
            letterSpacing: "0.05em",
            transition: "all 0.15s",
          }}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
