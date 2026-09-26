#!/usr/bin/env python3
"""Reconstruct M1 source from its delivered archive for the identical harness."""
from pathlib import Path
import shutil
import tarfile

root=Path(__file__).resolve().parents[1]
dest=root/'.baseline'
dest.mkdir(exist_ok=True)
with tarfile.open(root/'dist/rustj-m1-source.tar.gz') as archive:
    archive.extractall(dest,filter='data')
shutil.copyfile(root/'benches/comparison.rs',dest/'benches/comparison.rs')
manifest=dest/'Cargo.toml'
text=manifest.read_text()
if 'name = "comparison"' not in text:
    manifest.write_text(text+'\n[[bench]]\nname = "comparison"\nharness = false\n')
print(dest)
