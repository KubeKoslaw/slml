"""Smoke-test the built extension in an isolated, headless Firefox profile.

The temporary background test is never included in the distributed XPI.
It tests Firefox's extension APIs and the real WASM engine against a local
static source server, without accessing a user's Firefox profile.
"""
import functools
import http.server
import json
from pathlib import Path
import queue
import shutil
import subprocess
import tempfile
import threading
import time

root = Path(__file__).resolve().parent.parent
reports = queue.Queue()


class Handler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_POST(self):
        if self.path != "/_report":
            self.send_error(404)
            return
        length = min(int(self.headers.get("Content-Length", "0")), 16384)
        reports.put(json.loads(self.rfile.read(length)))
        self.send_response(200)
        self.end_headers()


server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=str(root / "examples/starter")))
threading.Thread(target=server.serve_forever, daemon=True).start()
origin = f"http://127.0.0.1:{server.server_port}"

with tempfile.TemporaryDirectory(prefix="slml-firefox-test-") as tmp:
    source = Path(tmp) / "extension"
    shutil.copytree(root / "target/firefox", source)
    manifest = json.loads((source / "manifest.json").read_text())
    manifest["background"]["scripts"].append("smoke.mjs")
    (source / "manifest.json").write_text(json.dumps(manifest))
    smoke = r'''
import {SlmlEngine, loadDocument} from "./engine.mjs";
const origin = ORIGIN;
async function test() {
  const bytes = await (await fetch(browser.runtime.getURL("slml.wasm"))).arrayBuffer();
  const engine = await SlmlEngine.create(bytes);
  await loadDocument(engine, origin + "/index.slml");
  const pixels = engine.render(390, 600, 1, 0);
  if (pixels.length !== 390*600*4 || pixels[0] !== 245 || !engine.info().title.startsWith("Fieldwork")) throw new Error("Firefox WASM render failed");
  await loadDocument(engine, origin + "/contact.slml");
  engine.render(390,600,1,0);
  if (engine.info().controls.length !== 6) throw new Error("Firefox form metadata missing");
  const tab = await browser.tabs.create({url: origin + "/index.slml"});
  let current;
  for (let attempt=0; attempt<60; attempt++) {
    current = await browser.tabs.get(tab.id);
    if (current.url?.startsWith(browser.runtime.getURL("viewer.html")) && current.title?.includes("Fieldwork")) break;
    await new Promise(resolve => setTimeout(resolve,250));
  }
  if (!current.url?.startsWith(browser.runtime.getURL("viewer.html")) || !current.title?.includes("Fieldwork")) throw new Error("SLML navigation was not intercepted and rendered: " + current.url + " / " + current.title);
  return {ok:true, title:current.title, message:"Firefox intercepted a raw .slml navigation and rendered the page; imports, pixels and form metadata passed"};
}
test().catch(error=>({ok:false,error:String(error.message) + "\n" + String(error.stack || "")})).then(result => fetch(origin + "/_report", {method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(result)}));
'''
    (source / "smoke.mjs").write_text(smoke.replace("ORIGIN", json.dumps(origin)))
    with (Path(tmp) / "firefox.log").open("w+") as log:
        process = subprocess.Popen(["npx", "--yes", "web-ext", "run", "--args=-headless", "--no-reload", "--no-input", "--source-dir", str(source)], cwd=root, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            deadline = time.monotonic() + 60
            while True:
                try:
                    result = reports.get(timeout=0.5)
                    break
                except queue.Empty:
                    if process.poll() is not None or time.monotonic() >= deadline:
                        raise
            print(json.dumps(result, ensure_ascii=False))
            if not result.get("ok"):
                raise SystemExit(1)
        except queue.Empty:
            log.seek(0)
            raise SystemExit("Firefox smoke test timed out:\n" + log.read())
        finally:
            # Stop the isolated test runner and its browser, never a user's profile.
            import os
            import signal
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            process.wait(timeout=10)
            server.shutdown()
