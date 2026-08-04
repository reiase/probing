# 节点上报超时故障注入

本文用于验证节点上报在 master 不可达或响应停滞时的行为，重点确认：

- 上报请求按配置超时，不会长期挂住；
- 同步 HTTP I/O 不会阻塞 probing 的 Tokio 服务线程；
- 连续失败按 10、20、40、60 秒退避，且不会产生并发请求堆积；
- 心跳过期后节点变为 `offline`，链路恢复后重新变为 `running`。

仅在测试集群执行。开始前记录 master IP、probing 端口、一个非零 rank 的 probing 地址，以及 rank 0 PID：

```bash
export MASTER_IP=<master-ip>
export PROBING_PORT=<probing-port>
export WORKER_PROBING_ADDR=<worker-ip:probing-port>
export RANK0_PID=<rank-0-pid>
```

## 方案一：只阻断节点上报端口（推荐）

在一个非零 rank 所在节点执行。规则只丢弃发往 master probing 端口的 TCP 流量，不暂停训练进程。`DROP` 会制造真实超时；不要换成 `REJECT`，后者通常立即返回连接错误。

先安装自动恢复 trap，再添加规则：

```bash
iptables -I OUTPUT 1 -p tcp -d "$MASTER_IP" --dport "$PROBING_PORT" -j DROP 
iptables -I OUTPUT 1 -p tcp -d "$MASTER_IP" --dport "$PROBING_PORT" --sport 20000:20099 -j ACCEPT

iptables -nvL OUTPUT --line-numbers 

```

在另一个终端持续访问该非零 rank 自己的 probing API：

```bash
curl -sS --local-port 20000-20099 --connect-timeout 1 --max-time 2 "http://$MASTER_IP:$PROBING_PORT/apis/nodes"
```

同时从 master 查询节点状态：

```bash
watch -n 2 "curl -sS http://$MASTER_IP:$PROBING_PORT/apis/nodes"  # no timeout

curl -v --connect-timeout 1 --max-time 3  http://10.107.204.71:9933/apis/nodes
```

预期结果：

1. 首次失败在单次请求超时（默认 3 秒）附近出现；
2. worker 本地 API 仍能稳定响应，没有接近 3 秒的整体卡顿；
3. debug 日志中的后续尝试逐步退避，最大间隔不超过 60 秒；
4. 默认 30 秒没有新心跳后，对应节点显示为 `offline`。

完成观察后恢复网络：

```bash
iptables -D OUTPUT -p tcp -d "$MASTER_IP" --dport "$PROBING_PORT" --sport 20000:20099 -j ACCEPT 
iptables -D OUTPUT -p tcp -d "$MASTER_IP" --dport "$PROBING_PORT" -j DROP
iptables -nvL OUTPUT --line-numbers
```

恢复后，节点应在下一次退避到期时成功上报，日志出现 recovered，状态重新变为 `running`。最坏情况下需要等待约 60 秒。

## 方案二：暂停 rank 0 进程

`SIGSTOP` 会暂停 rank 0 的训练逻辑和 probing server。监听 socket 通常仍存在，其他 rank 可能完成 TCP 连接但收不到 HTTP 响应，从而触发 timeout。

先注册恢复 trap，避免终端中断后遗留暂停进程：

```bash
resume_rank0() {
  kill -CONT "$RANK0_PID" 2>/dev/null || true
}
trap resume_rank0 EXIT INT TERM

kill -STOP "$RANK0_PID"
```

- 暂停 5–15 秒：验证单次 timeout、worker API 响应和恢复日志；
- 暂停超过 30 秒：验证离线状态，但可能触发 NCCL watchdog、Torch Elastic 重启或训练任务失败。

恢复 rank 0：

```bash
resume_rank0
trap - EXIT INT TERM
```

rank 0 暂停期间无法查询 master 的 `/apis/nodes`。恢复后再查询，并等待下一次心跳确认节点状态恢复。

## 验收标准

| 检查项 | 通过标准 |
| --- | --- |
| 单次请求超时 | 默认约 3 秒返回错误，而非无限等待 |
| 服务线程隔离 | 上报超时期间 worker 本地 API 仍在 1 秒内响应 |
| 请求并发 | 每个节点同一时刻最多一个上报请求 |
| 失败退避 | 约按 10、20、40、60 秒递增，60 秒封顶并带少量抖动 |
| 离线检测 | 默认 30 秒无心跳后显示 `offline` |
| 故障恢复 | 链路恢复后自动上报并回到 `running` |

## 清理检查

确认没有遗留 iptables 规则或暂停进程：

```bash
sudo iptables -C OUTPUT \
  -p tcp -d "$MASTER_IP" --dport "$PROBING_PORT" -j DROP \
  2>/dev/null && echo 'WARNING: DROP rule still exists'

ps -o pid,stat,cmd -p "$RANK0_PID"
```

如果进程状态包含 `T`，执行：

```bash
kill -CONT "$RANK0_PID"
```
