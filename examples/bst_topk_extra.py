def append(args, ctx):
    if len(args) != 2:
        raise Exception("append expects 2 args")
    return list(args[0]) + list(args[1])


def take_k(args, ctx):
    if len(args) != 2:
        raise Exception("take_k expects 2 args")
    xs = list(args[0])
    k = args[1]
    if k < 0:
        k = 0
    return xs[: int(k)]


def if_lang_register(registry):
    registry["append"] = append
    registry["take_k"] = take_k
