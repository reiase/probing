#import "@preview/  :0.6.1": *
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

= Probing 介绍

== 为何需要Probing

大规模训练的现实挑战

- 高昂的计算资源成本：千卡训练每小时成本数万美元，效率问题直接转化为经济损失
- 难以复现的故障：
    - 训练突然hang住或崩溃，但无法追踪原因
    - 节点间性能差异显著，但无工具可查
    - 分布式通信出现瓶颈，但无法定位
- 调试成本极高：停机调试意味着浪费珍贵计算资源

---

现有工具的局限
- 单机工具无法扩展：VTune/Nsight等优秀工具仅限单机分析
- PyTorch内置profiler的困境：
    - 性能开销大（全量采集模式影响训练速度）
    - 数据爆炸（千卡环境产生TB级数据）
    - 缺乏节点间协调能力
- Timeline方法的死胡同：
    - 数据量庞大导致可视化分析困难
    - 无法刻画系统统计特性（如99%分位耗时）
    - 难以发现分布式环境中的负载不均现象

---

解决问题的理想工具

- 零侵入：无需代码修改，通过动态注入实现透明接入
- 低开销：按需采样，控制性能影响
- 分布式：支持大规模集群环境
- 全链路：覆盖硬件、系统、框架到模型的完整监控

Probing 核心设计

- 探针: 动态代码注入；
- 采样: 按需采集，控制性能开销；
- 分布式: 支持大规模集群环境；

--- 

== 使用CASE分析

