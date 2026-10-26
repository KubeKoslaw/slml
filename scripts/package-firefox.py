"""Package only the reviewed browser runtime and manifest, never workspace notes."""
from pathlib import Path
import shutil
import zipfile

root = Path(__file__).resolve().parent.parent
package = root / "target" / "firefox"
package.mkdir(parents=True, exist_ok=True)
files = {
    "manifest.json": root / "extensions/firefox/manifest.json",
    "background.mjs": root / "extensions/firefox/background.mjs",
    **{name: root / "web" / name for name in ("viewer.html", "viewer.css", "viewer.mjs", "engine.mjs", "text.mjs")},
    "slml.wasm": root / "target/wasm32-unknown-unknown/release/slml_web.wasm",
    "OFL.txt": root / "assets/fonts/OFL.txt",
    "LICENSE-MIT": root / "LICENSE-MIT",
    "LICENSE-APACHE": root / "LICENSE-APACHE",
}
for name, source in files.items():
    shutil.copyfile(source, package / name)
output = root / "target" / "slml-firefox-0.1.0.xpi"
with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as archive:
    for name in files:
        archive.write(package / name, name)
print(f"Firefox package: {output}")
print(f"Temporary installation manifest: {package / 'manifest.json'}")
