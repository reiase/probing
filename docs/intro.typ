#import "@preview/touying:0.6.1": *
#import themes.metropolis: *

#import "@preview/numbly:0.1.0": numbly

#show: metropolis-theme.with(
  aspect-ratio: "16-9",
  footer: self => self.info.institution,
  config-info(
    title: [Probing 分布式探针系统],
    subtitle: [一种分布式问题与性能诊断工具],
    author: [侯杰],
    date: datetime.today(),
    institution: [昆仑芯],
    logo: emoji.city,
  ),
)

#set heading(numbering: numbly("{1}.", default: "1.1"))
#set text(font: "Source Han Serif")

#title-slide()

= Probing 简介

== 什么是 Probing？
  - *Probing* 是一款Python与AI应用的*性能与稳定性诊断工具*。
  - 旨在解决大规模、分布式、长周期AI异构计算任务（如LLM训练和推理）中的*调试与优化难题*。

== 为何需要Probing

  === 大规模训练的现实困境
    - *高昂成本*：千卡训练，效率问题直接转化为经济损失。
    - *故障难追*：训练偶发性hang住或崩溃，原因追踪困难。
    - *性能瓶颈*：节点间性能差异、分布式通信瓶颈，难以定位。
    - *调试低效*：停机调试意味着浪费珍贵的计算资源。

  ---

  === 现有工具的局限性
    - *单机工具限制*：VTune/Nsight等优秀工具仅限单机分析，无法扩展至分布式场景。
    - *PyTorch内置Profiler困境*：
        - 性能开销大：全量采集模式显著影响训练速度。
        - 数据爆炸：大规模集群环境下产生TB级数据，分析困难。
        - 缺乏节点间协调：难以进行有效的分布式诊断。
    - *Timeline方法瓶颈*：
        - 数据量庞大导致可视化分析困难。
        - 难以刻画系统统计特性（如P99分位耗时）。
        - 不易发现分布式环境中的负载不均等现象。

= Probing 核心设计理念
---
  - *零侵入*：无需代码改造、环境适配或流程变更，通过动态探针实现透明接入。
  - *零认知门槛*：采用标准SQL交互，将复杂性能分析转化为直观的数据库查询。
  - *零部署负担*：基于Rust的极简静态编译，实现单文件部署与弹性扩展。
  - *核心技术要素*：
    - 动态代码注入 (Instrumentation)
    - 按需智能采样 (Sampling)
    - 原生分布式设计

= 整体架构
---
  - 简洁可靠的*两层设计*：
    - *探针平面 (Probe Plane)*：作为动态链接库(.so)注入目标进程，负责核心数据采集、代码注入、内置查询引擎及功能扩展。
    - *控制平面 (Control Plane)*：提供用户交互界面，包括Web UI、命令行工具(CLI)和统一API。

= 核心功能
  == 堆栈分析 (Stack Analysis)
    - *Python 调用栈捕获*：获取Python调用栈、调用参数和局部变量；
    - *Native 调用栈捕获*：获取C/C++调用堆栈信息；

    #figure(
      image("imgs/stacks.png",width: 60%),
      caption: [堆栈分析示意图]
    )

  == 性能分析 (Profiling)
    - *Torch Profiling*：
      - 将模型执行按其Layer结构，自动拆分成不同的Span；
      - 对每个Span以采样方式计时，并记录相关元数据（如计算内容、输入特征）；
      - 同步采样底层硬件计数器（如NCCL通信量、内存访存数据）；
      - 结合性能建模，评估各Span硬件吞吐的合理性，计算算力、内存带宽、互联带宽利用率。
    #figure(
      image("imgs/torch_profiling.png",width: 60%),
      caption: [Torch Profiling示意图]
    )

  == 数据洞察（Timeseries）
    - *数据采集*：主要支持in-memory数据采集，无需额外的存储开销；
    - *数据分析*：基于内涵SQL的分析引擎，支持多种数据分析和可视化方式；

    #figure(
      image("imgs/data_explore.png",width: 60%),
      caption: [数据分析视图]
    )