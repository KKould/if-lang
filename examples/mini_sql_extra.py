def list_len(args, ctx):
    if len(args) != 1:
        raise Exception("list_len expects 1 arg")
    return len(args[0])


def list_get(args, ctx):
    if len(args) != 2:
        raise Exception("list_get expects 2 args")
    items = list(args[0])
    idx = int(args[1])
    if idx < 0 or idx >= len(items):
        raise Exception("list_get index out of bounds")
    return items[idx]


def list_push(args, ctx):
    if len(args) != 2:
        raise Exception("list_push expects 2 args")
    items = list(args[0])
    items.append(args[1])
    return items


def str_split_ws(args, ctx):
    if len(args) != 1:
        raise Exception("str_split_ws expects 1 arg")
    return str(args[0]).split()


def str_trim_end_char(args, ctx):
    if len(args) != 2:
        raise Exception("str_trim_end_char expects 2 args")
    value = str(args[0])
    suffix = str(args[1])
    if len(suffix) != 1:
        raise Exception("str_trim_end_char expects 1-char suffix")
    if value.endswith(suffix):
        return value[: -1]
    return value


def str_to_int(args, ctx):
    if len(args) != 1:
        raise Exception("str_to_int expects 1 arg")
    try:
        return int(str(args[0]))
    except Exception:
        raise Exception("str_to_int invalid Int")


def assert_eq(args, ctx):
    if len(args) != 2:
        raise Exception("assert_eq expects 2 args")
    left = args[0]
    right = args[1]
    if left == right:
        return True
    raise Exception(f"assert_eq failed: left={left} right={right}")


def if_lang_register(registry):
    registry["list_len"] = list_len
    registry["list_get"] = list_get
    registry["list_push"] = list_push
    registry["str_split_ws"] = str_split_ws
    registry["str_trim_end_char"] = str_trim_end_char
    registry["str_to_int"] = str_to_int
    registry["assert_eq"] = assert_eq
