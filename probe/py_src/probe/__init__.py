from probe._probe import init

__all__ = ["init"]

try:
    from importlib.metadata import version

    __version__ = version("probe")
except Exception:
    __version__ = "unknown version"
