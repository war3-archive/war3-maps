import { useMemo } from "react";
import type { MapMetadata } from "../../lib/wasm";
import { SearchTable } from "../ui/SearchTable";

export function StringsPanel({ data }: { data: MapMetadata }) {
  const entries = data.strings ?? [];
  const rows = useMemo(
    () => entries.map((e) => [String(e.id), e.value]),
    [entries],
  );

  if (!entries.length) return <div className="empty">No war3map.wts string table</div>;

  return (
    <SearchTable
      title="String table"
      headers={["ID", "Value"]}
      rows={rows}
      placeholder="Filter strings…"
    />
  );
}
