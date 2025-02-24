import os
import stat
import hashlib
import pathlib
from email.message import EmailMessage
from wheel.wheelfile import WheelFile
from zipfile import ZipInfo, ZIP_DEFLATED
from inspect import cleandoc

PLATFORM = "manylinux_2_12_x86_64.manylinux2010_x86_64"


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


def write_wheel(
    out_dir, *, name, version, tag, metadata, description, contents, entry_points
):
    name_snake = name.replace("-", "_")
    wheel_name = f"{name_snake}-{version}-{tag}.whl"
    dist_info = f"{name_snake}-{version}.dist-info"
    if entry_points:
        contents[f"{dist_info}/entry_points.txt"] = (
            (
                cleandoc(
                    """
            [console_scripts]
            {entry_points}
        """
                ).format(
                    entry_points="\n".join(
                        [f"{k} = {v}" for k, v in entry_points.items()]
                        if entry_points
                        else []
                    )
                )
            ).encode("ascii"),
        )
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
                    "Generator": "make_wheels.py",
                    "Root-Is-Purelib": "false",
                    "Tag": tag,
                }
            ),
        },
    )


def write_probing_wheel(out_dir, *, version, platform):
    contents = {}
    entry_points = {}

    # Create the output directory if it does not exist
    out_dir_path = pathlib.Path(out_dir)
    if not out_dir_path.exists():
        out_dir_path.mkdir(parents=True)

    for name, path in {
        "probing": "target/x86_64-unknown-linux-gnu/release/probing",
        "libprobing.so": "target/x86_64-unknown-linux-gnu/release/libprobing.so",
    }.items():
        zip_info = ZipInfo(f"probing-{version}.data/scripts/{name}")
        zip_info.external_attr = (stat.S_IFREG | 0o755) << 16
        with open(path, "rb") as f:
            contents[zip_info] = f.read()

    contents["probing/__init__.py"] = b""

    with open("README.md", "rb") as f:
        description = f.read()

    return write_wheel(
        out_dir,
        name="probing",
        version=version,
        tag=f"py3-none-{platform}",
        metadata={
            "Summary": "A Python package for probing and monitoring system performance",
            "Description-Content-Type": "text/markdown",
            "License": "MIT",
            "Classifier": [
                "License :: OSI Approved :: MIT License",
                "Programming Language :: Python :: 3",
                "Operating System :: POSIX :: Linux",
                "Topic :: System :: Monitoring",
                "Topic :: System :: Systems Administration",
            ],
            "Project-URL": [
                "Source Code, https://github.com/reiase/probing",
            ],
            "Requires-Python": ">=3.7",
        },
        description=description,
        contents=contents,
        entry_points=entry_points,
    )


def main():
    import toml

    meta = toml.load("Cargo.toml")
    wheel_version = meta["workspace"]["package"]["version"]
    print("--")
    print("Making Wheels for version", wheel_version)

    wheel_path = write_probing_wheel("dist/", version=wheel_version, platform=PLATFORM)
    with open(wheel_path, "rb") as wheel:
        print(f"  {wheel_path}")
        print(f"    {hashlib.sha256(wheel.read()).hexdigest()}")


if __name__ == "__main__":
    main()
