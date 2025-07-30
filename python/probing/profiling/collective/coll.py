import torch
import torch.distributed as dist
import time
from functools import wraps
from typing import List, Optional, Union, Tuple

function_names = [
    'all_reduce',
    'all_gather',
    'reduce_scatter',
    'broadcast',
    'reduce_scatter_base',
    'all_gather_base',
    'reduce_scatter_tensor',
    'all_gather_into_tensor',
]

# 'batch_isend_irecv'

# Store the group ranks for each function in a dictionary
GROUP_RANKS_CACHE = {}

"""
This function returns a list of participating ranks within a given process group."""
def get_participating_ranks(group: Optional[dist.ProcessGroup] = None) ->  Tuple[int, int, List[int]]:
    if not dist.is_initialized():
        return 0, 0, []

    group_rank = dist.get_rank(group=group)
    group_size = dist.get_world_size(group=group)

    if group is None or group == dist.group.WORLD:
        return group_rank, group_size, list(range(dist.get_world_size()))
    
    group_id = id(group)
    
    if group_id in GROUP_RANKS_CACHE:
        return group_rank, group_size, GROUP_RANKS_CACHE[group_id]

    
    # Method 1: Use all_gather_object to collect all ranks
    try:
        ranks_list = [None] * group_size
        global_rank = dist.get_rank()
        dist.all_gather_object(ranks_list, global_rank, group=group)
        ranks = [int(r) for r in ranks_list]
        GROUP_RANKS_CACHE[group_id] = ranks
        return group_rank, group_size, ranks
    
    except Exception as e:
        print(f"[Rank {dist.get_rank()}] all_gather_object failed: {e}. Using fallback method.")
    
    # Method 2: Use TCPStore to collect all ranks
    try:
        rank = dist.get_rank()
        world_size = dist.get_world_size()
        
        import os
        store = dist.TCPStore(
            host_name=os.environ['MASTER_ADDR'],
            port=int(os.environ['MASTER_PORT']),
            world_size=world_size,
            is_master=(rank == 0),
            timeout=torch.timedelta(seconds=30)
        )
        
        store_key = f'rank_in_group_{group_id}'
        store.set(store_key, str(rank))
        
        # If rank is 0, collect all ranks from the store
        if rank == 0:
            ranks = []
            for i in range(group_size):
                r = int(store.get(store_key).decode())
                ranks.append(r)
            ranks_tensor = torch.tensor(ranks, dtype=torch.int32)
        else:
            ranks_tensor = torch.zeros(group_size, dtype=torch.int32)
        
        # Broadcast the ranks_tensor to all ranks in the group
        dist.broadcast(ranks_tensor, src=0, group=group)
        ranks = ranks_tensor.tolist()
        
        # Clean up the store
        if rank == 0:
            store.delete_key(store_key)
        
        GROUP_RANKS_CACHE[group_id] = ranks
        return group_rank, group_size, ranks
    
    except Exception as e:
        print(f"[Rank {rank}] Failed to get ranks via TCPStore: {e}")
        # If all methods fail, return a list of all ranks in the group
        return group_rank, group_size, [dist.get_rank() for _ in range(group_size)]

class CollectiveTracer:
    """
    Trace collective operations for distributed training.
    """ 
    def __init__(self, trace_file=None, verbose=True):
        """
        Args:
            trace_file: Log file to store trace data, if None, no file will be created
            verbose: Whether to print messages or not
        """
        self.trace_file = trace_file
        self.verbose = verbose
        self.trace_data = []
        self.original_functions = {}
        self.hooked_functions = {}
        self.has_cuda = torch.cuda.is_available()
        for func_name in function_names:
            if hasattr(dist, func_name):
                self.hooked_functions[func_name] = getattr(dist, func_name)
            else:
                print(f"!!! torch.distributed 中未找到函数 {func_name}，已跳过")

        if not self.hooked_functions:
            print("!!! WARNING !!! 没有找到任何要追踪的函数")

        self.call_counts = {fn: 0 for fn in self.hooked_functions}
        self.my_rank = 0  # partly rank in group
        self.my_size = 1
        self.participate_ranks = []

        self.global_rank = 0
        
    def _log(self, message):
        """Log a message to console and/or file."""
        if self.verbose:
            print(message)
        if self.trace_file:
            ranked_filename = f"{self.trace_file}-{self.global_rank}"
            with open(ranked_filename, 'a') as f:
                f.write(message + '\n')
    
    def create_trace_entry(self, func_name, start_time, duration, tensor_info):
        """Create a trace entry."""
        return {
            'function': func_name,
            'timestamp': start_time,
            'duration': duration,
            'tensor_shape': tensor_info['shape'],
            'tensor_dtype': str(tensor_info['dtype']),
            'tensor_size': tensor_info['size']
        }
    
    def _trace_wrapper(self, func_name, orig_func):
        """Create a wrapper for the original function to trace its execution."""
        class TimedWork:
            def __init__(self, work, start_time, func_name, data_size, tensor_info=None, Tracer=None):
                self.work = work
                self.start_time = start_time
                self.func_name = func_name
                self.data_size = data_size
                self.tensor_info = tensor_info if tensor_info else {'shape': 'unknown', 'dtype': 'unknown', 'size': 0}
                self.tracer = Tracer
                
            def wait(self):
                result = self.work.wait()

                if self.tracer.has_cuda:
                    _cuda_sync()

                end_time = time.perf_counter()
                duration = end_time - self.start_time
                
                # Create a trace entry
                trace_entry = self.tracer.create_trace_entry(func_name, self.start_time, duration, self.tensor_info)
                self.tracer.trace_data.append(trace_entry)
                
                # Print trace information
                self.tracer._log(f"[TRACE] I am {self.tracer.my_rank} && in GROUP_{self.tracer.participate_ranks} - {func_name} - Shape: {self.tensor_info['shape']}, "
                        f"Dtype: {self.tensor_info['dtype']}, Size: {self.tensor_info['size']/1024/1024:.2f} MB, "
                        f"Duration: {duration*1e3:.3f} ms, "
                        f"size of coll is {self.tracer.my_size}  where the global rank is {self.tracer.global_rank}")
  
                return result
            
            def is_completed(self):
                return self.work.is_completed()
            
        @wraps(orig_func)
        def wrapper(*args, **kwargs):
            # ------------ Collective Counts +1 ------------
            self.call_counts[func_name] += 1
            # --------------------------------

            tensor_info = self._extract_tensor_info(args, kwargs)

            if self.has_cuda:
                _cuda_sync()
            start_time = time.perf_counter()
            tensor = args[0] if args else None
            print(f"tensor.numel={tensor.numel()}   tensor.element_size={tensor.element_size()}\n")
            data_size = tensor.numel() * tensor.element_size() if tensor is not None else 0

            group = kwargs.get('group') or (args[2] if len(args) > 2 else None)
            self.my_rank, self.my_size, self.participate_ranks = get_participating_ranks(group)

            self.global_rank = dist.get_rank()
            
            is_async = kwargs.get('async_op', False)
            if is_async:
                work = orig_func(*args, **kwargs)

                return TimedWork(work, start_time, func_name, data_size, tensor_info, self)
            else:
                work = orig_func(*args, **kwargs)
                
                if self.has_cuda:
                    _cuda_sync()

                end_time = time.perf_counter()
                duration = end_time - start_time
                
                trace_entry = self.create_trace_entry(func_name, start_time, duration, tensor_info)
                self.trace_data.append(trace_entry)
                
                # Print trace information
                self._log(f"[TRACE] I am {self.my_rank} && in GROUP_{self.participate_ranks} - {func_name} - Shape: {tensor_info['shape']}, "
                        f"Dtype: {tensor_info['dtype']}, Size: {tensor_info['size']/1024/1024:.2f} MB, "
                        f"Duration: {duration*1e3:.3f} ms, "
                        f"size of coll is {self.my_size}  where the global rank is {self.global_rank}")
                return work
        
        return wrapper
    
    def _extract_tensor_info(self, args, kwargs):
        """sub function to extract tensor information from arguments."""
        tensor = None
        
        # Try to find a tensor in positional arguments
        for arg in args:
            if isinstance(arg, torch.Tensor):
                tensor = arg
                break
                
        # If not found, try to find a tensor in keyword arguments
        if tensor is None:
            for key, value in kwargs.items():
                if isinstance(value, torch.Tensor):
                    tensor = value
                    break
        
        # If still not found, check if the first argument is an object with a tensor attribute
        if tensor is None and args:
            first_arg = args[0]
            for attr in dir(first_arg):
                try:
                    value = getattr(first_arg, attr)
                    if isinstance(value, torch.Tensor):
                        tensor = value
                        break
                except:
                    continue
        
        if tensor is None:
            return {'shape': 'unknown', 'dtype': 'unknown', 'size': 0}
            
        return {
            'shape': tuple(tensor.shape),
            'dtype': tensor.dtype,
            'size': tensor.element_size() * tensor.numel()
        }
     
    
    def apply_hooks(self):
        for func_name, orig_func in self.hooked_functions.items():
            if hasattr(dist, func_name):
                self.original_functions[func_name] = getattr(dist, func_name)
                setattr(dist, func_name, self._trace_wrapper(func_name, orig_func))
                self._log(f"Applyed hook to function: {func_name}")
    
    def remove_hooks(self):
        for func_name, orig_func in self.original_functions.items():
            if hasattr(dist, func_name):
                setattr(dist, func_name, orig_func)
                self._log(f"Removed hook from function: {func_name}")
    
    def get_trace_data(self):
        return self.trace_data
    
    def get_all_call_counts(self):
        return self.call_counts.copy()
    
    def export_to_csv(self, filename):
        import csv
        if not self.trace_data:
            self._log("No trace data to export.")
            return
            
        with open(filename, 'w', newline='') as csvfile:
            fieldnames = self.trace_data[0].keys()
            writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
            writer.writeheader()
            for row in self.trace_data:
                writer.writerow(row)
                
        self._log(f"Exported trace data to {filename}")

def _cuda_sync():
    torch.cuda.synchronize()