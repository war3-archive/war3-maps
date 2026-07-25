export type TabId =
  | "overview"
  | "players"
  | "images"
  | "strings"
  | "imports"
  | "files"
  | "json";

export interface TabDef {
  id: TabId;
  label: string;
  count?: number;
}

export function TabNav({
  tabs,
  active,
  onChange,
}: {
  tabs: TabDef[];
  active: TabId;
  onChange: (id: TabId) => void;
}) {
  return (
    <nav className="tab-rail" role="tablist" aria-label="Result sections">
      {tabs.map((t) => (
        <button
          key={t.id}
          type="button"
          role="tab"
          aria-selected={t.id === active}
          className={`tab${t.id === active ? " active" : ""}`}
          onClick={() => onChange(t.id)}
        >
          <span>{t.label}</span>
          {t.count != null ? <span className="count">{t.count}</span> : null}
        </button>
      ))}
    </nav>
  );
}
