import { SlmlEngine, documentURL, loadDocument, targetURL } from "./engine.mjs";
import { positionText, installTextSelection } from "./text.mjs";

const $ = id => document.getElementById(id);
const stage = $("stage"), canvas = $("canvas"), links = $("links"), status = $("status"), error = $("error");
const context = canvas.getContext("2d");
const textLayer = $("text-layer");
installTextSelection(textLayer);
const values = new Map();
let module, engine, page, initialURL, revision = null, edited = false, generation = 0, frame = null;

function fail(reason) {
  stage.hidden = true;
  links.replaceChildren();
  textLayer.replaceChildren();
  $("controls").replaceChildren();
  error.textContent = String(reason.message || reason);
  error.hidden = false;
  status.textContent = "SLML could not render this document. Fix the source and reload.";
  status.hidden = false;
}

function viewport() {
  return { width: Math.max(1, Math.floor(stage.clientWidth)), height: Math.max(1, innerHeight - 52), scale: devicePixelRatio || 1 };
}

function draw() {
  frame = null;
  if (!page || stage.hidden) return;
  try {
    const { width, height, scale } = viewport();
    const pixels = engine.render(width, height, scale, scrollY);
    const w = Math.ceil(width * scale), h = Math.ceil(height * scale);
    if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    context.putImageData(new ImageData(pixels, w, h), 0, 0);
    stage.style.height = `${engine.info().height}px`;
  } catch (reason) { fail(reason); }
}

function schedule() { if (frame === null) frame = requestAnimationFrame(draw); }

function positionLinks() {
  const info = engine.info();
  links.replaceChildren();
  const areas = [];
  for (const area of info.links || []) {
    const previous = areas[areas.length - 1];
    if (previous && previous.to === area.to && Math.abs(previous.y-area.y)<1 && Math.abs(previous.h-area.h)<1 && area.x <= previous.x+previous.w+16) {
      previous.w = Math.max(previous.x+previous.w, area.x+area.w)-previous.x;
    } else areas.push({...area});
  }
  for (const area of areas) {
    const link = document.createElement("a");
    configureLink(link, area.to);
    Object.assign(link.style, { left:`${area.x}px`, top:`${area.y}px`, width:`${area.w}px`, height:`${area.h}px` });
    links.append(link);
  }
  positionText(textLayer, info.text || [], configureLink);
  positionControls(info.controls || []);
}

function configureLink(link, target) {
  const url = targetURL(target, page.url);
  link.href = url.href;
  link.setAttribute("aria-label", target.startsWith("#") ? `Jump to ${target.slice(1)}` : `Open ${target}`);
  link.addEventListener("click", event => {
    if (event.detail && !document.getSelection().isCollapsed) { event.preventDefault(); return; }
    if (event.button || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
    if (url.origin === page.url.origin && url.pathname === page.url.pathname && url.hash) {
      event.preventDefault();
      const y = engine.info().anchors?.[url.hash.slice(1)];
      if (y !== undefined) scrollTo(0, y);
    } else if (["http:","https:"].includes(url.protocol) && url.pathname.endsWith(".slml")) {
      event.preventDefault();
      navigate(url.href, true);
    }
  });
}

function rectangle(element, area) {
  Object.assign(element.style, {left:`${area.x}px`,top:`${area.y}px`,width:`${area.w}px`,height:`${area.h}px`});
}

function positionControls(controls) {
  const layer = $("controls");
  layer.replaceChildren();
  const forms = new Map();
  for (const area of controls) {
    if (!area.form) continue;
    let form = forms.get(area.form.id);
    if (!form) {
      form = document.createElement("form");
      form.className = "control-form";
      const action = targetURL(area.form.to, page.url);
      if (!["http:", "https:"].includes(action.protocol)) continue;
      form.action = action.href;
      form.method = area.form.method;
      form.addEventListener("submit", event => {
        if (form.method === "get" && action.pathname.endsWith(".slml")) {
          event.preventDefault();
          const url = new URL(form.action);
          url.search = new URLSearchParams(new FormData(form)).toString();
          navigate(url.href, true);
        }
      });
      forms.set(area.form.id, form);
      layer.append(form);
    }
    if (area.kind === "submit") {
      const button = document.createElement("button");
      button.type = "submit";
      button.className = "slml-submit";
      button.setAttribute("aria-label", area.label);
      rectangle(button, area);
      form.append(button);
      continue;
    }
    const label = document.createElement("label");
    label.className = "slml-control";
    rectangle(label, area);
    const caption = document.createElement("span");
    caption.className = "visually-hidden";
    caption.textContent = area.label;
    label.append(caption);
    const tag = ["textarea", "select"].includes(area.kind) ? area.kind : "input";
    const control = document.createElement(tag);
    control.name = area.name;
    control.required = area.required;
    if (tag === "input") control.type = area.kind;
    if (area.kind === "select") for (const option of area.options) {
      const item = new Option(option.label, option.value, option.selected, option.selected);
      control.append(item);
    }
    if (area.kind === "checkbox") { control.checked = values.get(area.id) ?? area.checked; control.value = "on"; }
    else { control.value = values.get(area.id) ?? area.value ?? control.value; control.placeholder = area.placeholder || ""; }
    if (area.rows) control.rows = area.rows;
    control.className = "slml-input";
    rectangle(control, { ...area.box, x:area.box.x-area.x, y:area.box.y-area.y });
    Object.assign(control.style, {fontSize:`${area.size}px`,color:area.color,fontWeight:area.bold?"700":"400",fontStyle:area.italic?"italic":"normal",fontFamily:area.mono?'"SLML Mono",monospace':'"SLML Sans",sans-serif'});
    if (area.kind !== "checkbox") Object.assign(control.style, {background:area.fill,border:`${area.border}px solid ${area.borderColor}`,borderRadius:`${area.radius}px`});
    control.addEventListener("input", () => values.set(area.id, area.kind === "checkbox" ? control.checked : control.value));
    control.addEventListener("change", () => values.set(area.id, area.kind === "checkbox" ? control.checked : control.value));
    label.append(control);
    form.append(label);
  }
}

function show() {
  error.hidden = true;
  status.hidden = true;
  stage.hidden = false;
  draw();
  positionLinks();
}

async function navigate(value, push = false) {
  const ticket = ++generation;
  status.hidden = false;
  status.textContent = "Loading SLML…";
  error.hidden = true;
  stage.hidden = true;
  links.replaceChildren();
  try {
    const url = documentURL(value);
    $("address").value = url.href;
    // Each navigation gets its own memory and file namespace. A late fetch
    // from the previous origin cannot inject files into the next document.
    const next = await SlmlEngine.create(module);
    const loaded = await loadDocument(next, url.href);
    if (ticket !== generation) return;
    engine = next;
    page = loaded;
    values.clear();
    edited = false;
    $("code").value = page.source;
    document.title = `${engine.info().title} — SLML`;
    if (push) history.pushState({}, "", `?url=${encodeURIComponent(url.href)}`);
    scrollTo(0, 0);
    show();
    if (url.hash) {
      const y = engine.info().anchors?.[url.hash.slice(1)];
      if (y !== undefined) scrollTo(0, y);
    }
  } catch (reason) { if (ticket === generation) fail(reason); }
}

$("location").addEventListener("submit", event => { event.preventDefault(); navigate($("address").value, true); });
$("reload").addEventListener("click", () => navigate($("address").value));
$("source").addEventListener("click", () => {
  $("editor").hidden = !$("editor").hidden;
  document.body.classList.toggle("editing", !$("editor").hidden);
  $("source").setAttribute("aria-expanded", String(!$("editor").hidden));
  if (!stage.hidden) { draw(); positionLinks(); }
});
let editTimer;
$("code").addEventListener("input", () => {
  clearTimeout(editTimer);
  editTimer = setTimeout(async () => {
    if (!page) return;
    const ticket = ++generation;
    edited = true;
    const source = $("code").value;
    try {
      const next = await SlmlEngine.create(module);
      const loaded = await loadDocument(next, page.url.href, fetch, source);
      if (ticket !== generation) return;
      engine = next;
      page = loaded;
      values.clear();
      document.title = `${engine.info().title} — SLML`;
      show();
    } catch (reason) { if (ticket === generation) fail(reason); }
  }, 200);
});
addEventListener("scroll", schedule, { passive:true });
addEventListener("resize", () => { if (page && !stage.hidden) { draw(); positionLinks(); } });
let observedWidth = 0;
new ResizeObserver(() => {
  if (!stage.hidden && Math.floor(stage.clientWidth) !== observedWidth) {
    observedWidth = Math.floor(stage.clientWidth);
    draw();
    positionLinks();
  }
}).observe(stage);
addEventListener("popstate", () => { const url = new URL(location.href).searchParams.get("url") || initialURL; if (url) navigate(url); });

try {
  const response = await fetch(new URL("slml.wasm", import.meta.url));
  if (!response.ok) throw new Error("The SLML WebAssembly module is missing. Build the browser assets first.");
  module = await WebAssembly.compile(await response.arrayBuffer());
  const fonts = await SlmlEngine.create(module);
  const faces = [["SLML Sans",400,"normal"],["SLML Sans",700,"normal"],["SLML Sans",400,"italic"],["SLML Sans",700,"italic"],["SLML Mono",400,"normal"],["SLML Mono",700,"normal"],["SLML Symbols 2",400,"normal"],["SLML Symbols",400,"normal"],["SLML Math",400,"normal"],["SLML Emoji",400,"normal"]];
  for (const [index, [family, weight, style]] of faces.entries()) {
    const bytes = new Uint8Array(fonts.wasm.memory.buffer, fonts.wasm.slml_font(index), fonts.wasm.slml_font_len(index)).slice();
    const font = new FontFace(family, bytes, {weight:String(weight),style});
    document.fonts.add(await font.load());
  }
  const initial = new URL(location.href).searchParams.get("url") || document.body.dataset.source;
  initialURL = initial ? new URL(initial, location.href).href : null;
  if (initial) await navigate(new URL(initial, location.href).href);
  else status.textContent = "Enter an HTTP or HTTPS .slml address to open it with the SLML renderer.";
  // Only the local preview shell opts into file watching. The extension
  // makes no periodic requests to arbitrary remote hosts.
  if (document.body.dataset.watch === "true") setInterval(async () => {
    try {
      const response = await fetch("/_slml/version", { cache:"no-store" });
      if (!response.ok) return;
      const next = await response.text();
      if (revision !== null && next !== revision) {
        if (edited) { status.hidden = false; status.textContent = "Files changed on disk. Reload to discard the edits in this tab."; }
        else await navigate($("address").value);
      }
      revision = next;
    } catch { /* A stopped preview server must not erase the last good page. */ }
  }, 500);
} catch (reason) { fail(reason); }
