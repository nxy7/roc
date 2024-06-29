# Functions

### [Low-level details](#low-level) {#low-level}

At runtime, a capture is represented as a [tuple](tuples#low-level). Captures are calculated based on how they are actually used in the program.

For example, suppose we have a [`List`](List) of functions. What will be the size and alignment of each element in that list? The answer is that it will be based on the largest size and alignment of all the captures that could potentially go in that list in practice. (This also includes taking account what could happen in different conditional branches.)
