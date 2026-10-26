// Runs the real WebAssembly artifact, not a JavaScript reimplementation.
import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import { SlmlEngine, loadDocument, targetURL, encodePath, documentURL, readBytes } from "./engine.mjs";
import { redirect } from "../extensions/firefox/background.mjs";

const wasm = await readFile(process.argv[2] || "target/wasm32-unknown-unknown/release/slml_web.wasm");
const module = await WebAssembly.compile(wasm);
assert.deepEqual(WebAssembly.Module.imports(module), [], "the engine does not import a host parser or renderer");
const engine = await SlmlEngine.create(module);
const files = new Map([
  ["/site/index.slml", 'slml 0.1 use "parts.slml" meta(title: "Real SLML") page(fill: #123456) { greeting heading(id: "end") "End" }'],
  ["/site/parts.slml", 'slml 0.1 define greeting { text { link(to: "#end") "Jump" } image(src: "logo.png", alt: "Logo", width: 40) }'],
  ["/site/logo.png", await readFile("examples/site/logo.png")],
]);
let requests = [];
const fetcher = async (value, options) => {
  const url = new URL(value);
  assert.equal(url.origin, "https://site.test");
  assert.equal(options.credentials, "omit");
  assert.equal(options.redirect, "error");
  requests.push(url.pathname);
  const data = files.get(url.pathname);
  return new Response(data || "missing", { status: data ? 200 : 404 });
};
const page = await loadDocument(engine, "https://site.test/site/index.slml", fetcher);
assert.equal(page.name, "/site/index.slml");
assert.deepEqual(requests, ["/site/index.slml", "/site/parts.slml", "/site/logo.png"]);
assert.equal(engine.info().title, "Real SLML");
let pixels = engine.render(320, 240, 1, 0);
assert.equal(pixels.length, 320 * 240 * 4);
assert.deepEqual([...pixels.slice(0, 4)], [0x12, 0x34, 0x56, 255]);
assert.ok(engine.info().anchors.end > 0);
assert.equal(engine.info().links[0].to, "#end");
assert.ok(pixels.some((p, i) => i % 4 !== 3 && p !== pixels[i % 4]), "text and images actually paint into the buffer");
pixels = engine.render(390, 300, 2, 30);
assert.equal(pixels.length, 780 * 600 * 4);

files.set("/site/parts.slml", 'slml 0.1 define greeting { colum }');
await assert.rejects(loadDocument(engine, page.url.href, fetcher), /did you mean `column`/);
assert.equal(engine.wasm.slml_pixels_len(), 0, "errors erase earlier pixels");
assert.match(engine.info().error, /parts\.slml:1/);
files.set("/site/parts.slml", 'slml 0.1 define greeting { text "Fixed" }');
await loadDocument(engine, page.url.href, fetcher);
assert.equal(engine.render(320, 240, 1, 0).length, 320 * 240 * 4);

await loadDocument(engine, page.url.href, fetcher, `slml 0.1 page { form(to: "preview.slml", method: get) {
  row(wrap: true) { input(name: "email", label: "Email", kind: email, required: true) }
  select(name: "topic", label: "Topic") { option(value: "a") "A" option(value: "b", selected: true) "B" }
  checkbox(name: "yes", label: "Yes", checked: true)
  textarea(name: "message", label: "Message", value: "żółć") submit "Send"
} }`);
engine.render(320, 240, 1, 0);
const controls = engine.info().controls;
assert.equal(controls.length, 5);
assert.deepEqual(controls.map(c => c.kind), ["email", "select", "checkbox", "textarea", "submit"]);
assert.equal(controls[0].form.to, "preview.slml");
assert.equal(controls[0].required, true);
assert.ok(controls[0].box.y > controls[0].y);
assert.equal(controls[1].options[1].selected, true);
assert.equal(controls[3].value, "żółć");
assert.equal(controls[4].label, "Send");
assert.ok(engine.wasm.slml_font_len(0) > 1000);

assert.equal(targetURL("żółć 100%.slml#end", page.url).href, "https://site.test/site/%C5%BC%C3%B3%C5%82%C4%87%20100%25.slml#end");
assert.equal(targetURL("/index.slml", page.url).href, "https://site.test/index.slml");
assert.equal(targetURL("mailto:a@example.com", page.url).protocol, "mailto:");
for (const path of ["/../private", "/a/../b", "/a\\b", "/bad\0path"]) assert.throws(() => encodePath(path));
for (const url of ["file:///etc/passwd", "javascript:alert(1)", "https://user:secret@site.test/index.slml"]) assert.throws(() => documentURL(url));
await assert.rejects(readBytes(new URL("https://other.test/x.slml"), "https://site.test", fetcher), /origin/);
assert.throws(() => targetURL("javascript:alert(1)", page.url));
assert.deepEqual(redirect({ url:page.url.href, method:"GET", type:"main_frame" }, "moz-extension://id/viewer.html"), {
  redirectUrl:"moz-extension://id/viewer.html?url=https%3A%2F%2Fsite.test%2Fsite%2Findex.slml"
});
for (const detail of [
  {url:page.url.href, method:"POST", type:"main_frame"},
  {url:page.url.href, method:"GET", type:"xmlhttprequest"},
  {url:"https://site.test/index.html", method:"GET", type:"main_frame"},
]) assert.deepEqual(redirect(detail, "moz-extension://id/viewer.html"), {});
console.log("WASM rendering, imports, images, errors, URL boundaries and Firefox interception passed.");
