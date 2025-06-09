from dataclasses import dataclass
from probing.core.table import table

@table
@dataclass
class IterOutputTrace:
    iteration: int
    time_per_iter: float
    total_iter: int
    throughput: float


def init():
    from megatron.training import training
    from megatron.training.training import num_floating_point_operations
    from megatron.training.global_vars import get_args, get_timers
    from megatron.core.num_microbatches_calculator import get_num_microbatches 
    # 保存原始的 training_log 函数
    _original_training_log = training.training_log

    def custom_training_log(loss_dict, total_loss_dict, learning_rate, decoupled_learning_rate, iteration,
                        loss_scale, report_memory_flag, skipped_iter,
                        grad_norm, params_norm, num_zeros_in_grad):
        # from megatron.training.global_vars import _GLOBAL_NUM_MICROBATCHES_CALCULATOR      
        # print(f"iteration: {iteration}", flush=True)
        # 获取必要的参数
        args = get_args()
        timers = get_timers()
        advanced_iters_key = 'advanced iterations'
        skipped_iters_key = 'skipped iterations'
        nan_iters_key = 'nan iterations'
        
        # 计算当前迭代后的值（模拟原函数的逻辑）
        current_advanced_iters = total_loss_dict.get(advanced_iters_key, 0)
        current_skipped_iters = total_loss_dict.get(skipped_iters_key, 0)
        
        # 更新计数（模拟原函数会做的事情）
        if not skipped_iter:
            current_advanced_iters += 1
        current_skipped_iters += skipped_iter
        if iteration % args.log_interval == 0:
            # 计算总迭代数
            total_iterations = current_advanced_iters + current_skipped_iters
            # print(f"total_iterations: {total_iterations}", flush=True)
            if total_iterations > 0:
                # batch_size = args.micro_batch_size * args.data_parallel_size * _GLOBAL_NUM_MICROBATCHES_CALCULATOR.get()   
                batch_size = args.micro_batch_size * args.data_parallel_size * get_num_microbatches()
                elapsed_time = timers('interval-time').elapsed(reset=False, barrier=True)
                elapsed_time_per_iteration = elapsed_time / total_iterations

                # 计算 throughput (TFLOP/s/GPU)
                throughput = num_floating_point_operations(args, batch_size) / (
                    elapsed_time_per_iteration * 10**12 * args.world_size)
                # print(f"throughput: {throughput}", flush=True)
                # 创建并保存 IterOutputTrace
                IterOutputTrace(
                    iteration=iteration,
                    time_per_iter=elapsed_time_per_iteration,
                    total_iter=args.train_iters,
                    throughput=throughput
                ).save()
           
        
        result = _original_training_log(loss_dict, total_loss_dict, learning_rate, decoupled_learning_rate, 
                                    iteration, loss_scale, report_memory_flag, skipped_iter,
                                    grad_norm, params_norm, num_zeros_in_grad)
        
        return result

    training.training_log = custom_training_log
    print("==========================IterOutputTrace init done!============================", flush=True)
