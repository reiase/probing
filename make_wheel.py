import hashlib
import os
import pathlib
import stat
import sys
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
                member_info = ZipInfo(member_info)
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


def get_platform_tag():
    """Gets the appropriate platform tag for the wheel."""
    if sys.platform == "darwin":
        import platform

        # For macOS, use a baseline version for better compatibility, especially for arm64
        if platform.machine() == "arm64":
            return "macosx_11_0_arm64"
        else:
            # For x86_64, you might want to target an older version as well
            return f"macosx_10_9_{platform.machine()}"
    elif sys.platform.startswith("linux"):
        return "manylinux_2_12_x86_64.manylinux2010_x86_64"
    # Add other platforms as needed
    raise RuntimeError(f"Unsupported platform: {sys.platform}")


def write_probing_wheel(out_dir):
    platform = get_platform_tag()
    contents = {}
    meta = toml.load("Cargo.toml")
    package_meta = meta.get("package", {})
    workspace_meta = meta.get("workspace", {}).get("package", {})
    metadata = {
        "version": workspace_meta.get("version").replace("-", "")
        or package_meta.get("version").replace("-", ""),
        "authors": workspace_meta.get("authors", []) or package_meta.get("authors", []),
        "license": workspace_meta.get("license") or package_meta.get("license", ""),
        "description": workspace_meta.get("description", "")
        or package_meta.get("description", ""),  # Only in package
        "repository": package_meta.get("repository", ""),  # Only in package
        "homepage": package_meta.get("homepage", ""),  # Only in package
        "keywords": package_meta.get("keywords", []),  # Only in package
    }

    # Create the output directory if it does not exist
    out_dir_path = pathlib.Path(out_dir)
    if not out_dir_path.exists():
        out_dir_path.mkdir(parents=True)

    if "ZIG" in os.environ:
        target_dir_prefix = f"target/x86_64-unknown-linux-gnu/{target_dir}"
    else:
        target_dir_prefix = f"target/{target_dir}"

    lib_ext = "dylib" if sys.platform == "darwin" else "so"
    lib_name = f"libprobing.{lib_ext}"
    for name, path in {
        "probing": f"{target_dir_prefix}/probing",
        lib_name: f"{target_dir_prefix}/{lib_name}",
        "probing-repl": "python/probing-repl",
    }.items():
        zip_info = ZipInfo(f"probing-{metadata['version']}.data/scripts/{name}")
        zip_info.external_attr = (stat.S_IFREG | 0o755) << 16
        with open(path, "rb") as f:
            contents[zip_info] = f.read()

    def add_python_files_recursively(directory, contents, base_dir):
        """Recursively add Python files from directory to contents.

        Args:
            directory: The directory to process
            contents: The dict to add files to
            base_dir: The base directory for relative paths
        """
        dir_path = pathlib.Path(directory)

        # Process all entries in the directory
        for entry in dir_path.iterdir():
            if entry.is_file() and entry.suffix == ".py":
                # Handle Python file
                pkg_path = entry.relative_to(base_dir)
                with open(entry, "rb") as f:
                    zip_info = ZipInfo(str(pkg_path))
                    zip_info.external_attr = 0o644 << 16
                    contents[zip_info] = f.read()
                    print(f"add file: {pkg_path}")
            elif entry.is_dir():
                # Recursively process subdirectory
                add_python_files_recursively(entry, contents, base_dir)

    python_dir = pathlib.Path("python")
    add_python_files_recursively(python_dir, contents, python_dir)

    pth_info = ZipInfo(f"probing.pth")
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
