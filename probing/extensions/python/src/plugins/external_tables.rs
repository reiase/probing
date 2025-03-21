use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex};

use once_cell::sync::Lazy;
use probing_proto::types::{Ele, TimeSeries};
use pyo3::types::PyType;
use pyo3::{pyclass, pymethods, Bound, IntoPyObjectExt, PyObject, PyResult, Python};

fn value_to_object(py: Python, v: &Ele) -> PyObject {
    let ret = match v {
        Ele::Nil => Option::<i32>::None.into_bound_py_any(py),
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

pub static EXTERN_TABLES: Lazy<Mutex<HashMap<String, Arc<Mutex<TimeSeries>>>>> =
    Lazy::new(|| Mutex::new(Default::default()));

#[pyclass]
#[derive(Clone, Debug)]
pub struct ExternalTable(Arc<Mutex<TimeSeries>>, usize);

#[pymethods]
impl ExternalTable {
    #[new]
    fn new(name: &str, columns: Vec<String>) -> Self {
        let ncolumn = columns.len();
        let ts = Arc::new(Mutex::new(
            TimeSeries::builder().with_columns(columns).build(),
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
    ) -> PyResult<ExternalTable> {
        let mut binding = EXTERN_TABLES.lock().unwrap();
        let ts = binding.get(name);
        if let Some(ts) = ts {
            let ncolumn = ts.lock().unwrap().cols.len();
            Ok(ExternalTable(ts.clone(), ncolumn))
        } else {
            let ncolumn = columns.len();
            let ts = Arc::new(Mutex::new(
                TimeSeries::builder().with_columns(columns).build(),
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
mod specs {
    use crate::{create_probing_module, plugins::python::PythonPlugin};

    use super::*;

    use probing_core::{
        core::Engine,
        plugins::{envs::EnvPlugin, file::FilePlugin},
    };
    use pyo3::ffi::c_str;
    use rspec;

    #[test]
    fn external_table_specs() {
        rspec::run(&rspec::describe("External Table in Python", (), |ctx| {
            ctx.specify("External Table Rust APIs", |ctx| {
                ctx.before_all(|_| {
                    create_probing_module().unwrap();
                });

                ctx.it("should create a new table", |_| {
                    let table =
                        ExternalTable::new("table1", vec!["a".to_string(), "b".to_string()]);
                    assert_eq!(table.names(), vec!["a", "b"]);
                });

                ctx.it("should execute create table in python", |_| {
                    Python::with_gil(|py| {
                        create_probing_module().unwrap();
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
                });

                ctx.it("should drop a table in python", |_| {
                    Python::with_gil(|py| {
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
                });
            });

            ctx.specify("Access External Table in Engine", |ctx| {
                ctx.before_all(|_| {
                    create_probing_module().unwrap();

                    Python::with_gil(|py| {
                        py.run(
                            c_str!(
                                r#"
import probing
table3 = probing.ExternalTable.get_or_create("table3", ["a", "b"])
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
                });

                ctx.it("should see py table in engine", |_| {
                    let engine = Engine::builder()
                        .with_information_schema(true)
                        .with_default_catalog_and_schema("probe", "probe")
                        .with_plugin("probe", Arc::new(PythonPlugin::new("python")))
                        .with_plugin("probe", Arc::new(FilePlugin::new("file")))
                        .with_plugin("probe", Arc::new(EnvPlugin::new("envs", "process")))
                        .build()
                        .unwrap();
                    let tables = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(4)
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(async {
                            engine
                        .query(
                            "select * from probe.information_schema.tables where table_name = 'table3' ",
                        )
                        .unwrap()
                        });
                    assert_eq!(tables.len(), 1);
                });

                ctx.it("should see py table data in engine", |_| {
                    let engine = Engine::builder()
                        .with_information_schema(true)
                        .with_default_catalog_and_schema("probe", "probe")
                        .with_plugin("probe", Arc::new(PythonPlugin::new("python")))
                        .build()
                        .unwrap();
                    let tables = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(4)
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(async { engine.query("select * from python.table3 ").unwrap() });
                    assert_eq!(tables.len(), 3);
                });

                ctx.it("should support calculate in sql", |_| {
                    let engine = Engine::builder()
                        .with_information_schema(true)
                        .with_default_catalog_and_schema("probe", "probe")
                        .with_plugin("probe", Arc::new(PythonPlugin::new("python")))
                        .build()
                        .unwrap();
                    let tables = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(4)
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(async {
                            engine
                                .query(
                                    "select a + b as c from python.table3 where a > 1",
                                )
                                .unwrap()
                        });
                    assert_eq!(tables.len(), 2);
                });

                ctx.it("should support calculate in sql", |_| {
                    let engine = Engine::builder()
                    .with_information_schema(true)
                    .with_default_catalog_and_schema("probe", "probe")
                    .with_plugin("probe", Arc::new(PythonPlugin::new("python")))
                    .build()
                    .unwrap();
                    let tables = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(4)
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async {
                        engine
                            .query(
                                "select sum(a), sum(b) from python.table3",
                            )
                            .unwrap()
                    });
                    println!("{:?}", tables);
                });
            });
        }));
    }
}
