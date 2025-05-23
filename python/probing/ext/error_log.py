from dataclasses import dataclass
from probing.core.table import table

@table
@dataclass
class ErrorLog:
    type: str
    value: str
    traceback: str

def init():
    import sys
    import traceback

    def global_exception_hook(exc_type, exc_value, exc_traceback):
        tb_str = ''.join(traceback.format_exception(exc_type, exc_value, exc_traceback))
        ErrorLog(
            type=exc_type.__name__,
            value=str(exc_value),
            traceback=tb_str
        ).save()

        print(f"捕获到异常: {exc_type.__name__}, 值: {exc_value}, 堆栈跟踪:\n{tb_str}")
       

    sys.excepthook = global_exception_hook
