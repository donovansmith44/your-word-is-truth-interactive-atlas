import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit


class PublishedClient(SimpleHTTPRequestHandler):
    def do_GET(self):
        if not Path(urlsplit(self.path).path).suffix:
            self.path = "/index.html"
        super().do_GET()


def main():
    arguments = argparse.ArgumentParser()
    arguments.add_argument("--port", type=int, default=5100)
    arguments.add_argument("--root", type=Path, default=Path("client-fsharp/bin/Release/net10.0/publish/wwwroot"))
    options = arguments.parse_args()
    if not (options.root / "index.html").is_file():
        arguments.error("publish the F# client before serving it")
    with ThreadingHTTPServer(("127.0.0.1", options.port), partial(PublishedClient, directory=str(options.root.resolve()))) as server:
        server.serve_forever()


if __name__ == "__main__":
    main()
