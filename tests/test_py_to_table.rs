use probing_engine::core::DataType;
use probing_python::plugins::python::PythonSchema;
use pyo3::ffi::c_str;
use pyo3::Python;

use arrow::csv::Writer;

#[test]
fn test_int_to_table() {
    let rb = Python::with_gil(|py| {
        let value = py.eval(c_str!("1"), None, None).unwrap();
        PythonSchema::object_to_recordbatch(value).unwrap()
    });

    assert_eq!(*rb[0].schema().fields()[0].data_type(), DataType::Int64);

    let mut buf = Vec::new();
    Writer::new(&mut buf).write(&rb[0]).unwrap();
    assert_eq!("value\n1\n", String::from_utf8(buf).unwrap());
}

#[test]
fn test_float_to_table() {
    let rb = Python::with_gil(|py| {
        let value = py.eval(c_str!("2.0"), None, None).unwrap();
        PythonSchema::object_to_recordbatch(value).unwrap()
    });

    assert_eq!(*rb[0].schema().fields()[0].data_type(), DataType::Float64);

    let mut buf = Vec::new();
    Writer::new(&mut buf).write(&rb[0]).unwrap();
    assert_eq!("value\n2.0\n", String::from_utf8(buf).unwrap());
}

#[test]
fn test_string_to_table() {
    let rb = Python::with_gil(|py| {
        let value = py.eval(c_str!("'str'"), None, None).unwrap();
        PythonSchema::object_to_recordbatch(value).unwrap()
    });

    assert_eq!(*rb[0].schema().fields()[0].data_type(), DataType::Utf8);

    let mut buf = Vec::new();
    Writer::new(&mut buf).write(&rb[0]).unwrap();
    assert_eq!("value\nstr\n", String::from_utf8(buf).unwrap());
}

#[test]
fn test_dict_to_table() {
    let rb = Python::with_gil(|py| {
        let value = py.eval(c_str!("{'a':1, 'b':2}"), None, None).unwrap();
        PythonSchema::object_to_recordbatch(value).unwrap()
    });

    assert_eq!(*rb[0].schema().fields()[0].name(), "a");
    assert_eq!(*rb[0].schema().fields()[0].data_type(), DataType::Int64);

    let mut buf = Vec::new();
    Writer::new(&mut buf).write(&rb[0]).unwrap();
    assert_eq!("a,b\n1,2\n", String::from_utf8(buf).unwrap());
}

#[test]
fn test_object_to_table() {
    let rb = Python::with_gil(|py| {
        let value = py.eval(c_str!("lambda x: x*2"), None, None).unwrap();
        PythonSchema::object_to_recordbatch(value).unwrap()
    });

    assert_eq!(*rb[0].schema().fields()[0].data_type(), DataType::Utf8);

    let mut buf = Vec::new();
    Writer::new(&mut buf).write(&rb[0]).unwrap();
    assert!(String::from_utf8(buf)
        .unwrap()
        .starts_with("value\n<function <lambda> at "));
}
