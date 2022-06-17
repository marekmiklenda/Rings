This is the original compiler and interpreter for the esolang Rings.

The entire interpreter is stored in a module named 'rings', with main.rs containing a simple cli
implementation.

The default implementation uses stdin as input.

```
usage:
compile [-h] SOURCE OUTPUT
run [-h] PROGRAM

optional arguments:
-h, --help       show this help message and exit

positional arguments:
SOURCE           path to a .hrn file to compile
OUTPUT           path to put the resulting .rn file
PROGRAM          path to a .rn file to execute
```
