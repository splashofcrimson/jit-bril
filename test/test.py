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

def interpreter(json:
    os.chdir("../../")
    p1 = subprocess.Popen(["cargo", "build"])
if __name__ == "__main__":
    os.chdir("../unit")
    for file in glob.glob("*.json"):
        baseline(file)