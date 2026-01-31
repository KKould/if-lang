import socket

INTERNAL_ERROR = b"HTTP/1.1 500 Internal Server Error\r\nConnection: close\r\n\r\n"


def serve(args, ctx):
    if len(args) != 2:
        raise Exception("serve expects 2 args")
    port = args[0]
    handler = args[1]
    if not (1 <= port <= 65535):
        raise Exception("serve expects port in 1..=65535")

    addr = ("127.0.0.1", int(port))
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind(addr)
    server.listen(5)

    print(f"mini web server listening on http://{addr[0]}:{addr[1]}")
    while True:
        conn, _ = server.accept()
        try:
            data = conn.recv(4096)
            if not data:
                conn.close()
                continue
            request_text = data.decode("utf-8", "replace")
            method, path = _parse_request_line(request_text)
            path, query = _split_query(path)
            req = {
                "__variant__": "Request",
                "fields": {
                    "method": method,
                    "path": path,
                    "query": query,
                    "raw": request_text,
                },
            }
            try:
                if callable(handler):
                    response = handler(req)
                else:
                    response = ctx.call_fn(str(handler), [req])
            except Exception:
                response = INTERNAL_ERROR
            if isinstance(response, (bytes, bytearray)):
                conn.sendall(bytes(response))
            else:
                conn.sendall(str(response).encode("utf-8"))
        finally:
            conn.close()


def _parse_request_line(request_text):
    line = ""
    if request_text:
        line = request_text.splitlines()[0]
    parts = line.split()
    method = parts[0] if len(parts) > 0 else ""
    path = parts[1] if len(parts) > 1 else "/"
    return method, path


def _split_query(path):
    if "?" in path:
        left, right = path.split("?", 1)
        return left, right
    return path, ""


def str_join(args, ctx):
    if len(args) != 2:
        raise Exception("str_join expects 2 args")
    parts = args[0]
    sep = args[1]
    return str(sep).join(str(p) for p in parts)


def str_len(args, ctx):
    if len(args) != 1:
        raise Exception("str_len expects 1 arg")
    value = str(args[0])
    return len(value.encode("utf-8"))


def int_to_str(args, ctx):
    if len(args) != 1:
        raise Exception("int_to_str expects 1 arg")
    return str(int(args[0]))


def if_lang_register(registry):
    registry["serve"] = serve
    registry["str_join"] = str_join
    registry["str_len"] = str_len
    registry["int_to_str"] = int_to_str
