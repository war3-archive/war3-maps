/**
 * The map inspector behind /parse.
 *
 * Everything runs in the browser: the file never leaves the machine. This is
 * the Astro rewrite of the standalone React playground that used to live in its
 * own repository — same panels, same `war3parser` WASM build the map dialog
 * uses, rendered with plain DOM so the site keeps a single toolchain.
 */

import {
  availabilityName,
  bgraToCss,
  controllerLabel,
  decodeFlags,
  extensionOf,
  formatBuild,
  formatBytes,
  fourCC,
  gameDataSetName,
  gameDataVersionName,
  graphicsModeName,
  importFlagLabel,
  playerMaskLabel,
  playersInForce,
  raceName,
  scriptModeName,
  slotColor,
  stripColorCodes,
  tilesetName,
  w3iEraName,
  weatherName,
} from "./parse/format";
import { buildOverlayIcons, isMinimapImage, paintMinimapCover } from "./parse/minimap";
import { ensureWasm, parseMap, type MapMetadata } from "./parse/wasm";

const $ = <T extends HTMLElement>(sel: string) => document.querySelector(sel) as T;

const drop = $<HTMLElement>("#drop");
const input = $<HTMLInputElement>("#file");
const status = $<HTMLElement>("#status");
const secnav = $<HTMLElement>("#secnav");
const sections = $<HTMLElement>("#sections");

let ready = false;
let watching = false;

function setStatus(text: string, kind: "" | "ok" | "err" = "") {
  status.textContent = text;
  status.className = `drop-status${kind ? ` ${kind}` : ""}`;
}

// ---------------------------------------------------------------- DOM helpers

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className?: string | null,
  text?: unknown,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null && text !== "") node.textContent = String(text);
  return node;
}

function card(title: string, ...body: Node[]): HTMLElement {
  const box = el("div", "pcard");
  box.append(el("h3", null, title), ...body);
  return box;
}

/** Definition list, skipping rows with nothing to say. */
function kv(pairs: Array<[string, unknown]>): HTMLElement {
  const list = el("dl", "kv");
  for (const [term, value] of pairs) {
    if (value === null || value === undefined || value === "" || value === "—") continue;
    const dd = el("dd");
    if (value instanceof Node) dd.append(value);
    else dd.textContent = String(value);
    list.append(el("dt", null, term), dd);
  }
  return list;
}

function chips(labels: string[]): HTMLElement {
  const box = el("div", "chips");
  for (const label of labels) box.append(el("span", "chip", label));
  return box;
}

/** A table; `rows` cells may be strings or nodes. */
function table(headers: string[], rows: Array<Array<string | Node>>, tall = false): HTMLElement {
  const wrap = el("div", tall ? "scroll tall" : "scroll");
  const grid = el("table", "grid");
  const thead = el("thead");
  const hrow = el("tr");
  for (const h of headers) hrow.append(el("th", null, h));
  thead.append(hrow);
  const tbody = el("tbody");
  for (const row of rows) {
    const tr = el("tr");
    for (const cell of row) {
      const td = el("td");
      if (cell instanceof Node) td.append(cell);
      else td.textContent = cell;
      tr.append(td);
    }
    tbody.append(tr);
  }
  grid.append(thead, tbody);
  wrap.append(grid);
  return wrap;
}

/** A table with a filter box above it, for the long lists. */
function filterable(
  placeholder: string,
  headers: string[],
  rows: Array<{ keys: string; cells: Array<string | Node> }>,
): HTMLElement {
  const box = el("div");
  const search = el("input", "filter") as HTMLInputElement;
  search.type = "search";
  search.placeholder = placeholder;
  const host = el("div");
  const render = (needle: string) => {
    const wanted = needle.trim().toLowerCase();
    host.replaceChildren(
      table(
        headers,
        rows.filter((r) => !wanted || r.keys.includes(wanted)).map((r) => r.cells),
        true,
      ),
    );
  };
  search.addEventListener("input", () => render(search.value));
  render("");
  box.append(search, host);
  return box;
}

function swatch(color: string, label: string): HTMLElement {
  const span = el("span", "chip");
  const dot = el("span", "swatch");
  dot.style.background = color;
  span.append(dot, document.createTextNode(label));
  return span;
}

// ------------------------------------------------------------------- panels

function panelFor(id: string): HTMLElement {
  return sections.querySelector(`[data-panel="${id}"]`) as HTMLElement;
}

function setCount(id: string, count: number | string | null) {
  const node = secnav.querySelector(`[data-count="${id}"]`) as HTMLElement;
  node.textContent = count === null || count === "" ? "" : `(${count})`;
}

function renderOverview(data: MapMetadata, file: { name: string; size: number }) {
  const host = panelFor("overview");
  const info = data.map_info;
  const header = data.header;
  const parts: Node[] = [];

  // The minimap gets the mmp icons drawn over it, which is the one view the
  // catalog cannot show.
  const minimap = data.images?.find((i) => isMinimapImage(i.filename));
  if (minimap) {
    const canvas = el("canvas");
    const figure = el("figure", "shot");
    figure.append(canvas, el("figcaption", null, "小地图 + war3map.mmp 图标"));
    parts.push(card("小地图", figure));
    void paintMinimapCover(canvas, minimap.data_url, buildOverlayIcons(data)).catch((e) =>
      console.error(e),
    );
  }

  parts.push(
    card(
      "文件",
      kv([
        ["文件名", file.name],
        ["大小", formatBytes(file.size)],
        ["解析耗时", data.parse_ms == null ? null : `${Math.round(data.parse_ms)} ms`],
        ["HM3W 头", header.has_hm3w ? "有" : "无"],
        ["HM3W 标题", stripColorCodes(header.name)],
        ["HM3W 玩家上限", header.max_players],
      ]),
    ),
  );

  if (info) {
    parts.push(
      card(
        "地图信息",
        kv([
          ["名称", stripColorCodes(info.name)],
          ["作者", stripColorCodes(info.author)],
          ["建议人数", stripColorCodes(info.recommended_players)],
          ["简介", stripColorCodes(info.description)],
          ["w3i 版本", `${info.version}（${w3iEraName(info.version)}）`],
          ["编辑器版本", info.editor_version],
          ["构建版本", formatBuild(info.build_version)],
          ["保存次数", info.saves],
          ["地形", tilesetName(info.tileset)],
          [
            "可玩区域",
            info.playable_size ? `${info.playable_size[0]} × ${info.playable_size[1]}` : null,
          ],
          ["玩家槽位", info.players?.length],
          ["队伍", info.forces?.length],
          ["脚本", scriptModeName(info.script_mode)],
          ["画质", graphicsModeName(info.graphics_mode)],
          ["资料片", gameDataVersionName(info.game_data_version)],
          ["数据集", gameDataSetName(info.game_data_set)],
          ["天气", weatherName(info.global_weather)],
          ["音效环境", info.sound_environment],
          ["跳过可选段", info.skipped_optional_sections ? "是" : null],
        ]),
      ),
    );

    const flags = decodeFlags(info.flags);
    if (flags.length) parts.push(card(`标志位 0x${(info.flags >>> 0).toString(16)}`, chips(flags)));

    const fog = bgraToCss(info.fog_color);
    const water = bgraToCss(info.water_vertex_color);
    if (fog || water || info.fog_density != null || info.fog_style != null) {
      parts.push(
        card(
          "环境",
          kv([
            ["迷雾样式", info.fog_style],
            [
              "迷雾高度",
              info.fog_height ? `${info.fog_height[0]} → ${info.fog_height[1]}` : null,
            ],
            ["迷雾浓度", info.fog_density],
            ["迷雾颜色", fog ? swatch(fog, fog) : null],
            ["水面颜色", water ? swatch(water, water) : null],
          ]),
        ),
      );
    }

    const loading = kv([
      ["载入标题", stripColorCodes(info.loading_screen_title)],
      ["载入副标题", stripColorCodes(info.loading_screen_subtitle)],
      ["载入文本", stripColorCodes(info.loading_screen_text)],
      ["载入模型", info.loading_screen_model],
      ["序章标题", stripColorCodes(info.prologue_screen_title)],
      ["序章副标题", stripColorCodes(info.prologue_screen_subtitle)],
      ["序章文本", stripColorCodes(info.prologue_screen_text)],
    ]);
    if (loading.childElementCount) parts.push(card("载入画面", loading));
  }

  if (data.modification) {
    const mod = data.modification;
    parts.push(
      card(
        "第三方修改",
        kv([
          ["名称", mod.name],
          ["版本", mod.version],
          ["证据", mod.evidence],
        ]),
      ),
    );
  }

  host.replaceChildren(...parts);
}

function renderTeams(data: MapMetadata) {
  const host = panelFor("teams");
  const info = data.map_info;
  const players = info?.players ?? [];
  const forces = info?.forces ?? [];
  setCount("teams", players.length);

  const parts: Node[] = [];
  parts.push(
    card(
      "玩家",
      table(
        ["#", "名称", "控制", "种族", "起始点", "固定"],
        players.map((p) => [
          swatch(slotColor(p.id), String(p.id)),
          stripColorCodes(p.name),
          controllerLabel(p.player_type),
          raceName(p.race),
          p.start_location ? `${Math.round(p.start_location[0])}, ${Math.round(p.start_location[1])}` : "—",
          p.is_fixed_start_position ? "是" : "—",
        ]),
      ),
    ),
  );

  if (forces.length) {
    parts.push(
      card(
        "队伍",
        table(
          ["队伍", "成员", "标志"],
          forces.map((f) => {
            const members = playersInForce(f.player_masks, players);
            const box = el("div", "chips");
            for (const m of members) box.append(swatch(slotColor(m.id), stripColorCodes(m.name)));
            return [
              stripColorCodes(f.name),
              members.length ? box : playerMaskLabel(f.player_masks),
              `0x${(f.flags >>> 0).toString(16)}`,
            ];
          }),
        ),
      ),
    );
  }

  host.replaceChildren(...parts);
}

function renderTech(data: MapMetadata) {
  const host = panelFor("tech");
  const info = data.map_info;
  const upgrades = info?.upgrade_availability_changes ?? [];
  const techs = info?.tech_availability_changes ?? [];
  const unitTables = info?.random_unit_tables ?? [];
  const itemTables = info?.random_item_tables ?? [];
  const total = upgrades.length + techs.length + unitTables.length + itemTables.length;
  setCount("tech", total);

  const parts: Node[] = [];
  if (upgrades.length) {
    parts.push(
      card(
        "升级改动",
        table(
          ["ID", "等级", "可用性", "玩家"],
          upgrades.map((u) => [
            fourCC(u.id),
            String(u.level_affected),
            availabilityName(u.availability),
            playerMaskLabel(u.player_flags),
          ]),
          true,
        ),
      ),
    );
  }
  if (techs.length) {
    parts.push(
      card(
        "科技改动",
        table(
          ["ID", "玩家"],
          techs.map((t) => [fourCC(t.id), playerMaskLabel(t.player_flags)]),
          true,
        ),
      ),
    );
  }
  for (const t of unitTables) {
    parts.push(
      card(
        `随机单位表 · ${t.name || t.id}`,
        table(
          ["几率", "单位"],
          t.units.map((u) => [`${u.chance}%`, u.ids.map(fourCC).join(", ")]),
          true,
        ),
      ),
    );
  }
  for (const t of itemTables) {
    parts.push(
      card(
        `随机物品表 · ${t.name || t.id}`,
        table(
          ["组", "物品"],
          t.sets.map((s, i) => [
            String(i + 1),
            s.items.map((it) => `${fourCC(it.id)} ${it.chance}%`).join(", "),
          ]),
          true,
        ),
      ),
    );
  }
  if (!parts.length) parts.push(card("科技", el("p", null, "这张图没有改动科技树。")));
  host.replaceChildren(...parts);
}

function renderImages(data: MapMetadata) {
  const host = panelFor("images");
  const images = data.images ?? [];
  setCount("images", images.length);
  const gallery = el("div", "shots");
  for (const image of images) {
    const figure = el("figure", "shot");
    const img = el("img") as HTMLImageElement;
    img.src = image.data_url;
    img.alt = image.filename;
    img.loading = "lazy";
    figure.append(img, el("figcaption", null, `${image.filename} · ${image.width}×${image.height}`));
    gallery.append(figure);
  }
  host.replaceChildren(
    images.length ? card("图像", gallery) : card("图像", el("p", null, "没有可显示的图像。")),
  );
}

function renderStrings(data: MapMetadata) {
  const host = panelFor("strings");
  const strings = data.strings ?? [];
  setCount("strings", strings.length);
  host.replaceChildren(
    strings.length
      ? card(
          "war3map.wts",
          filterable(
            "搜索字符串…",
            ["ID", "内容"],
            strings.map((s) => ({
              keys: `${s.id} ${s.value}`.toLowerCase(),
              cells: [String(s.id), stripColorCodes(s.value)],
            })),
          ),
        )
      : card("war3map.wts", el("p", null, "没有字符串表。")),
  );
}

function renderImports(data: MapMetadata) {
  const host = panelFor("imports");
  const imports = data.imports ?? [];
  setCount("imports", imports.length);
  host.replaceChildren(
    imports.length
      ? card(
          "war3map.imp",
          filterable(
            "搜索导入路径…",
            ["路径", "类型"],
            imports.map((i) => ({
              keys: i.path.toLowerCase(),
              cells: [i.path, importFlagLabel(i.is_custom)],
            })),
          ),
        )
      : card("war3map.imp", el("p", null, "没有导入文件。")),
  );
}

function renderFiles(data: MapMetadata) {
  const host = panelFor("files");
  const files = data.files ?? [];
  setCount("files", files.length);
  host.replaceChildren(
    files.length
      ? card(
          "归档内文件",
          filterable(
            "搜索文件名…",
            ["路径", "扩展名"],
            files.map((f) => ({ keys: f.toLowerCase(), cells: [f, extensionOf(f)] })),
          ),
        )
      : card(
          "归档内文件",
          el("p", null, "读不到文件清单——地图没有 (listfile)，或它的索引表已被破坏。"),
        ),
  );
}

function renderJson(data: MapMetadata) {
  const host = panelFor("json");
  const pre = el("pre", "json", JSON.stringify(data, null, 2));
  host.replaceChildren(card("原始解析结果", pre));
}

// --------------------------------------------------------------- section nav

/** Mark one nav link as the section being read. */
function markCurrent(id: string) {
  for (const link of secnav.querySelectorAll<HTMLElement>(".seclink")) {
    const current = link.dataset.link === id;
    if (current) link.setAttribute("aria-current", "true");
    else link.removeAttribute("aria-current");
  }
}

/**
 * Highlight whichever section the reader is in.
 *
 * The observer fires on any crossing, so the current section is chosen by
 * looking at everything intersecting rather than by trusting the entry that
 * happened to fire — with a sticky bar at the top, two sections are often in
 * view at once and only the topmost one should win.
 */
function watchSections() {
  const all = [...sections.querySelectorAll<HTMLElement>(".section")];
  const visible = new Set<HTMLElement>();
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const node = entry.target as HTMLElement;
        if (entry.isIntersecting) visible.add(node);
        else visible.delete(node);
      }
      const first = all.find((node) => visible.has(node));
      if (first?.dataset.section) markCurrent(first.dataset.section);
    },
    { rootMargin: "-70px 0px -60% 0px", threshold: 0 },
  );
  for (const node of all) observer.observe(node);
}

// --------------------------------------------------------------------- input

async function handleFile(file: File) {
  if (!ready) return;
  setStatus(`正在读取 ${file.name}（${formatBytes(file.size)}）…`);
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    setStatus(`正在解析 ${file.name}…`);
    const data = parseMap(bytes);
    if (!data) {
      setStatus(`解析失败：${file.name} 不是可读的 War3 地图 / MPQ。`, "err");
      secnav.hidden = true;
      sections.hidden = true;
      return;
    }

    renderOverview(data, { name: file.name, size: file.size });
    renderTeams(data);
    renderTech(data);
    renderImages(data);
    renderStrings(data);
    renderImports(data);
    renderFiles(data);
    renderJson(data);

    secnav.hidden = false;
    sections.hidden = false;
    markCurrent("overview");
    if (!watching) {
      watchSections();
      watching = true;
    }

    const name = stripColorCodes(data.map_info?.name ?? data.header.name ?? "") || file.name;
    setStatus(`已解析：${name}`, "ok");
  } catch (error) {
    console.error(error);
    setStatus(`解析出错：${error instanceof Error ? error.message : String(error)}`, "err");
  }
}

drop.addEventListener("click", () => input.click());
drop.addEventListener("keydown", (event) => {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    input.click();
  }
});
input.addEventListener("change", () => {
  const file = input.files?.[0];
  if (file) void handleFile(file);
});
for (const type of ["dragenter", "dragover"]) {
  drop.addEventListener(type, (event) => {
    event.preventDefault();
    drop.classList.add("over");
  });
}
for (const type of ["dragleave", "drop"]) {
  drop.addEventListener(type, () => drop.classList.remove("over"));
}
drop.addEventListener("drop", (event) => {
  event.preventDefault();
  const file = (event as DragEvent).dataTransfer?.files?.[0];
  if (file) void handleFile(file);
});

void (async () => {
  try {
    const version = await ensureWasm();
    ready = true;
    setStatus(`解析器 v${version} 就绪 — 拖入 .w3x / .w3m 开始。`);
  } catch (error) {
    console.error(error);
    setStatus("解析器加载失败，请刷新页面重试。", "err");
  }
})();
