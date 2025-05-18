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

impl CustomNamespace for PythonNamespace {
    fn name() -> &'static str {
        "python"
    }

    fn list() -> Vec<String> {
        let binding = super::exttbls::EXTERN_TABLES.lock().unwrap();
        binding.keys().cloned().collect()
    }

    fn data(expr: &str) -> Vec<RecordBatch> {
        if Self::list().contains(&expr.to_string()) {
            {
                let binding = super::exttbls::EXTERN_TABLES.lock().unwrap();
                let table = binding.get(expr).unwrap();
                let names = table.lock().unwrap().names.clone();
                let ts = &table.lock().unwrap();

                let batches = Self::time_series_to_recordbatch(names, ts);
                if let Ok(batches) = batches {
                    return batches;
                } else {
                    error!("error convert time series to table: {:?}", batches.err());
                    return vec![];
                }
            }
        }
        Python::with_gil(|py| {
            let parts: Vec<&str> = expr.split(".").collect();
            let pkg = py.import(parts[0]);

            if pkg.is_err() {
                println!("import error: {:?}", pkg.err());
                return vec![];
            }
            let pkg = pkg.unwrap();

            let locals = PyDict::new(py);
            locals.set_item(parts[0], pkg).unwrap();

            let ret = (|| {
                let expr = CString::new(expr)?;
                py.eval(&expr, None, Some(&locals))
            })();

            if ret.is_err() {
                println!("eval error: {:?}", ret.err());
                return vec![];
            }

            let ret = ret.unwrap();
            if ret.is_instance_of::<PyList>() {
                if let Ok(_list) = ret.downcast::<PyList>() {
                    return Self::list_to_recordbatch(_list).unwrap_or(vec![]);
                }
                return vec![];
            }

            if ret.is_instance_of::<PyDict>() {
                if let Ok(_dict) = ret.downcast::<PyDict>() {
                    return Self::dict_to_recordbatch(_dict).unwrap_or(vec![]);
                }
                return vec![];
            }
            Self::object_to_recordbatch(ret).unwrap()
        })
    }

    fn make_lazy(expr: &str) -> Arc<LazyTableSource<Self>> {
        let binding = super::exttbls::EXTERN_TABLES.lock().unwrap();

        let schema = if binding.contains_key(expr) {
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

            Some(SchemaRef::new(Schema::new(fields)))
        } else {
            None
        };

        Arc::new(LazyTableSource::<Self> {
            name: expr.to_string(),
            schema,
            data: Default::default(),
        })
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
                if value.is_instance_of::<PyInt>() {
                    let array = Int64Array::from(vec![value.extract::<i64>()?]);
                    columns.push(Arc::new(array));
                    fields.push(Field::new(key_str, DataType::Int64, true));
                } else if value.is_instance_of::<PyFloat>() {
                    let array = Float64Array::from(vec![value.extract::<f64>()?]);
                    columns.push(Arc::new(array));
                    fields.push(Field::new(key_str, DataType::Float64, true));
                } else if value.is_instance_of::<PyString>() {
                    let array = StringArray::from(vec![value.extract::<String>()?]);
                    columns.push(Arc::new(array));
                    fields.push(Field::new(key_str, DataType::UInt8, true));
                } else {
                    let array = StringArray::from(vec![value.to_string()]);
                    columns.push(Arc::new(array));
                    fields.push(Field::new(key_str, DataType::UInt8, true));
                }
            }
        } else if obj.is_instance_of::<PyInt>() {
            let array = Int64Array::from(vec![obj.extract::<i64>()?]);
            columns.push(Arc::new(array));
            fields.push(Field::new("value", DataType::Int64, true));
        } else if obj.is_instance_of::<PyFloat>() {
            let array = Float64Array::from(vec![obj.extract::<f64>()?]);
            columns.push(Arc::new(array));
            fields.push(Field::new("value", DataType::Float64, true));
        } else if obj.is_instance_of::<PyString>() {
            let array = StringArray::from(vec![obj.extract::<String>()?]);
            columns.push(Arc::new(array));
            fields.push(Field::new("value", DataType::Utf8, true));
        } else {
            if obj.hasattr("_asdict")? {
                let dict = obj.call_method0("_asdict").unwrap();
                return Self::object_to_recordbatch(dict);
            }

            let array = StringArray::from(vec![obj.to_string()]);
            columns.push(Arc::new(array));
            fields.push(Field::new("value", DataType::Utf8, true));
        }

        let schema = SchemaRef::new(Schema::new(fields));
        let batches = vec![RecordBatch::try_new(schema, columns).unwrap()];

        Ok(batches)
    }

    pub fn dict_to_recordbatch(dict: &Bound<'_, PyDict>) -> Result<Vec<RecordBatch>> {
        let mut fields: Vec<Field> = vec![];
        let mut columns: Vec<ArrayRef> = vec![];

        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            if value.is_instance_of::<PyInt>() {
                let array = Int64Array::from(vec![value.extract::<i64>()?]);
                columns.push(Arc::new(array));
                fields.push(Field::new(key_str, DataType::Int64, true));
            } else if value.is_instance_of::<PyFloat>() {
                let array = Float64Array::from(vec![value.extract::<f64>()?]);
                columns.push(Arc::new(array));
                fields.push(Field::new(key_str, DataType::Float64, true));
            } else if value.is_instance_of::<PyString>() {
                let array = StringArray::from(vec![value.extract::<String>()?]);
                columns.push(Arc::new(array));
                fields.push(Field::new(key_str, DataType::Utf8, true));
            } else {
                let array = StringArray::from(vec![value.to_string()]);
                columns.push(Arc::new(array));
                fields.push(Field::new(key_str, DataType::Utf8, true));
            }
        }

        let schema = SchemaRef::new(Schema::new(fields));
        let batches = vec![RecordBatch::try_new(schema, columns).unwrap()];

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
                    Err(_) => None,
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
                            x.extract::<String>().ok()
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
