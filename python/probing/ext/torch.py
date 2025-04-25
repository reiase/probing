hooks = {}


def optimizer_step_post_hook(optimizer, *args, **kwargs):
    global hooks
    if optimizer not in hooks:
        from probing.torch.tracer import install_hooks
        from probing.torch.tracer import get_toplevel_module
        from probing.torch.tracer import MemTracer

        tracer = MemTracer()

        models = get_toplevel_module()
        for model in models:
            install_hooks(model, tracer=tracer)
        install_hooks(opt=optimizer, tracer=tracer)
        hooks[optimizer] = True

        from probing.torch.step import next_step

        next_step()


def init():
    from torch.optim.optimizer import register_optimizer_step_post_hook

    register_optimizer_step_post_hook(optimizer_step_post_hook)


def deinit():
    from probing.torch.tracer import uninstall_hooks

    uninstall_hooks()
