from IPython.core.magic import Magics, magics_class, line_magic
import gc
import __main__
import torch

@magics_class
class TorchMagic(Magics):

    def __init__(self, shell):
        super().__init__(shell)
        if not hasattr(__main__, "__probing__"):
            __main__.__probing__ = {}
        if "profiler" not in __main__.__probing__:
            __main__.__probing__["profiler"] = {}

    @line_magic
    def tprofile(self, line: str):
        """Profile PyTorch modules.

        Usage:
            %tprofile steps=1 mid=None

        """
        args = dict(item.split("=") for item in line.split()) if line else {}
        steps = int(args.get("steps", 1))
        mid = args.get("mid", None)
        if mid is not None:
            mid = int(mid)

        print(f"Profiling for {steps} steps")
        self.profile(steps, mid)

    @line_magic
    def tsummary(self, line: str):
        """Show profiler summary."""
        if "profiler" in __main__.__probing__:
            for k, v in __main__.__probing__["profiler"].items():
                v.summary()

    @staticmethod
    def get_top_level_modules() -> list:
        objs = gc.get_objects()
        objs = [obj for obj in objs if isinstance(obj, torch.nn.Module)]
        children = set()

        def walk(obj):
            if hasattr(obj, "children"):
                for child in obj.children():
                    children.add(id(child))
                    walk(child)

        for obj in objs:
            walk(obj)
        return [obj for obj in objs if id(obj) not in children]

    @staticmethod
    def install_profiler(module, steps=1):
        class _profiler:
            def __init__(self, steps) -> None:
                self._steps = steps
                self._profiler = None
                self._count = 0
                self._hooks = []
                self._module = None
                self._status = False

            def install(self, module):
                self._module = module
                self._profiler = torch.profiler.profile()
                print(f"installing profiler to module {module}")
                self._hooks.append(module.register_forward_pre_hook(self.module_hook))
                return self

            def module_hook(self, *args, **kwargs):
                if self._status is False and self._count < self._steps:
                    print("==== start profiling ====")
                    self._profiler.start()
                    self._status = True
                    self._count += 1
                    return
                if self._status is True and self._count >= self._steps:
                    print("==== stop profiling ====")
                    self._profiler.step()
                    self._profiler.stop()
                    self._status = False

                self._count += 1
                self._profiler.step()

            def summary(self):
                if self._profiler and self._profiler.events():
                    print(self._profiler.key_averages().table(sort_by="cpu_time_total", row_limit=10))
                else:
                    print("profiler is not started or has no events")

        return _profiler(steps).install(module)

    @staticmethod
    def profile(steps=1, mid=None):
        if mid is not None:
            tms = [m for m in gc.get_objects() if id(m) == mid]
        else:
            tms = TorchMagic.get_top_level_modules()
        for m in tms:
            p = TorchMagic.install_profiler(m, steps)
            __main__.__probing__["profiler"][id(m)] = p
