__ALL__ = ["trace_all_collectives"]

from .coll import CollectiveTracer

def trace_all_collectives(trace_file=None, verbose=True):
    """Fast API to trace all collective operations in PyTorch."""
    tracer = CollectiveTracer(trace_file=trace_file, verbose=verbose)
    tracer.apply_hooks()
    return tracer