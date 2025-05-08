import time
from dataclasses import dataclass
from probing.core.table import table

hooks = {}

@table
@dataclass
class IterOutputTrace:
    iteration: int
    time_per_iter: float
    total_iter: int
    throughput: float

# loss_dict, total_loss_dict, learning_rate, decoupled_learning_rate, iteration, loss_scale, report_memory_flag, skipped_iter, grad_norm, params_norm, num_zeros_in_grad

class IterOutputTracer:

    def __init__(self):
        self.start_time = time.time()
        self.total_iter = 0
        self.time_per_iter = 0
        self.throughput = 0

    def step_post_hook(self, optimizer, args, kwargs):
        print("step_post_hook triggered!", flush=True)
        import sys
        f = sys._getframe()
        while f and f.f_code.co_name != 'train':
            f = f.f_back
        if not f:
            f = sys._getframe().f_back 
            print("target frame not found", flush=True) 

        # 从train()中提取局部变量
        print(f"Found frame: {f.f_code.co_name}", flush=True)
        local_vars = f.f_locals
     
        total_loss_dict = local_vars.get('total_loss_dict')
        iteration = local_vars.get('iteration')
        advanced_iters_key = 'advanced iterations'
        skipped_iters_key = 'skipped iterations'
        total_iterations = total_loss_dict[advanced_iters_key] + \
                       total_loss_dict[skipped_iters_key]
        
        print(f"total_loss_dict: {total_loss_dict}", flush=True)
        print(f"iteration: {iteration}", flush=True)
        print(f"advanced_iters_key: {advanced_iters_key}", flush=True)
        print(f"skipped_iters_key: {skipped_iters_key}", flush=True)
        print(f"total_iterations: {total_iterations}", flush=True)

        # 全局变量
        # global _GLOBAL_ARGS, _GLOBAL_NUM_MICROBATCHES_CALCULATOR, _GLOBAL_TIMERS
    
        from megatron.training.global_vars import get_args, get_timers, _GLOBAL_NUM_MICROBATCHES_CALCULATOR
        args = get_args()
        print(f"args: {args}", flush=True)
        num_microbatches = _GLOBAL_NUM_MICROBATCHES_CALCULATOR.get()
        print(f"num_microbatches: {num_microbatches}", flush=True)
        timers = get_timers()

        batch_size = args.micro_batch_size * args.data_parallel_size * \
        num_microbatches

        elapsed_time = timers('interval-time').elapsed(barrier=True)
        elapsed_time_per_iteration = elapsed_time / total_iterations

        throughput = num_floating_point_operations(args, batch_size) / (
            elapsed_time_per_iteration * 10**12 * args.world_size)
        
        IterOutputTrace(
        iteration=iteration,
        time_per_iter=elapsed_time_per_iteration,
        total_iter=args.train_iters,
        throughput=throughput
        ).save()
        


def optimizer_step_post_hook(optimizer, *args, **kwargs):
    global hooks
    print(f"optimizer_step_post_hook called with {optimizer}", flush=True)
    if optimizer not in hooks:
        tracer = IterOutputTracer()
        optimizer.register_step_post_hook(tracer.step_post_hook)
        hooks[optimizer] = True



def init():
    print("iteroutput_hook.init() called!", flush=True)
    from torch.optim.optimizer import register_optimizer_step_post_hook

    register_optimizer_step_post_hook(optimizer_step_post_hook)

    print("register_optimizer_step_post_hook done!", flush=True)


def deinit():
    from probing.torch.tracer import uninstall_hooks

    uninstall_hooks()

def num_floating_point_operations(args, batch_size):
    def calculate_layer_counts():
        """Calculate the number of attention, Mamba, and MLP layers."""
        if args.hybrid_override_pattern:
            counts = {'M': 0, '*': 0, '-': 0}
            for layer_type in args.hybrid_override_pattern:
                if layer_type in counts:
                    counts[layer_type] += 1
            return counts['*'], counts['M'], counts['-']
        else:
            num_attn_layers = round(args.num_layers * args.hybrid_attention_ratio)
            num_mlp_layers = round(args.num_layers * args.hybrid_mlp_ratio)
            num_mamba_layers = args.num_layers - num_attn_layers - num_mlp_layers
            return num_attn_layers, num_mamba_layers, num_mlp_layers

    def mlp_layer_flops(batch_size, seq_len, hidden_size, expansion=4.0, swiglu=False):
        """Calculate FLOPs for an MLP layer."""
        scale_factor = 3.0 / 2.0 if swiglu else 1.0
        return 4 * expansion * scale_factor * batch_size * seq_len * hidden_size ** 2

    def attn_layer_flops(batch_size, seq_len, hidden_size, num_heads, gqa=True,
                         gqa_groups=8, kv_channels=None):
        """Calculate FLOPs for an attention layer."""
        p = (kv_channels * num_heads / hidden_size) if kv_channels else 1
        g = gqa_groups if gqa else num_heads
        return 4 * batch_size * seq_len * hidden_size * p * (
                hidden_size + (hidden_size * (g / num_heads)) + (seq_len / 2 ))

    def mamba_layer_flops(batch_size, seq_len, hidden_size, state_dim=16,
                          head_dim=64, num_groups=1, num_heads=128):
        """Calculate FLOPs for a Mamba layer."""
        # Note (rwaleffe): flops estimate for scan should be updated based on new SSD kernels,
        # but small percent of overall layer flops
        d_in = 2 * hidden_size
        if num_heads:
            nheads = num_heads
        else:
            nheads = d_in // head_dim
        return (
                (2 * batch_size * seq_len * hidden_size * (
                        2 * d_in + 2 * num_groups * state_dim + nheads)) +  # in_proj
                (7 * batch_size * seq_len * d_in * state_dim) +  # scan
                (2 * batch_size * seq_len * d_in * hidden_size)  # out_proj
        )

    def hybrid_flops(batch_size, seq_len, hidden_size,
                     num_attn_layers, num_mamba_layers, num_mlp_layers,
                     mamba_state_dim=128, mamba_head_dim=64,
                     mamba_num_groups=8, mamba_num_heads=128,
                     num_attn_heads=32,gqa=True, 
                     gqa_groups=8, kv_channels=None,
                     mlp_expansion=4.0, swiglu=False,
                     vocab_size=256000):
        """Calculate total FLOPs for the hybrid model."""
        flops_fwd = (
                num_attn_layers * attn_layer_flops(batch_size, seq_len, hidden_size,
                                                   num_attn_heads, gqa, gqa_groups, kv_channels) +
                num_mlp_layers * mlp_layer_flops(batch_size, seq_len, hidden_size,
                                                 mlp_expansion, swiglu) +
                num_mamba_layers * mamba_layer_flops(batch_size, seq_len, hidden_size,
                                                     mamba_state_dim, mamba_head_dim,
                                                     mamba_num_groups, mamba_num_heads) +
                (2 * batch_size * seq_len * hidden_size * vocab_size)  # logits computation
        )
        return flops_fwd * 3

    def transformer_flops():
        """Calculate FLOPs for a standard Transformer model."""
        # TODO(helenn/dnarayanan): Refactor this to reuse the helper methods.
        # Attention projection size.
        query_projection_size = args.kv_channels * args.num_attention_heads
        query_projection_to_hidden_size_ratio = query_projection_size / args.hidden_size
        # Group Query Attention.
        if not args.group_query_attention:
            args.num_query_groups = args.num_attention_heads
        # MoE.
        if args.num_experts is None:
            # Every Transformer MLP is dense.
            num_dense_layers = args.num_layers
            num_moe_layers = 0
            num_experts_routed_to = 0
            last_layer_is_moe = 0
        else:
            # Calculate number of dense and MoE Transformer MLPs.
            if isinstance(args.moe_layer_freq, int):
                moe_layer_pattern = [
                    1 if (i % args.moe_layer_freq == 0) else 0 for i in range(args.num_layers)
                ]
            elif isinstance(args.moe_layer_freq, list):
                moe_layer_pattern = args.moe_layer_freq
            else:
                raise RuntimeError("Illegal --moe-layer-freq argument provided!")
            assert len(moe_layer_pattern) == args.num_layers, (
                f"Invalid length of moe_layer_pattern: {len(moe_layer_pattern)}, "
                f"expected {args.num_layers}, "
                f"current moe layer pattern: {args.moe_layer_freq}"
            )
            num_moe_layers = sum(moe_layer_pattern)  # Number of 1s in `moe_layer_pattern`.
            num_dense_layers = args.num_layers - num_moe_layers
            num_experts_routed_to = args.moe_router_topk
            last_layer_is_moe = moe_layer_pattern[-1]
        
        if args.mtp_num_layers is not None:
            mtp_num_layers = args.mtp_num_layers
            num_moe_layers += last_layer_is_moe * mtp_num_layers
            num_dense_layers += (1 - last_layer_is_moe) * mtp_num_layers
            num_layers = args.num_layers + mtp_num_layers
        else:
            mtp_num_layers = 0
            num_layers = args.num_layers

        moe_ffn_hidden_size = args.moe_ffn_hidden_size if args.moe_ffn_hidden_size is not None else args.ffn_hidden_size
        shared_expert_ffn_hidden_size = (
            0
            if args.moe_shared_expert_intermediate_size is None
            else args.moe_shared_expert_intermediate_size
        )
        # SwiGLU.
        gated_linear_multiplier = 3 / 2 if args.swiglu else 1

        # The 12x term below comes from the following factors; for more details, see
        # "APPENDIX: FLOATING-POINT OPERATIONS" in https://arxiv.org/abs/2104.04473.
        # - 3x: Each GEMM in the model needs to be performed 3 times (forward pass,
        #       backward wgrad [weight gradient], backward dgrad [data gradient]).
        # - 2x: GEMMs of a particular size are stacked twice in the standard Transformer model
        #       architectures implemented in this codebase (e.g., h->ffn_h GEMM and ffn_h->h GEMM
        #       in MLP layer).
        # - 2x: A GEMM of a m*n tensor with a n*k tensor requires 2mnk floating-point operations.
        expansion_factor = 3 * 2 * 2

        if args.multi_latent_attention:
            assert not args.group_query_attention
            '''
            Basic arithmetic
            let B is batch size, s is seq_len, h is embedding dim,
            for one self_attnetion block (prenorm is not included)
            qkv projection:  6Bsh^2
            attn:            2Bs^2h
            attn over value: 2Bs^2h
            oproj:           2Bsh^2

            references
            https://arxiv.org/abs/2305.10403
            https://arxiv.org/abs/2205.05198
            '''
            ## MLA
            if args.q_lora_rank is None:
                q_term = args.hidden_size * args.num_attention_heads * (args.qk_head_dim + args.qk_pos_emb_head_dim)
            else:
                q_term = args.q_lora_rank * (args.hidden_size + args.num_attention_heads * (args.qk_head_dim + args.qk_pos_emb_head_dim) + 1) 
            self_attn_term = (
                3*2 # fwd(1) + bwd(2) *FMA
                * num_layers 
                * (
                    ## q lora + rope + q norm
                    q_term

                    ## kv lora + rope + kv norm
                    + args.kv_lora_rank
                    * (args.hidden_size + args.num_attention_heads * (args.qk_head_dim + args.v_head_dim) + 1)
                    + args.hidden_size * args.qk_pos_emb_head_dim

                    ## o proj
                    + (args.num_attention_heads * args.v_head_dim) * args.hidden_size

                    ## core attn
                    + args.seq_length * (args.num_attention_heads * (args.qk_head_dim + args.qk_pos_emb_head_dim)) / 2
                    + args.seq_length * args.num_attention_heads * args.v_head_dim / 2
                )
            )

        else:
            ## MHA or GQA
            self_attn_term = (
                expansion_factor
                * num_layers
                * args.hidden_size
                * args.hidden_size
                * (
                    (
                        1
                        + (args.num_query_groups / args.num_attention_heads)
                        # # Only half of the attention matrix is non-zero and needs to be multiplied with V.
                        + (args.seq_length / args.hidden_size / 2)
                    ) * query_projection_to_hidden_size_ratio
                )
            )

        total_floating_point_operations = batch_size * args.seq_length * (
            # MLP
            expansion_factor
            * num_layers
            * args.hidden_size
            * (
                # dense layer (deepseek v2, v3 style)
                (
                    args.ffn_hidden_size
                    * gated_linear_multiplier
                ) * (num_dense_layers/num_layers)
                # routed experts
                + (
                    moe_ffn_hidden_size
                    * num_experts_routed_to
                    * gated_linear_multiplier
                ) * (num_moe_layers/num_layers)
                # Shared Experts.
                + (
                    shared_expert_ffn_hidden_size 
                    * gated_linear_multiplier
                ) * (num_moe_layers/num_layers)
            )
            # Self Attention
            + self_attn_term
            # MTP norms and proj
            + 3*2
            * mtp_num_layers
            * (
                # MTP eh norm + final nrom
                3 * args.hidden_size
                # MTH eh proj
                + 2 * args.hidden_size * args.hidden_size
            )
            # Logit.
            + 3*2
            * args.hidden_size
            * args.padded_vocab_size 
            * (mtp_num_layers + 1)
        )
        return total_floating_point_operations


    # Main entrypoint for FLOPs calculation.
    if args.is_hybrid_model:
        # Calculate the number of each type of layer.
        num_attn_layers, num_mamba_layers, num_mlp_layers = calculate_layer_counts()

        # Compute hybrid model FLOPs.
        return hybrid_flops(
            batch_size=batch_size,
            seq_len=args.seq_length,
            hidden_size=args.hidden_size,
            num_attn_layers=num_attn_layers,
            num_mamba_layers=num_mamba_layers,
            num_mlp_layers=num_mlp_layers,
            mamba_state_dim=args.mamba_state_dim,
            mamba_head_dim=args.mamba_head_dim,
            mamba_num_groups=args.mamba_num_groups,
            mamba_num_heads=args.mamba_num_heads,
            num_attn_heads=args.num_attention_heads,
            gqa=args.group_query_attention,
            gqa_groups=args.num_query_groups,
            kv_channels=args.kv_channels,
            mlp_expansion=args.ffn_hidden_size / args.hidden_size,
            swiglu=args.swiglu,
            vocab_size=args.padded_vocab_size
        )
    else:
        # Compute standard Transformer model FLOPs.
        return transformer_flops()
    
