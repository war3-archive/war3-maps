// In-browser map inspector.
//
// The catalog only carries what fits in a search index. Everything else — the
// full w3i, players, forces, imports, the archive's file list, the string table
// — is read here by running the same parser the CLI uses, compiled to WASM, on
// bytes fetched straight from the dataset. Nothing is uploaded anywhere.
//
// Both the WASM module (1.6 MB) and the map itself are only fetched when a
// visitor actually opens a map, and maps above AUTO_PARSE_LIMIT wait for an
// explicit click: the median map here is 2.5 MB but the largest is 596 MB.

import { coverUrl, downloadUrl, formatBytes } from "./map-browser.js";

const AUTO_PARSE_LIMIT = 8 * 1024 * 1024;

const TILESETS = new Map([
  ["A", "灰谷"],
  ["B", "贫瘠之地"],
  ["C", "费伍德森林"],
  ["D", "地下城"],
  ["F", "洛丹伦秋季"],
  ["G", "地下"],
  ["I", "冰冠冰川"],
  ["J", "达拉然废墟"],
  ["L", "洛丹伦夏季"],
  ["N", "诺森德"],
  ["O", "外域"],
  ["Q", "village fall"],
  ["V", "村庄"],
  ["W", "洛丹伦冬季"],
  ["X", "达拉然"],
  ["Y", "城市"],
  ["Z", "沉没的遗迹"],
]);

const RACES = new Map([
  [1, "人类"],
  [2, "兽族"],
  [3, "不死族"],
  [4, "暗夜精灵"],
]);

const PLAYER_TYPES = new Map([
  [1, "玩家"],
  [2, "电脑"],
  [3, "中立"],
  [4, "可解救"],
]);

const SCRIPT_MODES = new Map([
  [0, "JASS"],
  [1, "Lua"],
]);

const GRAPHICS_MODES = new Map([
  [1, "SD"],
  [2, "HD"],
  [3, "SD + HD"],
]);

// Raw w3i strings carry WC3 markup: |cAARRGGBB … |r for colour, |n for a break.
// The catalog is already stripped; parser output is not.
const stripCodes = (value) =>
  typeof value === "string"
    ? value.replace(/\|c[0-9a-fA-F]{8}/g, "").replace(/\|r/g, "").replace(/\|n/g, "\n").trim()
    : value;

let dialog = null;
let wasmReady = null;

async function ensureWasm() {
  if (!wasmReady) {
    wasmReady = import("@wesleyel/war3parser").then(async (module) => {
      await module.default();
      return module;
    });
  }
  return wasmReady;
}

function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null) node.textContent = String(text);
  return node;
}

function kvList(pairs) {
  const list = element("dl", "detail-kv");
  for (const [term, value] of pairs) {
    if (value === null || value === undefined || value === "") continue;
    const row = element("div");
    row.append(element("dt", null, term), element("dd", null, value));
    list.append(row);
  }
  return list;
}

function tilesetLabel(code) {
  if (code === null || code === undefined) return null;
  const char = String.fromCharCode(code);
  const name = TILESETS.get(char);
  return name ? `${name}（${char}）` : `${char}（${code}）`;
}

function ensureDialog() {
  if (dialog) return dialog;
  dialog = element("dialog", "detail");
  dialog.innerHTML = `
    <form method="dialog" class="detail-close-form">
      <button class="detail-close" value="close" aria-label="关闭">✕</button>
    </form>
    <header class="detail-head"></header>
    <div class="detail-body"></div>
  `;
  document.body.append(dialog);
  // Clicking the backdrop closes, matching what people expect of a modal.
  dialog.addEventListener("click", (event) => {
    if (event.target === dialog) dialog.close();
  });
  return dialog;
}

function renderHead(map) {
  const head = dialog.querySelector(".detail-head");
  head.replaceChildren();

  const cover = element("img", "detail-cover");
  cover.loading = "lazy";
  cover.decoding = "async";
  cover.alt = "";
  if (map.hasCover) {
    cover.src = coverUrl(map.sha256);
  } else {
    cover.hidden = true;
  }

  const main = element("div", "detail-head-main");
  main.append(element("p", "eyebrow", map.collection || "未分类"));
  main.append(element("h2", null, map.name || "地图"));
  if (map.description) main.append(element("p", "detail-description", map.description));

  const actions = element("div", "detail-actions");
  const href = downloadUrl(map.sha256, map.extension);
  if (href) {
    const download = element("a", "download", "单图下载");
    download.href = href;
    download.setAttribute("download", "");
    actions.append(download);
  }
  const copy = element("button", "copy-hash", "复制 SHA-256");
  copy.type = "button";
  copy.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(map.sha256);
      copy.textContent = "已复制";
    } catch {
      copy.textContent = "复制失败";
    }
    setTimeout(() => {
      copy.textContent = "复制 SHA-256";
    }, 1400);
  });
  actions.append(copy);
  main.append(actions);

  head.append(cover, main);
}

function renderCatalogFacts(map) {
  return kvList([
    ["作者", map.author || "未知"],
    ["玩家", map.players ? `${map.players} 人` : null],
    ["格式", map.extension],
    ["最低版本", map.minVersion ? `${map.clientName} ${map.minVersion}` : "未知"],
    ["版本依据", map.evidence],
    ["文件大小", formatBytes(map.size)],
    ["SHA-256", map.sha256],
  ]);
}

function panel(title, node) {
  const section = element("section", "detail-panel");
  section.append(element("h3", null, title), node);
  return section;
}

function table(headers, rows) {
  const wrap = element("div", "detail-table-wrap");
  const node = element("table", "detail-table");
  const thead = element("thead");
  const headRow = element("tr");
  for (const header of headers) headRow.append(element("th", null, header));
  thead.append(headRow);
  const tbody = element("tbody");
  for (const row of rows) {
    const tr = element("tr");
    for (const cell of row) tr.append(element("td", null, cell ?? ""));
    tbody.append(tr);
  }
  node.append(thead, tbody);
  wrap.append(node);
  return wrap;
}

function list(values, limit = 400) {
  const node = element("ul", "detail-list");
  for (const value of values.slice(0, limit)) node.append(element("li", null, value));
  if (values.length > limit) {
    node.append(element("li", "detail-list-more", `… 其余 ${values.length - limit} 项已省略`));
  }
  return node;
}

function renderSnapshot(snapshot, container) {
  container.replaceChildren();
  const info = snapshot.map_info;

  if (!info) {
    container.append(
      element(
        "p",
        "detail-empty",
        "地图内没有可读的 war3map.w3i —— 通常是被保护或加密过。下面仍会列出能读到的内容。",
      ),
    );
  } else {
    container.append(
      panel(
        "地图信息",
        kvList([
          ["w3i 格式", `v${info.version}`],
          ["编辑器 build", info.editor_version],
          ["保存次数", info.saves],
          ["游戏版本", info.build_version ? info.build_version.join(".") : null],
          ["地图名", stripCodes(info.name)],
          ["作者", stripCodes(info.author)],
          ["建议人数", stripCodes(info.recommended_players)],
          ["地形", tilesetLabel(info.tileset)],
          ["可用尺寸", info.playable_size ? info.playable_size.join(" × ") : null],
          ["脚本", SCRIPT_MODES.get(info.script_mode) ?? null],
          ["画质", GRAPHICS_MODES.get(info.graphics_mode) ?? null],
          ["载入画面标题", stripCodes(info.loading_screen_title)],
          ["载入画面文本", stripCodes(info.loading_screen_text)],
          ["简介", stripCodes(info.description)],
        ]),
      ),
    );

    if (info.players?.length) {
      container.append(
        panel(
          `玩家（${info.players.length}）`,
          table(
            ["#", "名称", "类型", "种族", "出生点"],
            info.players.map((player) => [
              player.id,
              stripCodes(player.name),
              PLAYER_TYPES.get(player.player_type) ?? player.player_type,
              RACES.get(player.race) ?? player.race,
              player.start_location?.map((value) => Math.round(value)).join(", "),
            ]),
          ),
        ),
      );
    }

    if (info.forces?.length) {
      container.append(
        panel(
          `阵营（${info.forces.length}）`,
          table(
            ["名称", "玩家掩码", "标志"],
            info.forces.map((force) => [
              stripCodes(force.name),
              `0x${(force.player_masks >>> 0).toString(16)}`,
              `0x${(force.flags >>> 0).toString(16)}`,
            ]),
          ),
        ),
      );
    }
  }

  if (snapshot.images?.length) {
    const gallery = element("div", "detail-gallery");
    for (const image of snapshot.images) {
      const figure = element("figure");
      const img = element("img");
      img.src = image.data_url;
      img.alt = image.filename;
      img.loading = "lazy";
      figure.append(img, element("figcaption", null, `${image.filename} · ${image.width}×${image.height}`));
      gallery.append(figure);
    }
    container.append(panel(`内嵌图像（${snapshot.images.length}）`, gallery));
  }

  if (snapshot.imports?.length) {
    container.append(
      panel(
        `导入资源（${snapshot.imports.length}）`,
        list(snapshot.imports.map((entry) => entry.path)),
      ),
    );
  }

  if (snapshot.files?.length) {
    container.append(panel(`归档内文件（${snapshot.files.length}）`, list(snapshot.files)));
  }

  if (snapshot.strings?.length) {
    container.append(
      panel(
        `字符串表（${snapshot.strings.length}）`,
        table(
          ["ID", "内容"],
          snapshot.strings.slice(0, 300).map((entry) => [entry.id, stripCodes(entry.value)]),
        ),
      ),
    );
  }

  if (snapshot.parse_ms !== null && snapshot.parse_ms !== undefined) {
    container.append(element("p", "detail-footnote", `解析耗时 ${snapshot.parse_ms.toFixed(0)} ms`));
  }
}

async function runParse(map, container, status) {
  const href = downloadUrl(map.sha256, map.extension);
  if (!href) {
    status.textContent = "这张地图没有可用的下载地址，无法解析。";
    return;
  }
  try {
    status.textContent = "正在载入解析器…";
    const wasm = await ensureWasm();
    status.textContent = `正在下载地图（${formatBytes(map.size)}）…`;
    const response = await fetch(href);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    status.textContent = "正在解析…";
    const snapshot = wasm.parse_map(bytes);
    if (!snapshot) {
      status.textContent = "解析失败：这不是一个可读的 MPQ 地图归档。";
      return;
    }
    status.hidden = true;
    renderSnapshot(snapshot, container);
  } catch (error) {
    status.hidden = false;
    status.textContent = `解析失败：${error.message}`;
  }
}

/** Open the inspector for one map. `map` carries the catalog fields we already have. */
export function openDetail(map) {
  ensureDialog();
  renderHead(map);

  const body = dialog.querySelector(".detail-body");
  body.replaceChildren();
  body.append(panel("索引信息", renderCatalogFacts(map)));

  const parseSection = element("section", "detail-parse");
  const status = element("p", "detail-status");
  const output = element("div", "detail-output");
  parseSection.append(status, output);
  body.append(parseSection);

  if (map.size <= AUTO_PARSE_LIMIT) {
    runParse(map, output, status);
  } else {
    status.replaceChildren();
    const note = element(
      "span",
      null,
      `这张地图有 ${formatBytes(map.size)}，解析需要先完整下载。`,
    );
    const button = element("button", "load-more detail-parse-button", "下载并解析");
    button.type = "button";
    button.addEventListener("click", () => {
      status.replaceChildren();
      status.textContent = "正在准备…";
      runParse(map, output, status);
    });
    status.append(note, button);
  }

  dialog.showModal();
}
