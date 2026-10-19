// Live reload and the panel for pages answered by `statac serve`.
//
// One tab in the browser holds the waiting request, the one holding the Web
// Lock; the others hear from it over a BroadcastChannel and ask for their own
// answer. A browser opens only six HTTP/1.1 connections to one host, so a
// request held by every tab would stall the seventh.
(() => {
  const script = document.currentScript;
  const live = new URL(".", script.src);
  let seen = Number(script.dataset.live);
  const channel = "BroadcastChannel" in self ? new BroadcastChannel("statac-live") : null;
  const kept = "statac-live-panel";

  // Every word the panel puts together itself, from serve: each the pieces
  // of a sentence, or for a count, those said of one and of many.
  const words = /* the panel's words */ {};

  // The panel's choices, kept for this browser alone.
  const defaults = {
    open: false,
    tab: "problems",
    sits: "corner",
    corner: "bottom-left",
    colours: "system",
    stops: "card",
    shortcut: { alt: true, shift: true, ctrl: false, meta: false, code: "KeyS" },
    height: 0,
    width: 0,
    hideEmpty: true,
    onlyUsed: false,
    away: "",
    undocked: "corner",
    editor: "none",
  };
  const settings = structuredClone(defaults);
  try {
    const stored = localStorage.getItem(kept);
    if (stored === "open" || stored === "closed") settings.open = stored === "open";
    else if (stored) {
      const read = JSON.parse(stored);
      for (const key of Object.keys(defaults)) {
        if (typeof read?.[key] === typeof defaults[key]) settings[key] = read[key];
      }
    }
  } catch {}
  const editors = [
    ["none", "editorNone"],
    ["vscode", "editorCode"],
    ["vscode-insiders", "editorInsiders"],
    ["cursor", "editorCursor"],
    ["zed", "editorZed"],
    ["sublime", "editorSublime"],
  ];
  if (!editors.some(([editor]) => editor === settings.editor)) settings.editor = defaults.editor;
  const keep = () => {
    try {
      localStorage.setItem(kept, JSON.stringify(settings));
    } catch {}
  };
  keep();

  const ask = async (timeout) => {
    const asked = `wait?since=${seen}&timeout=${timeout}&building=${building ? 1 : 0}`;
    const response = await fetch(new URL(asked, live), { cache: "no-store" });
    if (!response.ok) throw new Error(`statac serve: ${response.status}`);
    return response.json();
  };

  // Words.

  const grouped = (n) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ",");

  // A word's pieces with its holes filled in, a count's by its number.
  const fill = (word, holes = {}) => {
    const pieces = Array.isArray(word) ? word : holes.n === 1 ? word.one : word.many;
    const filled = (text) =>
      text.replace(/\{(\w+)\}/g, (hole, name) => {
        if (!(name in holes)) return hole;
        return name === "n" ? grouped(holes.n) : String(holes[name]);
      });
    return (pieces || []).map((piece) =>
      "code" in piece ? { code: filled(piece.code) } : { text: filled(piece.text) },
    );
  };
  const plain = (pieces) => pieces.map((piece) => piece.code ?? piece.text ?? "").join("");
  const told = (name, holes) => plain(fill(words[name], holes));

  // Elements, every word in them set as text.

  const make = (name, attributes = {}, ...children) => {
    const made = document.createElement(name);
    for (const [key, value] of Object.entries(attributes)) {
      if (value === undefined || value === null || value === false) continue;
      if (key === "class") made.className = value;
      else if (key.startsWith("on")) made.addEventListener(key.slice(2), value);
      else made.setAttribute(key, value === true ? "" : String(value));
    }
    made.append(...children.flat(Infinity).filter((child) => child != null && child !== false));
    return made;
  };

  // A sentence's pieces: words, names shown as code, and in a line of a
  // file, the place a problem is at.
  const pieces = (sentence) =>
    (sentence || []).map((piece) => {
      if ("code" in piece) return make("code", {}, piece.code);
      if ("mark" in piece) return make("span", { class: "marked" }, piece.mark);
      return document.createTextNode(piece.text);
    });
  const refill = (element, ...children) =>
    element.replaceChildren(
      ...children.flat(Infinity).filter((child) => child != null && child !== false),
    );
  const say = (name, holes) => pieces(fill(words[name], holes));
  const woven = (name, elements, holes) =>
    fill(words[name], holes).flatMap((piece) => {
      if ("code" in piece) {
        const hole = /^\{(\w+)\}$/.exec(piece.code)?.[1];
        return [elements[hole] ?? make("code", {}, piece.code)];
      }
      return piece.text
        .split(/(\{\w+\})/)
        .filter(Boolean)
        .map((part) => elements[part.slice(1, -1)] ?? document.createTextNode(part));
    });
  const spoken = (tag, sentence, attributes = {}) => make(tag, attributes, pieces(sentence));

  const drawn = (box, parts, attributes = {}) => {
    const made = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    made.setAttribute("viewBox", box);
    made.setAttribute("aria-hidden", "true");
    made.setAttribute("focusable", "false");
    for (const [key, value] of Object.entries(attributes)) made.setAttribute(key, value);
    for (const [name, values] of parts) {
      const part = document.createElementNS("http://www.w3.org/2000/svg", name);
      for (const [key, value] of Object.entries(values)) part.setAttribute(key, value);
      made.append(part);
    }
    return made;
  };
  const mark = () =>
    drawn(
      "0 0 750 750",
      [
        [
          "path",
          {
            "fill-rule": "evenodd",
            d: "M660,0C709.7,0 750,40.3 750,90L750,660C750,709.7 709.7,750 660,750L90,750C40.3,750 0,709.7 0,660L0,90C0,40.3 40.3,0 90,0L660,0ZM479.7,292.9C459.1,292.9 448.7,303.1 448.7,324.2L448.7,351.7L448.7,372C448.7,392.4 436.8,400.2 419.4,390.8L397.2,379.1L374.2,366.5C355.9,356.4 340.8,360.3 331.3,379.1L328.1,385.3L323.4,392.4C312.3,410.4 317,425.3 334.5,435.5L377.3,460.5C394.8,470.7 394.8,484.8 377.3,495L357.5,506.8L334.5,519.3C316.2,528.7 312.3,544.4 322.6,561.6L328.1,571L331.3,577.3C340.8,596.1 355.9,600 374.2,590.6L398.8,577.3L417.8,566.3C434.5,556.9 448,562.4 448.7,582.8L449.5,604.7L449.5,630.5C449.5,651.7 456.7,661.9 469.4,661.9L489.2,661.9L498.7,661.9C519.4,662.7 530.5,652.5 530.5,631.3L530.5,605.5L531.3,582.8C532.1,562.4 544.8,555.3 562.2,566.3L581.3,578.1L603.5,590.6C621.7,600.8 637.6,596.9 647.1,578.1L651.1,569.4L655.8,561.6C666.2,544.4 662.2,528.7 643.9,518.5L620.9,506L600.3,494.2C583.6,484 583.6,468.4 601.1,459L620.9,448L643.9,436.3C662.2,426.9 667,411.2 656.6,394L651.9,386.1L647.1,378.3C636.8,361.1 620.9,356.4 603.5,366.5L582,379.1L561.4,390C544,399.4 531.3,392.4 531.3,372L531.3,350.9L531.3,324.2C531.3,304.7 520.2,293.7 499.5,292.9L489.2,292.9L479.7,292.9Z",
          },
        ],
      ],
      { class: "mark" },
    );
  // Errors are squares, warnings triangles and notes rings, so each is told
  // apart without its colour.
  const shapes = {
    error: [["rect", { x: 1, y: 1, width: 10, height: 10, rx: 1.5 }]],
    warning: [["path", { d: "M6 1l5 9H1z" }]],
    note: [["circle", { cx: 6, cy: 6, r: 4.5 }]],
  };
  const shape = (severity) => drawn("0 0 12 12", shapes[severity], { class: `shape ${severity}` });
  const icon = (...paths) => drawn("0 0 24 24", paths, { class: "icon" });
  const icons = {
    settings: () =>
      icon(
        ["circle", { cx: 12, cy: 12, r: 3 }],
        [
          "path",
          {
            d: "M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z",
          },
        ],
      ),
    side: () =>
      icon(["rect", { x: 3, y: 4, width: 18, height: 16, rx: 2 }], ["path", { d: "M14 4v16" }]),
    bottom: () =>
      icon(["rect", { x: 3, y: 4, width: 18, height: 16, rx: 2 }], ["path", { d: "M3 14h18" }]),
    close: () => icon(["path", { d: "M6 9l6 6 6-6" }]),
    away: () => icon(["path", { d: "M6 6l12 12M18 6L6 18" }]),
    previous: () => icon(["path", { d: "M15 6l-6 6 6 6" }]),
    next: () => icon(["path", { d: "M9 6l6 6-6 6" }]),
    find: () => icon(["circle", { cx: 11, cy: 11, r: 7 }], ["path", { d: "M20 20l-3.5-3.5" }]),
    copy: () =>
      icon(
        ["rect", { x: 8, y: 8, width: 13, height: 13, rx: 2 }],
        ["path", { d: "M16 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h3" }],
      ),
    copied: () => icon(["path", { d: "M5 12.5l4.5 4.5L19 7.5" }]),
    edit: () =>
      icon(
        ["path", { d: "M12 20h9" }],
        ["path", { d: "M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z" }],
      ),
  };

  // Only ever a page of this site, or Statac's own explanations.
  const link = (href, ...children) => {
    const made = make("a", {}, children);
    try {
      const to = new URL(href, location.origin);
      if (to.origin === location.origin) made.href = to.href;
      else if (to.protocol === "https:" && to.host === "statac.dev") {
        made.href = to.href;
        made.target = "_blank";
        made.rel = "noopener";
      }
    } catch {}
    return made;
  };

  // A link that opens a file at its line in the reader's editor, or null
  // where there is none. No other scheme is ever made.
  const opener = (editor, folder, file, line, column) => {
    const schemes = {
      vscode: "vscode://file",
      "vscode-insiders": "vscode-insiders://file",
      cursor: "cursor://file",
      zed: "zed://file",
    };
    if (!folder || !file) return null;
    if (!Object.hasOwn(schemes, editor) && editor !== "sublime") return null;
    const windows = /^[a-z]:[\\/]|^\\\\/i.test(folder);
    const slashed = (path) => (windows ? path.replaceAll("\\", "/") : path);
    let whole = slashed(file);
    if (!/^\/|^[a-z]:\//i.test(whole)) whole = `${slashed(folder).replace(/\/+$/, "")}/${whole}`;
    if (!whole.startsWith("/")) whole = `/${whole}`;
    const at = Number.isInteger(line) ? [line] : [];
    if (at.length && Number.isInteger(column)) at.push(column);
    // SublimeUrl decodes the query value once and takes the path in the
    // file URL as it finds it.
    if (editor === "sublime") {
      const query = [`url=${encodeURIComponent(`file://${whole}`)}`];
      if (at.length) query.push(`line=${at[0]}`);
      if (at.length > 1) query.push(`column=${at[1]}`);
      return `subl://open?${query.join("&")}`;
    }
    const path = whole
      .split("/")
      .map((part, index) =>
        index === 1 && windows && /^[a-z]:$/i.test(part) ? part : encodeURIComponent(part),
      )
      .join("/");
    return `${schemes[editor]}${path}${at.map((n) => `:${n}`).join("")}`;
  };

  // The style. Sizes are in em, not rem: rem reads the page's root font
  // size through the shadow root. A length is its size in px at the
  // default 16px over the element's own font size there, 14px unless it
  // says otherwise.
  const em = (px, size = 14) => `${Math.round((px / size) * 10000) / 10000}em`;
  const tokens = {
    ground: ["#191918", "#FFFFFF"],
    "ground-2": ["#1D1D1B", "#F9F9F8"],
    raise: ["#222221", "#F9F9F8"],
    selected: ["#2A2A28", "#F1F0EF"],
    well: ["#111110", "#F1F0EF"],
    rule: ["#31312E", "#DAD9D6"],
    edge: ["#3B3A37", "#CFCECA"],
    "edge-strong": ["#494844", "#BCBBB5"],
    "line-number": ["#85847D", "#6B6A64"],
    "text-muted": ["#B5B3AD", "#63635E"],
    text: ["#EEEEEC", "#21201C"],
    "text-strong": ["#FAFAF9", "#21201C"],
    brand: ["#F76C14", "#F76C14"],
    focus: ["#F76C14", "#EA5F08"],
    warning: ["#FFC53D", "#7A4F00"],
    "warning-ground": ["#33260C", "#FFF1D1"],
    "warning-edge": ["#4A3A14", "#E8C77A"],
    error: ["#FF9592", "#CE2C31"],
    "error-text": ["#FFE7E4", "#7E1A1E"],
    "error-ground": ["#3C181A", "#FFF0EF"],
    "error-edge": ["#7A2E31", "#F4A9AA"],
    "error-line": ["#2A1517", "#FFF0EF"],
    note: ["#CFCECA", "#8D8D86"],
    shadow: ["rgb(17 17 16 / 0.3)", "rgb(33 32 28 / 0.14)"],
  };
  const palette = (at, scheme) =>
    `color-scheme: ${scheme}; ` +
    Object.entries(tokens)
      .map(([name, both]) => `--${name}: ${both[at]};`)
      .join(" ");
  const mono = `ui-monospace, "SF Mono", Menlo, Consolas, monospace`;

  const style = document.createElement("style");
  style.textContent = `
    :host { all: initial; display: block; }
    .layer {
      ${palette(0, "dark")}
      position: fixed; inset: 0; z-index: 2147483647; box-sizing: border-box;
      width: 100%; height: 100%; max-width: none; max-height: none;
      margin: 0; padding: 0; border: 0; background: transparent; overflow: visible;
      pointer-events: none; color: var(--text);
      font: 0.875em/1.45 system-ui, -apple-system, "Segoe UI", sans-serif;
      text-align: left;
    }
    @media (prefers-color-scheme: light) {
      .layer:not([data-theme="dark"]) { ${palette(1, "light")} }
    }
    .layer[data-theme="light"] { ${palette(1, "light")} }
    .layer > * { pointer-events: auto; position: absolute; box-sizing: border-box; }
    .layer *, .layer *::before, .layer *::after { box-sizing: border-box; }
    [hidden] { display: none !important; }
    button, input, select { font: inherit; color: inherit; margin: 0; }
    button { cursor: pointer; background: transparent; border: 0; padding: 0; text-align: inherit; }
    code, .code { font-family: ${mono}; font-size: ${em(13)}; }
    h2, h3, h4, p, ul, ol, dl, dd, figure, pre { margin: 0; }
    ul, ol { padding: 0; list-style: none; }
    a { color: var(--text); text-decoration: underline 1px var(--edge-strong); text-underline-offset: 0.2em; }
    a:hover, .link:hover { text-decoration-color: var(--brand); }
    .link {
      color: var(--text); text-decoration: underline 1px var(--edge-strong);
      text-underline-offset: 0.2em;
    }
    :focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
    .hidden-label {
      position: absolute; width: 1px; height: 1px; overflow: hidden;
      clip-path: inset(50%); white-space: nowrap;
    }
    .mark { display: block; flex: none; width: ${em(18)}; height: ${em(18)}; fill: var(--brand); }
    .shape { display: block; flex: none; width: ${em(11)}; height: ${em(11)}; }
    .shape.error { fill: var(--error); }
    .shape.warning { fill: var(--warning); width: ${em(12)}; height: ${em(12)}; }
    .shape.note { fill: none; stroke: var(--note); stroke-width: 2; }
    .icon {
      display: block; width: ${em(17)}; height: ${em(17)}; fill: none; stroke: currentColor;
      stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round;
    }
    .rule { flex: none; width: 1px; height: ${em(16)}; background: var(--edge); }
    .count { display: inline-flex; align-items: center; gap: ${em(5)}; font-weight: 600; }
    .count.error { color: var(--error); }
    .count.warning { color: var(--warning); }
    .count.note { color: var(--note); }
    .muted { color: var(--text-muted); }
    .meta { color: var(--text-muted); font-size: ${em(13)}; }
    .meta :is(code, .code) { font-size: 1em; }
    .none { color: var(--text-muted); font-style: italic; }
    .label {
      display: inline-block; padding: 0 ${em(7, 13)}; font-size: ${em(13)};
      border: 1px solid var(--edge-strong); border-radius: ${em(4, 13)}; white-space: nowrap;
    }
    .button {
      display: inline-flex; align-items: center; gap: ${em(6)}; min-height: ${em(32)};
      padding: 0 ${em(12)}; border: 1px solid var(--edge-strong); border-radius: ${em(6)};
      background: var(--raise); color: var(--text); white-space: nowrap;
    }
    .button:hover { border-color: var(--note); }
    .icon-button {
      display: flex; align-items: center; justify-content: center; flex: none;
      width: ${em(32)}; height: ${em(32)}; border: 1px solid transparent; border-radius: ${em(6)};
      color: var(--text-muted);
    }
    .icon-button:hover, .icon-button[aria-expanded="true"] { border-color: var(--rule); color: var(--text); }
    kbd {
      font-family: ${mono}; font-size: ${em(12)}; padding: 0 ${em(5, 12)};
      border: 1px solid var(--edge-strong); border-radius: ${em(4, 12)}; color: var(--text-muted);
    }

    /* The tab, and the bar that can stand for it. */
    .tab {
      bottom: 0; left: ${em(24)}; display: inline-flex; align-items: center; gap: ${em(10)};
      height: ${em(34)}; padding: 0 ${em(12)} 0 ${em(10)}; white-space: nowrap; overflow: hidden;
      background: var(--ground); color: var(--text); border: 1px solid var(--edge);
      border-bottom: 0; border-radius: ${em(6)} ${em(6)} 0 0;
      box-shadow: 0 ${em(-4)} ${em(16)} var(--shadow);
    }
    .layer[data-corner$="right"] .tab { left: auto; right: ${em(24)}; }
    .layer[data-corner^="top"] .tab {
      bottom: auto; top: ${em(12)}; border-bottom: 1px solid var(--edge); border-radius: ${em(6)};
      box-shadow: 0 ${em(6)} ${em(16)} var(--shadow);
    }
    .tab .time, .bar .time { color: var(--text-muted); }
    .stopped.tab, .stopped.bar { background: var(--error-ground); border-color: var(--error-edge); }
    .stopped .rule { background: var(--error-edge); }
    .stopped strong { font-weight: 650; color: var(--error-text); }
    .building-line {
      position: absolute; left: 0; top: 0; width: 45%; height: 2px; background: var(--brand);
      animation: sweep 1.4s ease-in-out infinite alternate;
    }
    @keyframes sweep { from { transform: translateX(-60%); } to { transform: translateX(180%); } }
    @media (prefers-reduced-motion: reduce) {
      .building-line { animation: none; transform: none; }
    }
    .bar {
      left: 0; right: 0; bottom: 0; display: flex; align-items: center; gap: ${em(12)};
      height: ${em(28)}; padding: 0 ${em(10)}; overflow: hidden; white-space: nowrap;
      background: var(--ground); color: var(--text); border-top: 1px solid var(--edge);
    }
    .bar .mark { width: ${em(14)}; height: ${em(14)}; }
    .bar .fill { flex: 1; }

    /* What the tab says under a pointer or focus. */
    .summary, .card {
      bottom: ${em(46)}; left: ${em(24)}; max-width: calc(100vw - ${em(32)});
      background: var(--ground); color: var(--text); border: 1px solid var(--edge);
      border-radius: ${em(6)}; box-shadow: 0 ${em(10)} ${em(30)} var(--shadow);
    }
    .layer[data-corner$="right"] :is(.summary, .card) { left: auto; right: ${em(24)}; }
    .layer[data-corner^="top"] :is(.summary, .card) { bottom: auto; top: ${em(58)}; }
    .layer[data-sits="bar"] :is(.summary, .card) { top: auto; bottom: ${em(40)}; left: ${em(24)}; right: auto; }
    .summary {
      width: ${em(380)}; padding: ${em(14)} ${em(16)}; display: flex; flex-direction: column;
      gap: ${em(10)};
    }
    .summary .top { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 0 ${em(12)}; }
    .said { display: flex; gap: ${em(8)}; align-items: baseline; }
    .said .shape { position: relative; top: 0.05em; }
    .summary .foot {
      display: flex; gap: ${em(16)}; padding-top: ${em(8)}; border-top: 1px solid var(--rule);
    }

    /* The card a stopped build opens. */
    .card { width: ${em(560)}; border-color: var(--error-edge); overflow: hidden; }
    .card > header {
      display: flex; align-items: flex-start; gap: ${em(10)};
      padding: ${em(8)} ${em(8)} ${em(8)} ${em(16)}; background: var(--error-ground);
    }
    .card > header .said { flex: 1; flex-wrap: wrap; gap: ${em(2)} ${em(10)}; padding-top: ${em(6)}; }
    .card > header strong { font-weight: 650; color: var(--error-text); }
    .card > header .kept { color: var(--error-text); font-size: ${em(13)}; }
    .card > header .steps { flex: none; display: flex; align-items: center; gap: ${em(2)}; }
    .card > header .steps .kept { margin-right: ${em(6, 13)}; }
    .card > header .icon-button { color: var(--error-text); width: ${em(30)}; height: ${em(30)}; }
    .card > header .icon-button:hover { border-color: var(--error-edge); }
    .card .body {
      padding: ${em(16)} ${em(18)} ${em(18)}; display: flex; flex-direction: column; gap: ${em(12)};
      max-height: min(${em(520)}, calc(100dvh - ${em(120)})); overflow: auto;
    }
    .card h2 { font-size: ${em(17)}; font-weight: 650; color: var(--text-strong); line-height: 1.35; }
    .card h2 code { font-size: ${em(16, 17)}; }
    .card .ends { display: flex; flex-wrap: wrap; gap: ${em(8)} ${em(18)}; }
    .card .ends .fill { flex: 1; }

    /* What a problem is told with: its place, its lines, its fix. */
    .place { font-family: ${mono}; font-size: ${em(13)}; color: var(--note); overflow-wrap: anywhere; }
    .frame {
      font-family: ${mono}; font-size: ${em(13)}; line-height: 1.7; padding: ${em(8, 13)} 0;
      background: var(--well); border: 1px solid var(--rule); border-radius: ${em(8, 13)};
      color: var(--text-muted); overflow-x: auto;
    }
    .frame .line { display: flex; min-width: max-content; padding-right: ${em(12, 13)}; }
    .frame .number {
      flex: none; width: ${em(40, 13)}; padding-right: ${em(12, 13)}; text-align: right;
      color: var(--line-number); user-select: none;
    }
    .frame .text { white-space: pre; }
    .frame .at { background: var(--error-line); color: var(--text); }
    .frame .at .number { color: var(--error); }
    .warning-frame .at { background: transparent; }
    .warning-frame .at .number { color: var(--warning); }
    .note-frame .at { background: transparent; }
    .note-frame .at .number { color: var(--text); }
    .marked {
      color: var(--text-strong);
      text-decoration: underline wavy var(--error); text-underline-offset: 0.3em;
      text-decoration-skip-ink: none;
    }
    .warning-frame .marked { text-decoration-color: var(--warning); }
    .note-frame .marked { text-decoration-color: var(--note); }
    .frame .runs { color: var(--text-muted); padding-left: ${em(12, 13)}; white-space: nowrap; }
    .fix {
      display: flex; flex-wrap: wrap; align-items: center; gap: ${em(8)} ${em(12)};
      padding: ${em(10)} ${em(12)}; border-radius: ${em(8)}; background: var(--raise);
      border: 1px solid var(--rule);
    }
    .fix p { flex: 1 1 ${em(240)}; }
    .fix code { color: var(--text-strong); }
    .printed pre { font-family: ${mono}; font-size: ${em(13)}; white-space: pre-wrap; color: var(--text-muted); }
    .related { display: flex; flex-direction: column; gap: ${em(6)}; }

    /* A file and line, with what opens and copies it. */
    .located { display: inline-flex; align-items: center; gap: ${em(4)}; max-width: 100%; }
    .located .where { min-width: 0; overflow-wrap: anywhere; }
    .icon-button.small {
      width: ${em(24)}; height: ${em(24)}; min-width: 24px; min-height: 24px; border-radius: ${em(4)};
    }
    .icon-button.small .icon { width: ${em(14)}; height: ${em(14)}; }
    .spot { display: flex; flex-wrap: wrap; align-items: center; gap: ${em(4)} ${em(14)}; }
    .spot .open {
      display: inline-flex; align-items: center; gap: ${em(6, 13)}; min-height: 24px;
      font-size: ${em(13)};
    }
    .spot .open .icon { width: ${em(13, 13)}; height: ${em(13, 13)}; }

    /* The drawer. */
    .drawer {
      left: 0; right: 0; bottom: 0; height: var(--height, min(${em(600)}, 66dvh));
      min-height: ${em(240)}; max-height: calc(100dvh - ${em(24)});
      display: flex; flex-direction: column; background: var(--ground); color: var(--text);
      border-top: 1px solid var(--rule); box-shadow: 0 ${em(-12)} ${em(40)} var(--shadow);
      container: drawer / inline-size;
    }
    .layer[data-sits="side"] .drawer {
      top: 0; left: auto; width: var(--width, min(${em(440)}, 100vw)); height: auto;
      min-width: ${em(320)}; max-width: 100vw; max-height: none;
      border-top: 0; border-left: 1px solid var(--rule);
      box-shadow: ${em(-12)} 0 ${em(36)} var(--shadow);
    }
    .handle {
      position: absolute; z-index: 1; left: 0; right: 0; top: ${em(-4)}; height: ${em(10)};
      cursor: ns-resize; display: flex; justify-content: center; align-items: center;
      touch-action: none;
    }
    .handle::after {
      content: ""; width: ${em(40)}; height: 3px; border-radius: 2px; background: var(--edge-strong);
    }
    .handle:hover::after { background: var(--brand); }
    .handle:focus-visible::after { background: var(--focus); }
    .layer[data-sits="side"] .handle {
      left: ${em(-4)}; right: auto; top: 0; bottom: 0; width: ${em(10)}; height: auto;
      cursor: ew-resize;
    }
    .layer[data-sits="side"] .handle::after { width: 3px; height: ${em(40)}; }
    .head {
      flex: none; display: flex; flex-wrap: wrap; align-items: center; gap: 0 ${em(4)};
      padding: 0 ${em(12)} 0 ${em(18)}; border-bottom: 1px solid var(--rule);
      min-height: ${em(48)};
    }
    .brand {
      display: flex; align-items: center; gap: ${em(10)}; height: ${em(28)};
      padding-right: ${em(14)}; margin-right: ${em(6)}; border-right: 1px solid var(--rule);
    }
    .brand strong { font-weight: 650; font-size: ${em(15)}; color: var(--text-strong); }
    .tabs { display: flex; align-items: stretch; align-self: stretch; overflow-x: auto; }
    .tabs [role="tab"] {
      flex: none; display: flex; align-items: center; gap: ${em(8)}; min-height: ${em(48)};
      padding: 0 ${em(14)}; color: var(--text-muted); border-bottom: 2px solid transparent;
      white-space: nowrap;
    }
    .tabs [role="tab"]:hover { color: var(--text); }
    .tabs [role="tab"][aria-selected="true"] { color: var(--text-strong); border-bottom-color: var(--brand); }
    .tabs [role="tab"]:focus-visible { outline-offset: -2px; }
    .badge {
      display: inline-flex; align-items: center; justify-content: center; min-width: ${em(18, 12)};
      height: ${em(20, 12)}; padding: 0 ${em(6, 12)}; border-radius: ${em(4, 12)};
      font-size: ${em(12)}; font-weight: 600; color: var(--note); background: var(--selected);
    }
    .badge.error { color: var(--error); background: var(--error-ground); }
    .badge.warning { color: var(--warning); background: var(--warning-ground); }
    .tally { font-size: ${em(13)}; color: var(--text-muted); }
    .head .fill { flex: 1; }
    .status {
      color: var(--text-muted); margin-right: ${em(12)}; white-space: nowrap; overflow: hidden;
      text-overflow: ellipsis; min-width: 0;
    }
    .status.stopped { color: var(--error); }
    .find {
      display: flex; align-items: center; gap: ${em(8)}; height: ${em(32)}; width: ${em(280)};
      padding: 0 ${em(8)} 0 ${em(10)}; margin-right: ${em(6)}; border: 1px solid var(--rule);
      border-radius: ${em(6)}; background: var(--raise); color: var(--text-muted);
    }
    .find:focus-within { border-color: var(--focus); }
    .find input {
      flex: 1; min-width: 0; background: transparent; border: 0; color: var(--text); outline: none;
    }
    .find input::placeholder { color: var(--text-muted); opacity: 1; }
    .find .icon { width: ${em(15)}; height: ${em(15)}; }
    .pane { flex: 1; min-height: 0; display: flex; }
    .pane > * { min-width: 0; }
    .rail {
      flex: none; width: ${em(236)}; padding: ${em(14)} ${em(12)}; display: flex; flex-direction: column;
      gap: ${em(4)}; overflow: auto; border-right: 1px solid var(--rule);
    }
    .rail hr { width: calc(100% - ${em(8)}); border: 0; border-top: 1px solid var(--rule); margin: ${em(12)} ${em(4)}; }
    .rail .fill { flex: 1; }
    .rail > p { padding: 0 ${em(8)}; }
    .choice {
      display: flex; justify-content: space-between; align-items: center; gap: ${em(8)}; width: 100%;
      padding: ${em(8)} ${em(10)}; border-radius: ${em(6)}; border: 1px solid transparent;
    }
    .choice.stacked { flex-direction: column; align-items: flex-start; gap: ${em(2)}; }
    .choice:hover { background: var(--raise); }
    .choice[aria-pressed="true"] { background: var(--selected); border-color: var(--edge-strong); }
    .check {
      display: flex; gap: ${em(10)}; align-items: center; padding: ${em(6)} ${em(8)}; cursor: pointer;
      border-radius: ${em(6)};
    }
    .check input { margin: 0; width: ${em(16)}; height: ${em(16)}; flex: none; accent-color: var(--brand); }
    .check .fill { flex: 1; }
    .dot {
      display: inline-block; flex: none; width: ${em(7)}; height: ${em(7)}; border-radius: 50%;
      border: 1px solid var(--edge-strong);
    }
    .dot.used { background: var(--brand); border-color: var(--brand); }
    .main { flex: 1; overflow: auto; }
    .side-pane {
      flex: none; width: ${em(400)}; padding: ${em(18)} ${em(20)}; overflow: auto;
      display: flex; flex-direction: column; gap: ${em(14)}; background: var(--ground-2);
      border-left: 1px solid var(--rule);
    }
    .empty { padding: ${em(24)} ${em(18)}; color: var(--text-muted); }

    /* Problems. */
    .problems { padding: ${em(14)} ${em(18)} ${em(18)}; display: flex; flex-direction: column; gap: ${em(10)}; }
    .problem {
      border: 1px solid var(--rule); border-radius: ${em(6)}; background: var(--ground-2);
    }
    .problem.error { border-color: var(--error-edge); }
    .problem.warning { border-color: var(--warning-edge); }
    .problem h3 { font-size: 1em; font-weight: 600; }
    .problem h3 button {
      display: flex; align-items: center; gap: ${em(10)}; width: 100%; padding: ${em(12)} ${em(16)};
      border-radius: ${em(6)}; color: var(--text-strong);
    }
    .problem h3 button[aria-expanded="true"] .chevron { transform: rotate(90deg); }
    .problem h3 .headline { flex: 1; min-width: 0; overflow-wrap: anywhere; }
    .problem h3 .place { flex: none; max-width: 45%; font-weight: 400; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .problem .chevron { width: ${em(15)}; height: ${em(15)}; color: var(--text-muted); flex: none; }
    .problem .body { padding: 0 ${em(16)} ${em(14)}; display: flex; flex-direction: column; gap: ${em(10)}; }
    .problem.chosen { box-shadow: inset 2px 0 0 var(--brand); }
    .group > h3 button { color: var(--text); }
    .group > .problems { padding: 0 ${em(10)} ${em(10)}; }
    .explanation { line-height: 1.55; }
    .explanation h3 { font-size: ${em(16)}; font-weight: 650; color: var(--text-strong); }
    .explanation h4 { font-size: 1em; font-weight: 650; color: var(--text-strong); margin-top: ${em(4)}; }
    .explanation pre {
      font-family: ${mono}; font-size: ${em(13)}; padding: ${em(8, 13)} ${em(10, 13)};
      background: var(--well); border: 1px solid var(--rule); border-radius: ${em(6, 13)};
      white-space: pre-wrap; overflow-wrap: anywhere;
    }
    .explanation .fill { flex: 1; }

    /* Names. */
    .names { width: 100%; border-collapse: collapse; table-layout: fixed; }
    .names col.write { width: 36%; }
    .names col.holds { width: ${em(110)}; }
    .names col.at { width: 26%; }
    .names th {
      position: sticky; top: 0; z-index: 1; background: var(--ground); text-align: left;
      font-weight: 400; font-size: ${em(13)}; color: var(--text-muted);
      padding: ${em(10, 13)} ${em(12, 13)} ${em(8, 13)} 0; border-bottom: 1px solid var(--rule);
    }
    .names th:first-child, .names td:first-child { padding-left: ${em(16)}; }
    .names td {
      padding: ${em(6)} ${em(12)} ${em(6)} 0; white-space: nowrap; overflow: hidden;
      text-overflow: ellipsis; vertical-align: middle;
    }
    .names tr.name-row { cursor: pointer; }
    .names tr.name-row:hover { background: var(--raise); }
    .names tr.name-row.chosen { background: var(--selected); box-shadow: inset 2px 0 0 var(--brand); }
    .names .write button {
      display: flex; align-items: center; gap: ${em(8)}; max-width: 100%; color: var(--text-strong);
      padding: ${em(2)} 0;
    }
    .names .write code { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .names .holds, .names .at { color: var(--text-muted); }
    .names .holds { font-size: ${em(13)}; }
    .names .at { font-family: ${mono}; font-size: ${em(13)}; }
    .names .at .located { display: flex; }
    .names .at .where { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .detail dl { display: grid; grid-template-columns: ${em(96)} minmax(0, 1fr); gap: ${em(8)} ${em(12)}; }
    .detail dt { color: var(--text-muted); }
    .detail dd { overflow-wrap: anywhere; }
    .detail .written-at { display: flex; flex-direction: column; gap: ${em(8)}; }
    .expression { display: flex; align-items: center; gap: ${em(10)}; }
    .expression code {
      flex: 1; font-size: ${em(15)}; color: var(--text-strong); background: var(--selected);
      border: 1px solid var(--edge); border-radius: ${em(6, 15)}; padding: ${em(7, 15)} ${em(10, 15)};
      overflow-wrap: anywhere;
    }
    .detail h3 { font-size: ${em(13)}; font-weight: 400; color: var(--text-muted); }
    .chips { display: flex; flex-wrap: wrap; gap: ${em(6)}; }
    .chips code {
      padding: ${em(3, 13)} ${em(8, 13)}; border-radius: ${em(5, 13)}; background: var(--selected);
      border: 1px solid var(--edge);
    }
    .way { display: flex; flex-direction: column; gap: ${em(4)}; }
    .way pre {
      font-family: ${mono}; font-size: ${em(13)}; color: var(--text); background: var(--well);
      border: 1px solid var(--rule); border-radius: ${em(6, 13)}; padding: ${em(7, 13)} ${em(9, 13)};
      white-space: pre-wrap; overflow-wrap: anywhere;
    }

    /* Search. */
    .search-pane .main { display: flex; flex-direction: column; }
    .search-status { padding: ${em(14)} ${em(18)} 0; color: var(--text-muted); }
    .search-status:empty { display: none; }
    .search-pane h3.language {
      padding: ${em(14)} ${em(18)} 0; font-size: 1em; font-weight: 400; color: var(--text-muted);
    }
    .found { padding: ${em(8)} ${em(10)} ${em(10)}; display: flex; flex-direction: column; gap: ${em(2)}; }
    .result button {
      display: flex; flex-direction: column; gap: ${em(4)}; width: 100%; padding: ${em(10)} ${em(12)};
      border: 1px solid transparent; border-radius: ${em(6)};
    }
    .result button:hover { background: var(--raise); }
    .result.chosen button {
      background: var(--selected); border-color: var(--edge-strong); box-shadow: inset 2px 0 0 var(--brand);
    }
    .result .title { font-weight: 600; color: var(--text-strong); overflow-wrap: anywhere; }
    .part { display: flex; flex-wrap: wrap; align-items: baseline; gap: ${em(2)} ${em(12)}; }
    .result .excerpt { line-height: 1.55; overflow-wrap: anywhere; }
    mark {
      background: color-mix(in srgb, var(--warning) 30%, transparent); color: var(--text-strong);
      border-radius: 2px;
    }
    .search-pane .more { padding: 0 ${em(22)} ${em(14)}; }
    .why .why-title { font-size: ${em(16)}; font-weight: 650; overflow-wrap: anywhere; }
    .why h4 {
      font-size: ${em(13)}; font-weight: 400; color: var(--text-muted); padding-top: ${em(12)};
      border-top: 1px solid var(--rule);
    }
    .why .sum {
      display: flex; flex-wrap: wrap; justify-content: space-between; gap: ${em(4)} ${em(12)};
      font-weight: 600; color: var(--text-strong);
    }
    .word {
      display: flex; flex-direction: column; gap: ${em(8)}; padding: ${em(12)} ${em(14)};
      border: 1px solid var(--rule); border-radius: ${em(6)}; background: var(--ground);
    }
    .word .top { display: flex; justify-content: space-between; align-items: baseline; gap: ${em(12)}; }
    .word .top code { font-size: ${em(15)}; color: var(--text-strong); overflow-wrap: anywhere; }
    .places { width: 100%; border-collapse: collapse; font-variant-numeric: tabular-nums; }
    .places th, .places td { padding: ${em(4)} 0; text-align: end; font-weight: 400; }
    .places :is(th, td):first-child { text-align: start; }
    .places thead th { font-size: ${em(13)}; color: var(--text-muted); border-bottom: 1px solid var(--rule); }

    /* This page, and the site. */
    .columns { flex: 1; display: grid; grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr) minmax(0, 1fr); overflow: auto; }
    .column {
      padding: ${em(18)} ${em(22)}; display: flex; flex-direction: column; gap: ${em(14)};
      overflow: auto; min-width: 0; border-right: 1px solid var(--rule);
    }
    .column:last-child { border-right: 0; }
    .column h2 { font-size: ${em(18)}; font-weight: 650; color: var(--text-strong); }
    .column h3 { font-size: 1em; font-weight: 650; color: var(--text-strong); }
    .column h3.later { margin-top: ${em(8)}; }
    .column .fill { flex: 1; }
    .facts { display: grid; grid-template-columns: ${em(82)} minmax(0, 1fr); gap: ${em(10)} ${em(14)}; align-items: baseline; }
    .facts dt { color: var(--text-muted); }
    .facts dd { overflow-wrap: anywhere; display: flex; flex-direction: column; gap: ${em(8)}; }
    .chain { display: flex; flex-wrap: wrap; align-items: baseline; gap: ${em(4)} ${em(8)}; }
    .column .label { white-space: normal; }
    .list-card {
      border: 1px solid var(--rule); border-radius: ${em(8)}; padding: ${em(10)} ${em(12)};
      display: flex; flex-direction: column; gap: ${em(6)}; background: var(--ground-2);
    }
    .list-card .top, .list-card .ends { display: flex; flex-wrap: wrap; justify-content: space-between; gap: ${em(4)} ${em(12)}; }
    .files { display: flex; flex-direction: column; gap: ${em(7)}; }
    .files li { font-family: ${mono}; font-size: ${em(13)}; overflow-wrap: anywhere; }
    .files li.also { font-family: inherit; font-size: 1em; color: var(--text-muted); }
    .files li.also code { font-size: ${em(13)}; }
    .tally-line {
      display: flex; gap: ${em(10)}; align-items: baseline; padding-top: ${em(12)};
      border-top: 1px solid var(--rule);
    }
    .tally-line .fill { flex: 1; }
    .column > * { flex: none; }
    .took {
      display: grid; grid-template-columns: max-content 1fr max-content;
      column-gap: ${em(12)}; row-gap: ${em(6)}; margin: 0; align-items: center;
    }
    .took > div { display: grid; grid-column: 1 / -1; grid-template-columns: subgrid; align-items: center; }
    .took :is(dt, dd) { margin: 0; }
    .took dt { color: var(--text-muted); }
    .took .length { display: block; height: ${em(8)}; border-radius: 0 ${em(4)} ${em(4)} 0; background: var(--brand); }
    .took .length.some { min-width: 2px; }
    .took dd:last-child { text-align: end; font-variant-numeric: tabular-nums; }
    .rounds li {
      display: grid; grid-template-columns: ${em(72)} minmax(0, 1fr) auto; gap: ${em(10)};
      padding: ${em(8)} 0; border-top: 1px solid var(--rule);
    }
    .rounds .end { display: flex; gap: ${em(8)}; }
    .rounds .error { color: var(--error); }
    .rounds .warning { color: var(--warning); }
    .kinds { display: flex; flex-direction: column; gap: ${em(10)}; }
    .kinds li {
      display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: ${em(4)} ${em(10)};
      align-items: baseline;
    }
    .kinds li p { grid-column: 2 / -1; }
    .kinds .shape { position: relative; top: 0.05em; }
    .links { display: flex; flex-direction: column; gap: ${em(6)}; }
    .links li { display: flex; flex-wrap: wrap; gap: 0 ${em(8)}; align-items: baseline; }

    /* Settings. */
    .settings {
      position: absolute; z-index: 2; top: ${em(52)}; right: ${em(12)}; width: ${em(360)};
      max-width: calc(100% - ${em(24)}); max-height: calc(100% - ${em(60)}); overflow: auto;
      padding: ${em(16)}; display: flex; flex-direction: column; gap: ${em(16)};
      background: var(--ground); border: 1px solid var(--edge); border-radius: ${em(6)};
      box-shadow: 0 ${em(16)} ${em(40)} var(--shadow);
    }
    .settings fieldset { margin: 0; padding: 0; border: 0; display: flex; flex-direction: column; gap: ${em(8)}; min-width: 0; }
    .settings legend { padding: 0; margin-bottom: ${em(8)}; color: var(--text-muted); }
    .segments { display: flex; flex-wrap: wrap; }
    .segments button {
      flex: 1 1 auto; min-height: ${em(32)}; padding: ${em(4)} ${em(8)}; text-align: center;
      border: 1px solid var(--rule); margin-left: -1px; background: var(--ground);
    }
    .segments button:first-child { margin-left: 0; border-radius: ${em(6)} 0 0 ${em(6)}; }
    .segments button:last-child { border-radius: 0 ${em(6)} ${em(6)} 0; }
    .segments button[aria-pressed="true"] {
      position: relative; background: var(--selected); border-color: var(--edge-strong);
      color: var(--text-strong); font-weight: 600;
    }
    .segments.four { display: grid; grid-template-columns: 1fr 1fr; gap: ${em(4)}; }
    .segments.four button, .segments.four button:first-child, .segments.four button:last-child {
      margin: 0; border-radius: ${em(6)};
    }
    .settings .option { display: flex; gap: ${em(8)}; align-items: center; cursor: pointer; }
    .settings .option input { margin: 0; width: ${em(16)}; height: ${em(16)}; accent-color: var(--brand); }
    .settings .ends {
      display: flex; flex-wrap: wrap; align-items: center; gap: ${em(10)}; padding-top: ${em(12)};
      border-top: 1px solid var(--rule);
    }
    .settings .keys { display: flex; flex-wrap: wrap; align-items: center; gap: ${em(6)}; }
    .settings .field { display: flex; flex-direction: column; gap: ${em(8)}; }
    .settings .field > label { color: var(--text-muted); }
    .settings select {
      min-height: ${em(32)}; padding: 0 ${em(8)}; border: 1px solid var(--edge-strong);
      border-radius: ${em(6)}; background: var(--raise); color: var(--text);
    }

    /* Narrow: docked to the side, or a phone. */
    @container drawer (width < 64em) {
      .head { padding-left: ${em(16)}; }
      .brand { border-right: 0; margin-right: 0; padding-right: 0; flex: 1; }
      .tabs { order: 3; flex: 1 1 100%; border-top: 1px solid var(--rule); margin: 0 ${em(-12)} 0 ${em(-16)}; padding: 0 ${em(4)}; }
      .tabs [role="tab"] { min-height: ${em(44)}; padding: 0 ${em(12)}; }
      .head .fill { display: none; }
      .find { order: 4; flex: 1 1 100%; width: auto; margin: ${em(10)} 0; }
      .pane { flex-direction: column; overflow: auto; }
      .rail {
        width: auto; flex-direction: row; flex-wrap: wrap; align-items: center; gap: ${em(8)};
        border-right: 0; border-bottom: 1px solid var(--rule); padding: ${em(10)} ${em(16)}; overflow: visible;
      }
      .rail hr, .rail .fill { display: none; }
      .rail > p { padding: 0; flex-basis: 100%; }
      .names-pane .rail > p, .names-pane .choice .meta { display: none; }
      .names-pane .choice code { font-size: 1em; }
      .choice, .choice.stacked { width: auto; flex-direction: row; align-items: center; border-color: var(--rule); padding: ${em(4)} ${em(12)}; min-height: ${em(32)}; }
      .main { overflow: visible; flex: none; }
      .pane.names-pane .main { flex: 1; min-height: ${em(160)}; overflow: auto; }
      .pane.names-pane { overflow: hidden; }
      .side-pane { width: auto; border-left: 0; border-top: 1px solid var(--rule); overflow: visible; flex: none; }
      .pane.names-pane .side-pane { max-height: 40%; overflow: auto; }
      .pane.search-pane { overflow: hidden; }
      .pane.search-pane .main { flex: 1; min-height: ${em(160)}; overflow: auto; }
      .pane.search-pane .side-pane { max-height: 45%; overflow: auto; }
      .names thead { display: none; }
      .names, .names tbody { display: block; }
      .names tr.name-row { display: flex; flex-direction: column; gap: ${em(3)}; padding: ${em(8)} ${em(16)}; border-bottom: 1px solid var(--selected); }
      .names td, .names td:first-child { padding: 0; }
      .names td.holds, .names td.at { display: none; }
      .names td.value { padding-left: ${em(15)}; font-size: ${em(13)}; color: var(--text-muted); }
      .columns { display: flex; flex-direction: column; }
      .column { border-right: 0; border-bottom: 1px solid var(--rule); overflow: visible; }
    }
    @media (width < 40em) {
      .layer[data-sits] .drawer {
        top: auto; left: 0; right: 0; width: auto; min-width: 0;
        height: var(--height, min(${em(560)}, 72dvh));
        border-radius: ${em(8)} ${em(8)} 0 0; border: 0; border-top: 1px solid var(--rule);
      }
      .layer[data-sits] .handle {
        left: 0; right: 0; top: 0; bottom: auto; width: auto; height: ${em(20)}; cursor: ns-resize;
      }
      .layer[data-sits] .handle::after { width: ${em(36)}; height: 4px; }
      .head { padding-top: ${em(12)}; }
      .status { display: none; }
      .tab { left: ${em(16)}; }
      .head .dock { display: none; }
      .layer[data-corner$="right"] .tab { right: ${em(16)}; }
      .summary, .card, .layer[data-corner] :is(.summary, .card) { left: ${em(16)}; right: ${em(16)}; width: auto; max-width: none; }
      .settings { left: ${em(12)}; width: auto; }
    }
    @media (width < 40em), (pointer: coarse) {
      .icon-button, .tabs [role="tab"], .button, .segments button, .choice, .check, .settings .option,
      .settings select, .spot .open { min-height: 44px; }
      .icon-button, .icon-button.small { min-width: 44px; }
      .icon-button.small { min-height: 44px; }
      .names .write button { min-height: 44px; }
      .problem h3 button { min-height: 44px; }
    }

    @media (forced-colors: active) {
      .tab, .bar, .summary, .card, .drawer, .settings, .problem, .list-card, .frame, .fix,
      .find, .badge, .label, .button, .expression code, .chips code, .way pre, .word {
        border: 1px solid CanvasText;
      }
      .tab { border-bottom: 0; }
      .layer[data-corner^="top"] .tab { border-bottom: 1px solid CanvasText; }
      .rule { background: CanvasText; }
      .mark { forced-color-adjust: none; }
      .shape.error, .shape.warning { fill: CanvasText; }
      .shape.note { stroke: CanvasText; }
      .building-line { forced-color-adjust: none; background: Highlight; }
      .tabs [role="tab"][aria-selected="true"] { border-bottom-color: Highlight; }
      .choice[aria-pressed="true"], .segments button[aria-pressed="true"],
      .names tr.name-row.chosen, .problem.chosen, .result.chosen button {
        outline: 2px solid Highlight; outline-offset: -2px;
      }
      mark { forced-color-adjust: none; background: Mark; color: MarkText; }
      .dot { border-color: CanvasText; }
      .dot.used { forced-color-adjust: none; background: Highlight; border-color: Highlight; }
      .frame .at { border-left: 3px solid Highlight; }
      .handle::after { background: CanvasText; }
      :focus-visible { outline-color: Highlight; }
    }
  `;

  // The panel lives in a shadow root, so the page's styles don't reach it
  // and its own don't reach the page, and above everything else on the
  // page, where the browser has a top layer.
  const host = document.createElement("statac-live");
  const root = host.attachShadow({ mode: "open" });
  const layer = make("div", { class: "layer", hidden: true });
  const tab = make("button", { class: "tab", type: "button", "aria-expanded": "false" });
  const bar = make("button", {
    class: "bar",
    type: "button",
    "aria-expanded": "false",
    hidden: true,
  });
  const summary = make("div", { class: "summary", "aria-hidden": "true", hidden: true });
  const card = make("section", { class: "card", role: "alert", hidden: true });
  const drawer = make("section", { class: "drawer", role: "region", id: "drawer", hidden: true });
  tab.setAttribute("aria-controls", "drawer");
  bar.setAttribute("aria-controls", "drawer");
  layer.append(tab, bar, summary, card, drawer);
  root.append(style, layer);

  // What is known, and what the reader has chosen to look at.
  let panel = null;
  let version = 0;
  let building = false;
  let hidden = false;
  let site = null;
  const view = {
    scope: "page",
    shown: { error: true, warning: true, note: true },
    quietened: false,
    kind: null,
    opened: new Set(),
    chosen: null,
    query: "",
    group: "page",
    name: null,
    error: 0,
    settings: false,
    capturing: false,
    searched: "",
    result: null,
  };
  // The words searched for and the result chosen are kept for this tab
  // alone, so following a result to its page keeps them.
  const searchKept = "statac-live-search";
  try {
    const stored = JSON.parse(sessionStorage.getItem(searchKept));
    if (typeof stored?.words === "string") view.searched = stored.words;
    if (typeof stored?.result === "string") view.result = stored.result;
  } catch {}
  const keepSearch = () => {
    try {
      sessionStorage.setItem(
        searchKept,
        JSON.stringify({ words: view.searched, result: view.result }),
      );
    } catch {}
  };

  // Where the panel is drawn and how.
  const arrange = () => {
    layer.dataset.sits = settings.sits;
    layer.dataset.corner = settings.corner;
    if (settings.colours === "system") delete layer.dataset.theme;
    else layer.dataset.theme = settings.colours;
    const length = (name, px) => {
      if (px) layer.style.setProperty(name, `${px}px`);
      else layer.style.removeProperty(name);
    };
    length("--height", settings.height);
    length("--width", settings.width);
  };

  // The page ends with room for the bar, so the bar covers nothing that
  // can't be scrolled clear of it.
  const room = () => {
    host.style.height = !layer.hidden && !bar.hidden ? `${bar.offsetHeight}px` : "0";
  };

  const place = () => {
    if (!host.isConnected && document.body) document.body.append(host);
    layer.hidden = hidden || !panel?.round;
    if (layer.hidden) {
      room();
      return;
    }
    if (layer.showPopover && !layer.matches(":popover-open")) {
      try {
        layer.popover = "manual";
        layer.showPopover();
      } catch {}
    }
    room();
  };

  // What the counts are, and how the round stands.

  const severities = ["error", "warning", "note"];
  const plural = { error: "errors", warning: "warnings", note: "notes" };
  const counts = () => panel?.round?.counts || { error: 0, warning: 0, note: 0 };
  const stopped = () => panel?.stopped || null;
  const time = (at) => (at || "").slice(0, 5);
  const page = () => panel?.page || null;
  const leftOut = () => {
    const facts = page();
    if (facts?.draft) return { word: "draft", label: "labelDraft" };
    if (facts?.waiting) return { word: "waits", label: "labelWaits", day: facts.waiting };
    return null;
  };

  const counted = (className, severity, n) =>
    make("span", { class: `${className} ${severity}` }, shape(severity), grouped(n));

  // The tab.

  const drawTab = () => {
    const parts = [mark()];
    const label = [];
    const halted = stopped();
    tab.className = "tab";
    if (building) {
      parts.push(
        make("span", { class: "rule" }),
        make("span", { class: "time" }, told("building")),
      );
      parts.push(make("span", { class: "building-line", "aria-hidden": "true" }));
      label.push(told("labelBuilding"));
    } else if (halted) {
      tab.classList.add("stopped");
      const errors = halted.errors.length;
      parts.push(make("strong", {}, told("buildStopped")), make("span", { class: "rule" }));
      parts.push(counted("count", "error", errors));
      label.push(told("labelStopped"), told("errors", { n: errors }));
    } else {
      const shown = severities.filter((severity) => counts()[severity] > 0);
      const left = leftOut();
      if (left) {
        parts.push(make("span", { class: "label" }, told(left.word, { day: left.day })));
        label.push(told(left.label, { day: left.day }));
      }
      if (shown.length) {
        parts.push(make("span", { class: "rule" }));
        for (const severity of shown) {
          parts.push(counted("count", severity, counts()[severity]));
          label.push(told(plural[severity], { n: counts()[severity] }));
        }
      }
      parts.push(
        make("span", { class: "rule" }),
        make("span", { class: "time" }, time(panel.round.at)),
      );
      label.push(told("labelBuiltAt", { time: time(panel.round.at) }));
    }
    refill(tab, ...parts);
    tab.setAttribute("aria-label", told("tabLabel", { said: label.join(", ") }));

    const line = [mark()];
    bar.className = "bar";
    if (building) {
      line.push(make("span", {}, told("building")));
      line.push(make("span", { class: "building-line", "aria-hidden": "true" }));
    } else if (halted) {
      bar.classList.add("stopped");
      line.push(make("strong", {}, told("buildStopped")));
      line.push(counted("count", "error", halted.errors.length));
    } else {
      const left = leftOut();
      if (left) line.push(make("span", { class: "label" }, told(left.word, { day: left.day })));
      for (const severity of severities) {
        const n = counts()[severity];
        if (n)
          line.push(
            make(
              "span",
              { class: `count ${severity}` },
              shape(severity),
              told(plural[severity], { n }),
            ),
          );
      }
    }
    line.push(make("span", { class: "fill" }), make("span", { class: "time" }, panel.round.at));
    refill(bar, ...line);
    bar.setAttribute("aria-label", tab.getAttribute("aria-label"));
    const corner = settings.sits !== "bar";
    tab.hidden = !corner || settings.open;
    bar.hidden = corner || settings.open;
  };

  // What the tab says under a pointer or focus.

  const drawSummary = () => {
    const halted = stopped();
    const round = panel.round;
    const parts = [];
    const top = make(
      "div",
      { class: "top meta" },
      make(
        "span",
        {},
        halted ? told("stoppedAt", { time: round.at }) : told("builtAt", { time: round.at }),
      ),
      make("span", {}, pieces(panel.rounds[0]?.said)),
    );
    parts.push(top);
    const own = halted ? halted.errors : panel.problems.filter((problem) => !problem.quietened);
    for (const problem of own.slice(0, 5)) {
      parts.push(
        make("p", { class: "said" }, shape(problem.severity), spoken("span", problem.headline)),
      );
    }
    if (!halted) {
      for (const severity of severities) {
        const here = own.filter((problem) => problem.severity === severity).length;
        const elsewhere = counts()[severity] - here;
        if (elsewhere > 0) {
          parts.push(
            make(
              "p",
              { class: "said" },
              shape(severity),
              make("span", {}, say(`${plural[severity]}Elsewhere`, { n: elsewhere })),
            ),
          );
        }
      }
    }
    parts.push(
      make(
        "div",
        { class: "foot meta" },
        make("span", {}, told("clickToOpen")),
        make("span", {}, shortcutKeys().join("+")),
      ),
    );
    refill(summary, ...parts);
  };

  let summaryTimer = 0;
  const showSummary = (wanted) => {
    clearTimeout(summaryTimer);
    const able = wanted && !settings.open && card.hidden && panel?.round && settings.sits !== "bar";
    if (!able) {
      summaryTimer = setTimeout(() => (summary.hidden = true), wanted === false ? 120 : 0);
      return;
    }
    summaryTimer = setTimeout(() => {
      drawSummary();
      summary.hidden = false;
    }, 250);
  };
  // A finger has no hover: a tap opens the drawer.
  tab.addEventListener("pointerenter", (event) => showSummary(event.pointerType !== "touch"));
  tab.addEventListener("pointerleave", () => showSummary(false));
  summary.addEventListener("pointerenter", () => clearTimeout(summaryTimer));
  summary.addEventListener("pointerleave", () => showSummary(false));
  tab.addEventListener("focus", () => {
    if (tab.matches(":focus-visible")) showSummary(true);
  });
  tab.addEventListener("blur", () => showSummary(false));

  // A problem's parts, as a card or the drawer tells it.

  const frame = (excerpt, severity) => {
    if (!excerpt?.length) return null;
    return make(
      "div",
      { class: `frame ${severity}-frame` },
      excerpt.map((line) => {
        const at = line.text.some((piece) => "mark" in piece);
        return make(
          "div",
          { class: at ? "line at" : "line" },
          make("span", { class: "number" }, String(line.line)),
          make("span", { class: "text" }, pieces(line.text)),
          line.runs ? make("span", { class: "runs" }, pieces(line.runs)) : null,
        );
      }),
    );
  };

  const where = (file, line) => {
    if (!file) return null;
    return line == null ? file : told("place", { file, line });
  };

  const editorName = () => told(editors.find(([editor]) => editor === settings.editor)[1]);
  const opens = (file, line, column) => opener(settings.editor, panel?.root, file, line, column);

  // A file and its line as the panel shows it, linked to the reader's
  // editor unless `linked` is false, and a button that copies it as an
  // editor's go-to-file box takes it.
  const located = (file, line, column, { code = false, linked = true } = {}) => {
    const place = where(file, line);
    const href = linked ? opens(file, line, column) : null;
    const opening = told("openPlace", { place, editor: editorName() });
    const shown = href
      ? make(
          "a",
          { class: code ? "where code" : "where", href, title: opening, "aria-label": opening },
          place,
        )
      : make("span", { class: code ? "where code" : "where" }, place);
    const copy = make(
      "button",
      {
        class: "icon-button small",
        type: "button",
        title: told("copyPlace"),
        "aria-label": told("copyPlaceLabel", { place }),
      },
      icons.copy(),
    );
    let shownAgain = 0;
    copy.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(line == null ? file : `${file}:${line}`);
        refill(copy, icons.copied());
        copy.title = told("copied");
        copy.setAttribute("aria-label", told("copied"));
        clearTimeout(shownAgain);
        shownAgain = setTimeout(() => {
          refill(copy, icons.copy());
          copy.title = told("copyPlace");
          copy.setAttribute("aria-label", told("copyPlaceLabel", { place }));
        }, 2000);
      } catch {
        getSelection()?.selectAllChildren(shown);
      }
    });
    return make("span", { class: "located" }, shown, copy);
  };

  // A problem's file and line, and the link that opens it.
  const spot = (file, line, column) => {
    const href = opens(file, line, column);
    return make(
      "div",
      { class: "spot" },
      make("span", { class: "place" }, located(file, line, column, { linked: false })),
      href
        ? make(
            "a",
            {
              class: "open",
              href,
              title: told("openPlace", { place: where(file, line), editor: editorName() }),
            },
            icons.edit(),
            told("openInEditor"),
          )
        : null,
    );
  };

  const named = (name) => {
    for (const group of panel?.names || []) {
      const row = group.rows.find((row) => row.name === name);
      if (row) return { group: group.group, row };
    }
    return null;
  };

  const fix = (problem) => {
    if (!problem.suggestion) return null;
    const said = [pieces(problem.suggestion)];
    const found = problem.suggests ? named(problem.suggests) : null;
    if (found) {
      said.push(" ");
      said.push(
        found.row.empty ? say("holdsNothing") : say("holdsHere", { value: found.row.value }),
      );
    }
    return make(
      "div",
      { class: "fix" },
      make("p", {}, said),
      found
        ? make(
            "button",
            { class: "button", type: "button", onclick: () => showName(problem.suggests) },
            told("showInNames"),
          )
        : null,
    );
  };

  const builtWhile = (problem) => {
    if (!problem.built?.length) return null;
    const here = problem.built.findIndex((built) => {
      try {
        return built.href && new URL(built.href, location.origin).pathname === location.pathname;
      } catch {
        return false;
      }
    });
    const others = problem.built.filter((_, at) => at !== here);
    const named = others.map((built) => {
      const page = built.href ? link(built.href, pieces(built.page)) : spoken("span", built.page);
      if (!built.file) return page;
      const place = make("span", { class: "place" }, located(built.file, built.line));
      // A page with no name of its own is named by its file.
      return plain(built.page) === built.file ? place : [page, " ", place];
    });
    const listed = named.flatMap((made, at) => (at ? [", ", made] : [made]));
    const more = problem.builtMore ? [", ", say("builtMore", { n: problem.builtMore })] : [];
    if (here >= 0) {
      const n = others.length + (problem.builtMore || 0);
      if (!n) return make("p", { class: "meta" }, say("builtThis"));
      return make("p", { class: "meta" }, say("builtThisAnd", { n }), " ", listed);
    }
    return make("p", { class: "meta" }, say("builtOn"), " ", listed, more);
  };

  const explained = (problem) =>
    make(
      "p",
      { class: "meta" },
      woven("more", { link: link(problem.explanation, problem.explanation) }),
    );

  const inFull = (problem) => [
    make(
      "p",
      { class: "meta" },
      make("code", {}, problem.kind),
      problem.quietened ? [" ", say("quietened")] : null,
    ),
    problem.file ? spot(problem.file, problem.line, problem.column) : null,
    frame(problem.excerpt, problem.severity),
    fix(problem),
    problem.printed
      ? make(
          "div",
          { class: "printed" },
          spoken("p", problem.printed.heading),
          make("pre", {}, problem.printed.lines.join("\n")),
        )
      : null,
    builtWhile(problem),
    problem.related.map((related) =>
      make(
        "div",
        { class: "related" },
        make(
          "p",
          {},
          pieces(related.label),
          related.file
            ? [" ", make("span", { class: "place" }, located(related.file, related.line))]
            : null,
        ),
        frame(related.excerpt, "note"),
      ),
    ),
    explained(problem),
  ];

  // The card a stopped build opens.

  const drawCard = () => {
    const halted = stopped();
    const wanted =
      halted && settings.stops === "card" && settings.away !== halted.at && !settings.open;
    if (!wanted) {
      card.hidden = true;
      return;
    }
    const errors = halted.errors;
    view.error = Math.min(view.error, errors.length - 1);
    const error = errors[view.error];
    const step = (by) => {
      view.error = (view.error + by + errors.length) % errors.length;
      drawCard();
    };
    const header = make(
      "header",
      {},
      make(
        "div",
        { class: "said" },
        shape("error"),
        make("strong", {}, told("stoppedAt", { time: halted.at })),
        panel.round.builtAt ? make("span", { class: "kept" }, told("pageKept")) : null,
      ),
      make(
        "div",
        { class: "steps" },
        errors.length > 1
          ? [
              make(
                "span",
                { class: "kept meta" },
                told("atOf", { at: view.error + 1, of: errors.length }),
              ),
              make(
                "button",
                {
                  class: "icon-button",
                  type: "button",
                  "aria-label": told("previousError"),
                  onclick: () => step(-1),
                },
                icons.previous(),
              ),
              make(
                "button",
                {
                  class: "icon-button",
                  type: "button",
                  "aria-label": told("nextError"),
                  onclick: () => step(1),
                },
                icons.next(),
              ),
            ]
          : null,
        make(
          "button",
          { class: "icon-button", type: "button", "aria-label": told("putAway"), onclick: putAway },
          icons.away(),
        ),
      ),
    );
    const body = make(
      "div",
      { class: "body" },
      spoken("h2", error.headline),
      error.file ? spot(error.file, error.line, error.column) : null,
      frame(error.excerpt, "error"),
      fix(error),
      builtWhile(error),
      make(
        "div",
        { class: "ends" },
        make(
          "button",
          { class: "link", type: "button", onclick: () => showProblem(error) },
          told("explain"),
        ),
        make("span", { class: "fill" }),
        make(
          "button",
          { class: "link", type: "button", onclick: () => openOn("problems") },
          told("allProblems"),
        ),
      ),
    );
    card.setAttribute("aria-label", told("buildStopped"));
    refill(card, header, body);
    card.hidden = false;
    summary.hidden = true;
  };

  const putAway = () => {
    const halted = stopped();
    if (halted) settings.away = halted.at;
    keep();
    card.hidden = true;
    if (card.contains(root.activeElement)) visibleOpener().focus();
  };

  // The drawer.

  const panes = ["problems", "names", "page", "site", "search"];
  const tabs = {};
  const sections = {};
  const handle = make("div", { class: "handle", role: "separator", tabindex: "0" });
  const status = make("span", { class: "status" });
  const search = make("input", {
    type: "search",
    autocomplete: "off",
    spellcheck: "false",
    oninput: (event) => {
      if (settings.tab === "search") {
        searchFor(event.target.value, true);
        return;
      }
      view.query = event.target.value;
      drawSoon();
    },
  });
  const find = make(
    "label",
    { class: "find" },
    icons.find(),
    make("span", { class: "hidden-label" }),
    search,
    make("kbd", {}, "/"),
  );
  const settingsButton = make("button", {
    class: "icon-button",
    type: "button",
    "aria-expanded": "false",
    "aria-controls": "settings",
    onclick: () => showSettings(!view.settings),
  });
  const dockButton = make("button", {
    class: "icon-button dock",
    type: "button",
    onclick: () => {
      if (settings.sits === "side") settings.sits = settings.undocked;
      else [settings.undocked, settings.sits] = [settings.sits, "side"];
      keep();
      arrange();
      drawHead();
    },
  });
  const closeButton = make(
    "button",
    { class: "icon-button", type: "button", onclick: () => show(false, true) },
    icons.close(),
  );
  const tablist = make("div", { class: "tabs", role: "tablist" });
  const settingsBox = make("div", {
    class: "settings",
    id: "settings",
    role: "dialog",
    hidden: true,
  });

  for (const name of panes) {
    tabs[name] = make("button", {
      type: "button",
      role: "tab",
      id: `tab-${name}`,
      "aria-controls": `pane-${name}`,
      onclick: () => choose(name),
    });
    sections[name] = make("div", {
      class: `pane ${name}-pane`,
      role: "tabpanel",
      id: `pane-${name}`,
      "aria-labelledby": `tab-${name}`,
      hidden: true,
    });
    tablist.append(tabs[name]);
  }
  // Arrows move between the tabs, each shown as it is reached.
  tablist.addEventListener("keydown", (event) => {
    const at = panes.indexOf(settings.tab);
    const to = { ArrowRight: at + 1, ArrowLeft: at - 1, Home: 0, End: panes.length - 1 }[event.key];
    if (to === undefined) return;
    event.preventDefault();
    const name = panes[(to + panes.length) % panes.length];
    choose(name);
    tabs[name].focus();
  });
  drawer.append(
    handle,
    make(
      "header",
      { class: "head" },
      make("div", { class: "brand" }, mark(), make("strong", {})),
      tablist,
      make("span", { class: "fill" }),
      status,
      find,
      settingsButton,
      dockButton,
      closeButton,
    ),
    ...Object.values(sections),
    settingsBox,
  );

  // The drawer's height, or its width at the side, as the reader drags it.
  const resize = (px) => {
    const side = settings.sits === "side" && innerWidth >= 640;
    const most = side ? innerWidth : innerHeight - 24;
    const least = side ? 320 : 240;
    const clamped = Math.round(Math.max(least, Math.min(most, px)));
    if (side) settings.width = clamped;
    else settings.height = clamped;
    arrange();
    measure();
  };
  // How much of the window the drawer takes, for the handle to say.
  const measure = () => {
    const side = settings.sits === "side" && innerWidth >= 640;
    const share = side ? drawer.offsetWidth / innerWidth : drawer.offsetHeight / innerHeight;
    handle.setAttribute("aria-valuenow", String(Math.round(share * 100)));
  };
  handle.addEventListener("pointerdown", (event) => {
    handle.setPointerCapture(event.pointerId);
    const moved = (move) => {
      const side = settings.sits === "side" && innerWidth >= 640;
      resize(side ? innerWidth - move.clientX : innerHeight - move.clientY);
    };
    const done = () => {
      handle.removeEventListener("pointermove", moved);
      keep();
    };
    handle.addEventListener("pointermove", moved);
    handle.addEventListener("pointerup", done, { once: true });
    handle.addEventListener("pointercancel", done, { once: true });
  });
  handle.addEventListener("keydown", (event) => {
    const side = settings.sits === "side" && innerWidth >= 640;
    const now = side ? drawer.offsetWidth : drawer.offsetHeight;
    const by = event.shiftKey ? 64 : 16;
    const step = side
      ? { ArrowLeft: by, ArrowRight: -by }[event.key]
      : { ArrowUp: by, ArrowDown: -by }[event.key];
    if (!step) return;
    event.preventDefault();
    resize(now + step);
    keep();
  });

  const drawHead = () => {
    drawer.setAttribute("aria-label", told("name"));
    drawer.querySelector(".brand strong").textContent = told("name");
    tablist.setAttribute("aria-label", told("parts"));
    const side = settings.sits === "side";
    handle.setAttribute("aria-label", told("resize"));
    handle.setAttribute("aria-orientation", side ? "vertical" : "horizontal");
    handle.setAttribute("aria-valuemin", "0");
    handle.setAttribute("aria-valuemax", "100");
    for (const name of panes) {
      const chosen = settings.tab === name;
      tabs[name].setAttribute("aria-selected", String(chosen));
      tabs[name].tabIndex = chosen ? 0 : -1;
      sections[name].hidden = !chosen;
    }
    const worst = stopped()
      ? { severity: "error", n: stopped().errors.length }
      : severities
          .map((severity) => ({ severity, n: counts()[severity] }))
          .find((count) => count.n > 0);
    refill(
      tabs.problems,
      told("problems"),
      worst
        ? make(
            "span",
            { class: `badge ${worst.severity}`, "aria-hidden": "true" },
            grouped(worst.n),
          )
        : null,
    );
    tabs.problems.setAttribute(
      "aria-label",
      worst
        ? `${told("problems")}, ${told(plural[worst.severity], { n: worst.n })}`
        : told("problems"),
    );
    const total = (panel?.names || []).reduce((sum, group) => sum + group.rows.length, 0);
    refill(
      tabs.names,
      told("names"),
      total ? make("span", { class: "tally" }, grouped(total)) : null,
    );
    tabs.names.setAttribute(
      "aria-label",
      total ? `${told("names")} ${grouped(total)}` : told("names"),
    );
    tabs.page.textContent = told("page");
    tabs.site.textContent = told("site");
    tabs.search.textContent = told("search");
    const halted = stopped();
    status.className = halted ? "status stopped" : "status";
    status.textContent = building
      ? told("building")
      : halted
        ? panel.round.builtAt
          ? told("statusStopped", { time: halted.at, built: panel.round.builtAt })
          : told("statusStoppedFirst", { time: halted.at })
        : told("builtAt", { time: panel.round.at });
    const searching = settings.tab === "search";
    find.hidden = settings.tab !== "names" && !searching;
    find.querySelector(".hidden-label").textContent = told(searching ? "searchLabel" : "find");
    search.placeholder = told(searching ? "searchHow" : "findHow");
    const typed = searching ? view.searched : view.query;
    if (search.value !== typed) search.value = typed;
    settingsButton.setAttribute("aria-label", told("settings"));
    refill(settingsButton, icons.settings());
    dockButton.setAttribute("aria-label", side ? told("dockBottom") : told("dockSide"));
    refill(dockButton, side ? icons.bottom() : icons.side());
    closeButton.setAttribute("aria-label", told("close"));
  };

  // Each pane is drawn again only when what it shows has changed.
  const drawnFor = {};
  const drawPane = () => {
    const name = settings.tab;
    const key = [
      version,
      site?.version,
      JSON.stringify([
        view.scope,
        view.shown,
        view.quietened,
        view.kind,
        view.query,
        view.group,
        settings.hideEmpty,
        settings.onlyUsed,
        settings.editor,
        view.searched,
        view.result,
        searched.drawn,
      ]),
    ].join("|");
    if (drawnFor[name] === key) return;
    drawnFor[name] = key;
    ({
      problems: drawProblems,
      names: drawNames,
      page: drawPage,
      site: drawSite,
      search: drawSearch,
    })[name]();
  };
  let soon = 0;
  const drawSoon = () => {
    cancelAnimationFrame(soon);
    soon = requestAnimationFrame(() => drawPane());
  };

  const drawDrawer = () => {
    drawer.hidden = !settings.open;
    if (!settings.open) return;
    drawHead();
    drawPane();
    measure();
    if (view.settings) drawSettings();
  };

  const choose = (name) => {
    settings.tab = name;
    keep();
    drawHead();
    drawPane();
  };

  const visibleOpener = () => (bar.hidden ? tab : bar);

  const show = (wanted, byHand) => {
    clearTimeout(summaryTimer);
    settings.open = wanted;
    // What stopped the build is in the drawer, so the card stays put away.
    if (wanted && stopped()) settings.away = stopped().at;
    keep();
    if (!wanted) showSettings(false, false);
    summary.hidden = true;
    drawTab();
    drawCard();
    drawDrawer();
    room();
    if (!byHand) return;
    if (wanted) tabs[settings.tab].focus();
    else visibleOpener().focus();
  };
  tab.addEventListener("click", () => show(true, true));
  bar.addEventListener("click", () => show(true, true));

  const openOn = (name) => {
    settings.tab = name;
    show(true, true);
  };

  const showProblem = (problem) => {
    view.scope = "page";
    view.kind = null;
    view.chosen = keyOf(problem);
    view.opened.add(keyOf(problem));
    view.opened.add(`kind:${problem.kind}`);
    delete drawnFor.problems;
    openOn("problems");
  };

  const showName = (name) => {
    const found = named(name);
    if (!found) return;
    view.query = name;
    view.name = name;
    delete drawnFor.names;
    openOn("names");
  };

  // Problems.

  const keyOf = (problem) =>
    [problem.kind, problem.file, problem.line, problem.column, plain(problem.headline)].join("|");

  // The site's problems, asked for once a round, when first wanted.
  const siteProblems = () => {
    if (site?.version === version) return site.answer;
    if (site?.asking === version) return null;
    site = { asking: version };
    const asked = version;
    fetch(new URL("problems", live), { cache: "no-store" })
      .then((response) => (response.ok ? response.json() : null))
      .then((answer) => {
        if (asked !== version || !answer) return;
        site = { version, answer };
        if (settings.open && settings.tab === "problems") drawPane();
      })
      .catch(() => {});
    return null;
  };

  const drawProblems = () => {
    const pane = sections.problems;
    const halted = stopped()?.errors || [];
    const kinds = panel.site.kinds;
    const wanted = (problem) => view.quietened || !problem.quietened;
    const pageList = [...halted, ...panel.problems];
    const siteCount =
      halted.length +
      kinds.reduce((sum, kind) => sum + kind.count - (view.quietened ? 0 : kind.quietened), 0);
    const answer = view.scope === "site" ? siteProblems() : null;
    const said = { ...panel.explanations, ...answer?.explanations };
    const scoped =
      view.scope === "site" ? (answer ? [...halted, ...answer.problems] : null) : pageList;
    const inScope = scoped ? scoped.filter(wanted) : [];
    // Before the site's problems are here, its counts by kind stand in.
    const count = (severity) => {
      if (scoped) return inScope.filter((problem) => problem.severity === severity).length;
      const ofKinds = kinds.filter((kind) => kind.severity === severity);
      const counted = ofKinds.reduce(
        (sum, kind) => sum + kind.count - (view.quietened ? 0 : kind.quietened),
        0,
      );
      return counted + (severity === "error" ? halted.length : 0);
    };
    const scope = (name, n) =>
      make(
        "button",
        {
          class: "choice",
          type: "button",
          "aria-pressed": String(view.scope === name),
          onclick: () => {
            view.scope = name;
            drawPane();
          },
        },
        make("span", {}, told(name === "page" ? "onThisPage" : "wholeSite")),
        make("span", { class: "muted" }, grouped(n)),
      );
    const filter = (severity) =>
      make(
        "label",
        { class: "check" },
        make("input", {
          type: "checkbox",
          checked: view.shown[severity],
          onchange: (event) => {
            view.shown[severity] = event.target.checked;
            drawPane();
          },
        }),
        shape(severity),
        make("span", { class: "fill" }, told(`${plural[severity]}Shown`)),
        make("span", { class: "muted" }, grouped(count(severity))),
      );
    const gone = panel.site.gone ? pieces(panel.site.gone) : say("goneNone");
    const only = view.kind ? kinds.find((kind) => kind.kind === view.kind) : null;
    const rail = make(
      "div",
      { class: "rail" },
      scope("page", pageList.filter(wanted).length),
      scope("site", siteCount),
      make("hr"),
      severities.map(filter),
      make(
        "label",
        { class: "check muted" },
        make("input", {
          type: "checkbox",
          checked: view.quietened,
          onchange: (event) => {
            view.quietened = event.target.checked;
            drawPane();
          },
        }),
        make("span", { class: "fill" }, say("quietened")),
      ),
      only
        ? make(
            "button",
            {
              class: "choice",
              type: "button",
              "aria-pressed": "true",
              onclick: () => {
                view.kind = null;
                drawPane();
              },
            },
            make("span", {}, told("onlyKind", { title: only.title })),
            make("span", { class: "muted" }, told("everyKind")),
          )
        : null,
      make("span", { class: "fill" }),
      make("p", { class: "meta" }, gone),
    );

    const list = make("div", { class: "main" });
    const shown = inScope.filter(
      (problem) => view.shown[problem.severity] && (!view.kind || problem.kind === view.kind),
    );
    if (!scoped) {
      list.append(make("p", { class: "empty" }, told("asking")));
    } else if (!shown.length) {
      const clear = !inScope.length;
      list.append(
        make(
          "p",
          { class: "empty" },
          clear ? told(view.scope === "page" ? "pageClear" : "siteClear") : told("filteredOut"),
        ),
      );
    } else {
      const groups = new Map();
      for (const problem of shown) {
        if (!groups.has(problem.kind)) groups.set(problem.kind, []);
        groups.get(problem.kind).push(problem);
      }
      const keys = shown.map(keyOf);
      if (!keys.some((key) => view.opened.has(key))) {
        view.opened.add(keys[0]);
        view.opened.add(`kind:${shown[0].kind}`);
      }
      if (!keys.includes(view.chosen))
        view.chosen = keys.find((key) => view.opened.has(key)) || keys[0];
      const items = make("div", { class: "problems" });
      for (const [kind, problems] of groups) {
        if (problems.length === 1) {
          items.append(problemCard(problems[0]));
          continue;
        }
        const key = `kind:${kind}`;
        const open = view.opened.has(key);
        const title =
          kinds.find((counted) => counted.kind === kind)?.title || said[kind]?.title || kind;
        const group = make(
          "section",
          { class: `problem group ${problems[0].severity}` },
          make(
            "h3",
            {},
            make(
              "button",
              {
                type: "button",
                "aria-expanded": String(open),
                onclick: () => {
                  if (open) view.opened.delete(key);
                  else view.opened.add(key);
                  drawProblems();
                },
              },
              chevron(),
              shape(problems[0].severity),
              make("span", { class: "headline" }, told("times", { title, n: problems.length })),
            ),
          ),
          open ? make("div", { class: "problems" }, problems.map(problemCard)) : null,
        );
        items.append(group);
      }
      if (view.scope === "site" && answer?.more) {
        items.append(make("p", { class: "meta" }, say("moreProblems", { n: answer.more })));
      }
      list.append(items);
    }
    const chosen = shown.find((problem) => keyOf(problem) === view.chosen);
    refill(pane, rail, list, explanation(chosen, said));
  };

  const chevron = () => {
    const made = icons.next();
    made.classList.add("chevron");
    return made;
  };

  const problemCard = (problem) => {
    const key = keyOf(problem);
    const open = view.opened.has(key);
    const toggle = () => {
      if (open) view.opened.delete(key);
      else view.opened.add(key);
      view.chosen = key;
      drawProblems();
      sections.problems.querySelector(`[data-key="${CSS.escape(key)}"] h3 button`)?.focus();
    };
    return make(
      "article",
      {
        class: `problem ${problem.severity}${view.chosen === key ? " chosen" : ""}`,
        "data-key": key,
      },
      make(
        "h3",
        {},
        make(
          "button",
          { type: "button", "aria-expanded": String(open), onclick: toggle },
          chevron(),
          shape(problem.severity),
          spoken("span", problem.headline, { class: "headline" }),
          !open && problem.file
            ? make("span", { class: "place" }, where(problem.file, problem.line))
            : null,
        ),
      ),
      open ? make("div", { class: "body" }, inFull(problem)) : null,
    );
  };

  const explanation = (problem, explanations) => {
    const said = problem && explanations[problem.kind];
    if (!said) return make("aside", { class: "side-pane explanation", hidden: true });
    const paragraph = (part) =>
      "code" in part ? make("pre", {}, part.code) : spoken("p", part.said);
    return make(
      "aside",
      { class: "side-pane explanation", "aria-label": told("aboutKind") },
      make("p", { class: "meta" }, say("fromExplain", { kind: problem.kind })),
      make("h3", {}, said.title),
      said.opening.map(paragraph),
      said.fix.length ? [make("h4", {}, told("howToFix")), said.fix.map(paragraph)] : null,
      make("span", { class: "fill" }),
      make("p", {}, link(problem.explanation, told("readOnline"))),
    );
  };

  // Names.

  // Ways a template can write a name, by what it holds, each with what
  // it is for. `{name}` stands for the name.
  const ways = {
    date: [
      ["{{ {name} }}", "wayAsWritten"],
      ['{{ {name} | date("2025") }}', "wayYear"],
      ['<time datetime="{{ {name} | moment }}">{{ {name} }}</time>', "wayTime"],
    ],
    list: [
      ["{% for post in {name}.posts %}\n  {{ headline(post) }}\n{% endfor %}", "wayEachPost"],
      ['<a href="{{ {name}.url }}">{{ {name}.title }}</a>', "wayLink"],
    ],
    items: [["{% for item in {name} %}\n  ...\n{% endfor %}", "wayEachItem"]],
    lists: [
      [
        '{% for list in {name} %}\n  <a href="{{ list.url }}">{{ list.name }}</a>\n{% endfor %}',
        "wayEachList",
      ],
    ],
    pages: [["{% for post in {name} %}\n  {{ headline(post) }}\n{% endfor %}", "wayEachPage"]],
    page: [
      [
        '{% if {name} %}\n  <a href="{{ {name}.url }}">{{ headline({name}) }}</a>\n{% endif %}',
        "wayLinkIf",
      ],
    ],
    "true or false": [["{% if {name} %}\n  ...\n{% endif %}", "wayTrue"]],
    HTML: [["{{ {name} }}", "wayHtml"]],
    webmentions: [["{% for face in {name}.faces %}\n  ...\n{% endfor %}", "wayFaces"]],
    "": [["{% if {name} %}{{ {name} }}{% endif %}", "wayHasOne"]],
    text: [["{{ {name} }}", "wayAsIs"]],
    number: [["{{ {name} }}", "wayAsIs"]],
  };
  const waysOf = (row) => {
    // A Statac list answers `.posts`; any other list is of plain items.
    const holds = row.holds === "list" && !row.has.includes("posts") ? "items" : row.holds;
    return (ways[holds] || []).map(([code, said]) => [code.replaceAll("{name}", row.name), said]);
  };
  const expression = (name) => `{{ ${name} }}`;

  const drawNames = () => {
    const pane = sections.names;
    if (!panel.names.length) {
      refill(pane, make("p", { class: "empty" }, told("noPage")));
      return;
    }
    const needle = view.query.trim().toLowerCase();
    const keepRow = (row) =>
      (!settings.hideEmpty || !row.empty || needle) &&
      (!settings.onlyUsed || row.used) &&
      (!needle ||
        row.name.toLowerCase().includes(needle) ||
        row.value.toLowerCase().includes(needle));
    const groups = panel.names.map((group) =>
      make(
        "button",
        {
          class: "choice stacked",
          type: "button",
          "aria-pressed": String(!needle && view.group === group.group),
          onclick: () => {
            view.group = group.group;
            view.query = "";
            search.value = "";
            drawPane();
          },
        },
        make("code", {}, group.group),
        make(
          "span",
          { class: "meta" },
          needle
            ? told("found", { n: group.rows.filter(keepRow).length })
            : told("held", { held: grouped(group.held), count: grouped(group.count) }),
        ),
      ),
    );
    const check = (setting, word, extra) =>
      make(
        "label",
        { class: "check" },
        make("input", {
          type: "checkbox",
          checked: settings[setting],
          onchange: (event) => {
            settings[setting] = event.target.checked;
            keep();
            drawPane();
          },
        }),
        make("span", {}, told(word), extra),
      );
    const layout = page()?.layout;
    const rail = make(
      "div",
      { class: "rail" },
      groups,
      make("hr"),
      check("hideEmpty", "hideEmpty"),
      check("onlyUsed", "onlyUsed", [
        " ",
        make("span", { class: "dot used", "aria-hidden": "true" }),
      ]),
      make("span", { class: "fill" }),
      layout?.file
        ? make(
            "p",
            { class: "meta" },
            woven(
              "layoutFrom",
              { file: located(layout.file, null, null, { code: true }) },
              { name: layout.name },
            ),
          )
        : null,
    );
    const rows = (needle ? panel.names : panel.names.filter((group) => group.group === view.group))
      .flatMap((group) => group.rows)
      .filter(keepRow);
    if (!rows.some((row) => row.name === view.name)) view.name = rows[0]?.name ?? null;
    const body = make("tbody");
    for (const row of rows) body.append(nameRow(row));
    const main = make("div", { class: "main" });
    if (rows.length) {
      const column = (name, className) =>
        make("th", { scope: "col", class: className }, told(name));
      main.append(
        make(
          "table",
          { class: "names" },
          make(
            "colgroup",
            {},
            make("col", { class: "write" }),
            make("col", { class: "holds" }),
            make("col"),
            make("col", { class: "at" }),
          ),
          make(
            "thead",
            {},
            make(
              "tr",
              {},
              column("columnWrite"),
              column("columnHolds"),
              column("columnHere"),
              column("columnAt"),
            ),
          ),
          body,
        ),
      );
    } else {
      main.append(
        make(
          "p",
          { class: "empty" },
          needle ? say("noMatch", { query: view.query.trim() }) : told("noneLeft"),
        ),
      );
    }
    const detail = make("aside", { class: "side-pane detail", "aria-label": told("chosen") });
    refill(pane, rail, main, detail);
    drawDetail();
  };

  const nameRow = (row) => {
    const chosen = row.name === view.name;
    const button = make(
      "button",
      { type: "button", "aria-pressed": String(chosen), tabindex: chosen ? "0" : "-1" },
      make("span", { class: row.used ? "dot used" : "dot", "aria-hidden": "true" }),
      make("code", {}, expression(row.written ?? row.name)),
    );
    const value = make(
      "td",
      { class: row.empty || row.holds === "call" ? "value none" : "value" },
      row.value,
    );
    value.title = row.value;
    const at = row.from ? where(row.from.file, row.from.line) : "";
    const made = make(
      "tr",
      {
        class: chosen ? "name-row chosen" : "name-row",
        "data-name": row.name,
        onclick: () => pick(row.name, false),
      },
      make("td", { class: "write" }, button),
      make("td", { class: "holds" }, row.holds || told("holdsUnknown")),
      value,
      make("td", { class: "at", title: at }, row.from ? located(row.from.file, row.from.line) : ""),
    );
    reachable(made, chosen);
    return made;
  };

  // Only the chosen row's place is reached by Tab, as only its name is.
  const reachable = (row, chosen) => {
    for (const made of row.querySelectorAll(".at a, .at button")) made.tabIndex = chosen ? 0 : -1;
  };

  const pick = (name, focus) => {
    const body = sections.names.querySelector("tbody");
    if (!body) return;
    for (const row of body.rows) {
      const chosen = row.dataset.name === name;
      row.classList.toggle("chosen", chosen);
      const button = row.querySelector("button");
      button.setAttribute("aria-pressed", String(chosen));
      button.tabIndex = chosen ? 0 : -1;
      reachable(row, chosen);
      if (chosen && focus) button.focus();
    }
    view.name = name;
    drawDetail();
  };

  // Up and down move between the names, each chosen as it is reached.
  sections.names.addEventListener("keydown", (event) => {
    const row = event.target.closest?.("tr.name-row");
    if (!row) return;
    const rows = [...row.parentElement.rows];
    const at = rows.indexOf(row);
    const to = { ArrowDown: at + 1, ArrowUp: at - 1, Home: 0, End: rows.length - 1 }[event.key];
    if (to === undefined || !rows[to]) return;
    event.preventDefault();
    pick(rows[to].dataset.name, true);
  });

  const drawDetail = () => {
    const detail = sections.names.querySelector(".detail");
    if (!detail) return;
    const found = view.name ? named(view.name) : null;
    if (!found) {
      detail.hidden = true;
      return;
    }
    detail.hidden = false;
    const row = found.row;
    const copy = make(
      "button",
      { class: "button", type: "button" },
      icons.copy(),
      make("span", {}, told("copy")),
    );
    copy.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(expression(row.written ?? row.name));
        copy.lastChild.textContent = told("copied");
      } catch {
        // Where the browser won't write to the clipboard, the expression
        // is left chosen for the reader to copy.
        const selection = getSelection();
        selection?.selectAllChildren(detail.querySelector(".expression code"));
      }
    });
    const ways = waysOf(row);
    refill(
      detail,
      make(
        "div",
        { class: "expression" },
        make("code", {}, expression(row.written ?? row.name)),
        copy,
      ),
      make(
        "dl",
        {},
        make("dt", {}, told("columnHolds")),
        make("dd", {}, row.holds || told("holdsUnknown")),
        make("dt", {}, told("here")),
        make("dd", { class: row.empty ? "none" : "" }, row.value),
        make("dt", {}, told("page")),
        make("dd", {}, told(row.used ? "uses" : "usesNot")),
        row.from
          ? [
              make("dt", {}, told("columnAt")),
              make(
                "dd",
                { class: "written-at" },
                make("div", { class: "code" }, located(row.from.file, row.from.line)),
                frame(row.from.excerpt, "note"),
              ),
            ]
          : null,
      ),
      row.has.length
        ? make(
            "div",
            { class: "way" },
            make("h3", {}, told("answers")),
            make(
              "div",
              { class: "chips" },
              row.has.map((name) => make("code", {}, `${row.name}.${name}`)),
            ),
          )
        : null,
      ways.length
        ? [
            make("h3", {}, told("ways")),
            ways.map(([code, said]) =>
              make(
                "div",
                { class: "way" },
                make("pre", {}, code),
                make("span", { class: "meta" }, told(said)),
              ),
            ),
          ]
        : null,
    );
  };

  // Search.

  // What serve last answered, the words and round it answered for, and the
  // round last asked about.
  const searched = { drawn: 0, asked: 0, answer: null, words: "", typing: false, round: -1 };
  const searchStatus = make("p", { class: "search-status", role: "status" });
  const searchFound = make("div");
  const searchMain = make("div", { class: "main" }, searchStatus, searchFound);
  const searchWhy = make("aside", { class: "side-pane why", hidden: true });
  const placeWords = {
    title: "placeTitle",
    heading: "placeHeading",
    tags: "placeTags",
    summary: "placeSummary",
    pictures: "placePictures",
    text: "placeText",
  };

  const searchFor = (words, typing) => {
    view.searched = words;
    if (search.value !== words) search.value = words;
    searched.typing = typing;
    keepSearch();
    askSearch();
  };

  const askSearch = () => {
    const words = view.searched;
    const mine = ++searched.asked;
    searched.round = version;
    const asked = [
      `words=${encodeURIComponent(words)}`,
      `typing=${searched.typing ? 1 : 0}`,
      `page=${encodeURIComponent(location.pathname)}`,
    ];
    fetch(new URL(`search?${asked.join("&")}`, live), { cache: "no-store" })
      .then((response) => (response.ok ? response.json() : null))
      .then((answer) => {
        // A later search has begun, and its answer is the one to show.
        if (mine !== searched.asked || !answer) return;
        Object.assign(searched, { answer, words });
        searched.drawn += 1;
        if (settings.open && settings.tab === "search") drawPane();
      })
      .catch(() => {});
  };

  const searchResults = () =>
    (searched.answer?.languages || []).flatMap((language) =>
      language.results.map((result) => ({ result, language })),
    );

  const drawSearch = () => {
    const pane = sections.search;
    if (!pane.contains(searchMain)) refill(pane, searchMain, searchWhy);
    searchWhy.setAttribute("aria-label", told("whyRanked"));
    // Each round may find otherwise, so each is asked again.
    if (searched.round !== version) askSearch();
    const answer = searched.answer;
    const languages = answer?.languages;
    if (!searched.words.trim() || !languages) {
      refill(searchStatus);
      refill(
        searchFound,
        make(
          "div",
          { class: "empty" },
          make("p", {}, told("searchStart")),
          answer?.drafts ? make("p", {}, told("searchDrafts")) : null,
        ),
      );
      searchWhy.hidden = true;
      return;
    }
    const pages = languages.reduce((sum, language) => sum + language.pages, 0);
    const suggestion = languages.find((language) => language.suggestion)?.suggestion;
    if (pages) {
      const some = languages.some((language) => language.some && language.pages);
      refill(searchStatus, told("pagesFound", { n: pages }), some ? ` ${told("foundSome")}` : null);
    } else {
      const meant = make(
        "button",
        { class: "link", type: "button", onclick: () => searchFor(suggestion, false) },
        suggestion,
      );
      refill(
        searchStatus,
        told("foundNone"),
        suggestion ? [" ", woven("meant", { words: meant })] : null,
      );
    }
    const found = searchResults();
    if (!found.some(({ result }) => result.href === view.result)) {
      view.result = found[0]?.result.href ?? null;
      keepSearch();
    }
    refill(
      searchFound,
      languages.map((language) => {
        if (!language.results.length) return null;
        const more = language.pages - language.results.length;
        return [
          languages.length > 1
            ? make("h3", { class: "language" }, make("code", {}, language.code))
            : null,
          make("ol", { class: "found" }, language.results.map(resultRow)),
          more > 0 ? make("p", { class: "meta more" }, told("moreFound", { n: more })) : null,
        ];
      }),
    );
    drawWhy();
  };

  const leftOutLabel = (result) => {
    const said = result.draft
      ? told("draft")
      : result.waiting
        ? told("waits", { day: result.waiting })
        : null;
    return said ? make("span", { class: "label" }, said) : null;
  };

  // A result's excerpt, its words matched marked, with what stands between
  // the blocks of text it runs across.
  const excerptOf = (excerpt) => {
    let before = "";
    return excerpt.map((piece) => {
      const text = piece.text.replace(/\n/g, (_, at, whole) => {
        const last = whole[at - 1] ?? before;
        return ".,:;!?…。、，：；！？".includes(last) ? " " : " · ";
      });
      before = text.at(-1) ?? before;
      return piece.marked ? make("mark", {}, text) : text;
    });
  };

  const resultRow = (result) => {
    const chosen = result.href === view.result;
    return make(
      "li",
      { class: chosen ? "result chosen" : "result", "data-href": result.href },
      make(
        "button",
        {
          type: "button",
          "aria-pressed": String(chosen),
          tabindex: chosen ? "0" : "-1",
          onclick: () => pickResult(result.href, false),
        },
        make("span", { class: "title" }, result.title),
        make(
          "span",
          { class: "meta part" },
          make("span", {}, result.heading ?? told("opening")),
          result.day ? make("span", {}, result.day) : null,
          leftOutLabel(result),
        ),
        result.excerpt.length
          ? make("span", { class: "excerpt" }, excerptOf(result.excerpt))
          : null,
      ),
    );
  };

  const pickResult = (href, focus) => {
    for (const row of sections.search.querySelectorAll("li.result")) {
      const chosen = row.dataset.href === href;
      row.classList.toggle("chosen", chosen);
      const button = row.querySelector("button");
      button.setAttribute("aria-pressed", String(chosen));
      button.tabIndex = chosen ? 0 : -1;
      if (chosen && focus) button.focus();
    }
    view.result = href;
    keepSearch();
    drawWhy();
  };

  // Up and down move between the results, each chosen as it is reached.
  sections.search.addEventListener("keydown", (event) => {
    const row = event.target.closest?.("li.result");
    if (!row) return;
    const rows = [...sections.search.querySelectorAll("li.result")];
    const at = rows.indexOf(row);
    const to = { ArrowDown: at + 1, ArrowUp: at - 1, Home: 0, End: rows.length - 1 }[event.key];
    if (to === undefined || !rows[to]) return;
    event.preventDefault();
    pickResult(rows[to].dataset.href, true);
  });

  const drawWhy = () => {
    const chosen = searchResults().find(({ result }) => result.href === view.result);
    if (!chosen) {
      searchWhy.hidden = true;
      return;
    }
    searchWhy.hidden = false;
    const { result, language } = chosen;
    const score = (n) => n.toFixed(2);
    const word = (asked) =>
      make(
        "section",
        { class: "word" },
        make(
          "div",
          { class: "top" },
          make("code", {}, asked.word),
          asked.forms.length
            ? make("span", { class: "meta" }, told("adds", { score: score(asked.score) }))
            : null,
        ),
        asked.forms.length
          ? [
              make(
                "p",
                {},
                woven("foundAs", {
                  forms: make(
                    "span",
                    {},
                    asked.forms.flatMap((form, at) => [
                      at ? ", " : null,
                      make("code", {}, form),
                    ]),
                  ),
                }),
              ),
              make(
                "table",
                { class: "places" },
                make(
                  "thead",
                  {},
                  make(
                    "tr",
                    {},
                    ["columnWhere", "columnTimes", "columnCounts"].map((name) =>
                      make("th", { scope: "col" }, told(name)),
                    ),
                  ),
                ),
                make(
                  "tbody",
                  {},
                  asked.places.map((place) =>
                    make(
                      "tr",
                      {},
                      make("th", { scope: "row" }, told(placeWords[place.place])),
                      make("td", {}, grouped(place.times)),
                      make("td", {}, grouped(place.counts)),
                    ),
                  ),
                ),
              ),
              make(
                "p",
                { class: "meta" },
                told("holding", { n: asked.holding, count: grouped(language.parts) }),
              ),
              asked.starting ? make("p", { class: "meta" }, told("starting")) : null,
            ]
          : make("p", { class: "muted" }, told("notHere")),
      );
    refill(
      searchWhy,
      make("h3", { class: "why-title" }, link(result.href, result.title)),
      make(
        "p",
        { class: "meta part" },
        make("span", {}, result.heading ?? told("opening")),
        result.day ? make("span", {}, result.day) : null,
        leftOutLabel(result),
      ),
      result.file ? make("p", { class: "code" }, located(result.file, null)) : null,
      make("h4", {}, told("whyRanked")),
      make(
        "p",
        { class: "sum" },
        make("span", {}, told("wordsHeld", { n: result.matched, count: result.words.length })),
        make("span", {}, told("score", { score: score(result.score) })),
      ),
      make("p", { class: "meta" }, told("rankRule")),
      result.words.map(word),
      make(
        "p",
        { class: "meta length" },
        told("partLength", {
          n: result.length,
          average: grouped(Math.round(language.average)),
        }),
      ),
    );
  };

  // This page.

  const drawPage = () => {
    const pane = sections.page;
    const facts = page();
    if (!facts) {
      refill(pane, make("p", { class: "empty" }, told("noPage")));
      return;
    }
    const left = facts.draft
      ? say("isDraft")
      : facts.waiting
        ? say("isWaiting", { day: facts.waiting })
        : say("published");
    const layout = facts.layout;
    let chosen = null;
    if (layout?.chosenAt) {
      const at = located(layout.chosenAt.file, layout.chosenAt.line, null, { code: true });
      chosen = woven(
        layout.handedDown ? "layoutHanded" : "layoutNamed",
        { place: at },
        { name: layout.name },
      );
    } else if (layout) {
      chosen = say("layoutUnnamed", { name: layout.name });
    }
    const chain = layout?.file
      ? make(
          "span",
          { class: "chain" },
          located(layout.file, null, null, { code: true }),
          layout.extends.map((file) => [
            make("span", { class: "muted" }, told("extends")),
            located(file, null, null, { code: true }),
          ]),
        )
      : null;
    const first = make(
      "div",
      { class: "column" },
      make(
        "div",
        { class: "chain" },
        make("h2", {}, facts.title || facts.address),
        make("span", { class: "label" }, left),
      ),
      make(
        "dl",
        { class: "facts" },
        make("dt", {}, told("file")),
        make(
          "dd",
          {},
          facts.file ? located(facts.file, null, null, { code: true }) : say("noFile"),
        ),
        make("dt", {}, told("address")),
        make("dd", {}, make("code", {}, facts.address)),
        make("dt", {}, told("layout")),
        make(
          "dd",
          {},
          layout
            ? [chain, chosen ? make("span", { class: "muted" }, chosen) : null]
            : say("noLayout"),
        ),
        make("dt", {}, told("language")),
        make(
          "dd",
          {},
          told(facts.language.direction === "rtl" ? "rightToLeft" : "leftToRight", {
            tag: facts.language.tag,
          }),
        ),
        make("dt", {}, told("feeds")),
        make(
          "dd",
          {},
          facts.feeds.length ? facts.feeds.map((feed) => make("code", {}, feed)) : say("noFeed"),
        ),
      ),
    );
    const second = make(
      "div",
      { class: "column" },
      make(
        "h3",
        {},
        facts.lists.length ? told("inLists", { n: facts.lists.length }) : told("inNoList"),
      ),
      facts.lists.map((list) =>
        make(
          "div",
          { class: "list-card" },
          make(
            "div",
            { class: "top" },
            spoken("span", list.said),
            make(
              "span",
              { class: "muted" },
              told("atOf", { at: grouped(list.at), of: grouped(list.of) }),
            ),
          ),
          list.previous || list.next
            ? make(
                "div",
                { class: "ends meta" },
                list.previous
                  ? link(list.previous.href, told("previous", { title: list.previous.text }))
                  : make("span"),
                list.next ? link(list.next.href, told("next", { title: list.next.text })) : null,
              )
            : null,
        ),
      ),
    );
    const own = panel.problems.filter((problem) => !problem.quietened);
    const tallies = severities
      .map((severity) => ({
        severity,
        n: own.filter((problem) => problem.severity === severity).length,
      }))
      .filter((count) => count.n);
    const third = make(
      "div",
      { class: "column" },
      make("h3", {}, told("rebuilt")),
      make("p", { class: "muted" }, told("rebuiltHow")),
      make(
        "ul",
        { class: "files" },
        facts.reads.files.map((file) => make("li", {}, located(file))),
        facts.reads.also.map((said) => spoken("li", said, { class: "also" })),
      ),
      make("span", { class: "fill" }),
      tallies.length
        ? tallies.map(({ severity, n }) =>
            make(
              "p",
              { class: "tally-line" },
              shape(severity),
              make("span", { class: "fill" }, told(`${plural[severity]}Here`, { n })),
              make(
                "button",
                {
                  class: "link",
                  type: "button",
                  onclick: () => {
                    view.scope = "page";
                    view.kind = null;
                    choose("problems");
                  },
                },
                told("show"),
              ),
            ),
          )
        : make("p", { class: "tally-line muted" }, told("pageClear")),
    );
    refill(pane, make("div", { class: "columns" }, first, second, third));
  };

  // The site.

  const drawSite = () => {
    const pane = sections.site;
    const round = panel.round;
    const changed = round.changed;
    const opening =
      changed.length === 0
        ? []
        : changed.length === 1
          ? say("changedOne", { file: changed[0] })
          : changed.length === 2
            ? say("changedTwo", { file: changed[0], other: changed[1] })
            : say("changedMany", { file: changed[0], n: changed.length - 1 });
    const parts = [
      ["before", "tookBefore"],
      ["reading", "tookReading"],
      ["comparing", "tookComparing"],
      ["building", "tookBuilding"],
      ["concluding", "tookConcluding"],
      ["writing", "tookWriting"],
    ];
    const ms = (n) => told("ms", { n: n >= 10 ? Math.round(n) : n });
    const took = round.took;
    const longest = Math.max(...parts.map(([key]) => took[key]));
    const first = make(
      "div",
      { class: "column" },
      make("h3", {}, told("lastRound")),
      make("p", {}, opening, opening.length ? " " : null, pieces(panel.rounds[0]?.said)),
      make(
        "dl",
        { class: "took" },
        parts.map(([key, word]) =>
          make(
            "div",
            {},
            make("dt", {}, told(word)),
            make(
              "dd",
              { "aria-hidden": "true" },
              make("span", {
                class: took[key] > 0 ? "length some" : "length",
                style: `width: ${longest > 0 ? (took[key] / longest) * 100 : 0}%`,
              }),
            ),
            make("dd", { class: "code" }, ms(took[key])),
          ),
        ),
      ),
      make("h3", { class: "later" }, told("rounds")),
      make(
        "ol",
        { class: "rounds" },
        panel.rounds.map((round) => {
          const worst = severities.find((severity) => round.counts[severity] > 0);
          return make(
            "li",
            {},
            make("span", { class: "muted" }, round.at),
            spoken("span", round.said),
            make(
              "span",
              { class: "end" },
              worst
                ? make("span", { class: worst }, told(plural[worst], { n: round.counts[worst] }))
                : null,
              round.first ? make("span", { class: "muted" }, told("firstRound")) : null,
            ),
          );
        }),
      ),
    );
    const kinds = panel.site.kinds;
    const second = make(
      "div",
      { class: "column" },
      make("h3", {}, told("acrossSite")),
      kinds.length
        ? make(
            "ul",
            { class: "kinds" },
            kinds.map((kind) =>
              make(
                "li",
                {},
                shape(kind.severity),
                make(
                  "span",
                  {},
                  told("times", { title: kind.title, n: kind.count }),
                  kind.quietened ? [" ", make("span", { class: "meta" }, say("quietened"))] : null,
                ),
                make(
                  "button",
                  {
                    class: "link",
                    type: "button",
                    onclick: () => {
                      view.scope = "site";
                      view.kind = kind.kind;
                      if (kind.quietened === kind.count) view.quietened = true;
                      choose("problems");
                    },
                  },
                  told("show"),
                ),
                kind.suggestion ? spoken("p", kind.suggestion, { class: "muted" }) : null,
              ),
            ),
          )
        : make("p", { class: "muted" }, told("siteClear")),
    );
    // A page left out has no address, only the file it is written in.
    const linked = (links) =>
      make(
        "ul",
        { class: "links" },
        links.map((to) =>
          make(
            "li",
            {},
            "href" in to
              ? link(to.href, to.text)
              : [
                  to.text !== to.file ? make("span", { class: "code" }, to.text) : null,
                  to.file ? located(to.file, to.line, null, { code: true }) : null,
                ],
            to.day ? make("span", { class: "meta" }, to.day) : null,
          ),
        ),
      );
    const { drafts, waiting, written } = panel.site;
    const unbuilt = [...drafts, ...waiting].some((to) => !("href" in to));
    const third = make(
      "div",
      { class: "column" },
      make("h3", {}, say("leftOut")),
      drafts.length || waiting.length
        ? [
            drafts.length ? [make("p", { class: "muted" }, told("drafts")), linked(drafts)] : null,
            waiting.length
              ? [make("p", { class: "muted" }, told("waiting")), linked(waiting)]
              : null,
            unbuilt ? make("p", { class: "muted" }, say("leftOutUnbuilt")) : null,
          ]
        : make("p", { class: "muted" }, say("leftOutNone")),
      make("h3", { class: "later" }, told("written")),
      written.length
        ? make(
            "ul",
            { class: "links" },
            written.map((line) => spoken("li", line)),
          )
        : make("p", { class: "muted" }, say("writtenNone")),
    );
    refill(pane, make("div", { class: "columns" }, first, second, third));
  };

  // Settings.

  const modifiers = ["alt", "shift", "ctrl", "meta"];
  const shortcutKeys = () => {
    const keys = modifiers.filter((key) => settings.shortcut[key]).map((key) => told(key));
    keys.push(settings.shortcut.code.replace(/^Key|^Digit/, ""));
    return keys;
  };
  const isShortcut = (event) =>
    event.code === settings.shortcut.code &&
    modifiers.every((key) => event[`${key}Key`] === Boolean(settings.shortcut[key]));

  const showSettings = (wanted, focus = true) => {
    view.settings = wanted;
    view.capturing = false;
    settingsButton.setAttribute("aria-expanded", String(wanted));
    settingsBox.hidden = !wanted;
    if (wanted) {
      drawSettings();
      if (focus) settingsBox.querySelector("button")?.focus();
    } else if (focus && settingsBox.contains(root.activeElement)) {
      settingsButton.focus();
    }
  };

  const drawSettings = () => {
    const focused = root.activeElement?.dataset?.setting;
    const segments = (label, setting, choices, then) =>
      make(
        "fieldset",
        {},
        make("legend", {}, told(label)),
        make(
          "div",
          { class: choices.length > 3 ? "segments four" : "segments" },
          choices.map(([value, word]) =>
            make(
              "button",
              {
                type: "button",
                "data-setting": `${setting}-${value}`,
                "aria-pressed": String(settings[setting] === value),
                onclick: () => {
                  if (setting === "sits" && settings.sits !== "side" && value === "side") {
                    settings.undocked = settings.sits;
                  }
                  settings[setting] = value;
                  keep();
                  arrange();
                  then?.();
                  drawSettings();
                },
              },
              told(word),
            ),
          ),
        ),
      );
    const redraw = () => {
      drawTab();
      drawHead();
      room();
    };
    const keys = make("span", { class: "keys" });
    if (view.capturing) keys.append(make("span", { class: "meta" }, told("pressKeys")));
    else keys.append(...shortcutKeys().map((key) => make("kbd", {}, key)));
    settingsBox.setAttribute("aria-label", told("settingsLabel"));
    refill(
      settingsBox,
      segments(
        "sits",
        "sits",
        [
          ["corner", "corner"],
          ["bar", "bar"],
          ["side", "side"],
        ],
        redraw,
      ),
      settings.sits === "bar"
        ? null
        : segments(
            "whichCorner",
            "corner",
            [
              ["bottom-left", "bottomLeft"],
              ["bottom-right", "bottomRight"],
              ["top-left", "topLeft"],
              ["top-right", "topRight"],
            ],
            redraw,
          ),
      segments("colours", "colours", [
        ["system", "system"],
        ["light", "light"],
        ["dark", "dark"],
      ]),
      make(
        "div",
        { class: "field" },
        make("label", { for: "editor" }, told("openIn")),
        make(
          "select",
          {
            id: "editor",
            "data-setting": "editor",
            "aria-describedby": settings.editor === "sublime" ? "editor-note" : null,
            onchange: (event) => {
              settings.editor = event.target.value;
              keep();
              drawCard();
              if (settings.open) drawPane();
              drawSettings();
            },
          },
          editors.map(([value, word]) =>
            make("option", { value, selected: settings.editor === value }, told(word)),
          ),
        ),
        settings.editor === "sublime"
          ? make("span", { class: "meta", id: "editor-note" }, told("sublimeNeeds"))
          : null,
      ),
      make(
        "fieldset",
        {},
        make("legend", {}, told("whenStopped")),
        [
          ["card", "openCard"],
          ["tab", "tabOnly"],
        ].map(([value, word]) =>
          make(
            "label",
            { class: "option" },
            make("input", {
              type: "radio",
              name: "stops",
              "data-setting": `stops-${value}`,
              checked: settings.stops === value,
              onchange: () => {
                settings.stops = value;
                keep();
              },
            }),
            told(word),
          ),
        ),
      ),
      make(
        "fieldset",
        {},
        make("legend", {}, told("shortcut")),
        make(
          "div",
          { class: "settings-keys keys" },
          keys,
          make(
            "button",
            {
              class: "button",
              type: "button",
              "data-setting": "shortcut",
              onclick: () => {
                view.capturing = !view.capturing;
                drawSettings();
              },
            },
            told("change"),
          ),
        ),
      ),
      make(
        "div",
        { class: "ends" },
        make(
          "button",
          {
            class: "button",
            type: "button",
            onclick: () => {
              hidden = true;
              place();
            },
          },
          told("hide"),
        ),
        make("span", { class: "meta" }, told("keptHere")),
      ),
    );
    if (focused) settingsBox.querySelector(`[data-setting="${focused}"]`)?.focus();
  };

  // A new shortcut is the next keys pressed with Alt, Ctrl or Meta.
  settingsBox.addEventListener("keydown", (event) => {
    if (!view.capturing) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      view.capturing = false;
      drawSettings();
      return;
    }
    if (
      /^(Alt|Shift|Control|Meta)/.test(event.code) ||
      !(event.altKey || event.ctrlKey || event.metaKey)
    )
      return;
    settings.shortcut = {
      alt: event.altKey,
      shift: event.shiftKey,
      ctrl: event.ctrlKey,
      meta: event.metaKey,
      code: event.code,
    };
    keep();
    view.capturing = false;
    drawSettings();
  });

  // Clicking away from the settings puts them away.
  drawer.addEventListener("pointerdown", (event) => {
    if (
      view.settings &&
      !event.composedPath().some((at) => at === settingsBox || at === settingsButton)
    ) {
      showSettings(false, false);
    }
  });

  // Keys.

  const typing = (target) =>
    target instanceof HTMLElement &&
    (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName));

  addEventListener(
    "keydown",
    (event) => {
      if (hidden || !panel?.round || view.capturing) return;
      const path = event.composedPath();
      const inside = path.includes(host);
      const target = path[0];
      const away = !inside && (document.activeElement === document.body || !document.activeElement);
      if (isShortcut(event)) {
        if (typing(target) && !inside) return;
        event.preventDefault();
        show(!settings.open, true);
        return;
      }
      if (event.key === "Escape" && (inside || away)) {
        if (target === search && search.value) {
          search.value = "";
          if (settings.tab === "search") searchFor("", false);
          else {
            view.query = "";
            drawSoon();
          }
        } else if (view.settings) showSettings(false);
        else if (settings.open) show(false, true);
        else if (!card.hidden) putAway();
        else if (!summary.hidden) summary.hidden = true;
        else return;
        event.preventDefault();
        return;
      }
      if (event.key === "/" && settings.open && !typing(target) && (inside || away)) {
        event.preventDefault();
        if (settings.tab !== "names" && settings.tab !== "search") choose("names");
        search.focus();
      }
    },
    true,
  );

  // Drawing what serve said.

  const draw = (answer) => {
    panel = answer;
    version += 1;
    if (!panel.stopped) view.error = 0;
    if (view.name && !named(view.name)) view.name = null;
    arrange();
    place();
    if (layer.hidden) return;
    drawTab();
    drawCard();
    drawDrawer();
    if (!summary.hidden) drawSummary();
    room();
  };

  const look = async () => {
    try {
      const page = encodeURIComponent(location.pathname);
      const response = await fetch(new URL(`panel?page=${page}`, live), { cache: "no-store" });
      if (response.ok) draw(await response.json());
    } catch {}
  };

  const swap = (styles, generation) => {
    for (const link of document.querySelectorAll('link[rel~="stylesheet"]')) {
      const url = new URL(link.href, location.href);
      if (url.origin !== location.origin) continue;
      const moved = styles.find(([old]) => old === url.pathname);
      if (!moved) continue;
      url.pathname = moved[1];
      // A stylesheet rewritten at the same address is asked for afresh.
      if (moved[0] === moved[1]) url.searchParams.set("live", generation);
      const fresh = link.cloneNode();
      fresh.href = url.href;
      // The old sheet stays until the new one has loaded, so the page is
      // never shown without its styles.
      fresh.addEventListener("load", () => link.remove(), { once: true });
      fresh.addEventListener("error", () => fresh.remove(), { once: true });
      link.after(fresh);
    }
  };

  const apply = (reply) => {
    if (reply.building !== building) {
      building = reply.building;
      if (panel?.round && !layer.hidden) {
        drawTab();
        if (settings.open) drawHead();
      }
    }
    if (reply.kind === "none" || reply.generation === seen) return;
    if (reply.kind === "reload") {
      location.reload();
      return;
    }
    seen = reply.generation;
    if (reply.kind === "styles") swap(reply.styles, reply.generation);
    look();
  };

  const lead = async () => {
    let pause = 250;
    for (;;) {
      try {
        const reply = await ask(25000);
        pause = 250;
        if (reply.kind !== "none" || reply.building !== building) {
          channel?.postMessage(reply.generation);
          apply(reply);
        }
      } catch {
        await new Promise((resolve) => setTimeout(resolve, pause));
        pause = Math.min(pause * 2, 5000);
      }
    }
  };

  const follow = () => ask(0).then(apply, () => {});

  const arrive = () => {
    look();
    follow();
  };
  if (document.readyState === "loading") addEventListener("DOMContentLoaded", arrive);
  else arrive();

  addEventListener("resize", room);
  addEventListener("pageshow", (event) => {
    if (event.persisted) arrive();
  });
  // A browser may freeze the hidden tab that holds the request, so a tab
  // coming into view asks for itself.
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) follow();
  });

  if (channel && navigator.locks) {
    channel.onmessage = follow;
    navigator.locks.request("statac-live", lead);
  } else {
    lead();
  }
})();
