import argparse
import random

def gen_prologue(op, n_1, n_2):
    print("    start_1:int = const {};".format(n_1))
    print("    start_2:int = const {};".format(n_2))
    print("    val_1:int = {} start_1 start_2;".format(op))

def gen_op(op, val, i):
    print("    c_{}:int = const {};".format(i, val))
    print("    val_{}:int = {} val_{} c_{};".format(i, op, i-1, i))

def gen_epilogue(i):
    print("    print val_{};".format(i))

def print_bril(op, n):
    print("\n")
    print("main {")
    n_1 = random.randint(1, 5)
    n_2 = random.randint(1, 5)
    gen_prologue(op, n_1, n_2)
    for i in range(2, int(n)+2):
        n = random.randint(1, 5)
        gen_op(op, n, i)
    gen_epilogue(i)
    print("}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument('--op', help='opcode')
    parser.add_argument('--n', help='number of ops')

    args = parser.parse_args()
    print_bril(args.op, args.n)