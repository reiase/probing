import time
import random
import math
import sys
from typing import List, Dict, Any
import json

import probing

class ProfilerBenchmark:
    """综合性能测试，包含各种函数调用模式"""
    
    def __init__(self):
        self.data = list(range(1000))
        self.cache = {}
        self.results = []
        
    def recursive_fibonacci(self, n: int) -> int:
        """递归调用测试"""
        if n <= 1:
            return n
        return self.recursive_fibonacci(n-1) + self.recursive_fibonacci(n-2)
    
    def nested_loops(self, size: int = 100) -> float:
        """嵌套循环和数学运算"""
        result = 0.0
        for i in range(size):
            for j in range(size):
                for k in range(10):
                    result += math.sqrt(i * j + k + 1)
                    result = math.sin(result) * math.cos(result)
        return result
    
    def list_comprehension_heavy(self) -> List[int]:
        """列表推导式和内置函数调用"""
        return [
            sum(x * y for y in range(10))
            for x in range(100)
            for _ in range(5)
        ]
    
    def dict_operations(self, iterations: int = 1000) -> Dict[str, Any]:
        """字典操作和字符串处理"""
        result = {}
        for i in range(iterations):
            key = f"key_{i}_{random.randint(0, 100)}"
            value = {
                'data': [random.random() for _ in range(10)],
                'sum': sum(random.random() for _ in range(10)),
                'nested': {str(j): j**2 for j in range(5)}
            }
            result[key] = value
            
            # 随机删除一些键
            if len(result) > 500 and random.random() > 0.5:
                del_key = random.choice(list(result.keys()))
                del result[del_key]
        
        return result
    
    def exception_handling(self, iterations: int = 100) -> int:
        """异常处理测试"""
        caught = 0
        for i in range(iterations):
            try:
                if i % 3 == 0:
                    raise ValueError("Test exception")
                elif i % 5 == 0:
                    raise KeyError("Test key error")
                else:
                    _ = 1 / (i % 7 - 3)  # 可能的ZeroDivisionError
            except (ValueError, KeyError):
                caught += 1
            except ZeroDivisionError:
                caught += 2
        return caught
    
    def generator_chain(self, size: int = 1000) -> int:
        """生成器和迭代器测试"""
        def gen1():
            for i in range(size):
                yield i * 2
                
        def gen2(g):
            for item in g:
                if item % 3 == 0:
                    yield item
                    
        def gen3(g):
            for item in g:
                yield item ** 2
                
        return sum(gen3(gen2(gen1())))
    
    def class_method_calls(self, depth: int = 5) -> float:
        """类方法调用链"""
        class Calculator:
            def __init__(self, value):
                self.value = value
                
            def add(self, x):
                self.value += x
                return self
                
            def multiply(self, x):
                self.value *= x
                return self
                
            def power(self, x):
                self.value **= x
                return self
                
            def sqrt(self):
                self.value = math.sqrt(abs(self.value))
                return self
        
        result = 0.0
        for i in range(100):
            calc = Calculator(i + 1)
            for _ in range(depth):
                calc.add(1).multiply(2).power(0.5).sqrt()
            result += calc.value
            
        return result
    
    def mixed_workload(self, n) -> Dict[str, Any]:
        """混合工作负载，运行指定时长"""
        iteration = 0
        results = {
            'iterations': 0,
            'recursive_calls': 0,
            'loop_results': [],
            'exceptions_caught': 0,
            'generator_sums': 0,
        }
        
        for i in range(n):
            iteration += 1
            
            # 1. 递归调用（控制深度避免栈溢出）
            fib_n = 5 + (iteration % 10)
            results['recursive_calls'] += self.recursive_fibonacci(fib_n)
            
            # 2. 嵌套循环
            if iteration % 5 == 0:
                loop_result = self.nested_loops(20 + (iteration % 30))
                results['loop_results'].append(loop_result)
            
            # 3. 列表操作
            if iteration % 3 == 0:
                _ = self.list_comprehension_heavy()
            
            # 4. 字典操作
            if iteration % 7 == 0:
                _ = self.dict_operations(100)
            
            # 5. 异常处理
            results['exceptions_caught'] += self.exception_handling(50)
            
            # 6. 生成器
            if iteration % 4 == 0:
                results['generator_sums'] += self.generator_chain(100)
            
            # 7. 类方法调用
            if iteration % 6 == 0:
                _ = self.class_method_calls(3)
            
            results['iterations'] = iteration
            
            # 添加一些CPU密集型操作
            for _ in range(100):
                _ = sum(math.sin(i) * math.cos(i) for i in range(100))
        
        return results

def run_benchmark(n, with_profiler=None):
    """运行基准测试
    
    Args:
        duration: 运行时长（秒）
        with_profiler: profiler对象，如果提供则启用profiling
    """
    benchmark = ProfilerBenchmark()
    
    if with_profiler:
        probing.enable_tracer()
    
    start_time = time.time()
    results = benchmark.mixed_workload(n)
    end_time = time.time()
    
    results['total_time'] = end_time - start_time
    results['profiler_enabled'] = with_profiler is not None
    
    return results

def compare_overhead():
    """比较有无profiler的性能差异"""
    print("=== Profiler Overhead Benchmark ===")
    print(f"Target duration: 10 seconds per test\n")
    
    n = 1000
    # 1. 无profiler基准测试
    print("Running baseline (no profiler)...")
    baseline_results = run_benchmark(n, with_profiler=None)
    print(f"Baseline completed: {baseline_results['iterations']} iterations in {baseline_results['total_time']:.2f}s")
    
    # 2. 使用简单profiler
    def simple_profiler(frame, event, arg):
        # 最简单的profiler，只计数
        pass
    
    print("\nRunning with simple profiler...")
    profiled_results = run_benchmark(n, with_profiler=simple_profiler)
    print(f"Profiled completed: {profiled_results['iterations']} iterations in {profiled_results['total_time']:.2f}s")
    
    # 3. 计算开销
    baseline_iter_per_sec = baseline_results['iterations'] / baseline_results['total_time']
    profiled_iter_per_sec = profiled_results['iterations'] / profiled_results['total_time']
    
    overhead_percent = ((baseline_iter_per_sec - profiled_iter_per_sec) / baseline_iter_per_sec) * 100
    
    print("\n=== Results ===")
    print(f"Baseline performance: {baseline_iter_per_sec:.2f} iterations/second")
    print(f"Profiled performance: {profiled_iter_per_sec:.2f} iterations/second")
    print(f"Profiler overhead: {overhead_percent:.1f}%")
    
    return {
        'baseline': baseline_results,
        'profiled': profiled_results,
        'overhead_percent': overhead_percent
    }

if __name__ == "__main__":
    # 运行性能对比测试
    results = compare_overhead()
    
    # 保存详细结果
    with open('profiler_benchmark_results.json', 'w') as f:
        json.dump(results, f, indent=2)
    
    print("\nDetailed results saved to profiler_benchmark_results.json")