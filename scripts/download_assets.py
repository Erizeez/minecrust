#!/usr/bin/env python3
import urllib.request
import json
import os
import sys
import concurrent.futures
import threading

MANIFEST_URL = "https://launchermeta.mojang.com/mc/game/version_manifest.json"
TARGET_VERSION = "1.21.1"
ASSETS_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets", "raw")
OUTPUT_JAR = os.path.join(ASSETS_DIR, f"{TARGET_VERSION}.jar")

# Thread-safe counter for progress tracking
progress_lock = threading.Lock()
downloaded_count = 0
skipped_count = 0
failed_count = 0

def download_file(key, obj):
    global downloaded_count, skipped_count, failed_count
    
    hash_val = obj["hash"]
    size = obj["size"]
    
    # Minecraft resource URL
    url = f"https://resources.download.minecraft.net/{hash_val[:2]}/{hash_val}"
    
    # Target path preserving original structure
    target_path = os.path.join(ASSETS_DIR, key)
    
    # If file exists and size matches, skip it
    if os.path.exists(target_path) and os.path.getsize(target_path) == size:
        with progress_lock:
            skipped_count += 1
        return
        
    # Ensure target directory exists
    os.makedirs(os.path.dirname(target_path), exist_ok=True)
    
    # Download with retries
    for attempt in range(3):
        try:
            # Temporary file to prevent corrupt incomplete downloads
            temp_path = target_path + ".tmp"
            urllib.request.urlretrieve(url, temp_path)
            
            # Validate size
            if os.path.getsize(temp_path) == size:
                os.replace(temp_path, target_path)
                with progress_lock:
                    downloaded_count += 1
                return
            else:
                os.remove(temp_path)
        except Exception:
            pass
            
    with progress_lock:
        failed_count += 1
    print(f"\n[FAIL] Failed to download {key} after 3 attempts.")

def main():
    global downloaded_count, skipped_count, failed_count
    
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
    else:
        print(f"Downloading to {OUTPUT_JAR}...")
        try:
            urllib.request.urlretrieve(client_url, OUTPUT_JAR)
            print("Jar download complete!")
        except Exception as e:
            print(f"Failed to download jar: {e}")
            sys.exit(1)

    # Extract en_us.json from Client JAR
    import zipfile
    print("\nExtracting en_us.json from client jar...")
    try:
        with zipfile.ZipFile(OUTPUT_JAR, 'r') as zip_ref:
            target_lang_path = os.path.join(ASSETS_DIR, "minecraft", "lang", "en_us.json")
            os.makedirs(os.path.dirname(target_lang_path), exist_ok=True)
            with zip_ref.open("assets/minecraft/lang/en_us.json") as source_file:
                with open(target_lang_path, 'wb') as target_file:
                    target_file.write(source_file.read())
            print("Successfully extracted en_us.json!")
    except Exception as e:
        print(f"Failed to extract en_us.json: {e}")

    # Download Minecraft Font
    font_url = "https://github.com/tryashtar/minecraft-ttf/releases/latest/download/MinecraftDefault-Regular.ttf"
    font_target_path = os.path.join(ASSETS_DIR, "font", "MinecraftDefault-Regular.ttf")
    print(f"\nDownloading Minecraft font from {font_url}...")
    try:
        os.makedirs(os.path.dirname(font_target_path), exist_ok=True)
        urllib.request.urlretrieve(font_url, font_target_path)
        print(f"Font download complete! Saved to {font_target_path}")
    except Exception as e:
        print(f"Failed to download font: {e}")

    # Download GNU Unifont (Minecraft CJK Pixel Font)
    unifont_url = "https://github.com/multitheftauto/unifont/releases/download/v16.0.04/unifont-16.0.04.ttf"
    unifont_target_path = os.path.join(ASSETS_DIR, "font", "unifont.ttf")
    print(f"\nDownloading Minecraft Chinese font (GNU Unifont) from {unifont_url}...")
    try:
        os.makedirs(os.path.dirname(unifont_target_path), exist_ok=True)
        if os.path.exists(unifont_target_path):
            print(f"Chinese font already exists at {unifont_target_path}. Skipping.")
        else:
            urllib.request.urlretrieve(unifont_url, unifont_target_path)
            print(f"Chinese font download complete! Saved to {unifont_target_path}")
    except Exception as e:
        print(f"Failed to download Chinese font: {e}")

    print("\nFetching asset index for complete asset syncing...")
    try:
        asset_index_url = version_data.get("assetIndex", {}).get("url")
        if not asset_index_url:
            print("Failed to find asset index URL.")
            sys.exit(1)
            
        with urllib.request.urlopen(asset_index_url) as response:
            asset_index = json.loads(response.read().decode('utf-8'))
            
        objects = asset_index.get("objects", {})
        total_files = len(objects)
        print(f"Loaded asset index. Total files to verify/download: {total_files}")
        
        # Write asset catalog for dynamic on-demand loading in client
        catalog_path = os.path.join("assets", "processed", "asset_catalog.json")
        os.makedirs(os.path.dirname(catalog_path), exist_ok=True)
        try:
            with open(catalog_path, 'w') as f:
                json.dump(objects, f, indent=2)
            print(f"Serialized complete asset catalog to {catalog_path}")
        except Exception as e:
            print(f"Failed to serialize asset catalog: {e}")
        
        # Concurrent downloads
        max_workers = 16
        print(f"Starting sync with {max_workers} concurrent download threads...")
        
        with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = {
                executor.submit(download_file, key, obj): key 
                for key, obj in objects.items()
            }
            
            # Print a neat progress updates
            last_total = -1
            for i, future in enumerate(concurrent.futures.as_completed(futures)):
                total_processed = downloaded_count + skipped_count + failed_count
                percent = (total_processed / total_files) * 100
                
                # Update output once in a while or when a new milestone is reached to keep output clean
                if total_processed // 100 > last_total // 100 or total_processed == total_files:
                    last_total = total_processed
                    sys.stdout.write(
                        f"\rProgress: {total_processed}/{total_files} ({percent:.1f}%) | "
                        f"Downloaded: {downloaded_count} | Skipped: {skipped_count} | Failed: {failed_count}"
                    )
                    sys.stdout.flush()
                    
        print(f"\n\nSync complete!")
        print(f"Successfully processed all {total_files} assets.")
        print(f"- Downloaded: {downloaded_count}")
        print(f"- Skipped (Already existed): {skipped_count}")
        print(f"- Failed: {failed_count}")
        
    except Exception as e:
        print(f"\nFailed to process asset sync: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
