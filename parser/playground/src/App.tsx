import { useCallback, useEffect, useMemo, useState } from "react";
import { DropZone } from "./components/DropZone";
import { OverviewPanel } from "./components/panels/OverviewPanel";
import { TeamsPanel } from "./components/panels/TeamsPanel";
import { TechPanel, techEntryCount } from "./components/panels/TechPanel";
import { ImagesPanel } from "./components/panels/ImagesPanel";
import { StringsPanel } from "./components/panels/StringsPanel";
import { ImportsPanel } from "./components/panels/ImportsPanel";
import { FilesPanel } from "./components/panels/FilesPanel";
import { JsonPanel } from "./components/panels/JsonPanel";
import { TabNav, type TabDef, type TabId } from "./components/TabNav";
import { formatBytes, w3iEraName } from "./lib/format";
import { ensureWasm, parseMap, type MapMetadata } from "./lib/wasm";

type StatusKind = "" | "ok" | "err" | "busy";

interface LoadedMap {
  data: MapMetadata;
  file: { name: string; size: number };
}

export default function App() {
  const [version, setVersion] = useState("…");
  const [wasmReady, setWasmReady] = useState(false);
  const [status, setStatus] = useState("Loading WASM…");
  const [statusKind, setStatusKind] = useState<StatusKind>("busy");
  const [current, setCurrent] = useState<LoadedMap | null>(null);
  const [activeTab, setActiveTab] = useState<TabId>("overview");

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const ver = await ensureWasm();
        if (cancelled) return;
        setVersion(`v${ver}`);
        setWasmReady(true);
        setStatus("Ready — drop a .w3x / .w3m map to inspect.");
        setStatusKind("");
      } catch (e) {
        console.error(e);
        if (cancelled) return;
        setVersion("WASM error");
        setStatus(
          "Failed to load WASM. Run `just build-wasm` from the repo root, then restart the playground.",
        );
        setStatusKind("err");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const handleFile = useCallback(
    async (file: File) => {
      if (!wasmReady) return;
      setStatus(`Reading ${file.name} (${formatBytes(file.size)})…`);
      setStatusKind("busy");
      setCurrent(null);
      try {
        if (file.size > 40 * 1024 * 1024) {
          setStatus(
            `Warning: ${formatBytes(file.size)} is large — parse may be slow or memory-heavy…`,
          );
          setStatusKind("busy");
        }
        const buf = new Uint8Array(await file.arrayBuffer());
        setStatus(`Parsing ${file.name}…`);
        setStatusKind("busy");
        const data = parseMap(buf);
        if (!data) {
          setStatus(`Failed to parse “${file.name}” — not a readable War3 map/MPQ.`);
          setStatusKind("err");
          return;
        }
        setCurrent({ data, file: { name: file.name, size: file.size } });
        setActiveTab("overview");
        const info = data.map_info;
        setStatus(
          `OK · ${file.name} · ${formatBytes(file.size)} · ${Math.round(data.parse_ms ?? 0)} ms · w3i v${
            info?.version ?? "?"
          }${info ? ` (${w3iEraName(info.version)})` : ""} · ${info?.players?.length ?? 0} players`,
        );
        setStatusKind("ok");
      } catch (e) {
        console.error(e);
        setStatus(`Error: ${e instanceof Error ? e.message : String(e)}`);
        setStatusKind("err");
      }
    },
    [wasmReady],
  );

  const tabs: TabDef[] = useMemo(() => {
    const data = current?.data;
    if (!data) return [];
    return [
      { id: "overview", label: "Overview" },
      {
        id: "players",
        label: "Teams",
        count: data.map_info?.forces?.length || data.map_info?.players?.length,
      },
      { id: "tech", label: "Tech", count: techEntryCount(data) || undefined },
      { id: "images", label: "Images", count: data.images?.length },
      { id: "strings", label: "Strings", count: data.strings?.length },
      { id: "imports", label: "Imports", count: data.imports?.length },
      { id: "files", label: "Files", count: data.files?.length },
      { id: "json", label: "Raw JSON" },
    ];
  }, [current]);

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            ⚔
          </span>
          <div>
            <h1>
              war3parser <span>playground</span>
            </h1>
            <p className="tagline">Warcraft III map inspector · fully local WASM</p>
          </div>
        </div>
        <div className="topbar-actions">
          <span className="badge">{version}</span>
          <a
            className="btn btn-ghost"
            href="https://github.com/wesleyel/war3parser"
            target="_blank"
            rel="noreferrer"
          >
            GitHub
          </a>
        </div>
      </header>

      <DropZone disabled={!wasmReady} onFile={(f) => void handleFile(f)} />

      <div className={`status ${statusKind}`.trim()} role="status">
        {status}
      </div>

      {current ? (
        <main className="workspace">
          <TabNav tabs={tabs} active={activeTab} onChange={setActiveTab} />
          <section className="panel" role="tabpanel">
            {activeTab === "overview" && (
              <OverviewPanel data={current.data} file={current.file} />
            )}
            {activeTab === "players" && <TeamsPanel data={current.data} />}
            {activeTab === "tech" && <TechPanel data={current.data} />}
            {activeTab === "images" && <ImagesPanel data={current.data} />}
            {activeTab === "strings" && <StringsPanel data={current.data} />}
            {activeTab === "imports" && <ImportsPanel data={current.data} />}
            {activeTab === "files" && <FilesPanel data={current.data} />}
            {activeTab === "json" && <JsonPanel data={current.data} />}
          </section>
        </main>
      ) : null}

      <footer className="footer">
        Supports RoC beta → WC3 2.0 (w3i v8–33) · HM3W + pure MPQ · WTS / IMP / MMP / images
        <br />
        <span className="faint">
          Real World Materials UI · nothing leaves your browser ·{" "}
          <code>just serve-playground</code>
        </span>
      </footer>
    </div>
  );
}
