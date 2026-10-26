"""Create GitHub/Obsidian Markdown from the canonical SLML handbook.

Uses the existing compiler's semantic HTML export as an intermediate format;
no additional parser, renderer or third-party Python packages are required.
"""
import argparse
from dataclasses import dataclass, field
from html.parser import HTMLParser
from pathlib import Path
import re
import subprocess
import tempfile
from urllib.parse import unquote, urlsplit


@dataclass
class Element:
    tag: str
    attrs: dict = field(default_factory=dict)
    children: list = field(default_factory=list)

    def text(self):
        return "".join(c if isinstance(c, str) else c.text() for c in self.children)


class Parser(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.root = Element("root")
        self.stack = [self.root]

    def handle_starttag(self, tag, attrs):
        node = Element(tag, dict(attrs))
        self.stack[-1].children.append(node)
        if tag not in {"meta", "link", "img", "input", "hr", "br"}:
            self.stack.append(node)

    def handle_endtag(self, tag):
        if self.stack[-1].tag == tag:
            self.stack.pop()

    def handle_data(self, data):
        self.stack[-1].children.append(data)


def elements(node):
    return [c for c in node.children if isinstance(c, Element)]


def walk(node):
    yield node
    for child in elements(node):
        yield from walk(child)


def slug(text):
    return re.sub(r"\s", "-", re.sub(r"[^\w\s-]", "", text.lower()))


def code(text):
    fence = "`" * max(1, 1 + max((len(s) for s in re.findall(r"`+", text)), default=0))
    pad = " " if text.startswith("`") or text.endswith("`") else ""
    return fence + pad + text + pad + fence


class Markdown:
    def __init__(self, filename, anchors):
        self.filename = filename
        self.anchors = anchors

    def link(self, href):
        parsed = urlsplit(href)
        if parsed.scheme or parsed.netloc:
            return href
        path = unquote(parsed.path)
        name = Path(path).name if path else self.filename
        if name.endswith(".html"):
            name = name[:-5] + ".md"
        elif path:
            return "../../handbook/" + href.lstrip("/")
        fragment = parsed.fragment
        if fragment:
            fragment = self.anchors.get(name, {}).get(fragment, fragment)
        return (name if path else "") + ("#" + fragment if fragment else "")

    def inline(self, node):
        if isinstance(node, str):
            return node
        text = "".join(self.inline(c) for c in node.children)
        if node.tag == "code":
            return code(node.text())
        if node.tag == "a":
            href = self.link(node.attrs.get("href", ""))
            return f"[{text}](<{href}>)" if " " in href else f"[{text}]({href})"
        if node.tag == "img":
            return f"![{node.attrs.get('alt', '')}]({self.link(node.attrs['src'])})"
        if node.tag == "strong" or "font-weight:700" in node.attrs.get("style", ""):
            return f"**{text}**"
        if node.tag == "em":
            return f"*{text}*"
        return text

    def reference(self, node):
        children = elements(node)
        if len(children) == 2 and children[-1].tag == "hr":
            cells = elements(children[0])
            if len(cells) == 3 and cells[0].tag == "p":
                return [self.inline(c).replace("|", "\\|").replace("\n", "<br>") for c in cells]
        return None

    def block(self, node, level=0):
        if isinstance(node, str):
            return ""
        if node.tag in {"h1", "h2", "h3", "h4"}:
            return "#" * int(node.tag[1]) + " " + "".join(self.inline(c) for c in node.children) + "\n\n"
        if node.tag == "pre":
            text = node.text()
            fence = "`" * max(3, 1 + max((len(s) for s in re.findall(r"`+", text)), default=0))
            first = text.lstrip().splitlines()[0] if text.strip() else ""
            language = "sh" if re.match(r"(slml (?![0-9]+\.[0-9]+)|cargo |curl |make |cp |rustup )", first) else "slml"
            if "actions/" in text or first.startswith("# .github/"):
                language = "yaml"
            return f"{fence}{language}\n{text.rstrip()}\n{fence}\n\n"
        if node.tag == "p":
            text = self.inline(node)
            if text.startswith("SLML handbook ·") or text == "RESULT":
                return ""
            return text + "\n\n"
        if node.tag == "hr":
            return ""
        if node.tag == "a":
            heading = next((c for c in elements(node) if c.tag in {"h1", "h2", "h3", "h4"}), None)
            if heading:
                return f"### [{heading.text()}]({self.link(node.attrs['href'])})\n\n" + "".join(self.block(c) for c in elements(node) if c is not heading)
            return self.inline(node) + "\n\n"
        if node.tag == "img":
            return self.inline(node) + "\n\n"
        if node.tag in {"ul", "ol"}:
            out = ""
            for index, li in enumerate(elements(node), 1):
                children = [c for c in elements(li) if "s-marker" not in c.attrs.get("class", "")]
                nested = [c for c in children if c.tag in {"ul", "ol"}]
                if nested:
                    out += "".join(self.block(c, level + 1) for c in nested)
                else:
                    label = f"{index}. " if node.tag == "ol" else "- "
                    out += "  " * level + label + " ".join(self.inline(c) for c in children) + "\n"
            return out + "\n"
        children = elements(node)
        # A source-and-result demo needs only its source in Markdown. This
        # prevents its sample headings from becoming handbook headings.
        if len(children) == 2 and children[0].tag == "pre" and any(c.text() == "RESULT" for c in elements(children[1])):
            return self.block(children[0])
        refs = [self.reference(c) for c in children]
        if refs and all(ref is not None for ref in refs):
            out = "| Name | Values | Description |\n| --- | --- | --- |\n"
            return out + "".join("| " + " | ".join(ref) + " |\n" for ref in refs) + "\n"
        return "".join(self.block(c, level) for c in children)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--slml", default="target/debug/slml")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    target = root / "docs/handbook"
    target.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="slml-handbook-") as tmp:
        subprocess.run([str((root / args.slml).resolve()), str(root / "handbook"), "-o", tmp, "-q"], check=True)
        pages, anchors = {}, {}
        for file in sorted(Path(tmp).glob("*.html")):
            parsed = Parser()
            parsed.feed(file.read_text())
            title = next(n.text() for n in walk(parsed.root) if n.tag == "title")
            content = next(n for n in walk(parsed.root) if elements(n) and elements(n)[0].tag == "h1" and elements(n)[0].text() == title)
            name = file.stem + ".md"
            pages[name] = (title, content)
            anchors[name] = {}
            for node in walk(content):
                if "id" in node.attrs:
                    heading = next((n for n in walk(node) if n.tag in {"h1", "h2", "h3", "h4"}), None)
                    if heading:
                        anchors[name][node.attrs["id"]] = slug(heading.text())
        outputs = {}
        for name, (_, content) in pages.items():
            md = Markdown(name, anchors).block(content).strip()
            outputs[name] = md + f"\n\n[SLML source](../../handbook/{Path(name).stem}.slml)\n"
        order = ["index", "install", "first-page", "syntax", "text", "layout-guide", "styling", "content", "links", "components", "forms", "sites", "website-tutorial", "cli", "errors", "from-html", "reference"]
        outputs["README.md"] = "# SLML Handbook\n\nRead these Markdown chapters on GitHub, or open this folder in Obsidian.\nThe canonical source is the [SLML handbook](../../handbook/).\n\n" + "".join(f"{i}. [{pages[name + '.md'][0]}]({name}.md)\n" for i, name in enumerate(order, 1)) + "\nTo refresh this edition after changing the SLML sources:\n\n```sh\ncargo build -p slml-cli\npython3 scripts/sync-handbook-markdown.py\n```\n"
        for name, text in outputs.items():
            # Every local chapter link must survive outside a web server.
            # SLML's own link markup looks like Markdown, so code samples are
            # left out of the check.
            prose = re.sub(r"```.*?```", "", text, flags=re.S)
            prose = re.sub(r"`[^`\n]*`", "", prose)
            for href in re.findall(r"\]\(([^)]+)\)", prose):
                parsed = urlsplit(href)
                if not parsed.scheme and parsed.path.endswith(".md") and not parsed.path.startswith("../"):
                    if parsed.path not in outputs:
                        raise ValueError(f"{name}: missing Markdown target {href}")
                if not parsed.scheme and parsed.fragment and (not parsed.path or parsed.path in pages):
                    destination = parsed.path or name
                    known = set(anchors.get(destination, {}).values())
                    if destination in pages:
                        known.add(slug(pages[destination][0]))
                    if parsed.fragment not in known:
                        raise ValueError(f"{name}: missing Markdown heading {href}")
            output = target / name
            if args.check:
                if not output.is_file() or output.read_text() != text:
                    raise SystemExit(f"Markdown handbook is out of date: {output}")
            else:
                output.write_text(text)
    print(f"Markdown handbook {'verified' if args.check else 'written'}: {len(outputs) - 1} chapters")


if __name__ == "__main__":
    main()
