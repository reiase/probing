use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;

use probing_engine::core::{
    ArrayRef, CustomSchema, DataType, Field, Float64Array, Int64Array, RecordBatch, Schema,
    SchemaPlugin, SchemaRef, StringArray,
};
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
pub struct PythonSchema {}

impl CustomSchema for PythonSchema {
    fn name() -> &'static str {
        "python"
    }

    fn list() -> Vec<String> {
        vec![]
    }

    fn data(expr: &str) -> Vec<RecordBatch> {
        Python::with_gil(|py| {
            let parts: Vec<&str> = expr.split(".").collect();
            let pkg = py.import_bound(parts[0]);

            if pkg.is_err() {
                println!("import error: {:?}", pkg.err());
                return vec![];
            }
            let pkg = pkg.unwrap();

            let locals = PyDict::new_bound(py);
            locals.set_item(parts[0], pkg).unwrap();

            let ret = py.eval_bound(expr, None, Some(&locals));
            if ret.is_err() {
                println!("eval error: {:?}", ret.err());
                return vec![];
            }

            let ret = ret.unwrap();
            if ret.is_instance_of::<PyList>() {
                println!("list: {ret}");
                if let Ok(_list) = ret.downcast::<PyList>() {
                    return vec![];
                }
                return vec![];
            }

            if ret.is_instance_of::<PyDict>() {
                println!("dict: {ret}");
                if let Ok(_dict) = ret.downcast::<PyDict>() {
                    return vec![];
                }
                return vec![];
            }
            println!("object: {ret}");
            Self::object_to_recordbatch(ret).unwrap()
        })
    }
}

impl PythonSchema {
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
            let array = StringArray::from(vec![obj.to_string()]);
            columns.push(Arc::new(array));
            fields.push(Field::new("value", DataType::Utf8, true));
        }

        let schema = SchemaRef::new(Schema::new(fields));
        let batches = vec![RecordBatch::try_new(schema, columns).unwrap()];

        Ok(batches)
    }

    pub fn list_to_recordbatch(&self, list: Bound<'_, PyList>) -> Result<Vec<RecordBatch>> {
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

pub type PythonPlugin = SchemaPlugin<PythonSchema>;
