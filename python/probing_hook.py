"""
Probing Hook - Conditionally activates the probing library based on environment variables.

This module intercepts process startup and conditionally imports the probing library
based on the PROBE environment variable:
- '0': Disabled (default)
- '1' or 'followed': Enable only in current process
- '2' or 'nested': Enable in current and all child processes
- 'regex:PATTERN': Enable if current script name matches the regex pattern
- 'SCRIPTNAME': Enable if current script name matches exactly
"""

import os
import sys


def get_current_script_name():
    """Get the name of the current running script."""
    try:
        script_path = sys.argv[0]
        return os.path.basename(script_path)
    except (IndexError, AttributeError):
        return None


# Get the PROBE environment variable
probe_value = os.environ.get("PROBE", "0")
current_script = get_current_script_name()

try:
    # Remove the variable by default - we'll set it back if needed
    if "PROBE" in os.environ:
        del os.environ["PROBE"]

    if probe_value.lower() in ["1", "followed"]:
        print(
            f"Activating probing in 'followed' mode (current process only)",
            file=sys.stderr,
        )
        import probing

        # Environment variable is intentionally not preserved

    elif probe_value.lower() in ["2", "nested"]:
        print(
            f"Activating probing in 'nested' mode (all child processes)",
            file=sys.stderr,
        )
        import probing

        # Preserve for child processes
        os.environ["PROBE"] = probe_value

    elif probe_value.lower().startswith("regex:"):
        pattern = probe_value.split(":", 1)[1]
        try:
            import re

            if re.search(pattern, current_script):
                print(
                    f"Activating probing for script matching '{pattern}'",
                    file=sys.stderr,
                )
                import probing
            # Always preserve valid regex patterns for child processes
            os.environ["PROBE"] = probe_value
        except Exception as e:
            print(f"Error in regex pattern '{pattern}': {e}", file=sys.stderr)
            # Don't preserve invalid regex patterns

    elif probe_value != "0":
        # Script name comparison
        if probe_value == current_script:
            print(
                f"Activating probing for '{current_script}' (current process only)",
                file=sys.stderr,
            )
            import probing
        # Always preserve the script name filter for child processes
        os.environ["PROBE"] = probe_value

except ImportError as e:
    print(f"Error loading probing library: {e}", file=sys.stderr)
except Exception as e:
    print(f"Unexpected error in probing hook: {e}", file=sys.stderr)
    # In case of unexpected errors, don't enable probing
