import { useMemo } from "react";
import { extensionOf } from "../../lib/format";
import type { MapMetadata } from "../../lib/wasm";
import { Chip } from "../ui/Chip";
import { SearchTable } from "../ui/SearchTable";

export function FilesPanel({ data }: { data: MapMetadata }) {
  const files = data.files ?? [];
  const rows = useMemo(
    () => files.map((f) => [f, extensionOf(f) || ""]),
    [files],
  );

  const extChips = useMemo(() => {
    const extCount = new Map<string, number>();
    for (const f of files) {
      const ext = extensionOf(f) || "(none)";
      extCount.set(ext, (extCount.get(ext) ?? 0) + 1);
    }
    return [...extCount.entries()].sort((a, b) => b[1] - a[1]).slice(0, 12);
  }, [files]);

  if (!files.length) {
    return (
      <div className="empty">
        (listfile) missing — archive may still open known names like war3map.w3i
      </div>
    );
  }

  return (
    <div className="panel-stack">
      <div className="chip-row">
        {extChips.map(([ext, n]) => (
          <Chip key={ext}>
            {ext} · {n}
          </Chip>
        ))}
      </div>
      <SearchTable
        title="Listfile"
        headers={["Path", "Ext"]}
        rows={rows}
        placeholder="Filter files…"
      />
    </div>
  );
}
