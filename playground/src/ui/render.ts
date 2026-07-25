import type { MapMetadata } from "../lib/wasm";
import {
  controllerLabel,
  decodeFlags,
  extensionOf,
  formatBuild,
  formatBytes,
  gameDataVersionName,
  graphicsModeName,
  playersInForce,
  raceIcon,
  raceName,
  scriptModeName,
  slotColor,
  stripColorCodes,
  tilesetName,
} from "../lib/format";
import {
  buildOverlayIcons,
  countByType,
  iconTypeLabel,
  isMinimapImage,
  paintMinimapCover,
} from "../lib/minimap";

export type TabId =
  | "overview"
  | "players"
  | "images"
  | "strings"
  | "imports"
  | "files"
  | "json";

export interface FileContext {
  name: string;
  size: number;
}

export function tabDefs(data: MapMetadata): Array<{ id: TabId; label: string; count?: number }> {
  return [
    { id: "overview", label: "Overview" },
    {
      id: "players",
      label: "Teams",
      count: data.map_info?.forces?.length || data.map_info?.players?.length,
    },
    { id: "images", label: "Images", count: data.images?.length },
    { id: "strings", label: "Strings", count: data.strings?.length },
    { id: "imports", label: "Imports", count: data.imports?.length },
    { id: "files", label: "Files", count: data.files?.length },
    { id: "json", label: "Raw JSON" },
  ];
}

export function renderTab(
  root: HTMLElement,
  id: TabId,
  data: MapMetadata,
  file: FileContext,
): void {
  root.innerHTML = "";
  switch (id) {
    case "overview":
      root.append(renderOverview(data, file));
      break;
    case "players":
      root.append(renderPlayers(data));
      break;
    case "images":
      root.append(renderImages(data));
      break;
    case "strings":
      root.append(renderStrings(data));
      break;
    case "imports":
      root.append(renderImports(data));
      break;
    case "files":
      root.append(renderFiles(data));
      break;
    case "json":
      root.append(renderJson(data));
      break;
  }
}

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className?: string,
  html?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (html !== undefined) node.innerHTML = html;
  return node;
}

function card(title: string, body: HTMLElement, badge?: string): HTMLElement {
  const c = el("div", "card");
  const head = el("div", "section-title");
  head.append(el("h2", undefined, title));
  if (badge) head.append(el("span", "chip", badge));
  c.append(head, body);
  return c;
}

function kv(rows: Array<[string, string]>): HTMLElement {
  const dl = el("dl", "kv");
  for (const [k, v] of rows) {
    dl.append(el("dt", undefined, k), el("dd", "mono", escapeHtml(v)));
  }
  return dl;
}

function escapeHtml(s: string): string {
  return s
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function renderOverview(data: MapMetadata, file: FileContext): HTMLElement {
  const info = data.map_info;
  const wrap = el("div");
  const hero = el("div", "card hero-card");
  const name = stripColorCodes(info?.name) || stripColorCodes(data.header.name) || file.name;

  const heroTop = el("div", "hero-layout");
  const heroText = el("div", "hero-text");
  heroText.append(el("div", "hero-name", escapeHtml(name)));
  const chips = el("div", "flags");
  chips.append(el("span", "chip ok", `w3i v${info?.version ?? "?"}`));
  chips.append(el("span", "chip", formatBytes(file.size)));
  chips.append(el("span", "chip", `${Math.round(data.parse_ms)} ms`));
  chips.append(
    el(
      "span",
      data.header.has_hm3w ? "chip ok" : "chip warn",
      data.header.has_hm3w ? "HM3W header" : "No HM3W (pure MPQ)",
    ),
  );
  if (info?.skipped_optional_sections) {
    chips.append(el("span", "chip warn", "Optional sections skipped (0xFF)"));
  }
  heroText.append(chips);
  if (info?.description) {
    heroText.append(el("p", "desc", escapeHtml(stripColorCodes(info.description))));
  }
  heroTop.append(heroText);

  // Cover minimap with gold mines / buildings / player starts
  const cover = buildCoverBlock(data);
  if (cover) heroTop.append(cover);
  hero.append(heroTop);
  wrap.append(hero);

  const grid = el("div", "grid-2");
  grid.append(
    card(
      "Map info",
      kv([
        ["Author", stripColorCodes(info?.author) || "—"],
        ["Recommended", stripColorCodes(info?.recommended_players) || "—"],
        ["Tileset", tilesetName(info?.tileset)],
        [
          "Playable size",
          info?.playable_size
            ? `${info.playable_size[0]} × ${info.playable_size[1]}`
            : "—",
        ],
        ["Saves", String(info?.saves ?? "—")],
        ["Editor ver", String(info?.editor_version ?? "—")],
        ["Build", formatBuild(info?.build_version as number[] | undefined)],
        ["Script", scriptModeName(info?.script_mode)],
        ["Graphics", graphicsModeName(info?.graphics_mode)],
        ["Game data", gameDataVersionName(info?.game_data_version)],
      ]),
    ),
  );

  const icons = buildOverlayIcons(data);
  const counts = countByType(icons);
  grid.append(
    card(
      "Container",
      kv([
        ["File", file.name],
        ["HM3W name", stripColorCodes(data.header.name) || "—"],
        ["Max players", String(data.header.max_players ?? "—")],
        ["Header flags", data.header.flags != null ? `0x${data.header.flags.toString(16)}` : "—"],
        ["Players", String(info?.players?.length ?? 0)],
        ["Forces", String(info?.forces?.length ?? 0)],
        ["Minimap icons", String(icons.length)],
        ["Gold mines", String(counts["Gold mine"] ?? 0)],
        ["Buildings", String(counts.Building ?? 0)],
        ["Start locs", String(counts["Player start"] ?? 0)],
        ["Strings", String(data.strings?.length ?? 0)],
        ["Imports", String(data.imports?.length ?? 0)],
        ["Listfile", data.files ? `${data.files.length} files` : "missing"],
      ]),
    ),
  );
  wrap.append(grid);

  const flags = decodeFlags(info?.flags);
  const flagCard = card(
    "Map flags",
    flags.length
      ? (() => {
          const box = el("div", "flags");
          for (const f of flags) box.append(el("span", "chip", escapeHtml(f)));
          return box;
        })()
      : el("div", "empty", "No flags decoded"),
    info?.flags != null ? `0x${(info.flags >>> 0).toString(16)}` : undefined,
  );
  wrap.append(el("div", undefined), flagCard);
  flagCard.style.marginTop = "0.85rem";

  if (info && (info.loading_screen_title || info.prologue_screen_title)) {
    const screens = el("div", "grid-2");
    screens.style.marginTop = "0.85rem";
    screens.append(
      card(
        "Loading screen",
        kv([
          ["Title", stripColorCodes(info.loading_screen_title) || "—"],
          ["Subtitle", stripColorCodes(info.loading_screen_subtitle) || "—"],
          ["Text", stripColorCodes(info.loading_screen_text) || "—"],
          ["Model", info.loading_screen_model || "—"],
        ]),
      ),
      card(
        "Prologue",
        kv([
          ["Title", stripColorCodes(info.prologue_screen_title) || "—"],
          ["Subtitle", stripColorCodes(info.prologue_screen_subtitle) || "—"],
          ["Text", stripColorCodes(info.prologue_screen_text) || "—"],
          ["Model", info.prologue_screen_model || "—"],
        ]),
      ),
    );
    wrap.append(screens);
  }

  return wrap;
}

function renderPlayers(data: MapMetadata): HTMLElement {
  const info = data.map_info;
  const wrap = el("div");
  if (!info) {
    wrap.append(el("div", "empty", "No map_info / players"));
    return wrap;
  }

  const players = [...(info.players ?? [])].sort((a, b) => a.id - b.id);
  const forces = info.forces ?? [];
  const assigned = new Set<number>();

  const board = el("div", "teams-board");

  const renderTeam = (title: string, members: typeof players, subtle?: boolean) => {
    const team = el("section", subtle ? "team-block team-block-subtle" : "team-block");
    const head = el("header", "team-head");
    head.innerHTML = `<h3>${escapeHtml(title)}</h3><span class="team-count">${members.length}</span>`;
    team.append(head);

    if (!members.length) {
      team.append(el("div", "team-empty", "No players in this force"));
      board.append(team);
      return;
    }

    const list = el("div", "team-list");
    for (const p of members) {
      assigned.add(p.id);
      const row = el("div", "team-row");
      const color = slotColor(p.id);
      const name = stripColorCodes(p.name) || `Player ${p.id + 1}`;
      const race = raceName(p.race);
      const ctrl = controllerLabel(p.player_type);
      row.innerHTML = `
        <div class="team-name" style="color:${color}">
          <span class="slot-dot" style="background:${color}"></span>
          <span class="team-name-text">${escapeHtml(name)}</span>
          <span class="slot-id mono">#${p.id}</span>
        </div>
        <div class="team-race">
          <span class="race-badge" title="${escapeHtml(race)}">${raceIcon(p.race)}</span>
          <span>${escapeHtml(race)}</span>
        </div>
        <div class="team-ctrl ${p.player_type === 1 ? "ctrl-user" : "ctrl-other"}">${escapeHtml(ctrl)}</div>
      `;
      list.append(row);
    }
    team.append(list);
    board.append(team);
  };

  if (forces.length) {
    forces.forEach((f, i) => {
      const title = stripColorCodes(f.name) || `Force ${i + 1}`;
      renderTeam(title, playersInForce(f.player_masks >>> 0, players));
    });
    const orphans = players.filter((p) => !assigned.has(p.id));
    if (orphans.length) renderTeam("Unassigned", orphans, true);
  } else {
    renderTeam("All players", players);
  }

  wrap.append(board);

  // Compact technical details (collapsed-looking secondary card)
  const details = el("details", "team-details card");
  details.innerHTML = `<summary>Slot details <span class="muted">start locations & flags</span></summary>`;
  const table = el("div", "table-wrap");
  const t = el("table", "data");
  t.innerHTML = `<thead><tr>
    <th>ID</th><th>Name</th><th>Type</th><th>Race</th><th>Fixed</th><th>Start</th>
  </tr></thead>`;
  const tb = el("tbody");
  for (const p of players) {
    const tr = el("tr");
    const color = slotColor(p.id);
    tr.innerHTML = `
      <td class="mono">${p.id}</td>
      <td style="color:${color}">${escapeHtml(stripColorCodes(p.name))}</td>
      <td>${escapeHtml(controllerLabel(p.player_type))}</td>
      <td>${escapeHtml(raceName(p.race))}</td>
      <td>${p.is_fixed_start_position ? "yes" : "no"}</td>
      <td class="mono">${Number(p.start_location?.[0] ?? 0).toFixed(1)}, ${Number(
        p.start_location?.[1] ?? 0,
      ).toFixed(1)}</td>`;
    tb.append(tr);
  }
  t.append(tb);
  table.append(t);
  details.append(table);
  wrap.append(details);

  return wrap;
}

function buildCoverBlock(data: MapMetadata): HTMLElement | null {
  const minimap = (data.images ?? []).find((img) => isMinimapImage(img.filename));
  if (!minimap) return null;

  const icons = buildOverlayIcons(data);
  const box = el("div", "cover-minimap");
  const canvas = el("canvas", "cover-canvas") as HTMLCanvasElement;
  canvas.setAttribute("role", "img");
  canvas.setAttribute("aria-label", "Minimap with player starts and resources");
  box.append(canvas);

  const legend = el("div", "cover-legend");
  const counts = countByType(icons);
  for (const [label, n] of Object.entries(counts)) {
    const item = el("span", "legend-item");
    const swatch = el("span", `legend-swatch legend-${label.toLowerCase().replace(/\s+/g, "-")}`);
    item.append(swatch, document.createTextNode(`${label} · ${n}`));
    legend.append(item);
  }
  box.append(legend);

  // Paint async after mount
  requestAnimationFrame(() => {
    void paintMinimapCover(canvas, minimap.data_url, icons, 300).catch((err) => {
      console.warn(err);
      box.classList.add("cover-error");
    });
  });

  return box;
}

function renderImages(data: MapMetadata): HTMLElement {
  const wrap = el("div");
  const images = data.images ?? [];
  if (!images.length) {
    wrap.append(el("div", "empty", "No minimap/preview images found"));
    return wrap;
  }

  const icons = buildOverlayIcons(data);
  const grid = el("div", "image-grid");

  for (const img of images) {
    const cardEl = el("div", "image-card");
    const isMap = isMinimapImage(img.filename);

    if (isMap && icons.length) {
      const canvas = el("canvas", "cover-canvas cover-canvas-lg") as HTMLCanvasElement;
      canvas.setAttribute("aria-label", img.filename);
      cardEl.append(canvas);
      requestAnimationFrame(() => {
        void paintMinimapCover(canvas, img.data_url, icons, 420);
      });
    } else {
      const image = el("img") as HTMLImageElement;
      image.src = img.data_url;
      image.alt = img.filename;
      cardEl.append(image);
    }

    const meta = el("div", "meta");
    meta.innerHTML = `<span class="mono">${escapeHtml(img.filename)}</span>
      <span>${img.width}×${img.height}${isMap ? " · annotated" : ""}</span>`;
    const actions = el("div", "meta");
    const a = el("a", "btn ghost") as HTMLAnchorElement;
    a.href = img.data_url;
    a.download = `${img.filename.replace(/[\\/]/g, "_")}.png`;
    a.textContent = "Download PNG";
    actions.append(a);
    cardEl.append(meta, actions);
    grid.append(cardEl);
  }

  wrap.append(card("Images", grid, String(images.length)));

  if (icons.length) {
    const list = el("div", "table-wrap");
    list.style.marginTop = "0.85rem";
    const t = el("table", "data");
    t.innerHTML = `<thead><tr><th>Type</th><th>X</th><th>Y</th><th>Color</th></tr></thead>`;
    const tb = el("tbody");
    for (const ic of icons.slice(0, 200)) {
      const tr = el("tr");
      tr.innerHTML = `<td>${escapeHtml(iconTypeLabel(ic.icon_type))}</td>
        <td class="mono">${Math.round(ic.x)}</td>
        <td class="mono">${Math.round(ic.y)}</td>
        <td><span class="swatch" style="background:${ic.color}"></span> <span class="mono">${escapeHtml(ic.color)}</span></td>`;
      tb.append(tr);
    }
    t.append(tb);
    list.append(t);
    wrap.append(card("Minimap icons (war3map.mmp)", list, String(icons.length)));
  }

  return wrap;
}

function searchableTable(
  title: string,
  headers: string[],
  rows: string[][],
  placeholder: string,
): HTMLElement {
  const wrap = el("div", "card");
  const bar = el("div", "toolbar");
  bar.append(el("h2", undefined, `${title}`));
  const input = el("input", "search") as HTMLInputElement;
  input.placeholder = placeholder;
  bar.append(input);
  wrap.append(bar);

  const tableWrap = el("div", "table-wrap");
  const table = el("table", "data");
  table.innerHTML = `<thead><tr>${headers.map((h) => `<th>${h}</th>`).join("")}</tr></thead>`;
  const tbody = el("tbody");
  table.append(tbody);
  tableWrap.append(table);
  wrap.append(tableWrap);

  const count = el("div", "muted");
  count.style.marginTop = "0.5rem";
  wrap.append(count);

  const paint = () => {
    const q = input.value.trim().toLowerCase();
    tbody.innerHTML = "";
    let n = 0;
    for (const row of rows) {
      if (q && !row.some((c) => c.toLowerCase().includes(q))) continue;
      n++;
      if (n > 500) continue;
      const tr = el("tr");
      tr.innerHTML = row.map((c) => `<td class="mono">${escapeHtml(c)}</td>`).join("");
      tbody.append(tr);
    }
    count.textContent =
      n > 500 ? `Showing 500 of ${n} matches` : `${n} row${n === 1 ? "" : "s"}`;
  };
  input.addEventListener("input", paint);
  paint();
  return wrap;
}

function renderStrings(data: MapMetadata): HTMLElement {
  const entries = data.strings ?? [];
  if (!entries.length) return el("div", "empty", "No war3map.wts string table");
  return searchableTable(
    `String table`,
    ["ID", "Value"],
    entries.map((e) => [String(e.id), e.value]),
    "Filter strings…",
  );
}

function renderImports(data: MapMetadata): HTMLElement {
  const entries = data.imports ?? [];
  if (!entries.length) return el("div", "empty", "No war3map.imp imports");
  return searchableTable(
    "Imports",
    ["Path", "Flag"],
    entries.map((e) => [e.path, String(e.is_custom)]),
    "Filter imports…",
  );
}

function renderFiles(data: MapMetadata): HTMLElement {
  const files = data.files ?? [];
  if (!files.length) {
    return el(
      "div",
      "empty",
      "(listfile) missing — archive may still open known names like war3map.w3i",
    );
  }
  const extCount = new Map<string, number>();
  for (const f of files) {
    const ext = extensionOf(f) || "(none)";
    extCount.set(ext, (extCount.get(ext) ?? 0) + 1);
  }
  const wrap = el("div");
  const chips = el("div", "flags");
  chips.style.marginBottom = "0.75rem";
  for (const [ext, n] of [...extCount.entries()].sort((a, b) => b[1] - a[1]).slice(0, 12)) {
    chips.append(el("span", "chip", `${ext} · ${n}`));
  }
  wrap.append(chips);
  wrap.append(
    searchableTable(
      "Listfile",
      ["Path", "Ext"],
      files.map((f) => [f, extensionOf(f) || ""]),
      "Filter files…",
    ),
  );
  return wrap;
}

function renderJson(data: MapMetadata): HTMLElement {
  const wrap = el("div", "card");
  const bar = el("div", "toolbar");
  bar.append(el("h2", undefined, "Raw JSON"));
  const actions = el("div");
  actions.style.display = "flex";
  actions.style.gap = "0.5rem";

  const slim = {
    ...data,
    images: (data.images ?? []).map((img) => ({
      filename: img.filename,
      width: img.width,
      height: img.height,
      data_url: `…omitted ${img.data_url?.length ?? 0} chars…`,
    })),
  };
  const text = JSON.stringify(slim, null, 2);

  const copy = el("button", "btn") as HTMLButtonElement;
  copy.textContent = "Copy";
  copy.onclick = async () => {
    await navigator.clipboard.writeText(text);
    copy.textContent = "Copied";
    setTimeout(() => (copy.textContent = "Copy"), 1200);
  };

  const download = el("a", "btn ghost") as HTMLAnchorElement;
  download.textContent = "Download";
  download.href = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  download.download = "war3parser-map.json";

  actions.append(copy, download);
  bar.append(actions);
  wrap.append(bar, el("pre", "pre", escapeHtml(text)));
  return wrap;
}
