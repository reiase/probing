hooks = {}


def is_true(value):
    if value in ["TRUE", "True", "true", "1", "YES", "Yes", "yes", "ON", "On", "on"]:
        return True
    return False


def optimizer_step_post_hook(optimizer, *args, **kwargs):
    global hooks
    if optimizer not in hooks:
        from probing.profiling.torch_probe import TorchProbe
        from probing.profiling.torch import install_hooks
        from probing.profiling.torch.module_utils import get_toplevel_module

        import os

        mode = os.getenv("PROBE_TORCH_MODE", "ordered")
        rate = float(os.getenv("PROBE_TORCH_RATE", "0.05"))
        tracepy = is_true(os.getenv("PROBE_TORCH_TRACEPY", "False"))
        sync = is_true(os.getenv("PROBE_TORCH_SYNC", "False"))
        exprs = os.getenv("PROBE_TORCH_EXPRS", "")

        tracer = TorchProbe(exprs=exprs)

        models = get_toplevel_module()
        for model in models:
            install_hooks(model, tracer=tracer)
        install_hooks(opt=optimizer, tracer=tracer)
        hooks[optimizer] = True

        from probing.profiling.torch import next_step

        next_step()


def init():
    from torch.optim.optimizer import register_optimizer_step_post_hook

    register_optimizer_step_post_hook(optimizer_step_post_hook)


def deinit():
    from probing.profiling.torch import uninstall_hooks

    uninstall_hooks()
