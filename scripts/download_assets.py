#!/usr/bin/env python3
import urllib.request
import json
import os
import sys

MANIFEST_URL = "https://launchermeta.mojang.com/mc/game/version_manifest.json"
TARGET_VERSION = "1.21.1"
ASSETS_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets", "raw")
OUTPUT_JAR = os.path.join(ASSETS_DIR, f"{TARGET_VERSION}.jar")

def main():
    if not os.path.exists(ASSETS_DIR):
        os.makedirs(ASSETS_DIR)

    print(f"Fetching version manifest from {MANIFEST_URL}...")
    try:
        with urllib.request.urlopen(MANIFEST_URL) as response:
            manifest = json.loads(response.read().decode('utf-8'))
    except Exception as e:
        print(f"Failed to fetch manifest: {e}")
        sys.exit(1)

    version_json_url = None
    for version in manifest.get("versions", []):
        if version.get("id") == TARGET_VERSION:
            version_json_url = version.get("url")
            break

    if not version_json_url:
        print(f"Version {TARGET_VERSION} not found in manifest.")
        sys.exit(1)

    print(f"Fetching version details from {version_json_url}...")
    try:
        with urllib.request.urlopen(version_json_url) as response:
            version_data = json.loads(response.read().decode('utf-8'))
    except Exception as e:
        print(f"Failed to fetch version details: {e}")
        sys.exit(1)

    client_download_info = version_data.get("downloads", {}).get("client")
    if not client_download_info:
        print("Client download info not found.")
        sys.exit(1)

    client_url = client_download_info.get("url")
    client_size = client_download_info.get("size")

    print(f"Found client jar: {client_url} (Size: {client_size} bytes)")

    if os.path.exists(OUTPUT_JAR):
        print(f"File {OUTPUT_JAR} already exists. Skipping download.")
        sys.exit(0)

    print(f"Downloading to {OUTPUT_JAR}...")
    try:
        urllib.request.urlretrieve(client_url, OUTPUT_JAR)
        print("Download complete!")
    except Exception as e:
        print(f"Failed to download jar: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
