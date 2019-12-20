import random
import argparse

n = 100

a_vars = [["a_{}_{}".format(i,j) for j in range(n)] for i in range(n)]
b_vars = [["b_{}_{}".format(i,j) for j in range(n)] for i in range(n)]
c_vars = [["c_{}_{}".format(i,j) for j in range(n)] for i in range(n)]

funcs = ["f_{}".format(i) for i in range(n)]

def write_fib(n_1):
  print("    n:int = const {};".format(n_1))
  print("    a:int = const 0;")
  print("    b:int = const 1;")
  print("    i:int = const 1;")
  print("    one:int = const 1;")

  print("    start:")
  print("    cmp:bool = le i n;")
  print("    br cmp here there;")
  print("    here:")
  print("    c:int = add a b;")
  print("    a:int = id b;")
  print("    b:int = id c;")
  print("    i:int = add i one;")
  print("    jmp start;")
  print("    there:")
  print("    print b;")
  print("    ret;")

def print_vars(vs):
    for row in vs:
        for v in row:
            print("print {};".format(v))

def create_func():
  for i in funcs:
    print("{} {{".format(i))
    write_fib(random.randint(100000, 200000))
    print("}")

create_func()
print("\n")
print("main {")
for func in funcs:
  print("    call {};".format(func))
print("}")