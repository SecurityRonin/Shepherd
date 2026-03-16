#!/usr/bin/env python3
"""
shepherd-bridge.py — iTerm2 AutoLaunch bridge

Must be installed as an iTerm2 Python API AutoLaunch script so the iTerm2
runtime injects ITERM2_COOKIE and ITERM2_KEY into the process environment.
A plain Python script NOT invoked via iterm2.run_until_complete() will NOT
receive these env vars.

Install at:
  ~/Library/Application Support/iTerm2/Scripts/AutoLaunch/shepherd-bridge.py

iTerm2 must have the Python API enabled:
  Preferences → General → Magic → Enable Python API
"""
import iterm2   # provided by iTerm2's embedded Python environment
import json
import os
import pathlib
import stat


async def main(connection):
    cookie = os.environ.get("ITERM2_COOKIE", "")
    key = os.environ.get("ITERM2_KEY", "")

    if not cookie or not key:
        print("shepherd-bridge: ITERM2_COOKIE/KEY not available")
        return

    auth_dir = pathlib.Path.home() / ".shepherd"
    auth_dir.mkdir(parents=True, exist_ok=True)
    auth_path = auth_dir / "iterm2-auth.json"
    auth_path.write_text(json.dumps({"cookie": cookie, "key": key}))
    auth_path.chmod(stat.S_IRUSR | stat.S_IWUSR)  # 0600
    print(f"shepherd-bridge: credentials written to {auth_path}")


iterm2.run_until_complete(main)
