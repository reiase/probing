import code
import io
from contextlib import redirect_stderr, redirect_stdout
from types import CodeType
from typing import Any, Dict, List, Type

class DebugConsole(code.InteractiveConsole):
    def resetoutput(self):
        out = self.output
        self.output = ""
        return out

    def write(self, data: str) -> None:
        self.output += data

    def runsource(
        self, source: str, filename: str = "<input>", symbol: str = "single"
    ) -> bool:
        try:
            code = self.compile(source, filename, symbol)
        except (OverflowError, SyntaxError, ValueError):
            # Case 1: wrong code
            self.showsyntaxerror(filename)
            self.resetbuffer()
            return self.resetoutput()

        if code is None:
            # Case 2: incomplete code
            return

        ret = self.runcode(code)
        self.resetbuffer()
        return ret

    def runcode(self, code: CodeType) -> None:
        with redirect_stderr(io.StringIO()) as err:
            with redirect_stdout(io.StringIO()) as out:
                try:
                    exec(code, self.locals, None)
                except SystemExit:
                    raise
                except Exception:
                    ret = err.getvalue() + out.getvalue()
                    self.showtraceback()
                    return ret + self.resetoutput()
        ret = out.getvalue()
        if len(ret) == 0:
            return None
        return ret

    def push(self, line: str) -> bool:
        if not hasattr(self, "output"):
            self.output = ""
        self.buffer.append(line)
        source = "\n".join(self.buffer)
        return self.runsource(source, self.filename)

try:
    from .magics import DebugConsole
except:
    print("DebugConsole not found, using default DebugConsole")
    
debug_console = DebugConsole()
