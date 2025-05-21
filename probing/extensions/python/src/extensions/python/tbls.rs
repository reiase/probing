use std::collections::HashMap;
use std::ffi::CString;
use std::sync::Arc;

use anyhow::Result;

use log::error;
use probing_core::core::{
    ArrayRef, CustomNamespace, DataType, Field, Float64Array, Int64Array, NamespacePluginHelper,
    RecordBatch, Schema, SchemaRef, StringArray,
};
use probing_core::core::{Float32Array, Int32Array, LazyTableSource};
use probing_proto::types::Ele;
use probing_proto::types::{self, TimeSeries};
use pyo3::types::PyAnyMethods;
use pyo3::types::PyDict;
use pyo3::types::PyDictMethods;
use pyo3::types::PyFloat;
use pyo3::types::PyInt;
use pyo3::types::PyList;
use pyo3::types::PyString;
use pyo3::Bound;
use pyo3::PyAny;
use pyo3::Python;

#[derive(Default, Debug)]
pub struct PythonNamespace {}

impl PythonNamespace {
    fn data_from_python(expr: &str) -> Result<Vec<RecordBatch>> {
        Python::with_gil(|py| {
            let parts: Vec<&str> = expr.split('.').collect();
            if parts.is_empty() {
                return Err(anyhow::anyhow!("Invalid Python expression: {}", expr));
            }

            // Import the package
            let pkg = py.import(parts[0])
                .map_err(|e| anyhow::anyhow!("Failed to import {}: {:?}", parts[0], e))?;

            // Set up locals dict with the imported package
            let locals = PyDict::new(py);
            locals.set_item(parts[0], pkg)
                .map_err(|e| anyhow::anyhow!("Failed to set up Python locals: {:?}", e))?;

            // Evaluate the expression
            let expr = CString::new(expr)
                .map_err(|e| anyhow::anyhow!("Failed to convert expression to CString: {:?}", e))?;
            
            let result = py.eval(&expr, None, Some(&locals))
                .map_err(|e| anyhow::anyhow!("Failed to evaluate Python expression: {:?}", e))?;

            // Handle different Python types
            if let Ok(list) = result.downcast::<PyList>() {
                return Self::list_to_recordbatch(list);
            }
            
            if let Ok(dict) = result.downcast::<PyDict>() {
                return Self::dict_to_recordbatch(dict);
            }
            
            // Handle other Python objects
            Self::object_to_recordbatch(result)
        })
    }

    fn data_from_extern(expr: &str) -> Result<Vec<RecordBatch>> {
        let binding = super::exttbls::EXTERN_TABLES.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock EXTERN_TABLES: {:?}", e))?;
            
        let table = binding.get(expr)
            .ok_or_else(|| anyhow::anyhow!("Table '{}' not found", expr))?;
            
        let names = table.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock table: {:?}", e))?
            .names.clone();
            
        let ts = table.lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock table: {:?}", e))?;

        Self::time_series_to_recordbatch(names, &ts)
    }
}

impl CustomNamespace for PythonNamespace {
    fn name() -> &'static str {
        "python"
    }

    fn list() -> Vec<String> {
        super::exttbls::EXTERN_TABLES.lock().map_or_else(
            |e| {
                log::error!("Failed to lock EXTERN_TABLES: {:?}", e);
                vec![]
            },
            |binding| binding.keys().cloned().collect(),
        )
    }

    fn data(expr: &str) -> Vec<RecordBatch> {
        if Self::list().contains(&expr.to_string()) {
            match Self::data_from_extern(expr) {
                Ok(batches) => batches,
                Err(e) => {
                    error!("Error getting data from extern: {:?}", e);
                    vec![]
                }
            }
        } else {
            match Self::data_from_python(expr) {
                Ok(batches) => batches,
                Err(e) => {
                    error!("Error getting data from Python: {:?}", e);
                    vec![]
                }
            }
        }
    }

    fn make_lazy(expr: &str) -> Arc<LazyTableSource> {
        let binding = super::exttbls::EXTERN_TABLES.lock().map_or_else(
            |e| {
                log::error!("Failed to lock EXTERN_TABLES: {:?}", e);
                Default::default()
            },
            |binding| binding.clone(),
        );

        if binding.contains_key(expr) {
            let table = binding.get(expr).unwrap();
            let names = table.lock().unwrap().names.clone();
            let dtypes = table
                .lock()
                .unwrap()
                .cols
                .iter()
                .map(|x| x.dtype())
                .collect::<Vec<_>>();
            let mut fields = vec![Field::new("timestamp", DataType::Int64, true)];

            for (name, dtype) in names.iter().zip(dtypes.iter()) {
                fields.push(Field::new(
                    name,
                    match dtype {
                        types::EleType::I64 => DataType::Int64,
                        types::EleType::F64 => DataType::Float64,
                        types::EleType::I32 => DataType::Int32,
                        types::EleType::F32 => DataType::Float32,
                        _ => DataType::Utf8,
                    },
                    false,
                ));
            }

            let schema = Some(SchemaRef::new(Schema::new(fields)));

            Arc::new(LazyTableSource {
                name: expr.to_string(),
                schema,
                data: Self::data_from_extern(expr).unwrap_or_default(),
            })
        } else {
            let data: Vec<RecordBatch> = Self::data_from_python(expr).unwrap_or_default();
            let schema = if data.is_empty() {
                None
            } else {
                Some(data[0].schema().clone())
            };
            Arc::new(LazyTableSource {
                name: expr.to_string(),
                schema,
                data,
            })
        }
    }
}

impl PythonNamespace {
    pub fn time_series_to_recordbatch(
        names: Vec<String>,
        ts: &TimeSeries,
    ) -> Result<Vec<RecordBatch>> {
        let mut fields: Vec<Field> = vec![];
        let mut columns: Vec<ArrayRef> = vec![];

        fields.push(Field::new("timestamp", DataType::Int64, true));
        names.iter().zip(ts.cols.iter()).for_each(|(name, col)| {
            let data_type = match col.dtype() {
                types::EleType::I64 => DataType::Int64,
                types::EleType::F64 => DataType::Float64,
                types::EleType::I32 => DataType::Int32,
                types::EleType::F32 => DataType::Float32,
                _ => DataType::Utf8,
            };
            fields.push(Field::new(name, data_type, false));
        });

        let length = ts.len();

        let timeseries = ts
            .timestamp
            .iter()
            .take(length)
            .map(|x| match x {
                Ele::I64(x) => x,
                _ => 0,
            })
            .collect::<Vec<_>>();
        columns.push(Arc::new(Int64Array::from(timeseries)));

        for col in ts.cols.iter() {
            let col = match col.dtype() {
                types::EleType::I64 => Arc::new(Int64Array::from(
                    col.iter()
                        .take(length)
                        .map(|x| match x {
                            Ele::I64(x) => x,
                            _ => 0,
                        })
                        .collect::<Vec<_>>(),
                )) as ArrayRef,
                types::EleType::F64 => Arc::new(Float64Array::from(
                    col.iter()
                        .take(length)
                        .map(|x| match x {
                            Ele::F64(x) => x,
                            _ => 0.0,
                        })
                        .collect::<Vec<_>>(),
                )) as ArrayRef,
                types::EleType::I32 => Arc::new(Int32Array::from(
                    col.iter()
                        .take(length)
                        .map(|x| match x {
                            Ele::I32(x) => x,
                            _ => 0,
                        })
                        .collect::<Vec<_>>(),
                )) as ArrayRef,
                types::EleType::F32 => Arc::new(Float32Array::from(
                    col.iter()
                        .take(length)
                        .map(|x| match x {
                            Ele::F32(x) => x,
                            _ => 0.0,
                        })
                        .collect::<Vec<_>>(),
                )) as ArrayRef,
                types::EleType::Text => Arc::new(StringArray::from(
                    col.iter()
                        .take(length)
                        .map(|x| match x {
                            Ele::Text(x) => x,
                            _ => x.to_string(),
                        })
                        .collect::<Vec<_>>(),
                )) as ArrayRef,
                _ => Arc::new(StringArray::from(
                    col.iter()
                        .take(length)
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>(),
                )) as ArrayRef,
            };

            columns.push(col);
        }

        Ok(vec![RecordBatch::try_new(
            SchemaRef::new(Schema::new(fields)),
            columns,
        )?])
    }

    pub fn object_to_recordbatch(obj: Bound<'_, PyAny>) -> Result<Vec<RecordBatch>> {
        let mut fields: Vec<Field> = vec![];
        let mut columns: Vec<ArrayRef> = vec![];

        if obj.is_instance_of::<PyDict>() {
            let item = obj.downcast::<PyDict>().unwrap();
            for (key, value) in item.iter() {
                let key_str = key.extract::<String>()?;
                Self::add_field_and_array(&mut fields, &mut columns, key_str, value)?;
            }
        } else if obj.hasattr("_asdict")? {
            // Handle namedtuple or any object with _asdict method
            let dict = obj.call_method0("_asdict")?;
            return Self::object_to_recordbatch(dict);
        } else {
            // Handle primitive types or fallback to string representation
            let field_name = "value";
            Self::add_field_and_array(&mut fields, &mut columns, field_name.to_string(), obj)?;
        }

        let schema = SchemaRef::new(Schema::new(fields));
        let batches = vec![RecordBatch::try_new(schema, columns)?];

        Ok(batches)
    }

    // Helper function to handle Python value conversion and add appropriate field
    fn add_field_and_array(
        fields: &mut Vec<Field>, 
        columns: &mut Vec<ArrayRef>,
        name: String, 
        value: Bound<'_, PyAny>
    ) -> Result<()> {
        if value.is_instance_of::<PyInt>() {
            let array = Int64Array::from(vec![value.extract::<i64>()?]);
            columns.push(Arc::new(array));
            fields.push(Field::new(name, DataType::Int64, true));
        } else if value.is_instance_of::<PyFloat>() {
            let array = Float64Array::from(vec![value.extract::<f64>()?]);
            columns.push(Arc::new(array));
            fields.push(Field::new(name, DataType::Float64, true));
        } else if value.is_instance_of::<PyString>() {
            let array = StringArray::from(vec![value.extract::<String>()?]);
            columns.push(Arc::new(array));
            fields.push(Field::new(name, DataType::Utf8, true));
        } else {
            let array = StringArray::from(vec![value.to_string()]);
            columns.push(Arc::new(array));
            fields.push(Field::new(name, DataType::Utf8, true));
        }
        Ok(())
    }

    pub fn dict_to_recordbatch(dict: &Bound<'_, PyDict>) -> Result<Vec<RecordBatch>> {
        let mut fields: Vec<Field> = vec![];
        let mut columns: Vec<ArrayRef> = vec![];

        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            Self::add_field_and_array(&mut fields, &mut columns, key_str, value)?;
        }

        let schema = SchemaRef::new(Schema::new(fields));
        let batches = vec![RecordBatch::try_new(schema, columns)?];

        Ok(batches)
    }

    pub fn list_to_recordbatch(list: &Bound<'_, PyList>) -> Result<Vec<RecordBatch>> {
        let mut names: Vec<String> = vec![];
        let mut datas: HashMap<String, Vec<Option<Bound<'_, PyAny>>>> = Default::default();

        for (index, item) in list.try_iter()?.enumerate() {
            let item = item?;
            let item = if let Ok(dict) = item.downcast::<PyDict>() {
                Some(dict.clone())
            } else {
                match item.getattr("__dict__") {
                    Ok(dict) => Some(dict.downcast::<PyDict>().unwrap().clone()),
                    Err(_) => {
                        let dict = PyDict::new(item.py());
                        dict.set_item("value", item).unwrap();
                        Some(dict)
                    }
                }
            };
            if let Some(ref item) = item {
                for (key, _) in item.iter() {
                    let key_str = key.extract::<String>()?;
                    if !datas.contains_key(&key_str) {
                        names.push(key_str.clone());
                        let value = vec![None; index];
                        datas.insert(key_str.clone(), value);
                    }
                }
            }

            for k in names.iter() {
                if let Some(item) = &item {
                    match item.get_item(k) {
                        Ok(value) => {
                            datas.entry(k.clone()).and_modify(|v| v.push(value));
                        }
                        Err(_) => {
                            datas.entry(k.clone()).and_modify(|v| v.push(None));
                        }
                    }
                } else {
                    datas.entry(k.clone()).and_modify(|v| v.push(None));
                }
            }
        }

        let mut fields: Vec<Field> = vec![];
        let mut columns: Vec<ArrayRef> = vec![];

        for name in names.iter() {
            let values = datas.get(name).unwrap();
            let array = StringArray::from(
                values
                    .iter()
                    .map(|x| {
                        if let Some(x) = x {
                            match x.extract::<String>() {
                                Ok(val) => Some(val),
                                Err(_) => Some(x.to_string()),
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>(),
            );
            columns.push(Arc::new(array));
            fields.push(Field::new(name, DataType::Utf8, true));
        }

        let schema = SchemaRef::new(Schema::new(fields));
        let batches = vec![RecordBatch::try_new(schema, columns).unwrap()];

        Ok(batches)
    }
}

pub type PythonPlugin = NamespacePluginHelper<PythonNamespace>;
