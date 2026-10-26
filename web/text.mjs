// Rust supplies every run's text, reading order and rectangle. This layer
// provides browser selection; it never measures or reflows the SLML layout.
export function positionText(layer, areas, configureLink) {
  const document = layer.ownerDocument;
  const measure = document.createElement("canvas").getContext("2d");
  measure.fontKerning = "normal";
  const fragment = document.createDocumentFragment();
  for (const area of areas) {
    const block = document.createElement("div");
    block.className = "slml-text-block";
    let offset = 0;
    for (const run of area.runs) {
      if (run.start > offset) gap(area.text.slice(offset, run.start));
      const span = document.createElement(run.to ? "a" : "span");
      span.className = "slml-text-run";
      span.textContent = area.text.slice(run.start, run.end);
      const family = `${run.mono ? '"SLML Mono"' : '"SLML Sans"'},"SLML Symbols 2","SLML Symbols","SLML Math","SLML Emoji",sans-serif`;
      const font = `${run.italic ? "italic" : "normal"} ${run.bold ? "700" : "400"} ${run.size}px ${family}`;
      measure.font = font;
      // Match the selection boxes to the renderer's advances, including any
      // difference between the browser's font metrics and fontdue's metrics.
      const width = measure.measureText(span.textContent).width;
      Object.assign(span.style, {
        left:`${run.x}px`, top:`${run.y}px`, height:`${run.h}px`,
        font, lineHeight:`${run.h}px`, transformOrigin:"left top",
        transform:width > 0 ? `scaleX(${run.w / width})` : "none",
      });
      if (run.to) { configureLink(span, run.to); span.tabIndex = -1; }
      block.append(span);
      offset = run.end;
    }
    if (offset < area.text.length) gap(area.text.slice(offset));
    fragment.append(block);

    function gap(text) {
      // Newlines and spaces discarded only for visual wrapping still belong
      // to the original paragraph. Keep them in the DOM's reading order.
      const span = document.createElement("span");
      span.className = "slml-text-gap";
      span.textContent = text;
      block.append(span);
    }
  }
  layer.replaceChildren(fragment);
}

export function selectedText(layer, selection = layer.ownerDocument.getSelection()) {
  if (!selection || selection.isCollapsed || !selection.rangeCount) return null;
  const { Range } = layer.ownerDocument.defaultView;
  const parts = [];
  for (let i = 0; i < selection.rangeCount; i++) {
    const range = selection.getRangeAt(i);
    for (const block of layer.children) {
      if (!range.intersectsNode(block)) continue;
      const part = layer.ownerDocument.createRange();
      part.selectNodeContents(block);
      if (range.compareBoundaryPoints(Range.START_TO_START, part) > 0) part.setStart(range.startContainer, range.startOffset);
      if (range.compareBoundaryPoints(Range.END_TO_END, part) < 0) part.setEnd(range.endContainer, range.endOffset);
      if (!part.collapsed) parts.push(part.toString());
    }
  }
  return parts.length ? parts.join("\n") : null;
}

export function installTextSelection(layer) {
  const document = layer.ownerDocument;
  document.addEventListener("copy", event => {
    // Native fields and the source editor retain their browser copy behavior.
    if (event.target.closest?.("input,textarea,select,[contenteditable=true]")) return;
    const text = selectedText(layer);
    if (text === null || !event.clipboardData) return;
    event.clipboardData.setData("text/plain", text);
    event.preventDefault();
  });
  document.addEventListener("keydown", event => {
    if (event.key.toLowerCase() !== "a" || !(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return;
    if (!layer.children.length || layer.closest("[hidden]")) return;
    if (event.target.closest?.("input,textarea,select,[contenteditable=true]")) return;
    const range = document.createRange();
    range.selectNodeContents(layer);
    const selection = document.getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
    event.preventDefault();
  });
}
