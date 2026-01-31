use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use if_lang::eval::{BuiltinContext, BuiltinFn, EvalError, Value, register_builtin};

const INTERNAL_ERROR: &str = "HTTP/1.1 500 Internal Server Error\r\nConnection: close\r\n\r\n";

#[unsafe(no_mangle)]
pub extern "C" fn if_lang_register(builtins: *mut HashMap<String, BuiltinFn>) {
    let builtins = unsafe { &mut *builtins };
    register_builtin(builtins, "serve", Arc::new(serve));
    register_builtin(builtins, "str_join", Arc::new(str_join));
    register_builtin(builtins, "str_len", Arc::new(str_len));
    register_builtin(builtins, "int_to_str", Arc::new(int_to_str));
}

fn serve(args: &[Value], ctx: &BuiltinContext) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::new("serve expects 2 args"));
    }
    let port = match &args[0] {
        Value::Int(v) => *v,
        _ => return Err(EvalError::new("serve expects Int")),
    };
    let handler = match &args[1] {
        Value::FnRef(value) => value.clone(),
        _ => return Err(EvalError::new("serve expects function handler")),
    };
    if port <= 0 || port > u16::MAX as i64 {
        return Err(EvalError::new("serve expects port in 1..=65535"));
    }

    let addr = format!("127.0.0.1:{}", port);
    let listener =
        TcpListener::bind(&addr).map_err(|err| EvalError::new(format!("bind failed: {err}")))?;

    eprintln!("mini web server listening on http://{addr}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = handle_client(stream, ctx, &handler) {
                    eprintln!("request error: {err}");
                }
            }
            Err(err) => eprintln!("accept error: {err}"),
        }
    }

    #[allow(unreachable_code)]
    Ok(Value::Bool(true))
}

fn handle_client(
    mut stream: TcpStream,
    ctx: &BuiltinContext,
    handler: &str,
) -> std::io::Result<()> {
    let mut buffer = [0u8; 4096];
    let read_len = match stream.read(&mut buffer) {
        Ok(0) => return Ok(()),
        Ok(n) => n,
        Err(err) => return Err(err),
    };

    let request = String::from_utf8_lossy(&buffer[..read_len]);
    let mut parts = request.lines().next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    let (path, query) = split_query(path);
    let request = request_value(method, path, query, &request);
    let response_bytes = match ctx.call_fn(handler, vec![request]) {
        Ok(Value::Str(body)) => body.into_bytes(),
        Ok(Value::Bytes(body)) => body,
        Ok(_) => INTERNAL_ERROR.as_bytes().to_vec(),
        Err(_) => INTERNAL_ERROR.as_bytes().to_vec(),
    };

    stream.write_all(&response_bytes)
}

fn split_query(path: &str) -> (&str, &str) {
    let mut parts = path.splitn(2, '?');
    let path = parts.next().unwrap_or(path);
    let query = parts.next().unwrap_or("");
    (path, query)
}

fn request_value(method: &str, path: &str, query: &str, raw: &str) -> Value {
    let mut fields = BTreeMap::new();
    fields.insert("method".to_string(), Value::Str(method.to_string()));
    fields.insert("path".to_string(), Value::Str(path.to_string()));
    fields.insert("query".to_string(), Value::Str(query.to_string()));
    fields.insert("raw".to_string(), Value::Str(raw.to_string()));
    Value::Variant {
        name: "Request".to_string(),
        fields,
    }
}

fn str_join(args: &[Value], _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::new("str_join expects 2 args"));
    }
    let items = match &args[0] {
        Value::List(items) => items,
        _ => return Err(EvalError::new("str_join expects List")),
    };
    let sep = match &args[1] {
        Value::Str(value) => value.as_str(),
        _ => return Err(EvalError::new("str_join expects Str separator")),
    };
    let mut out = String::new();
    for (idx, item) in items.iter().enumerate() {
        let value = match item {
            Value::Str(value) => value.as_str(),
            _ => return Err(EvalError::new("str_join expects List<Str>")),
        };
        if idx > 0 {
            out.push_str(sep);
        }
        out.push_str(value);
    }
    Ok(Value::Str(out))
}

fn str_len(args: &[Value], _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    if args.len() != 1 {
        return Err(EvalError::new("str_len expects 1 arg"));
    }
    let value = match &args[0] {
        Value::Str(value) => value,
        _ => return Err(EvalError::new("str_len expects Str")),
    };
    Ok(Value::Int(value.as_bytes().len() as i64))
}

fn int_to_str(args: &[Value], _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    if args.len() != 1 {
        return Err(EvalError::new("int_to_str expects 1 arg"));
    }
    let value = match &args[0] {
        Value::Int(value) => *value,
        _ => return Err(EvalError::new("int_to_str expects Int")),
    };
    Ok(Value::Str(value.to_string()))
}

fn main() {}
