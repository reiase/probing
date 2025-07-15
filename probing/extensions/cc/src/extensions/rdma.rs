use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;
use probing_core::core::Maybe;

use datafusion::arrow::array::{GenericStringBuilder, RecordBatch};
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};

use probing_core::core::{CustomTable, EngineCall, EngineDatasource, TablePluginHelper};

use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use std::collections::HashMap;
use async_trait::async_trait;

use std::fs::File;
use std::io::{self, Read};
use std::time::{Duration, Instant};


static GLOBAL_HCA_NAME: OnceLock<Mutex<String>> = OnceLock::new();
static GLOBAL_HCA_SAMPLE_RATE: OnceLock<Mutex<f64>> = OnceLock::new();


#[derive(Default, Debug)]
pub struct RdmaTable {}

impl CustomTable for RdmaTable {
    fn name() -> &'static str {
        "mlx_hca"
    }

    fn schema() -> datafusion::arrow::datatypes::SchemaRef {
        SchemaRef::new(Schema::new(vec![
            Field::new("hca_name", DataType::Utf8, false),
            Field::new("port_rcv_packets", DataType::Float64, false),
            Field::new("port_rcv_data", DataType::Float64, false),
            Field::new("port_xmit_packets", DataType::Float64, false),
            Field::new("port_xmit_data", DataType::Float64, false),
            Field::new("link_downed", DataType::Float64, false),
            Field::new("np_cnp_sent", DataType::Float64, false),
            Field::new("np_ecn_marked_roce_packets", DataType::Float64, false),
            Field::new("rcv_pkts_rate", DataType::Float64, false),
            Field::new("snd_pkts_rate", DataType::Float64, false),
        ]))
    }

    fn data() -> Vec<datafusion::arrow::array::RecordBatch> {
        let string = GLOBAL_HCA_NAME.get_or_init(|| Mutex::new(String::new()));
        let guard = string.lock().unwrap();
        let hca_name = guard.clone();

        let mut monitor = RDMAMonitor::new(&hca_name);
        monitor.obtain_newset();

        let mut hca_name = GenericStringBuilder::<i32>::new();
        let mut port_rcv_packets = datafusion::arrow::array::Float64Builder::new();
        let mut port_rcv_data = datafusion::arrow::array::Float64Builder::new();
        let mut port_xmit_packets = datafusion::arrow::array::Float64Builder::new();
        let mut port_xmit_data = datafusion::arrow::array::Float64Builder::new();
        let mut link_downed = datafusion::arrow::array::Float64Builder::new();
        let mut np_cnp_sent = datafusion::arrow::array::Float64Builder::new();
        let mut np_ecn_marked_roce_packets = datafusion::arrow::array::Float64Builder::new();
        let mut rcv_pkts_rate = datafusion::arrow::array::Float64Builder::new();
        let mut snd_pkts_rate = datafusion::arrow::array::Float64Builder::new();
        hca_name.append_value(monitor.hca_name.clone());
        port_rcv_packets.append_value(monitor.read_counter("port_rcv_packets"));
        port_rcv_data.append_value(monitor.read_counter("port_rcv_data"));
        port_xmit_packets.append_value(monitor.read_counter("port_xmit_packets"));
        port_xmit_data.append_value(monitor.read_counter("port_xmit_data"));
        link_downed.append_value(monitor.read_counter("link_downed"));
        np_cnp_sent.append_value(monitor.read_counter("np_cnp_sent"));
        np_ecn_marked_roce_packets.append_value(monitor.read_counter("np_ecn_marked_roce_packets"));
        
        let f64_val = GLOBAL_HCA_SAMPLE_RATE.get_or_init(|| Mutex::new(0.0));
        let guard = f64_val.lock().unwrap();
        let sleep_time = guard.clone() as u64;

        thread::sleep(Duration::from_secs(sleep_time));

        rcv_pkts_rate.append_value(monitor.calculate_rate(
            Some(monitor.read_counter("port_rcv_packets")),
            monitor.previous_port_rcv_packets,
            monitor.last_measurement_time.map(|t| t.elapsed()),
        ));
        snd_pkts_rate.append_value(monitor.calculate_rate(
            Some(monitor.read_counter("port_xmit_packets")),
            monitor.previous_port_xmit_packets,
            monitor.last_measurement_time.map(|t| t.elapsed()),
        ));
        let rbs = RecordBatch::try_new(
            Self::schema(),
            vec![
                Arc::new(hca_name.finish()),
                Arc::new(port_rcv_packets.finish()),
                Arc::new(port_rcv_data.finish()),
                Arc::new(port_xmit_packets.finish()),
                Arc::new(port_xmit_data.finish()),
                Arc::new(link_downed.finish()),
                Arc::new(np_cnp_sent.finish()),
                Arc::new(np_ecn_marked_roce_packets.finish()),
                Arc::new(rcv_pkts_rate.finish()),
                Arc::new(snd_pkts_rate.finish()),
            ],
        );
        if let Ok(rbs) = rbs {
            vec![rbs]
        } else {
            Default::default()
        }
    }
}


pub type RdmaPlugin = TablePluginHelper<RdmaTable>;

impl EngineDatasource for RdmaExtension {
    fn datasrc(
        &self,
        namespace: &str,
        name: Option<&str>,
    ) -> Option<std::sync::Arc<dyn probing_core::core::Plugin + Sync + Send>> {
        match name {
            Some(name) => Some(RdmaPlugin::create(namespace, name)),
            None => None,
        }
    }
}

#[derive(Debug, Default, EngineExtension)]
pub struct RdmaExtension {
    #[option(aliases=["sample.rate"])]
    sample_rate: Maybe<f64>,

    #[option(aliases=["hca.name"])]
    hca_name: Maybe<String>,
}

#[async_trait]
impl EngineCall for RdmaExtension {
    async fn call(
        &self,
        path: &str,
        params: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<Vec<u8>, EngineError> {
        if path == "" {
            let string = GLOBAL_HCA_NAME.get_or_init(|| Mutex::new(String::new()));
            let guard = string.lock().unwrap();
            let hca_name = guard.clone();

            let mut monitor = RDMAMonitor::new(&hca_name);

            const TEST_CALL: u32 = 5;
            let mut call_cnt = 0;

            while call_cnt < TEST_CALL {
                call_cnt += 1;
                monitor.obtain_newset();
                std::thread::sleep(Duration::from_millis(1000));
            }
            return Ok("RDMA request handled successfully".as_bytes().to_vec());
        }
        Err(EngineError::UnsupportedCall)
    }
}

impl RdmaExtension {
    fn set_sample_rate(&mut self, sample_rate: Maybe<f64>) -> Result<(), EngineError> {
        if let Maybe::Just(rate) = sample_rate {
            if !(0.0..=20.0).contains(&rate) {
                return Err(EngineError::InvalidOptionValue(
                    Self::OPTION_SAMPLE_RATE.to_string(),
                    rate.to_string(),
                ));
            }

            let f64_val = GLOBAL_HCA_SAMPLE_RATE.get_or_init(|| Mutex::new(0.0));
            let mut guard = f64_val.lock().unwrap();
            *guard = rate;
        }

        self.sample_rate = sample_rate;
        
        Ok(())
    }

    fn set_hca_name(&mut self, hca_name: Maybe<String>) -> Result<(), EngineError> {
        self.hca_name = hca_name;
        
        if let Maybe::Just(ref name) = self.hca_name {
            if name.is_empty() {
                return Err(EngineError::InvalidOptionValue(
                    Self::OPTION_HCA_NAME.to_string(),
                    "HCA name cannot be empty".to_string(),
                ));
            }

            let string = GLOBAL_HCA_NAME.get_or_init(|| Mutex::new(String::new()));
            let mut guard = string.lock().unwrap();
            *guard = name.clone();
        }

        Ok(())
    }
}


struct RDMAMonitor {
    hca_name: String,
    previous_port_rcv_packets: Option<f64>,
    previous_port_xmit_packets: Option<f64>,
    last_measurement_time: Option<Instant>,
}

impl RDMAMonitor {
    fn new(hca_name: &str) -> Self {
        RDMAMonitor {
            hca_name: hca_name.to_string(),
            previous_port_rcv_packets: None,
            previous_port_xmit_packets: None,
            last_measurement_time: None,
        }
    }

    fn read_counter(&self, counter_name: &str) -> f64 {
        let path = if counter_name == "np_cnp_sent" || counter_name == "np_ecn_marked_roce_packets" {
            format!("/sys/class/infiniband/{}/ports/1/hw_counters/{}", self.hca_name, counter_name)
        } else {
            format!("/sys/class/infiniband/{}/ports/1/counters/{}", self.hca_name, counter_name)
        };

        match read_file_to_f64(&path) {
            Ok(value) => value,
            Err(e) => {
                println!("Error reading counter {}: {}", counter_name, e);
                0.0
            }
        }
    }

    fn calculate_rate(&self, current: Option<f64>, previous: Option<f64>, interval: Option<Duration>) -> f64 {
        if current.is_none() || previous.is_none() || interval.is_none() {
            return 0.0;
        }

        let current = current.unwrap();
        let previous = previous.unwrap();
        let interval = interval.unwrap().as_secs_f64();

        let diff = if current < previous {
        current + 2u64.pow(64) as f64 - previous
        } else {
        current - previous
        };

        diff / interval
    }

    fn obtain_newset(&mut self) {
        let port_rcv_packets = self.read_counter("port_rcv_packets");
        let port_rcv_data = self.read_counter("port_rcv_data");
        let port_xmit_packets = self.read_counter("port_xmit_packets");
        let port_xmit_data = self.read_counter("port_xmit_data");
        let link_downed = self.read_counter("link_downed");
        let np_cnp_sent = self.read_counter("np_cnp_sent");
        let np_ecn_marked_roce_packets = self.read_counter("np_ecn_marked_roce_packets");

        let current_time = Instant::now();
        let interval = self.last_measurement_time.map(|t| current_time.duration_since(t));

        let rcv_pkts_rate = self.calculate_rate(
            Some(port_rcv_packets),
            self.previous_port_rcv_packets,
            interval,
        );

        let snd_pkts_rate = self.calculate_rate(
            Some(port_xmit_packets),
            self.previous_port_xmit_packets,
            interval,
        );

        self.previous_port_rcv_packets = Some(port_rcv_packets);
        self.previous_port_xmit_packets = Some(port_xmit_packets);
        self.last_measurement_time = Some(current_time);

        let new_data = [
            port_rcv_packets as f64,
            port_rcv_data as f64,
            port_xmit_packets as f64,
            port_xmit_data as f64,
            link_downed as f64,
            np_cnp_sent as f64,
            np_ecn_marked_roce_packets as f64,
            rcv_pkts_rate,
            snd_pkts_rate,
        ];

        println!("New RDMA data: {:?}", new_data);
    }
}

fn read_file_to_f64(path: &str) -> io::Result<f64> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    contents.trim().parse::<f64>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}