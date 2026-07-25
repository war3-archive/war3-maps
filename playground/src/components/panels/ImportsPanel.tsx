import { useMemo } from "react";
import type { MapMetadata } from "../../lib/wasm";
import { SearchTable } from "../ui/SearchTable";

export function ImportsPanel({ data }: { data: MapMetadata }) {
  const entries = data.imports ?? [];
  const rows = useMemo(
    () => entries.map((e) => [e.path, String(e.is_custom)]),
    [entries],
  );

  if (!entries.length) return <div className="empty">No war3map.imp imports</div>;

  return (
    <SearchTable
      title="Imports"
      headers={["Path", "Flag"]}
      rows={rows}
      placeholder="Filter imports…"
    />
  );
}
