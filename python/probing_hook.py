"""
Probing Hook - Conditionally activates the probing library based on environment variables.

This module intercepts process startup and conditionally imports the probing library
based on the PROBING environment variable:
- '0': Disabled (default)
- '1' or 'followed': Enable only in current process
- '2' or 'nested': Enable in current and all child processes

"""

import os
import psutil
import sys
import logging

logging.basicConfig(
    level=os.environ.get("LOG_LEVEL", "INFO").upper(),
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S"
)

logger = logging.getLogger(__name__)


def launched_by_torchrun(recursion=False) -> bool:
    if recursion:
        iter = 3
    else:
        iter = 1
    try:
        p = psutil.Process(os.getppid())
        for _ in range(iter):
            cmdline = p.cmdline()
            if (
                p.name() == "torchrun"
                or any(os.path.basename(arg) == "torchrun" for arg in cmdline)
                or "torch.distributed.run" in cmdline
            ):
                return True
            p = p.parent() if p.parent() else None
            if p is None:
                break
    except Exception:
        pass
    return False


def get_current_script_name():
    """Get the name of the current running script."""
    try:
        script_path = sys.argv[0]
        return os.path.basename(script_path)
    except (IndexError, AttributeError):
        return "<unknown>"


# Get the PROBING environment variable
probe_value = os.environ.get("PROBING", "0")
current_script = get_current_script_name()

def check_probe_value():
    if probe_value.lower() in ["1", "followed"]:
        return False
    elif probe_value.lower() in ["2", "nested"]:
        return True
    return False


def init_probing():
    try:
        if probe_value.lower() in ["1", "followed"] or probe_value.lower() in ["2", "nested"]:
            import probing

    except ImportError as e:
        print(f"Error loading probing library: {e}", file=sys.stderr)
    except Exception as e:
        print(f"Unexpected error in probing hook: {e}", file=sys.stderr)
        # In case of unexpected errors, don't enable probing


try:
    import re

    if re.fullmatch("torchrun", current_script) is None: # Exclude torchrun itself
        is_resurion = check_probe_value()
        if launched_by_torchrun(is_resurion):
            init_probing()
except Exception as e:
    print(f"Error in probing hook: {e}", file=sys.stderr)
    # In case of unexpected errors, don't enable probing
