use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex};

use once_cell::sync::Lazy;
use probing_proto::prelude::{Ele, TimeSeries, ExternalTableConfig};
use probing_proto::types::series::DiscardStrategy;
use pyo3::prelude::*;
use pyo3::types::{PyType, PyDict};
// use pyo3::exceptions::PyValueError;
use pyo3::{pyclass, pymethods, Bound, IntoPyObjectExt, PyObject, PyResult, Python};


fn value_to_object(py: Python, v: &Ele) -> PyObject {
    let ret = match v {
        Ele::Nil => Option::<i32>::None.into_bound_py_any(py),
        Ele::BOOL(v) => v.into_bound_py_any(py),
        Ele::I64(v) => v.into_bound_py_any(py),
        Ele::I32(v) => v.into_bound_py_any(py),
        Ele::F64(v) => v.into_bound_py_any(py),
        Ele::F32(v) => v.into_bound_py_any(py),
        Ele::Text(v) => v.into_bound_py_any(py),
        Ele::Url(_) => todo!(),
        Ele::DataTime(_) => todo!(),
    };
    ret.map(|x| x.unbind()).unwrap_or(py.None())
}

#[pyclass]
pub struct PyExternalTableConfig {
    #[pyo3(get)]
    chunk_size: usize,
    #[pyo3(get)]
    discard_threshold: usize,
    #[pyo3(get)]
    discard_strategy: String,
}

impl Default for PyExternalTableConfig {
    fn default() -> Self {
        PyExternalTableConfig {
            chunk_size: 10000,
            discard_threshold: 20_000_000,
            discard_strategy: "BaseMemorySize".to_string(),
        }
    }
}

impl FromPyObject<'_> for PyExternalTableConfig {
    fn extract_bound(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        let chunk_size: usize = ob.get_item("chunk_size")?.extract()?;
        let discard_threshold: usize = ob.get_item("discard_threshold")?.extract()?;
        let discard_strategy: String = ob
            .get_item("discard_strategy")
            .map_or(Ok("None".to_string()), |v| v.extract())?;
        Ok(PyExternalTableConfig { chunk_size, discard_threshold, discard_strategy })
    }
}

impl From<PyExternalTableConfig> for ExternalTableConfig {
    fn from(py_config: PyExternalTableConfig) -> Self {
        ExternalTableConfig {
            chunk_size: py_config.chunk_size,
            discard_threshold: py_config.discard_threshold,
            discard_strategy: match py_config.discard_strategy.as_str() {
                "BaseElementCount" => DiscardStrategy::BaseElementCount,
                "BaseMemorySize" => DiscardStrategy::BaseMemorySize,
                _ => DiscardStrategy::None,
            }
        }
    }
}

#[pymethods]
impl PyExternalTableConfig {
    #[new]
    fn new(chunk_size: usize, discard_threshold: usize, discard_strategy: String) -> Self {
        PyExternalTableConfig { 
            chunk_size, 
            discard_threshold, 
            discard_strategy,
        }
    }

    fn into_py(&self, py: Python<'_>) -> PyObject {
        let dict = PyDict::new(py);
        dict.set_item("chunk_size", &self.chunk_size).unwrap();
        dict.set_item("discard_threshold", self.discard_threshold).unwrap();
        dict.set_item("discard_strategy", &self.discard_strategy).unwrap();
        dict.into()
    }
}

pub static EXTERN_TABLES: Lazy<Mutex<HashMap<String, Arc<Mutex<TimeSeries>>>>> =
    Lazy::new(|| Mutex::new(Default::default()));

#[pyclass]
#[derive(Clone, Debug)]
pub struct ExternalTable(Arc<Mutex<TimeSeries>>, usize);

#[pymethods]
impl ExternalTable {
    #[new]
    fn new(name: &str, columns: Vec<String>, py_config: Option<PyExternalTableConfig>  ) -> Self {
        let ncolumn = columns.len();
        let config: ExternalTableConfig = py_config
            .unwrap_or_default()
            .into();           
        let ts = Arc::new(Mutex::new(
            TimeSeries::builder_with_config(config).with_columns(columns).build(),
        ));
        EXTERN_TABLES
            .lock()
            .unwrap()
            .insert(name.to_string(), ts.clone());
        ExternalTable(ts, ncolumn)
    }

    #[classmethod]
    fn get(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<ExternalTable> {
        let binding = EXTERN_TABLES.lock().unwrap();
        let ts = binding.get(name);
        if let Some(ts) = ts {
            let ncolumn = ts.lock().unwrap().cols.len();
            Ok(ExternalTable(ts.clone(), ncolumn))
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(format!(
                "table {} not found",
                name
            )))
        }
    }

    #[classmethod]
    fn get_or_create(
        _cls: &Bound<'_, PyType>,
        name: &str,
        columns: Vec<String>,
        py_config_param: Option<PyExternalTableConfig >,
    ) -> PyResult<ExternalTable> {
        let mut binding = EXTERN_TABLES.lock().unwrap();
        let ts = binding.get(name);
        if let Some(ts) = ts {
            let ncolumn = ts.lock().unwrap().cols.len();
            Ok(ExternalTable(ts.clone(), ncolumn))
        } else {
            let ncolumn = columns.len();
            let config: ExternalTableConfig = py_config_param
                .unwrap_or_default()
                .into();
            let ts = Arc::new(Mutex::new(
                TimeSeries::builder_with_config(config).with_columns(columns).build(),
            ));
            binding.insert(name.to_string(), ts.clone());
            Ok(ExternalTable(ts, ncolumn))
        }
    }

    #[classmethod]
    fn drop(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<()> {
        let _ = EXTERN_TABLES.lock().unwrap().remove(name);
        Ok(())
    }

    fn names(&self) -> Vec<String> {
        self.0.lock().unwrap().names.clone()
    }

    fn append(&mut self, values: Vec<PyObject>) -> PyResult<()> {
        if values.len() != self.1 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "column count mismatch",
            ));
        }
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;
        let values: Vec<Ele> = Python::with_gil(|py| {
            values
                .into_iter()
                .map(|v| {
                    if let Ok(v) = v.extract::<i64>(py) {
                        Ele::I64(v)
                    } else if let Ok(v) = v.extract::<f64>(py) {
                        Ele::F64(v)
                    } else if let Ok(v) = v.extract::<String>(py) {
                        Ele::Text(v)
                    } else {
                        Ele::Nil
                    }
                })
                .collect()
        });
        match self.0.lock().unwrap().append(t.into(), values) {
            Ok(_) => Ok(()),
            Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
        }
    }

    fn append_ts(&mut self, t: i64, values: Vec<PyObject>) -> PyResult<()> {
        if values.len() != self.1 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "column count mismatch",
            ));
        }
        let values: Vec<Ele> = Python::with_gil(|py| {
            values
                .into_iter()
                .map(|v| {
                    if let Ok(v) = v.extract::<i64>(py) {
                        Ele::I64(v)
                    } else if let Ok(v) = v.extract::<f64>(py) {
                        Ele::F64(v)
                    } else if let Ok(v) = v.extract::<String>(py) {
                        Ele::Text(v)
                    } else {
                        Ele::Nil
                    }
                })
                .collect()
        });
        let _ = self.0.lock().unwrap().append(t.into(), values);
        Ok(())
    }

    #[pyo3(signature = (limit=None))]
    fn take(&self, limit: Option<usize>) -> PyResult<Vec<(PyObject, Vec<PyObject>)>> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .take(limit)
            .iter()
            .map(|(t, vals)| {
                Python::with_gil(|py| {
                    let t = value_to_object(py, t);
                    let vals = vals
                        .iter()
                        .map(|v| value_to_object(py, v))
                        .collect::<Vec<_>>();
                    (t, vals)
                })
            })
            .collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_probing_module, extensions::python::PythonPlugin};
    use probing_cc::extensions::envs::EnvPlugin;
    use probing_cc::extensions::files::FilesPlugin;
    use probing_core::core::Engine;
    use pyo3::ffi::c_str;

    fn setup() {
        create_probing_module().unwrap();
    }

    fn setup_table3() {
        setup();
        Python::with_gil(|py| {
            py.run(
                c_str!(
                    r#"
import probing
config = probing.PyExternalTableConfig(chunk_size=10000, discard_threshold=20_000_000, discard_strategy="BaseMemorySize")
table3 = probing.ExternalTable.get_or_create("table3", ["a", "b"], config)
table3.append([1, 2])
table3.append([3, 4])
table3.append([5, 6])
                "#
                ),
                None,
                None,
            )
            .unwrap();
        });
    }

    #[test]
    fn test_create_new_table() {
        setup();
        let table = ExternalTable::new("table1", vec!["a".to_string(), "b".to_string()], None);
        assert_eq!(table.names(), vec!["a", "b"]);
    }

    #[test]
    fn test_create_table_in_python() {
        setup();
        Python::with_gil(|py| {
            py.run(
                c_str!(
                    r#"
import probing
table = probing.ExternalTable.get_or_create("table2", ["a", "b"])
"#
                ),
                None,
                None,
            )
            .unwrap();
            let binding = EXTERN_TABLES.lock().unwrap();
            let table1 = binding.get("table2");
            assert!(table1.is_some());
        });
    }

    #[test]
    fn test_drop_table_in_python() {
        setup();
        Python::with_gil(|py| {
            // Create the table first
            py.run(
                c_str!(
                    r#"
import probing
probing.ExternalTable.get_or_create("table2", ["a", "b"])
                    "#
                ),
                None,
                None,
            )
            .unwrap();

            // Now drop it
            py.run(
                c_str!(
                    r#"
import probing
probing.ExternalTable.drop("table2")
                    "#
                ),
                None,
                None,
            )
            .unwrap();
            let binding = EXTERN_TABLES.lock().unwrap();
            let table1 = binding.get("table2");
            assert!(table1.is_none());
        });
    }

    #[test]
    fn test_see_py_table_in_engine() {
        setup_table3();
        let engine = Engine::builder()
            .with_default_namespace("probe")
            .with_plugin(PythonPlugin::create("python"))
            .with_plugin(FilesPlugin::create("file"))
            .with_plugin(EnvPlugin::create("process", "envs"))
            .build()
            .unwrap();
        let tables = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                engine
                    .async_query(
                        "select * from probe.information_schema.tables where table_name = 'table3' ",
                    ).await
                    .unwrap()
            });
        assert_eq!(tables.len(), 1);
    }

    #[test]
    fn test_see_py_table_data_in_engine() {
        setup_table3();
        let engine = Engine::builder()
            .with_default_namespace("probe")
            .with_plugin(PythonPlugin::create("python"))
            .build()
            .unwrap();
        let tables = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                engine
                    .async_query("select * from python.table3 ")
                    .await
                    .unwrap()
            });
        assert_eq!(tables.len(), 3);
    }

    #[test]
    fn test_calculate_in_sql_with_filter() {
        setup_table3();
        let engine = Engine::builder()
            .with_default_namespace("probe")
            .with_plugin(PythonPlugin::create("python"))
            .build()
            .unwrap();
        let tables = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                engine
                    .async_query("select a + b as c from python.table3 where a > 1")
                    .await
                    .unwrap()
            });
        assert_eq!(tables.len(), 2);
    }

    #[test]
    fn test_aggregate_in_sql() {
        setup_table3();
        let engine = Engine::builder()
            .with_default_namespace("probe")
            .with_plugin(PythonPlugin::create("python"))
            .build()
            .unwrap();
        let tables = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                engine
                    .async_query("select sum(a), sum(b) from python.table3")
                    .await
                    .unwrap()
            });
        println!("{:?}", tables);
        assert!(!tables.is_empty());
    }
}
