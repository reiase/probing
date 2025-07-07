import time
import probing


class RDMAMonitor():
    """
    A class to monitor RDMA statistics using an external table.
    
    This class extends probing.ExternalTable to provide a structured way to
    manage RDMA-related data.
    """
    def __init__(self, hca_name="mlx5_cx6_0", tbl_name="rdma_monitor_0"):
        self.table = probing.ExternalTable(
            tbl_name, 
            [
                "port_rcv_packets", 
                "port_rcv_data", 
                "port_xmit_packets", 
                "port_xmit_data",
                "link_downed",
                "np_cnp_sent",
                "np_ecn_marked_roce_packets",
                "rcv_pkts_rate",
                "snd_pkts_rate"
            ],
            chunk_size=10,
            discard_threshold=10,
            discard_strategy="BaseElementCount"
        )
        self.hca_name = hca_name
        self._previous_port_rcv_packets = None
        self._previous_port_xmit_packets = None
        self._last_measurement_time = None
    
    def read_counter(self, counter_name):
        """从/sys/class/infiniband目录读取指定的计数器值"""
        try:
            if counter_name == "np_cnp_sent" or counter_name == "np_ecn_marked_roce_packets":
                with open(f"/sys/class/infiniband/{self.hca_name}/ports/1/hw_counters/{counter_name}", 'r') as f:
                    return int(f.read().strip())
            else:
                with open(f"/sys/class/infiniband/{self.hca_name}/ports/1/counters/{counter_name}", 'r') as f:
                    return int(f.read().strip())
        except Exception as e:
            print(f"Error reading counter {counter_name}: {e}")
            return 0

    def calculate_rate(self, current, previous, interval):
        if current is None or previous is None:
            return 0.0
        
        if current < previous:
            current += 2**64 
        return (current - previous) / interval
    
    def obtain_newset(self):
        port_rcv_packets = self.read_counter("port_rcv_packets")
        port_rcv_data = self.read_counter("port_rcv_data")
        port_xmit_packets = self.read_counter("port_xmit_packets")
        port_xmit_data = self.read_counter("port_xmit_data")
        link_downed = self.read_counter("link_downed")
        np_cnp_sent = self.read_counter("np_cnp_sent")
        np_ecn_marked_roce_packets = self.read_counter("np_ecn_marked_roce_packets")

        current_time = time.time()
        interval = current_time - self._last_measurement_time if self._last_measurement_time else None
        
        rcv_pkts_rate = self.calculate_rate(
            port_rcv_packets, self._previous_port_rcv_packets, interval
        )
        
        snd_pkts_rate = self.calculate_rate(
            port_xmit_packets, self._previous_port_xmit_packets, interval
        )
        
        
        self._previous_port_rcv_packets = port_rcv_packets
        self._previous_port_xmit_packets = port_xmit_packets
        self._last_measurement_time = current_time

        new_data = [
            port_rcv_packets,
            port_rcv_data,
            port_xmit_packets,
            port_xmit_data,
            link_downed,
            np_cnp_sent,
            np_ecn_marked_roce_packets,
            rcv_pkts_rate,
            snd_pkts_rate
        ]
        
        print(new_data)

        self.table.append(new_data) 
        
        return None
    

if __name__ == "__main__":
    monitor = RDMAMonitor(tbl_name="rdma_monitor_mlx0", hca_name="mlx5_cx6_0")
    while True:
        monitor.obtain_newset()
        time.sleep(5)
        