#import "@preview/polylux:0.3.1": *
#import themes.metropolis: *

#import "@preview/gentle-clues:0.9.0": *
#import "@preview/showybox:2.0.1": showybox
#import "@preview/fletcher:0.5.1" as fletcher: diagram, node, edge

#show: metropolis-theme.with(aspect-ratio: "16-9")
#set text(size: 24pt)

#title-slide(
  author: [侯杰],
  title: "Probing -- 性能与稳定性诊断工具",
  subtitle: "一种非侵入式诊断工具",
  date: "2024.07.20",
)

#new-section-slide("大模型训练的几个典型问题")

#slide(title: "大模型训练的几个典型问题")[
  - 稳定性问题
  - 性能问题
  - 精度问题
]

#new-section-slide("调试与诊断方法")

#slide(title: "Overview")[
  性能分析与问题诊断是个庞杂的系统工作

  #diagram(
    node-stroke: 1pt,
    node((0,0.1), "底层手段", width: 5cm, stroke: 0pt),
    node((2,0.1), "分析手段", width: 5cm, stroke: 0pt),
    node((3,0.1), "分析任务", width: 5cm, stroke: 0pt),
    node((0,0.5), "插桩", width: 5cm),
    node((0,1.0), "采样", width: 5cm),
    node((0,1.5), "性能计数器", width: 5cm),
    node((0,2.0), "注入", width: 5cm),
    node((0,2.5), "Backtrace", width: 5cm), 
    node((2,0.5), "火焰图", width: 5cm), 
    node((2,1.0), "timeline", width: 5cm),
    node((2,1.5), "callgraph", width: 5cm),
    node((2,2.0), "time serise", width: 5cm),
    node((3,0.5), "热点分析", width: 5cm),
    node((3,1.0), "时序分析", width: 5cm),
    node((3,1.5), "峰值内存", width: 5cm),
    node((3,2.0), "通信分析", width: 5cm),
  )

]