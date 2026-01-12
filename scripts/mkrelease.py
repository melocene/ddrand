#!/usr/bin/env python3

import hashlib
import os
import shutil
import subprocess

proj_name = ""
proj_vers = ""

with open("Cargo.toml", "r") as cfile:
    for line in cfile.readlines():
        if line.startswith("name"):
            proj_name = line.split("=")[1].strip().strip('"')
        if line.startswith("version"):
            proj_vers = line.split("=")[1].strip().strip('"')

output_file = f"{proj_name}-v{proj_vers}-x86_64-windows"
zip_name = f"{output_file}.zip"

rel_path = os.path.join("target", "release", proj_name)
if not os.path.exists(rel_path):
    subprocess.run(["cargo", "build", "--release"])

if os.path.exists(proj_name):
    shutil.rmtree(proj_name)
if os.path.exists(zip_name):
    os.remove(zip_name)

os.makedirs(proj_name)

packing_list = [
    "CHANGELOG.md",
    "LICENSE",
    "README.md",
    f"target\\release\\{proj_name}.exe",
]
for file in packing_list:
    srcfile = os.path.abspath(file)
    dstfile = os.path.join(proj_name, os.path.basename(srcfile))
    shutil.copy(srcfile, dstfile)

os.chdir(proj_name)

digest_blake2 = {}
digest_sha256 = {}

for file in os.scandir("."):
    if file.name != "B2SUMS" and file.name != "SHA256SUMS":
        with open(file, "rb") as infile:
            buf = infile.read()

            hasher_blake2 = hashlib.blake2b()
            hasher_blake2.update(buf)
            digest_blake2[file.name] = hasher_blake2.hexdigest()

            hasher_sha256 = hashlib.sha256()
            hasher_sha256.update(buf)
            digest_sha256[file.name] = hasher_sha256.hexdigest()

with open("B2SUMS", "a") as b2sums:
    for k, v in digest_blake2.items():
        b2sums.write(f"{v} {k}\n")
with open("SHA256SUMS", "a") as sha256sums:
    for k, v in digest_sha256.items():
        sha256sums.write(f"{v} {k}\n")

os.chdir("..")

if os.name == "nt":
    shutil.make_archive(output_file, "zip", root_dir=".", base_dir=proj_name)

shutil.rmtree(proj_name)
