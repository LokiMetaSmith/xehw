
## Intro

This tutorial is focused on how to use XEH playground
for parsing and exploration of a binary data.


## How to import data

Click File -> Open ... -> Browse..., select file, click OK
Or open file explorer, drag and drop file into the window.

## HEX panel

Hex Panel display the current input binary.
The first column is offset from beginning of the binary.
The second is data and the third is the text representation of each byte.
Header on the top display current read position and total data size.
Scroll data with mouse, click, hold and move.
Or using the keyboard arrows:
    Up/Down - scroll one row
    PageUp/PageDown - scroll one page
Scroll to exact offset using "Go To" 
    Click View -> Go To ...
Goto text is evaluated using fresh XEH interpreter.
Then result is used as the new offset.
Please note that "Go To" operate in bits, not in bytes.

## REPL

By default REPL starts in immutable TRIAL mode.
First let switch to the traditional REPL mode in the menu.
For example, put a numer 8 on top of the stack.
Type the number 8 in the REPL editor, click run.
Status line say OK, evaluation take ? point ? seconds.
Result is frozen.
Stack panel display the current stack state.
Now we want multiply 8 by 32, but  lets make a missprint and see whats happen.
    32 **
Status line display the error message: undefined word **.
Dashline point to the error location.
Aslo note that expression is partially evaluated.
Integers already on the stack but multiplication is interrupted.
Lets make a correction: type * click run.
Thats it, type - run - repeat.

## Program State.
## TRIAL
## Reverse Debugging
