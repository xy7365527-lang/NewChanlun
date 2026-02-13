import { useAppStore, SUPPORTED_TFS } from "../store/appStore";
import { SearchDropdown } from "./SearchDropdown";

export function Toolbar() {
  const { symbol, tf, setTf } = useAppStore();

  return (
    <div className="toolbar">
      <span className="toolbar-symbol">{symbol}</span>
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
    </div>
  );
}
