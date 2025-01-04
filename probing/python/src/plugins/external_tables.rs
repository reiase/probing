use std::sync::Arc;
use std::{collections::HashMap, sync::Mutex};

use once_cell::sync::Lazy;
use probing_proto::types::{TimeSeries, Value};
use pyo3::types::PyType;
use pyo3::{pyclass, pymethods, Bound, IntoPyObjectExt, PyObject, PyResult, Python};

pub static EXTERN_TABLES: Lazy<Mutex<HashMap<String, Arc<Mutex<TimeSeries>>>>> =
    Lazy::new(|| Mutex::new(Default::default()));

#[pyclass]
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
        let values: Vec<Value> = Python::with_gil(|py| {
            values
                .into_iter()
                .map(|v| {
                    if let Ok(v) = v.extract::<i64>(py) {
                        Value::Int64(v)
                    } else if let Ok(v) = v.extract::<f64>(py) {
                        Value::Float64(v)
                    } else if let Ok(v) = v.extract::<String>(py) {
                        Value::Text(v)
                    } else {
                        Value::Nil
                    }
                })
                .collect()
        });
        let _ = self.0.lock().unwrap().append(t.into(), values);
        Ok(())
    }

    fn append_ts(&mut self, t: i64, values: Vec<PyObject>) -> PyResult<()> {
        if values.len() != self.1 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "column count mismatch",
            ));
        }
        let values: Vec<Value> = Python::with_gil(|py| {
            values
                .into_iter()
                .map(|v| {
                    if let Ok(v) = v.extract::<i64>(py) {
                        Value::Int64(v)
                    } else if let Ok(v) = v.extract::<f64>(py) {
                        Value::Float64(v)
                    } else if let Ok(v) = v.extract::<String>(py) {
                        Value::Text(v)
                    } else {
                        Value::Nil
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
                    let t = value_to_object(py, &t);
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

// #[pyfunction]
// pub fn create_extern_table(name: &str, columns: Vec<String>) -> PyResult<&ExternTable> {
//     let table = TimeSeries::builder().with_column(columns).build();
//     EXTERN_TABLES
//         .lock()
//         .unwrap()
//         .insert(name.to_string(), ExternTable(table));
//     EXTERN_TABLES
//         .lock()
//         .unwrap()
//         .get(name)
//         .map(|t| {
//             Python::with_gil(|py| {
//                 t.into_bound_py_any(py);
//             });

//             let t = t as *const ExternTable;
//             unsafe { &*t }
//         })
//         .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(format!("table {} not found", name)))
// }

// #[pyfunction]
// pub fn drop_extern_table(name: &str) {
//     let _ = EXTERN_TABLES.lock().unwrap().remove(name);
// }

// #[pyfunction]
// pub fn extern_table_append(name: &str, values: Vec<PyObject>) {
//     let t = std::time::SystemTime::now()
//         .duration_since(std::time::UNIX_EPOCH)
//         .unwrap()
//         .as_micros() as i64;
//     let values: Vec<Value> = Python::with_gil(|py| {
//         values
//             .into_iter()
//             .map(|v| {
//                 if let Ok(v) = v.extract::<i64>(py) {
//                     Value::Int64(v)
//                 } else if let Ok(v) = v.extract::<f64>(py) {
//                     Value::Float64(v)
//                 } else if let Ok(v) = v.extract::<String>(py) {
//                     Value::Text(v)
//                 } else {
//                     Value::Nil
//                 }
//             })
//             .collect()
//     });
//     let _ = EXTERN_TABLES.lock().unwrap().get_mut(name).map(|ts| {
//         let _ = ts.append(t.into(), values);
//     });
// }

// #[pyfunction]
// pub fn extern_table_append_ts(name: &str, t: i64, values: Vec<PyObject>) {
//     let values: Vec<Value> = Python::with_gil(|py| {
//         values
//             .into_iter()
//             .map(|v| {
//                 if let Ok(v) = v.extract::<i64>(py) {
//                     Value::Int64(v)
//                 } else if let Ok(v) = v.extract::<f64>(py) {
//                     Value::Float64(v)
//                 } else if let Ok(v) = v.extract::<String>(py) {
//                     Value::Text(v)
//                 } else {
//                     Value::Nil
//                 }
//             })
//             .collect()
//     });
//     let _ = EXTERN_TABLES.lock().unwrap().get_mut(name).map(|ts| {
//         let _ = ts.append(t.into(), values);
//     });
// }

// #[pyfunction]
// #[pyo3(signature = (name, limit=None))]
// pub fn extern_table_take(
//     name: &str,
//     limit: Option<usize>,
// ) -> PyResult<Vec<(PyObject, Vec<PyObject>)>> {
//     let mut table = EXTERN_TABLES.lock().unwrap();
//     let ts = table.get_mut(name);
//     if let Some(ts) = ts {
//         Ok(ts
//             .take(limit)
//             .iter()
//             .map(|(t, vals)| {
//                 Python::with_gil(|py| {
//                     let t = value_to_object(py, &t);
//                     let vals = vals
//                         .iter()
//                         .map(|v| value_to_object(py, v))
//                         .collect::<Vec<_>>();
//                     (t, vals)
//                 })
//             })
//             .collect::<Vec<_>>())
//     } else {
//         Ok(vec![])
//     }
// }

fn value_to_object(py: Python, v: &Value) -> PyObject {
    let ret = match v {
        Value::Nil => Option::<i32>::None.into_bound_py_any(py),
        Value::Int64(v) => v.into_bound_py_any(py),
        Value::Int32(v) => v.into_bound_py_any(py),
        Value::Float64(v) => v.into_bound_py_any(py),
        Value::Float32(v) => v.into_bound_py_any(py),
        Value::Text(v) => v.into_bound_py_any(py),
        Value::Url(_) => todo!(),
        Value::DataTime(_) => todo!(),
    };
    ret.map(|x| x.unbind()).unwrap_or(py.None())
}
