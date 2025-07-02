use std::time::Duration;

use probing_store::store::TCPStore as _TCPStore;
use pyo3::{exceptions::PyException, pyclass, pymethods, PyErr, PyResult};

#[pyclass]
pub struct TCPStore {
    store: _TCPStore,
}

#[pymethods]
impl TCPStore {
    #[new]
    pub fn new(endpoint: String, timeout: Option<u64>) -> Self {
        let timeout = timeout.unwrap_or(1000);
        let store = _TCPStore::new(endpoint).with_timeout(Duration::from_millis(timeout));
        TCPStore { store }
    }

    pub fn set(&mut self, key: &str, value: &str) -> PyResult<()> {
        let ret = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(self.store.set(key, value));
        ret.map_err(|e| PyErr::new::<PyException, _>(format!("Set error: {e}")))
    }

    pub fn get(&mut self, key: &str) -> PyResult<String> {
        let ret = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(self.store.get(key));
        ret.map_err(|e| PyErr::new::<PyException, _>(format!("Get error: {e}")))
    }
}
