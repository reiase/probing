# 欢迎使用Probing

*Probing* 是一款Python与AI应用的性能与稳定性诊断工具。旨在解决大规模、分布式、长周期AI异构计算任务（如LLM训练和推理）中的调试与优化难题。通过向目标进程植入探针，可以更详细地采集性能数据，或实时修改目标进程的执行行为。

Probing设计遵循如下核心原则——零侵入（无需代码改造/环境适配/流程变更，通过动态探针实现透明接入）、零认知门槛（采用标准SQL交互，将复杂性能分析转化为直观的数据库查询）、零部署负担（基于Rust的极简静态编译，实现单文件部署与弹性扩展）