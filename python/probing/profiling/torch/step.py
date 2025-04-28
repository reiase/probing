LAYER_ITER = 0
OPTIM_ITER = 1


def reset_layer(cnt=None):
    global LAYER_ITER
    cnt = 0 if cnt is None else cnt
    LAYER_ITER = cnt


def next_layer():
    global LAYER_ITER
    LAYER_ITER += 1


def reset_step(cnt=None):
    global OPTIM_ITER
    cnt = 0 if cnt is None else cnt
    OPTIM_ITER = cnt


def next_step():
    global OPTIM_ITER
    OPTIM_ITER += 1


def step():
    return OPTIM_ITER


def layer():
    return LAYER_ITER
