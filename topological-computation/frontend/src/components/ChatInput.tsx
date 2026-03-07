import { useState, useCallback } from "react";
import { T, FONT } from "../tokens";

interface Props {
  onSend: (text: string) => void;
  disabled?: boolean;
}

export function ChatInput({ onSend, disabled }: Props) {
  const [value, setValue] = useState("");

  const handleSubmit = useCallback(() => {
    const trimmed = value.trim();
    if (!trimmed || disabled) return;
    onSend(trimmed);
    setValue("");
  }, [value, onSend, disabled]);

  const handleKey = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter") handleSubmit();
    },
    [handleSubmit]
  );

  return (
    <div style={{
      display: "flex", gap: 8, padding: "8px 12px",
      borderTop: `1px solid ${T.border}`,
      flexShrink: 0,
    }}>
      <input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={handleKey}
        disabled={disabled}
        placeholder="查询概念、定向穿越、注入文本..."
        style={{
          flex: 1, background: T.bgCard, border: `1px solid ${T.border}`,
          borderRadius: 6, padding: "8px 12px",
          fontFamily: FONT.mono, fontSize: 12, color: T.text,
          outline: "none",
          opacity: disabled ? 0.5 : 1,
        }}
      />
      <button
        onClick={handleSubmit}
        disabled={disabled}
        style={{
          background: T.accent, border: "none", borderRadius: 6,
          padding: "8px 16px", color: "#fff",
          fontFamily: FONT.mono, fontSize: 11, cursor: disabled ? "not-allowed" : "pointer",
          letterSpacing: "0.05em",
          opacity: disabled ? 0.5 : 1,
        }}
      >
        穿越
      </button>
    </div>
  );
}
