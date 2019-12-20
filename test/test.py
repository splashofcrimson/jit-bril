import argparse
import glob, os
import subprocess

def baseline(json):
    p1 = subprocess.Popen(["cat", json], stdout=subprocess.PIPE)
    p2 = subprocess.Popen(["brili"], stdin=p1.stdout, stdout=subprocess.PIPE)
    p1.stdout.close()
    output, err = p2.communicate()
    f = open("{}.txt".format(os.path.splitext(json)[0]), "w+")
    f.write("{}".format(int(output)))
    f.close()

def interpreter(json):
    file_name = os.path.splitext(json)[0]
    print("test {} ...".format(file_name), end=" ")
    p1 = subprocess.Popen(["cat", json], stdout=subprocess.PIPE)
    p2 = subprocess.Popen(["../target/release/jit-bril", "interp"], stdin=p1.stdout, stdout=subprocess.PIPE)
    p1.stdout.close()
    output, err = p2.communicate()
    baseline = open("{}.txt".format(file_name), 'r')
    baseline_values = baseline.read()
    interpreter_output = str(output).split("\\n")[1].strip()
    if (str(baseline_values) == interpreter_output):
        print("ok")
    else:
        print("FAILED. Expected {}, Got {}".format(baseline_values, interpreter_output))

def jit(json):
    file_name = os.path.splitext(json)[0]
    print("test {} ...".format(file_name), end=" ")
    p1 = subprocess.Popen(["cat", json], stdout=subprocess.PIPE)
    p2 = subprocess.Popen(["../target/release/jit-bril", "jit"], stdin=p1.stdout, stdout=subprocess.PIPE)
    p1.stdout.close()
    output, err = p2.communicate()
    baseline = open("{}.txt".format(file_name), 'r')
    baseline_values = baseline.read()
    interpreter_output = str(output).split("\\n")[1].strip()
    if (str(baseline_values) == interpreter_output):
        print("ok")
    else:
        print("FAILED. Expected {}, Got {}".format(baseline_values, interpreter_output))

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument('--mode', help='mode to test, interp or jit', required=True)
    args = parser.parse_args()
    mode = args.mode
    print("running {} tests".format(len(glob.glob("./unit/*.json"))))
    for file in glob.glob("./unit/*.json"):
        if mode == "interp":
            interpreter(file)
        elif mode == "jit":
            jit(file)