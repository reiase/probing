from IPython.core.magic import Magics, magics_class, line_magic
import __main__
from typing import Any, Dict

@magics_class
class DebugMagic(Magics):

    @line_magic
    def remote_debug(self, line: str):
        """Enable remote debugging.

        Usage:
            %remote_debug host=127.0.0.1 port=9999 try_install=True
        """
        args = dict(item.split("=") for item in line.split()) if line else {}
        host = args.get("host", "127.0.0.1")
        port = int(args.get("port", 9999))
        try_install = args.get("try_install", "True").lower() in ("true", "1", "t")

        if not self.detect_debugger() and try_install:
            self.install_debugger()
        if self.detect_debugger():
            self.enable_debugger(host, port)

    @staticmethod
    def status() -> Dict[str, Any]:
        if not hasattr(__main__, "__probing__"):
            __main__.__probing__ = {}
        if "debug" not in __main__.__probing__:
            __main__.__probing__["debug"] = {}
        __main__.__probing__["debug"][
            "debugger_installed"
        ] = DebugMagic.detect_debugger()

        return __main__.__probing__["debug"]

    @staticmethod
    def detect_debugger():
        try:
            import debugpy
            return True
        except ImportError:
            return False

    @staticmethod
    def install_debugger():
        try:
            from pip import main as pipmain
        except ImportError:
            from pip._internal import main as pipmain
        pipmain(["install", "debugpy"])

    @staticmethod
    def enable_debugger(host: str = "127.0.0.1", port: int = 9999):
        status = DebugMagic.status()
        try:
            import debugpy
        except Exception:
            print("debugpy is not installed, please install debugpy with pip:")
            print("\tpip install debugpy")
            return
        debugpy.listen((host, port))
        status["debugger_address"] = f"{host}:{port}"
        print(f"debugger is started at {host}:{port}")
