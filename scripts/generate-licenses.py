#!/usr/bin/env python3
"""Generate src/THIRD-PARTY-LICENSES.txt from cargo metadata.

Run this before a release build so that the embedded license text is up to date:

    python3 scripts/generate-licenses.py
    cargo build --release
"""

import hashlib
import json
import os
import glob
import subprocess
import sys
from collections import defaultdict

OUTPUT = os.path.join(os.path.dirname(__file__), "..", "src", "THIRD-PARTY-LICENSES.txt")


def main():
    result = subprocess.run(
        ["cargo", "metadata", "--format-version=1"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print("cargo metadata failed:", result.stderr, file=sys.stderr)
        sys.exit(1)

    meta = json.loads(result.stdout)
    workspace_members = set(meta.get("workspace_members", []))
    resolved_ids = {node["id"] for node in meta.get("resolve", {}).get("nodes", [])}

    # Group crates by identical license text (hash)
    license_groups: dict[str, tuple[str, str, list]] = {}
    crates_no_file: list[tuple[str, str, str, str]] = []

    pkgs = sorted(meta["packages"], key=lambda p: p["name"].lower())
    for pkg in pkgs:
        if pkg["id"] in workspace_members:
            continue
        if pkg["id"] not in resolved_ids:
            continue

        name = pkg["name"]
        version = pkg["version"]
        license_field = pkg.get("license", "Unknown")
        repo = pkg.get("repository", "")
        crate_dir = os.path.dirname(pkg.get("manifest_path", ""))

        license_files = []
        for pattern in ["LICENSE*", "LICENCE*", "COPYING*", "COPYRIGHT*"]:
            license_files.extend(glob.glob(os.path.join(crate_dir, pattern)))

        if license_files:
            for lf in sorted(license_files):
                try:
                    text = open(lf).read().strip()
                    h = hashlib.sha256(text.encode()).hexdigest()[:16]
                    if h not in license_groups:
                        license_groups[h] = (text, os.path.basename(lf), [])
                    license_groups[h][2].append((name, version, license_field, repo))
                except OSError:
                    crates_no_file.append((name, version, license_field, repo))
        else:
            crates_no_file.append((name, version, license_field, repo))

    # Build output
    lines = [
        "THIRD-PARTY SOFTWARE LICENSES",
        "=" * 30,
        "",
        "PicoNote includes the following third-party software.",
        "",
        "Licenses are grouped to avoid repetition of identical license texts.",
        "",
    ]

    group_num = 0
    sorted_groups = sorted(
        license_groups.items(), key=lambda x: x[1][2][0][0].lower()
    )
    for _h, (text, _fname, crates) in sorted_groups:
        group_num += 1
        plural = "s" if len(crates) != 1 else ""
        lines.append("=" * 80)
        lines.append(f"License Group {group_num} ({len(crates)} crate{plural}):")
        lines.append("")
        for cname, cver, clic, crepo in sorted(crates, key=lambda x: x[0].lower()):
            line = f"  - {cname} v{cver} ({clic})"
            if crepo:
                line += f"  {crepo}"
            lines.append(line)
        lines.append("")
        lines.append(text)
        lines.append("")

    if crates_no_file:
        plural = "s" if len(crates_no_file) != 1 else ""
        lines.append("=" * 80)
        lines.append(
            f"Crates with license declared in Cargo.toml only ({len(crates_no_file)} crate{plural}):"
        )
        lines.append("")
        for cname, cver, clic, crepo in sorted(
            crates_no_file, key=lambda x: x[0].lower()
        ):
            line = f"  - {cname} v{cver} ({clic})"
            if crepo:
                line += f"  {crepo}"
            lines.append(line)

    output_path = os.path.normpath(OUTPUT)
    with open(output_path, "w") as f:
        f.write("\n".join(lines))

    print(
        f"Wrote {output_path}: {group_num} license groups, "
        f"{len(crates_no_file)} crates without files"
    )


if __name__ == "__main__":
    main()
