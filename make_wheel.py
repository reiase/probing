import hashlib
import os
import pathlib
import stat
from email.message import EmailMessage
from zipfile import ZIP_DEFLATED, ZipInfo

import toml
from wheel.wheelfile import WheelFile

target_dir = "debug" if "DEBUG" in os.environ else "release"
if target_dir == "debug":
    print("Building in debug mode...")

def make_message(headers, payload=None):
    msg = EmailMessage()
    for name, value in headers.items():
        if isinstance(value, list):
            for value_part in value:
                msg[name] = value_part
        else:
            msg[name] = value
    if payload:
        msg.set_payload(payload)
    return msg


def write_wheel_file(filename, contents):
    with WheelFile(filename, "w") as wheel:
        for member_info, member_source in contents.items():
            if not isinstance(member_info, ZipInfo):
                member_info = ZipInfo(str(member_info))
                member_info.external_attr = 0o644 << 16
            member_info.file_size = len(member_source)
            member_info.compress_type = ZIP_DEFLATED
            wheel.writestr(member_info, bytes(member_source))
    return filename


def write_wheel(out_dir, *, name, version, tag, metadata, description, contents):
    name_snake = name.replace("-", "_")
    wheel_name = f"{name_snake}-{version}-{tag}.whl"
    dist_info = f"{name_snake}-{version}.dist-info"
    return write_wheel_file(
        os.path.join(out_dir, wheel_name),
        {
            **contents,
            f"{dist_info}/METADATA": make_message(
                {
                    "Metadata-Version": "2.1",
                    "Name": name,
                    "Version": version,
                    **metadata,
                },
                description,
            ),
            f"{dist_info}/WHEEL": make_message(
                {
                    "Wheel-Version": "1.0",
                    "Generator": "probing.make_wheel",
                    "Root-Is-Purelib": "false",
                    "Tag": tag,
                }
            ),
        },
    )


def write_probing_wheel(
    out_dir, *, platform="manylinux_2_12_x86_64.manylinux2010_x86_64"
):
    contents = {}
    meta = toml.load("Cargo.toml")
    package_meta = meta.get("package", {})
    workspace_meta = meta.get("workspace", {}).get("package", {})
    metadata = {
        "version": workspace_meta.get("version").replace("-", "") or package_meta.get("version").replace("-", ""),
        "authors": workspace_meta.get("authors", []) or package_meta.get("authors", []),
        "license": workspace_meta.get("license") or package_meta.get("license", ""),
        "description": workspace_meta.get("description", "") or package_meta.get("description", ""),  # Only in package
        "repository": package_meta.get("repository", ""),  # Only in package
        "homepage": package_meta.get("homepage", ""),  # Only in package
        "keywords": package_meta.get("keywords", []),  # Only in package
    }

    # Create the output directory if it does not exist
    out_dir_path = pathlib.Path(out_dir)
    out_dir_path.mkdir(parents=True, exist_ok=True)

    if "ZIG" in os.environ:
        target_dir_prefix = pathlib.Path(f"target/x86_64-unknown-linux-gnu/{target_dir}")
    else:
        target_dir_prefix = pathlib.Path(f"target/{target_dir}")

    # Add executables and the shared library directly from the build directory
    for dest, src in {
        pathlib.Path(f"probing-{metadata['version']}.data/scripts/probing"):target_dir_prefix / "probing",
        pathlib.Path(f"probing-{metadata['version']}.data/scripts/probing-repl"):pathlib.Path("python/probing-repl"),
        pathlib.Path("probing/libprobing.so"): target_dir_prefix / "libprobing.so",
    }.items():
        zip_info = ZipInfo(str(dest))
        if ".data/scripts/" in str(dest):
            zip_info.external_attr = (stat.S_IFREG | 0o755) << 16
        else:
            zip_info.external_attr = (stat.S_IFREG | 0o644) << 16

        with open(src, "rb") as f:
            contents[zip_info] = f.read()

    def add_package_files_recursively(directory, contents, base_dir):
        """Recursively add package files from directory to contents."""
        dir_path = pathlib.Path(directory)

        for entry in dir_path.iterdir():
            if entry.is_file() and entry.suffix == '.py':
                pkg_path = entry.relative_to(base_dir)
                with open(entry, "rb") as f:
                    zip_info = ZipInfo(str(pkg_path))
                    zip_info.external_attr = 0o644 << 16
                    contents[zip_info] = f.read()
                    print(f"add file: {pkg_path}")
            elif entry.is_dir():
                add_package_files_recursively(entry, contents, base_dir)

    python_dir = pathlib.Path("python")
    add_package_files_recursively(python_dir, contents, python_dir)

    pth_info = ZipInfo("probing.pth")
    contents[pth_info] = "import probing_hook".encode("utf-8")

    with open("README.md", "rb") as f:
        description = f.read()

    return write_wheel(
        out_dir,
        name="probing",
        version=metadata["version"],
        tag=f"py3-none-{platform}",
        metadata={
            "Summary": metadata["description"],
            "Description-Content-Type": "text/markdown",
            "License": metadata["license"],
            "Classifier": [
                "Programming Language :: Python :: 3",
                "Operating System :: POSIX :: Linux",
            ],
            "Project-URL": [
                f"Homepage, {metadata['homepage']}",
                f"Repository, {metadata['repository']}",
            ],
            "Keywords": ", ".join(metadata["keywords"]),
            "Author": ", ".join(metadata["authors"]),
            "Requires-Python": ">=3.7",
        },
        description=description,
        contents=contents,
    )


def main():
    wheel_path = write_probing_wheel("dist/")
    with open(wheel_path, "rb") as wheel:
        print(f"  {wheel_path}")
        print(f"    {hashlib.sha256(wheel.read()).hexdigest()}")


if __name__ == "__main__":
    main()