#![allow(unsafe_op_in_unsafe_fn)]

use crate::eval::{BuiltinContext, BuiltinFn, EvalError, Value, ValueKey, register_builtin};
use pyo3::exceptions::{PyRuntimeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{
    PyAny, PyBool, PyByteArray, PyBytes, PyDict, PyInt, PyList, PyModule, PyString, PyTuple,
};
use pyo3::DowncastError;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct PyExtraHandle {
    _extra: Arc<PyExtra>,
}

struct PyExtra {
    funcs: HashMap<String, Py<PyAny>>,
}

struct ContextState {
    ctx: *const BuiltinContext<'static>,
    valid: AtomicBool,
}

impl ContextState {
    fn new(ctx: &BuiltinContext) -> Self {
        Self {
            ctx: ctx as *const BuiltinContext as *const BuiltinContext<'static>,
            valid: AtomicBool::new(true),
        }
    }

    fn invalidate(&self) {
        self.valid.store(false, Ordering::SeqCst);
    }

    fn with_ctx<F, R>(&self, f: F) -> Result<R, EvalError>
    where
        F: FnOnce(&BuiltinContext) -> Result<R, EvalError>,
    {
        if !self.valid.load(Ordering::SeqCst) {
            return Err(EvalError::new("python context expired"));
        }
        let ctx = unsafe { &*self.ctx };
        f(ctx)
    }
}

#[pyclass(unsendable)]
struct PyContext {
    state: Arc<ContextState>,
}

#[pymethods]
impl PyContext {
    fn call_fn(&self, py: Python, name: &str, args: Vec<PyObject>) -> PyResult<PyObject> {
        let values = args
            .into_iter()
            .map(|arg| py_to_value(arg.bind(py).as_any()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(eval_error_to_py)?;
        let result = self
            .state
            .with_ctx(|ctx| ctx.call_fn(name, values))
            .map_err(eval_error_to_py)?;
        value_to_py(py, &result, &self.state)
    }

    fn variant(&self, py: Python, name: &str, fields: &Bound<'_, PyDict>) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("__variant__", name)?;
        dict.set_item("fields", fields)?;
        Ok(dict.to_object(py))
    }
}

#[pyclass(unsendable)]
struct PyFnRef {
    name: String,
    state: Arc<ContextState>,
}

#[pymethods]
impl PyFnRef {
    #[pyo3(signature = (*args))]
    fn __call__(&self, py: Python, args: &Bound<'_, PyTuple>) -> PyResult<PyObject> {
        let mut values = Vec::with_capacity(args.len());
        for item in args.iter() {
            values.push(py_to_value(&item).map_err(eval_error_to_py)?);
        }
        let result = self
            .state
            .with_ctx(|ctx| ctx.call_fn(&self.name, values))
            .map_err(eval_error_to_py)?;
        value_to_py(py, &result, &self.state)
    }

    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    fn __repr__(&self) -> String {
        format!("FnRef({})", self.name)
    }
}

pub fn is_python_source(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("py"))
        .unwrap_or(false)
}

pub fn load_python_extra(
    path: &Path,
    builtins: &mut HashMap<String, BuiltinFn>,
) -> io::Result<PyExtraHandle> {
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("extra source not found: {}", path.display()),
        ));
    }
    let source = fs::read_to_string(path)?;
    pyo3::prepare_freethreaded_python();
    let funcs = Python::with_gil(|py| -> PyResult<HashMap<String, Py<PyAny>>> {
        let module_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("if_lang_extra");
        let filename = path.to_str().unwrap_or("<extra>");
        let module = PyModule::from_code_bound(py, &source, filename, module_name)?;
        let registry = PyDict::new_bound(py);
        let register = module.getattr("if_lang_register")?;
        if !register.is_callable() {
            return Err(PyErr::new::<PyTypeError, _>(
                "if_lang_register must be callable",
            ));
        }
        register.call1((registry.clone(),))?;
        let mut funcs = HashMap::new();
        for (key, value) in registry.iter() {
            let name: String = key.extract()?;
            if !value.is_callable() {
                return Err(PyErr::new::<PyTypeError, _>(format!(
                    "builtin '{}' is not callable",
                    name
                )));
            }
            funcs.insert(name, value.into());
        }
        Ok(funcs)
    })
    .map_err(|err| io::Error::other(format!("python extra error: {err}")))?;

    let extra = Arc::new(PyExtra { funcs });
    for (name, func) in extra.funcs.iter() {
        let func = func.clone();
        let keepalive = extra.clone();
        register_builtin(
            builtins,
            name.clone(),
            Arc::new(move |args, ctx| {
                let _keepalive = &keepalive;
                call_python_builtin(&func, args, ctx)
            }),
        );
    }

    Ok(PyExtraHandle { _extra: extra })
}

fn call_python_builtin(
    func: &Py<PyAny>,
    args: &[Value],
    ctx: &BuiltinContext,
) -> Result<Value, EvalError> {
    let state = Arc::new(ContextState::new(ctx));
    let result = Python::with_gil(|py| -> Result<Value, EvalError> {
        let py_ctx = Py::new(py, PyContext { state: state.clone() })
            .map_err(|err| EvalError::new(format!("python context error: {err}")))?;
        let mut py_args = Vec::with_capacity(args.len());
        for arg in args {
            let obj = value_to_py(py, arg, &state)
                .map_err(|err| EvalError::new(format!("python arg error: {err}")))?;
            py_args.push(obj);
        }
        let py_args_list = PyList::new_bound(py, py_args);
        let result_obj = func
            .bind(py)
            .call1((py_args_list, py_ctx))
            .map_err(|err| EvalError::new(format!("python error: {err}")))?;
        py_to_value(&result_obj)
    });
    state.invalidate();
    result
}

fn value_to_py(py: Python, value: &Value, state: &Arc<ContextState>) -> PyResult<PyObject> {
    match value {
        Value::Int(v) => Ok(v.to_object(py)),
        Value::Bool(v) => Ok(v.to_object(py)),
        Value::Str(v) => Ok(v.to_object(py)),
        Value::Bytes(v) => Ok(PyBytes::new_bound(py, v).to_object(py)),
        Value::FnRef(name) => {
            let obj = Py::new(
                py,
                PyFnRef {
                    name: name.clone(),
                    state: state.clone(),
                },
            )?;
            Ok(obj.to_object(py))
        }
        Value::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(value_to_py(py, item, state)?);
            }
            Ok(PyList::new_bound(py, out).to_object(py))
        }
        Value::Map(entries) => {
            let dict = PyDict::new_bound(py);
            for (key, value) in entries {
                let key_obj = match key {
                    ValueKey::Int(v) => v.to_object(py),
                    ValueKey::Bool(v) => v.to_object(py),
                    ValueKey::Str(v) => v.to_object(py),
                    ValueKey::Bytes(v) => PyBytes::new_bound(py, v).to_object(py),
                };
                let value_obj = value_to_py(py, value, state)?;
                dict.set_item(key_obj, value_obj)?;
            }
            Ok(dict.to_object(py))
        }
        Value::Variant { name, fields } => {
            let dict = PyDict::new_bound(py);
            dict.set_item("__variant__", name)?;
            let field_dict = PyDict::new_bound(py);
            for (field, value) in fields {
                field_dict.set_item(field, value_to_py(py, value, state)?)?;
            }
            dict.set_item("fields", field_dict)?;
            Ok(dict.to_object(py))
        }
    }
}

fn py_to_value(obj: &Bound<'_, PyAny>) -> Result<Value, EvalError> {
    if obj.is_none() {
        return Err(EvalError::new("python returned None"));
    }
    if obj.is_instance_of::<PyFnRef>() {
        let fn_ref: PyRef<PyFnRef> = obj.extract().map_err(py_extract_err)?;
        return Ok(Value::FnRef(fn_ref.name.clone()));
    }
    if obj.is_instance_of::<PyBool>() {
        let value = obj.extract::<bool>().map_err(py_extract_err)?;
        return Ok(Value::Bool(value));
    }
    if obj.is_instance_of::<PyInt>() {
        let value = obj.extract::<i64>().map_err(py_extract_err)?;
        return Ok(Value::Int(value));
    }
    if obj.is_instance_of::<PyString>() {
        let value = obj.extract::<String>().map_err(py_extract_err)?;
        return Ok(Value::Str(value));
    }
    if obj.is_instance_of::<PyBytes>() {
        let value = obj
            .downcast::<PyBytes>()
            .map_err(py_downcast_err)?
            .as_bytes()
            .to_vec();
        return Ok(Value::Bytes(value));
    }
    if obj.is_instance_of::<PyByteArray>() {
        let value = obj
            .downcast::<PyByteArray>()
            .map_err(py_downcast_err)?
            .to_vec();
        return Ok(Value::Bytes(value));
    }
    if obj.is_instance_of::<PyList>() {
        let list = obj.downcast::<PyList>().map_err(py_downcast_err)?;
        let mut out = Vec::with_capacity(list.len());
        for item in list.iter() {
            out.push(py_to_value(&item)?);
        }
        return Ok(Value::List(out));
    }
    if obj.is_instance_of::<PyTuple>() {
        let list = obj.downcast::<PyTuple>().map_err(py_downcast_err)?;
        let mut out = Vec::with_capacity(list.len());
        for item in list.iter() {
            out.push(py_to_value(&item)?);
        }
        return Ok(Value::List(out));
    }
    if obj.is_instance_of::<PyDict>() {
        let dict = obj.downcast::<PyDict>().map_err(py_downcast_err)?;
        let name_any = dict.get_item("__variant__").map_err(py_extract_err)?;
        let fields_any = dict.get_item("fields").map_err(py_extract_err)?;
        if let (Some(name_any), Some(fields_any)) = (name_any, fields_any) {
            let name = name_any.extract::<String>().map_err(py_extract_err)?;
            let fields_dict = fields_any
                .downcast::<PyDict>()
                .map_err(|_| EvalError::new("variant fields must be dict"))?;
            let mut fields = BTreeMap::new();
            for (key, value) in fields_dict.iter() {
                let field = key
                    .extract::<String>()
                    .map_err(|_| EvalError::new("variant field names must be Str"))?;
                fields.insert(field, py_to_value(&value)?);
            }
            return Ok(Value::Variant { name, fields });
        }
        if let Some(name_any) = dict.get_item("__fn__").map_err(py_extract_err)? {
            let name = name_any.extract::<String>().map_err(py_extract_err)?;
            return Ok(Value::FnRef(name));
        }
        let mut entries = BTreeMap::new();
        for (key, value) in dict.iter() {
            let map_key = py_to_value_key(&key)?;
            let map_value = py_to_value(&value)?;
            entries.insert(map_key, map_value);
        }
        return Ok(Value::Map(entries));
    }
    Err(EvalError::new("unsupported python value"))
}

fn py_to_value_key(obj: &Bound<'_, PyAny>) -> Result<ValueKey, EvalError> {
    if obj.is_instance_of::<PyBool>() {
        let value = obj.extract::<bool>().map_err(py_extract_err)?;
        return Ok(ValueKey::Bool(value));
    }
    if obj.is_instance_of::<PyInt>() {
        let value = obj.extract::<i64>().map_err(py_extract_err)?;
        return Ok(ValueKey::Int(value));
    }
    if obj.is_instance_of::<PyString>() {
        let value = obj.extract::<String>().map_err(py_extract_err)?;
        return Ok(ValueKey::Str(value));
    }
    if obj.is_instance_of::<PyBytes>() {
        let value = obj
            .downcast::<PyBytes>()
            .map_err(py_downcast_err)?
            .as_bytes()
            .to_vec();
        return Ok(ValueKey::Bytes(value));
    }
    if obj.is_instance_of::<PyByteArray>() {
        let value = obj
            .downcast::<PyByteArray>()
            .map_err(py_downcast_err)?
            .to_vec();
        return Ok(ValueKey::Bytes(value));
    }
    Err(EvalError::new(
        "map keys must be Int, Bool, Str, or Bytes",
    ))
}

fn eval_error_to_py(err: EvalError) -> PyErr {
    PyErr::new::<PyRuntimeError, _>(err.message)
}

fn py_extract_err(err: PyErr) -> EvalError {
    EvalError::new(format!("python conversion error: {err}"))
}

fn py_downcast_err(err: DowncastError<'_, '_>) -> EvalError {
    EvalError::new(format!("python conversion error: {err}"))
}
