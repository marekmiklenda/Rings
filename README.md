This is the original compiler and interpreter for the esolang Rings.

The entire interpreter/compiler is stored in a module named 'rings', with main.rs containing a simple cli
implementation.

The default implementation uses stdin as input.

```
usage:
compile [-h] SOURCE OUTPUT
run [-h] PROGRAM

optional arguments:
-h, --help       show this help message and exit

positional arguments:
SOURCE           path to a text file to compile
OUTPUT           path to resulting .rn file
PROGRAM          path to .rn file to execute
```
