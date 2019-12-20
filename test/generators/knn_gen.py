import random
import argparse

n = 10000

x = [random.randint(0, 100) for i in range(n)]
y = [random.randint(0, 100) for i in range(n)]
label = [random.randint(0, 1) for i in range(n)]

def gen_train_set(samples):
    for i in range(samples):
        print("    x_{}: int = const {};".format(i, x[i]))
        print("    y_{}: int = const {};".format(i, y[i]))
        print("    l_{}: int = const {};".format(i, label[i]))

def gen_test_set(test_samples):
    for i in range(test_samples):
        val_x = random.randint(0, 100)
        val_y = random.randint(0, 100)
        labelt = random.randint(0, 1)
        print("    x_t_{}: int = const {};".format(i, val_x))
        print("    y_t_{}: int = const {};".format(i, val_y))
        print("    l_t_{}: int = const {};".format(i, labelt))

def gen_main(n):
    manhattan_distance(n)
    check()
    print("main {")
    print("    correct: int = const 0;")
    print("    total: int = const {};".format(n))
    print("    one: int = const 1;")
    gen_test_set(n)
    for i in range(n):
        print("    pred_t_{}: int = call manhattan x_t_{} y_t_{};".format(i, i, i))
        print("    correct_or_not: int = call check pred_t_{} l_t_{};".format(i, i))
        print("    correct: int = add correct correct_or_not;")
    print("    print correct;")
    print("    print total;")
    print("}")

def check():
    print("check (pred: int) (l: int) : int {")
    print("    one: int = const 1;")
    print("    zero: int = const 0;")
    print("    equal: bool = eq pred l;")
    print("    br equal yes no;")
    print("    yes:")
    print("        ret one;")
    print("    no:")
    print("        ret zero;")
    print("}")
    print("\n")

def manhattan_distance(samples):
    print("manhattan (x_t: int) (y_t: int) : int {")
    print("    min: int = const 300;")
    print("    min_label: int = const 0;")
    print("    zero: int = const 0;")
    gen_train_set(samples)
    for i in range(samples):
        if True:
            print("        xdiff: int = sub x_{} x_t;".format(i))
            print("        x2diff: int = sub x_t x_{};".format(i))
            print("        ydiff: int = sub y_{} y_t;".format(i))
            print("        y2diff: int = sub y_t y_{};".format(i))
            print("        xcheck: bool = le xdiff zero;")
            print("        br xcheck here_{} there_{};".format(i, i))
            print("    here_{}:".format(i))
            print("        x: int = id x2diff;")
            print("        jmp continue_{};".format(i))
            print("    there_{}:".format(i))
            print("        x: int = id xdiff;")
            print("        jmp continue_{};".format(i))
            print("    continue_{}:".format(i))
            print("        ycheck: bool = le ydiff zero;")
            print("        br ycheck here_y_{} there_y_{};".format(i, i))
            print("    here_y_{}:".format(i))
            print("        y: int = id y2diff;")
            print("        jmp continue_y_{};".format(i))
            print("    there_y_{}:".format(i))
            print("        y: int = id ydiff;")
            print("        jmp continue_y_{};".format(i))
            print("    continue_y_{}:".format(i))
            print("        mdist: int = add x y;")
            print("        check_min: bool = le mdist min;")
            print("        br check_min set_{} next_{};".format(i, i))
            print("    set_{}:".format(i))
            print("        min: int = id mdist;")
            print("        min_label: int = id l_{};".format(i))
            print("        jmp next_{};".format(i))
            print("    next_{}:".format(i))
    print("    ret min_label;")
    print("}")
    print("\n")

gen_main(500)