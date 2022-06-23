This is the original compiler and interpreter for the esolang [Rings](https://esolangs.org/wiki/Rings), made for the June 2022 Esoland Discord competition. This is my first Rust project, so I hope it won't be too much of an eyesore.

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
