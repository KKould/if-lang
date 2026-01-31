use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use if_lang::eval::{Args, BuiltinContext, BuiltinFn, EvalError, Value, register_builtin_args};

const INTERNAL_ERROR: &str = "HTTP/1.1 500 Internal Server Error\r\nConnection: close\r\n\r\n";

#[unsafe(no_mangle)]
pub extern "C" fn if_lang_register(builtins: *mut HashMap<String, BuiltinFn>) {
    let builtins = unsafe { &mut *builtins };
    register_builtin_args(builtins, "serve", serve);
    register_builtin_args(builtins, "str_join", str_join);
    register_builtin_args(builtins, "str_len", str_len);
    register_builtin_args(builtins, "int_to_str", int_to_str);
}

fn serve(args: Args<'_>, ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(2)?;
    let port = args.int(0)?;
    let handler = args.fn_ref(1)?.to_string();
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

fn str_join(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(2)?;
    let items = args.list(0)?;
    let sep = args.str(1)?;
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

fn str_len(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(1)?;
    let value = args.str(0)?;
    Ok(Value::Int(value.as_bytes().len() as i64))
}

fn int_to_str(args: Args<'_>, _ctx: &BuiltinContext) -> Result<Value, EvalError> {
    args.expect_len(1)?;
    let value = args.int(0)?;
    Ok(Value::Str(value.to_string()))
}

fn main() {}
