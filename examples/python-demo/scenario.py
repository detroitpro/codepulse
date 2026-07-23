"""Drive load against the python demo."""

from __future__ import annotations

import json
import sys
import urllib.request

BASE = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8000"


def post(path: str, body: dict) -> None:
    data = json.dumps(body).encode()
    req = urllib.request.Request(
        BASE + path,
        data=data,
        headers={"content-type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=5) as resp:
        resp.read()


def main() -> None:
    for i in range(40):
        post("/checkout", {"total": 10 + i, "sku": "SKU-1"})
    print("scenario complete")


if __name__ == "__main__":
    main()
