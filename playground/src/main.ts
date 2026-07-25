import { ensureWasm, parseMap, type MapMetadata } from "./lib/wasm";
import { formatBytes } from "./lib/format";
import { renderTab, tabDefs, type FileContext, type TabId } from "./ui/render";

const dropzone = document.querySelector<HTMLElement>("#dropzone")!;
const fileInput = document.querySelector<HTMLInputElement>("#file-input")!;
const browseBtn = document.querySelector<HTMLButtonElement>("#browse-btn")!;
const statusEl = document.querySelector<HTMLElement>("#status")!;
const workspace = document.querySelector<HTMLElement>("#workspace")!;
const tabsEl = document.querySelector<HTMLElement>("#tabs")!;
const panelEl = document.querySelector<HTMLElement>("#panel")!;
const versionEl = document.querySelector<HTMLElement>("#pkg-version")!;

let current: { data: MapMetadata; file: FileContext } | null = null;
let activeTab: TabId = "overview";

function setStatus(text: string, kind: "" | "ok" | "err" = "") {
  statusEl.textContent = text;
  statusEl.classList.remove("ok", "err");
  if (kind) statusEl.classList.add(kind);
}

function openFilePicker() {
  fileInput.click();
}

async function handleFile(file: File) {
  setStatus(`Reading ${file.name} (${formatBytes(file.size)})…`);
  workspace.classList.add("hidden");
  try {
    if (file.size > 40 * 1024 * 1024) {
      setStatus(
        `Warning: ${formatBytes(file.size)} is large — parse may be slow or memory-heavy…`,
      );
    }
    const buf = new Uint8Array(await file.arrayBuffer());
    setStatus(`Parsing ${file.name}…`);
    const data = parseMap(buf);
    if (!data) {
      setStatus(`Failed to parse “${file.name}” — not a readable War3 map/MPQ.`, "err");
      return;
    }
    current = {
      data,
      file: { name: file.name, size: file.size },
    };
    activeTab = "overview";
    paintWorkspace();
    const info = data.map_info;
    setStatus(
      `OK · ${file.name} · ${formatBytes(file.size)} · ${Math.round(data.parse_ms ?? 0)} ms · w3i v${
        info?.version ?? "?"
      } · ${info?.players?.length ?? 0} players`,
      "ok",
    );
  } catch (e) {
    console.error(e);
    setStatus(`Error: ${e instanceof Error ? e.message : String(e)}`, "err");
  }
}

function paintWorkspace() {
  if (!current) return;
  workspace.classList.remove("hidden");
  const defs = tabDefs(current.data);
  tabsEl.innerHTML = "";
  for (const t of defs) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = `tab${t.id === activeTab ? " active" : ""}`;
    btn.setAttribute("role", "tab");
    btn.setAttribute("aria-selected", String(t.id === activeTab));
    btn.innerHTML = `${t.label}${
      t.count != null ? `<span class="count">${t.count}</span>` : ""
    }`;
    btn.addEventListener("click", () => {
      activeTab = t.id;
      paintWorkspace();
    });
    tabsEl.append(btn);
  }
  renderTab(panelEl, activeTab, current.data, current.file);
}

function wireDropzone() {
  const onDrag = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };
  dropzone.addEventListener("dragenter", (e) => {
    onDrag(e);
    dropzone.classList.add("dragover");
  });
  dropzone.addEventListener("dragover", (e) => {
    onDrag(e);
    dropzone.classList.add("dragover");
  });
  dropzone.addEventListener("dragleave", (e) => {
    onDrag(e);
    dropzone.classList.remove("dragover");
  });
  dropzone.addEventListener("drop", (e) => {
    onDrag(e);
    dropzone.classList.remove("dragover");
    const file = e.dataTransfer?.files?.[0];
    if (file) void handleFile(file);
  });
  dropzone.addEventListener("click", (e) => {
    if ((e.target as HTMLElement).closest("button")) return;
    openFilePicker();
  });
  dropzone.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      openFilePicker();
    }
  });
  browseBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    openFilePicker();
  });
  fileInput.addEventListener("change", () => {
    const file = fileInput.files?.[0];
    if (file) void handleFile(file);
    fileInput.value = "";
  });
}

async function main() {
  wireDropzone();
  try {
    const ver = await ensureWasm();
    versionEl.textContent = `v${ver}`;
    setStatus("Ready — drop a .w3x / .w3m map to inspect.");
  } catch (e) {
    console.error(e);
    versionEl.textContent = "WASM error";
    setStatus(
      "Failed to load WASM. Run `just build-wasm` from the repo root, then restart the playground.",
      "err",
    );
  }
}

void main();
