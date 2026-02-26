import { useAppStore, SUPPORTED_TFS } from "../store/appStore";
import { SearchDropdown } from "./SearchDropdown";
import { SymbolSelector } from "./SymbolSelector";
import { LevelSelector } from "./LevelSelector";

interface ToolbarProps {
  maxLevel?: number;
}

export function Toolbar({ maxLevel = 0 }: ToolbarProps) {
  const { symbol, tf, setTf } = useAppStore();

  return (
    <div className="toolbar">
      <span className="toolbar-symbol">{symbol}</span>
      <SymbolSelector />
      <SearchDropdown />
      <div className="toolbar-sep" />
      {SUPPORTED_TFS.map((t) => (
        <button
          key={t}
          className={`tf-btn ${t === tf ? "active" : ""}`}
          onClick={() => setTf(t)}
        >
          {t}
        </button>
      ))}
      <div className="toolbar-sep" />
      <LevelSelector maxLevel={maxLevel} />
    </div>
  );
}
