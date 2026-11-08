#!/usr/bin/env python3
"""Makes the screenshots of the project website in docs/screenshots.

Usage: python3 tools/screenshots/generate.py [screen ...]

1. demo_repo.py builds "lumen", a made-up repository.
2. The docs_dump example (dump.rs) records the backend's responses for it.
3. The UI is built with mock.js standing in for the backend, answering from
   those responses.
4. Headless Firefox captures each screen that shots.json lists, at twice the
   window size for HiDPI screens.
5. Each capture is written as a full-size WebP and a thumbnail cropped to its
   feature. Composites, such as the themes, tile several captures.

Naming screens or composites captures and writes only those. Work files go to
target/screenshots. It needs Python 3 with Pillow, Firefox, git, Cargo, and
npm with ui/node_modules installed.
"""

import functools
import http.server
import json
import shutil
import subprocess
import sys
import threading
import time
import urllib.parse
from pathlib import Path

from PIL import Image

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]
WORK = REPO_ROOT / "target" / "screenshots"
OUTPUT = REPO_ROOT / "docs" / "screenshots"
THUMBNAIL_WIDTH = 800
# The gap between composite tiles, in the pixels of the full-size image.
TILE_GAP = 4
TILE_BACKGROUND = (10, 10, 11)


def run(command, cwd=REPO_ROOT):
    print("$", " ".join(str(part) for part in command), flush=True)
    subprocess.run([str(part) for part in command], cwd=cwd, check=True)


def build_demo_repository():
    run([sys.executable, HERE / "demo_repo.py", WORK / "demo"])
    return WORK / "demo" / "lumen"


def dump_responses(workdir, searches):
    dump = WORK / "dump.json"
    run(["cargo", "run", "--quiet", "--example", "docs_dump", "--", workdir, dump, *searches])
    return dump


def build_app(config, workdir, dump):
    """Builds the UI into target/screenshots/app with the mock and its data."""
    app = WORK / "app"
    npm = shutil.which("npm")
    if npm is None:
        sys.exit("npm wasn't found")
    run([npm, "run", "build", "--", "--base", "./", "--outDir", app, "--emptyOutDir"], cwd=REPO_ROOT / "ui")
    shutil.copy(HERE / "mock.js", app / "mock.js")
    index = app / "index.html"
    html = index.read_text(encoding="utf-8")
    # The mock has to be in place before the app's own scripts run.
    index.write_text(html.replace("<head>", '<head>\n    <script src="./mock.js"></script>', 1), encoding="utf-8")

    # The responses name the demo repository's real folder, which the
    # screenshots show as the first tab's folder instead.
    text = dump.read_text(encoding="utf-8").replace(str(workdir), config["tabs"][0])
    data = {"tabs": config["tabs"], "recent": config["recent"], "responses": json.loads(text)}
    (app / "data.json").write_text(json.dumps(data), encoding="utf-8")
    return app


class Server(http.server.ThreadingHTTPServer):
    """Serves the app, holds back its load event on /delay, and collects what
    the mock posts to /report."""

    def __init__(self, app):
        self.reports = []
        super().__init__(("127.0.0.1", 0), functools.partial(Handler, directory=str(app)))

    @property
    def port(self):
        return self.server_address[1]

    def handle_error(self, request, client_address):
        # Firefox drops the held frame's connection once it has captured the page.
        if not isinstance(sys.exc_info()[1], ConnectionError):
            super().handle_error(request, client_address)


class Handler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        url = urllib.parse.urlparse(self.path)
        if url.path != "/delay":
            super().do_GET()
            return
        milliseconds = int(urllib.parse.parse_qs(url.query).get("ms", ["0"])[0])
        time.sleep(milliseconds / 1000)
        body = b"<!doctype html><title>held</title>"
        self.send_response(200)
        self.send_header("Content-Type", "text/html")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        self.server.reports.append(self.rfile.read(length).decode("utf-8", "replace"))
        self.send_response(204)
        self.end_headers()

    def log_message(self, *args):
        pass


def firefox_profile():
    profile = WORK / "firefox-profile"
    shutil.rmtree(profile, ignore_errors=True)
    profile.mkdir(parents=True)
    # Without motion, nothing is captured halfway through a transition.
    (profile / "user.js").write_text('user_pref("ui.prefersReducedMotion", 1);\n', encoding="utf-8")
    return profile


def capture(firefox, server, profile, config, name, theme, steps):
    """Captures one screen to target/screenshots/captures, and returns the path."""
    width, height = config["window"]
    zoom = config["zoom"]
    query = urllib.parse.urlencode(
        {"theme": theme, "zoom": zoom, "hold": config["holdMs"], "steps": "||".join(steps)}
    )
    target = WORK / "captures" / f"{name}.png"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.unlink(missing_ok=True)
    server.reports.clear()
    subprocess.run(
        [
            firefox,
            "--headless",
            "--no-remote",
            "--profile",
            str(profile),
            "--screenshot",
            str(target),
            f"--window-size={width * zoom},{height * zoom}",
            f"http://127.0.0.1:{server.port}/index.html?{query}",
        ],
        capture_output=True,
        timeout=120,
    )
    for report in server.reports:
        print(f"  {name}: {report}")
    if not target.exists():
        sys.exit(f"Firefox didn't capture {name}")
    print(f"  captured {name}", flush=True)
    return target


def thumbnail(image, box, config):
    """Crops `box`, (x, y, width) in window pixels with the window's aspect
    ratio, and scales it to the thumbnail size."""
    x, y, width = box
    window_width, window_height = config["window"]
    zoom = config["zoom"]
    height = width * window_height / window_width
    region = image.crop((x * zoom, y * zoom, round((x + width) * zoom), round((y + height) * zoom)))
    return region.resize(thumbnail_size(config), Image.LANCZOS)


def thumbnail_size(config):
    window_width, window_height = config["window"]
    return THUMBNAIL_WIDTH, THUMBNAIL_WIDTH * window_height // window_width


def tile(images, size, gap):
    """Lays out `images` two by two in an image of `size`, `gap` apart."""
    width, height = size
    tile_width = (width - gap) // 2
    tile_height = (height - gap) // 2
    sheet = Image.new("RGB", size, TILE_BACKGROUND)
    for index, image in enumerate(images):
        column, row = index % 2, index // 2
        resized = image.resize((tile_width, tile_height), Image.LANCZOS)
        sheet.paste(resized, (column * (tile_width + gap), row * (tile_height + gap)))
    return sheet


def write_screen(name, capture_path, screen, config):
    image = Image.open(capture_path).convert("RGB")
    # Screenshots of the UI are mostly flat color, which lossless WebP keeps
    # smaller than lossy.
    image.save(OUTPUT / f"{name}.webp", lossless=True, method=6)
    thumbnail(image, screen["thumbnail"], config).save(OUTPUT / "thumbs" / f"{name}.webp", quality=90, method=6)


def write_composite(name, capture_paths, composite, config):
    images = [Image.open(path).convert("RGB") for path in capture_paths]
    full = tile(images, images[0].size, TILE_GAP)
    # Scaled down, the tiles lose their flat colors, so lossy is far smaller.
    full.save(OUTPUT / f"{name}.webp", quality=92, method=6)
    crops = [thumbnail(image, composite["thumbnail"], config) for image in images]
    tile(crops, thumbnail_size(config), TILE_GAP).save(OUTPUT / "thumbs" / f"{name}.webp", quality=90, method=6)


def main():
    config = json.loads((HERE / "shots.json").read_text(encoding="utf-8"))
    wanted = set(sys.argv[1:])
    known = set(config["screens"]) | set(config["composites"])
    if wanted - known:
        sys.exit(f"unknown screens: {', '.join(sorted(wanted - known))}")

    firefox = shutil.which("firefox")
    if firefox is None:
        sys.exit("firefox wasn't found")

    workdir = build_demo_repository()
    dump = dump_responses(workdir, config["searches"])
    app = build_app(config, workdir, dump)
    (OUTPUT / "thumbs").mkdir(parents=True, exist_ok=True)

    server = Server(app)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    profile = firefox_profile()
    try:
        for name, screen in config["screens"].items():
            if wanted and name not in wanted:
                continue
            path = capture(firefox, server, profile, config, name, screen.get("theme", "darkBlack"), screen["steps"])
            write_screen(name, path, screen, config)
        for name, composite in config["composites"].items():
            if wanted and name not in wanted:
                continue
            paths = [
                capture(firefox, server, profile, config, f"{name}-{theme}", theme, composite["steps"])
                for theme in composite["themes"]
            ]
            write_composite(name, paths, composite, config)
    finally:
        server.shutdown()
    print(f"Wrote the screenshots to {OUTPUT.relative_to(REPO_ROOT)}")


if __name__ == "__main__":
    main()
