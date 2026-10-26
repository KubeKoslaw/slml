// The module only transports bytes. Parsing, validation, layout and painting
// are performed by slml-syntax and slml-render inside WebAssembly.
const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });

export class SlmlEngine {
  static async create(bytes) {
    const result = await WebAssembly.instantiate(bytes, {});
    const instance = result.instance || result;
    return new SlmlEngine(instance.exports);
  }

  constructor(exports) { this.wasm = exports; }

  input(bytes) {
    const pointer = this.wasm.slml_input(bytes.length);
    new Uint8Array(this.wasm.memory.buffer, pointer, bytes.length).set(bytes);
  }

  record(name, data) {
    const prefix = encoder.encode(name + "\0");
    const bytes = new Uint8Array(prefix.length + data.length);
    bytes.set(prefix);
    bytes.set(data, prefix.length);
    this.input(bytes);
  }

  file(name, source) {
    this.record(name, encoder.encode(source));
    if (!this.wasm.slml_file()) throw new Error("The source file is not valid UTF-8.");
  }

  image(name, bytes) {
    this.record(name, bytes);
    return Boolean(this.wasm.slml_image());
  }

  reset() { this.wasm.slml_reset(); }

  info() {
    return JSON.parse(decoder.decode(new Uint8Array(this.wasm.memory.buffer,
      this.wasm.slml_info(), this.wasm.slml_info_len())));
  }

  open(name) {
    this.input(encoder.encode(name));
    this.wasm.slml_open();
    return this.info();
  }

  render(width, height, scale, scroll) {
    if (!this.wasm.slml_render(width, height, scale, scroll)) {
      throw new Error(this.info().error || "The SLML renderer could not draw this page.");
    }
    // Copy before another call grows WASM memory or replaces its pixel buffer.
    return new Uint8ClampedArray(this.wasm.memory.buffer,
      this.wasm.slml_pixels(), this.wasm.slml_pixels_len()).slice();
  }
}

export function documentURL(value) {
  const url = new URL(value);
  if (!["http:", "https:"].includes(url.protocol) || url.username || url.password) {
    throw new Error("Open an HTTP or HTTPS SLML address without embedded credentials.");
  }
  return url;
}

export function encodePath(path) {
  if (!path.startsWith("/") || path.includes("\\") || path.split("/").includes("..") || /[\x00-\x1f\x7f]/.test(path)) {
    throw new Error("A document path must stay inside its origin.");
  }
  return path.split("/").map(encodeURIComponent).join("/");
}

export function targetURL(target, page) {
  if (/^[^/#]+:/.test(target)) {
    const url = new URL(target);
    if (!["http:", "https:", "mailto:", "tel:"].includes(url.protocol)) throw new Error("This link scheme is not allowed.");
    return url;
  }
  if (target.startsWith("#")) return new URL(target, page);
  const split = target.indexOf("#");
  const path = split === -1 ? target : target.slice(0, split);
  const fragment = split === -1 ? "" : target.slice(split);
  const base = decodeURIComponent(page.pathname).replace(/[^/]*$/, "");
  const encoded = encodePath(path.startsWith("/") ? path : base + path);
  return new URL(page.origin + encoded + fragment);
}

export async function readBytes(url, origin, fetcher = fetch) {
  if (url.origin !== origin) throw new Error("Imports and local assets must stay inside the document origin.");
  const response = await fetcher(url.href, { credentials: "omit", redirect: "error", cache: "no-store", referrerPolicy: "no-referrer" });
  if (!response.ok) throw new Error(`Cannot load ${url.pathname}: HTTP ${response.status}`);
  // Redirects are rejected before any destination is fetched. Do not turn
  // untrusted documents into a cross-origin network client for the extension.
  const limit = 8 * 1024 * 1024;
  const reader = response.body.getReader();
  const chunks = [];
  let length = 0;
  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    length += value.length;
    if (length > limit) { await reader.cancel(); throw new Error(`File exceeds 8 MiB: ${url.pathname}`); }
    chunks.push(value);
  }
  const bytes = new Uint8Array(length);
  let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
  return bytes;
}

export async function loadDocument(engine, value, fetcher = fetch, sourceOverride = null) {
  const url = documentURL(value);
  const name = decodeURIComponent(url.pathname);
  engine.reset();
  const source = sourceOverride ?? decoder.decode(await readBytes(url, url.origin, fetcher));
  engine.file(name, source);
  const loaded = new Set([name]);
  for (let round = 0; round < 128; round++) {
    const info = engine.open(name);
    if (info.requests?.length) {
      for (const path of info.requests) {
        if (loaded.has(path)) throw new Error(info.error);
        const imported = new URL(url.origin + encodePath(path));
        let text;
        try { text = decoder.decode(await readBytes(imported, url.origin, fetcher)); }
        catch (reason) { throw new Error(`${info.error}\n${reason.message}`); }
        engine.file(path, text);
        loaded.add(path);
      }
      continue;
    }
    if (info.error) throw new Error(info.error);
    for (const src of info.images || []) {
      // Failed images use the same alt placeholder as the native renderer.
      try {
        const image = targetURL(src, url);
        if (image.origin === url.origin) engine.image(src, await readBytes(image, url.origin, fetcher));
      } catch { /* SPEC §5: image failures display alt text. */ }
    }
    return { url, name, source };
  }
  throw new Error("The document imports more than 128 dependency levels.");
}
