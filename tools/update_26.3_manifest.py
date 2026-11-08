#!/usr/bin/env python3
"""Refresh manifests/26.3.json from Mojang's public files.

Every hash must be one the repository's research lock accepts; anything else is
refused instead of written.
"""
import datetime
import hashlib
import json
import pathlib
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
LOCK = json.loads((ROOT / "backend/research-lock.json").read_text(encoding="utf-8"))["minecraft"]
VERSION = LOCK["version"]
PIN = LOCK["manifest_sha1"]
ACCEPTED_MANIFESTS = {PIN, *LOCK.get("accepted_manifest_sha1", [])}
ACCEPTED_INDEXES = {*LOCK.get("accepted_asset_index_sha1", [])}


def fetch(url: str) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": "LiteCraft-Launcher-Manifest"})
    with urllib.request.urlopen(request, timeout=90) as response:
        return response.read()


def sha1(data: bytes) -> str:
    return hashlib.sha1(data).hexdigest()


def version_json() -> bytes:
    direct = f"https://piston-meta.mojang.com/v1/packages/{PIN}/{VERSION}.json"
    data = fetch(direct)
    if sha1(data) in ACCEPTED_MANIFESTS:
        return data
    manifest = json.loads(fetch("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"))
    entry = next((v for v in manifest["versions"] if v["id"] == VERSION), None)
    if entry is None:
        raise SystemExit(f"{VERSION} is not in Mojang's version manifest")
    data = fetch(entry["url"])
    if sha1(data) not in ACCEPTED_MANIFESTS:
        raise SystemExit(f"{VERSION} metadata changed to an unaccepted hash: {sha1(data)}")
    return data


def main() -> None:
    data = version_json()
    meta = json.loads(data)
    index = meta["assetIndex"]
    if ACCEPTED_INDEXES and index["sha1"] not in ACCEPTED_INDEXES:
        raise SystemExit(f"asset index hash is not accepted: {index['sha1']}")
    manifest = {
        "version": VERSION,
        "manifest_url": meta.get("url") or f"https://piston-meta.mojang.com/v1/packages/{sha1(data)}/{VERSION}.json",
        "manifest_sha1": sha1(data),
        "accepted_manifest_sha1": LOCK.get("accepted_manifest_sha1", []),
        "client": {
            "sha1": meta["downloads"]["client"]["sha1"],
            "size": meta["downloads"]["client"]["size"],
            "url": meta["downloads"]["client"]["url"],
        },
        "server": {
            "sha1": meta["downloads"]["server"]["sha1"],
            "size": meta["downloads"]["server"]["size"],
            "url": meta["downloads"]["server"]["url"],
        },
        "asset_index": {
            "id": index.get("id"),
            "sha1": index["sha1"],
            "size": index["size"],
            "url": index["url"],
        },
        "java_major": LOCK["java_major"],
        "checked_at": datetime.date.today().isoformat(),
    }
    path = ROOT / "manifests/26.3.json"
    old = json.loads(path.read_text(encoding="utf-8")) if path.exists() else {}
    old.pop("checked_at", None)
    current = dict(manifest)
    current.pop("checked_at")
    if old == current:
        print("manifest is current")
        return
    path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"updated {path}")


if __name__ == "__main__":
    main()
