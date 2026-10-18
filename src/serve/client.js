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

  const ask = async (timeout) => {
    const response = await fetch(new URL(`wait?since=${seen}&timeout=${timeout}`, live), {
      cache: "no-store",
    });
    if (!response.ok) throw new Error(`statac serve: ${response.status}`);
    return response.json();
  };

  // The panel: one line at the foot of the page, and behind its button what
  // there is to say of the page and the site. It lives in a shadow root, so
  // the page's styles don't reach it and its own don't reach the page. Every
  // word in it comes from serve, and is set as text.
  const host = document.createElement("statac-live");
  const root = host.attachShadow({ mode: "open" });
  const style = document.createElement("style");
  style.textContent = `
    /* In em, not rem: rem reads the page's root font size through the shadow
       root. A length is its size in px at the default 16px over the element's
       own font size there. */
    :host { all: initial; display: block; }
    .bar {
      position: fixed; inset: auto 0 0 0; z-index: 2147483647; box-sizing: border-box;
      width: 100%; max-width: none; max-height: 70vh; margin: 0; padding: 0; border: 0;
      overflow: visible; display: flex; flex-direction: column;
      font: 0.9375em/1.45 ui-sans-serif, system-ui, sans-serif; color-scheme: dark;
      background: #14181c; color: #b9c1c8; box-shadow: 0 -1px 10px rgb(0 0 0 / 0.3);
      text-align: left;
    }
    .line {
      display: flex; flex: none; align-items: center; gap: calc(12em / 15);
      padding: calc(8em / 15) calc(16em / 15);
    }
    .brand { flex: none; font-weight: 650; color: #e4e9ed; }
    .status {
      margin: 0; flex: 1; min-width: 0; display: flex; flex-wrap: wrap; align-items: baseline;
      gap: 0 calc(12em / 15);
    }
    .where { font: calc(14em / 15)/1.5 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
    .stopped .line { background: #5b1d1d; color: #ffe4e4; }
    .stopped .brand { color: #fff; }
    .stopped .where { color: #ffc9c9; }
    .stopped .help, .stopped .more { color: #ffdcdc; }
    button {
      flex: none; font: inherit; font-size: calc(14em / 15); color: inherit;
      line-height: 1.45; background: transparent; cursor: pointer;
      padding: calc(2em / 14) calc(10em / 14);
      border: 1px solid #5d6770; border-radius: calc(6em / 14);
    }
    .stopped button { border-color: #c98f8f; }
    button:focus-visible, a:focus-visible, .details:focus-visible {
      outline: calc(3em / 15) solid #9cc4e8; outline-offset: calc(2em / 15);
    }
    .details:focus-visible { outline: 0; box-shadow: inset 0 calc(3em / 15) 0 #9cc4e8; }
    .details {
      overflow: auto; padding: calc(4em / 15) calc(16em / 15) calc(18em / 15);
      border-top: 1px solid #2a3138;
    }
    h2 {
      font-size: 1em; font-weight: 650; color: #e4e9ed;
      margin: calc(14em / 15) 0 calc(6em / 15);
    }
    p, ul { margin: calc(4em / 15) 0; }
    ul { padding-left: calc(20em / 15); }
    a { color: #9cc4e8; }
    pre {
      font: calc(14em / 15)/1.55 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      margin: calc(6em / 14) 0 calc(12em / 14); overflow-x: auto; white-space: pre;
    }
    dl {
      margin: calc(4em / 15) 0; display: grid; grid-template-columns: max-content 1fr;
      gap: calc(2em / 15) calc(16em / 15);
    }
    dt { grid-column: 1; color: #e4e9ed; }
    dd { grid-column: 2; margin: 0; overflow-wrap: anywhere; }
    details { margin: calc(6em / 15) 0; }
    summary { cursor: pointer; color: #e4e9ed; padding: calc(2em / 15) 0; }
    summary:focus-visible { outline: calc(3em / 15) solid #9cc4e8; outline-offset: calc(2em / 15); }
    table { border-collapse: collapse; width: 100%; margin: calc(6em / 15) 0 calc(12em / 15); }
    th, td {
      text-align: left; vertical-align: top; font-weight: 400;
      padding: calc(4em / 15) calc(16em / 15) calc(4em / 15) 0;
      border-top: 1px solid #2a3138; overflow-wrap: anywhere;
    }
    thead th { color: #e4e9ed; font-weight: 650; border-top: 0; }
    code, .from {
      font: calc(14em / 15)/1.5 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    }
    tbody th, tbody th + td { white-space: nowrap; }
    tbody th { color: #e4e9ed; }
    .none { color: #8d98a2; font-style: italic; }
    small { display: block; font-size: calc(14em / 15); color: #8d98a2; }
    .halted h2 { color: #ffc9c9; }
    .halted pre { color: #f3d6d6; }
    @media (forced-colors: active) {
      .details:focus-visible {
        outline: calc(3em / 15) solid Highlight; outline-offset: calc(-3em / 15);
      }
      .bar { border-top: 1px solid CanvasText; }
      .details { border-top-color: CanvasText; }
    }
    [hidden] { display: none !important; }
  `;
  const bar = document.createElement("div");
  bar.className = "bar";
  bar.hidden = true;
  const line = document.createElement("div");
  line.className = "line";
  const brand = document.createElement("span");
  brand.className = "brand";
  const status = document.createElement("p");
  status.className = "status";
  status.setAttribute("role", "status");
  const part = (name) => {
    const made = document.createElement("span");
    made.className = name;
    status.append(made);
    return made;
  };
  const said = part("said");
  const where = part("where");
  const help = part("help");
  const more = part("more");
  const button = document.createElement("button");
  button.type = "button";
  button.setAttribute("aria-expanded", "false");
  const details = document.createElement("div");
  details.className = "details";
  details.id = "details";
  details.setAttribute("role", "region");
  details.tabIndex = -1;
  details.hidden = true;
  button.setAttribute("aria-controls", "details");
  line.append(brand, status, button);
  bar.append(line, details);
  root.append(style, bar);

  let words = { show: "", hide: "" };
  let open = false;
  try {
    open = localStorage.getItem(kept) === "open";
  } catch {}

  // The page ends with room for the line, so the line covers nothing that
  // can't be scrolled clear of it.
  const room = () => {
    host.style.height = `${line.offsetHeight}px`;
  };

  const place = () => {
    if (!host.isConnected && document.body) document.body.append(host);
    if (bar.hidden) return;
    // Above everything else on the page, where the browser has a top layer.
    if (bar.showPopover && !bar.matches(":popover-open")) {
      try {
        bar.popover = "manual";
        bar.showPopover();
      } catch {}
    }
    room();
  };

  const show = (wanted, byHand) => {
    open = wanted;
    details.hidden = !open;
    button.setAttribute("aria-expanded", String(open));
    button.textContent = open ? words.hide : words.show;
    if (!byHand) return;
    try {
      localStorage.setItem(kept, open ? "open" : "closed");
    } catch {}
    if (open) details.focus();
    else button.focus();
  };

  button.addEventListener("click", () => show(!open, true));
  bar.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && open) {
      event.stopPropagation();
      show(false, true);
    }
  });

  const element = (name, text) => {
    const made = document.createElement(name);
    if (text !== undefined) made.textContent = text;
    return made;
  };

  // A sentence of serve's comes as pieces: words, and names to show as code.
  const pieces = (sentence) =>
    sentence.map((piece) =>
      "code" in piece ? element("code", piece.code) : document.createTextNode(piece.text),
    );
  const spoken = (name, sentence) => {
    const made = element(name);
    made.append(...pieces(sentence));
    return made;
  };
  const plain = (sentence) => sentence.map((piece) => piece.code ?? piece.text).join("");

  const draw = (panel) => {
    words = panel;
    brand.textContent = panel.name;
    // The first error is said on the line itself, with where it is and
    // what to do about it.
    const error = panel.error;
    said.replaceChildren(...pieces(panel.line));
    if (error) said.append(" ", ...pieces(error.headline));
    where.textContent = error ? error.location : "";
    help.replaceChildren(...pieces(error ? error.suggestion : []));
    more.replaceChildren(...pieces(error ? error.more : []));
    bar.classList.toggle("stopped", panel.stopped);
    details.setAttribute("aria-label", panel.name);
    const sections = panel.sections.map((section, at) => {
      const parts = [];
      parts.push(element("h2", section.heading));
      for (const sentence of section.lines) parts.push(spoken("p", sentence));
      if (section.drawn) parts.push(element("pre", section.drawn));
      for (const list of section.lists) {
        parts.push(spoken("p", list.label));
        const items = element("ul");
        for (const link of list.links) {
          const item = element("li");
          const anchor = element("a", link.text);
          // Only ever a page of this site.
          const to = new URL(link.href, location.origin);
          if (to.origin === location.origin) anchor.href = to.href;
          item.append(anchor);
          items.append(item);
        }
        parts.push(items);
      }
      if (section.facts) {
        const facts = element("dl");
        for (const fact of section.facts) {
          const said = fact.said.map((sentence) => spoken("dd", sentence));
          facts.append(element("dt", fact.label), ...said);
        }
        parts.push(facts);
      }
      for (const table of section.tables || []) {
        // Closed until asked for, and as it was left across a reload.
        const group = element("details");
        const key = `${kept}-${plain(table.caption).split(":")[0]}`;
        try {
          group.open = localStorage.getItem(key) === "open";
        } catch {}
        group.addEventListener("toggle", () => {
          try {
            localStorage.setItem(key, group.open ? "open" : "closed");
          } catch {}
        });
        const made = element("table");
        const head = element("tr");
        for (const column of section.columns) {
          const cell = element("th", column);
          cell.scope = "col";
          head.append(cell);
        }
        const body = element("tbody");
        for (const row of table.rows) {
          const line = element("tr");
          const name = element("th");
          name.scope = "row";
          name.append(element("code", row.name));
          const value = element("td", row.value);
          if (row.empty) value.className = "none";
          if (row.has.length) value.append(spoken("small", row.has));
          const from = element("td", row.from);
          from.className = "from";
          line.append(name, element("td", row.holds), value, from);
          body.append(line);
        }
        const top = element("thead");
        top.append(head);
        made.append(top, body);
        group.append(spoken("summary", table.caption), made);
        parts.push(group);
      }
      const within = element("section");
      // What stopped the build comes first where anything did.
      if (panel.stopped && at === 0) within.className = "halted";
      within.append(...parts);
      return within;
    });
    details.replaceChildren(...sections);
    bar.hidden = !panel.line.length;
    show(open, false);
    place();
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
        if (reply.kind !== "none") {
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
