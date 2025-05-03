# Awsm
Awsm can assemble x86-64 assembly code into into an executable file that you can directly run. No bloat, no complex toolchains, just you and the kernel.

Awsm aims to give you low-level control while still maintaining a syntax familiar to high-level language programmers. Here's a minimal "Hello World" example, which Awsm assembles into a 167-byte ELF executable:
```js
static message = "Hello, World!\n"
@syscall(eax = 1, edi = 1, rsi = message, edx = @len(message))
@syscall(eax = 60, edi ^= edi) // EXIT_SUCCESS
```

To build Awsm itself, make sure you have [Rust](https://rustup.rs/) and [Git](https://git-scm.com/) and then run:
```
git clone https://github.com/abgros/awsm
cd awsm
cargo run
```

### Features
- Basic arithmetic and logical operations
- Control flow
- Linux support (ELF)

### Planned (maybe)
- Windows support (PE)
- macOS support (Mach-O)
- SIMD instructions
- Kernel mode instructions
- Float instructions
- Helpful error messages (sorry)
- A standard library of useful functions
- Macros and inline constants
- Compile-time constant evaluation
- Peephole optimizations (for example, convert `rax = 0` to `eax ^= eax`)
- Modules
- Static analysis (for example, issue a warning if a function doesn't return)
- Smarter tokenizer and parser (it's currently a little hacky and whitespace sensitive)

## Manual
This section covers the basic principles of x86 assembly.

The most important difference between assembly and a high-level language like C is that you can't just "declare" a variable. *You have to explicitly put it somewhere*.
### Registers
Registers are essentially "slots" that your CPU can use to hold some data. x86 CPUs have a lot of registers (not all of which are supported in Awsm), but make sure you know these ones: rax, rcx, rdx, rbx, rsp, rbp, rsi, and rdi. Each of these registers can hold 64 bits of data. The CPU doesn't care *what kind* of data, so you can (for example) use a register to store an 8-character ASCII string.

Registers are extremely fast, but have the disadvantage of having a very limited capacity. Also, some operations can "clobber", or mess up, your registers. For example, on Linux, making a syscall results in rax, rcx, and r11 (that's another register) getting clobbered. If you care about the values in those registers, you'll need to save them somewhere.

rsp has a special status because when you start your program, its data happens to be a pointer to the stack.

Here's an example of passing around numbers between registers:
```js
rax = 34
rdx = rax + 5
rdx--
rax = rdx * 4 - 9
rcx = rax + rdx
```

### Stack
When you start up your program, you automatically have access to the stack. You can think of the stack as basically a very large array of bytes (usually it's a couple megabytes).  Most of it is starts off as zero, but it also contains your environment variables and command-line arguments passed to the program (argc, argv). Roughly:

```
<- negative                                   positive ->
B B B B B B B B B B B B B B B B B B B B B B B B B B B B B
                                              ^
                                             rsp points somewhere here
```
Unintuitively, a lower value of rsp actually represents the top of the stack. That's why the OS gives you a lot of room in the negative direction. You can use the stack however you want, but typically each function in your program should have its own "stack frame" that contains local variables. For example, we might use the stack to save a register before calling a function:
```js
function my_function() {
    rsp -= 8 // 8 byte stack frame can fit rax
    *rsp = rax
    another_function() // might clobber rax, but shouldn't mess with *our* stack frame
    rax = *rsp
    rsp += 8
    return
}
```
This pattern of putting stuff on and taking stuff off the stack is so common that there are special x86 operations for this: push and pop. This function can be rewritten as:
```js
function my_function() {
    <- rax // subtracts 8 from rsp, then writes rax to the stack
    another_function()
    -> rax // reads rax off the stack, then adds 8 to rsp
    return
}
```

### Heap
If you need to use lots of memory, the heap is the way to go. Unlike with the stack, you don't get a heap by default: you have to ask the kernel to give you some. On Linux, this is done using the `mmap` syscall.
```js
// rsi: length, pointer is returned in rax
function allocate_memory() {
    // mmap - don't worry about all of these flags
    @syscall(rax = 9, rdi = 0, rdx = 3, r10 = 0x22, r8 = -1, r9 = 0)

    // ensure that an error hasn't occurred
    @set_flags(rax & rax)
    goto fail if /sign
    return

    fail:
    static allocation_error = "memory allocation failed!\n"
    @syscall(rax = 1, rdi = 1, rsi = allocation_error, rdx = @len(allocation_error))
    trap
}
```
If this succeeds, rax contains a pointer to your new memory. You can access it using this cool indexing syntax:
```js
rdx = *rax
rdx = rax[34]
rdx = rax[rcx * 4 - 35]
rdx = rax[rcx >> 2 + 0x5555]
rdx = rax[-1] // allowed but will segfault!

// Invalid:
rdx = rax[rcx + rbx]
rdx = rax[rcx * 3] // multiplier must be 1, 2, 4, or 8
```
When you're done with the memory, you can free it using the `munmap` syscall:
```js
// rdi: pointer to be freed, rsi: the original length
function free_memory() {
    @syscall(rax = 11) // munmap
    return
}
```
### Static memory
The easiest way to allocate some memory is just to write it directly into the binary itself. This is done using the `static` keyword just like in the Hello World example.
```js
static thing1 = 0x1234
static thing2 = @f32(3.54354)
static thing3 = -5
static thing4 = "hi\n"

// our static variables are mutable!
thing3[u16]++
@negate(thing1[u8])
```
This is useful if you want to create a global variable that can be accessed from anywhere in the problem.

### Types of pointers
You might have noticed this weird indexing syntax like `thing3[u16]`. What's going on there? The key thing to know is that x86 cares about the *type* of a pointer.

```js
rax = rdi // move rdi into rax
rax[u8] = 34 // move 34 into the byte pointed at by rax
rax[u16] = 34 // move 34 into the word (16 bits) pointed at by rax
rax[u32] = 34 // move 34 into the dword (32 bits) pointed at by rax
rax[u64] = 34 // move 34 into the qword (64 bits) pointed at by rax

rax[u8] = rdi // invalid because rdi can't fit in a single byte
rax[u64] = rdi // move rdi into the qword pointed at by rax
*rax = rdi // same as the above - the * works if there is a single possible type that the pointer can be
*rax = 34 // invalid: you need to explicitly specify the pointer type
```
More complex examples:
```js
rax[1][u16] = 5 // move 5 into the word pointed at by (rax + 1)
rax[rdx] = rcx // move rdx into the qword pointed at by (rax + rdx) - type inferred automatically
rax[rcx * 8 + 0xfff] = cx // move cx into the word pointed at by (rax + rcx * 8 + 0xfff) - type inferred automatically
```

### Control flow
The main way to do control flow is with blocks. Blocks don't do anything on their own, but let you use the `break` and `continue` keywords.

These can be used on their own or with a flag conditional.
```js
{
	// do stuff...
	break if /zero
	// do stuff...
	continue
}
```

Sometimes blocks aren't flexible enough, so in that case you should use `goto`. This is especially useful for error-handling code where you might want to jump there from many parts of your program.
```js
rax += 8975489
goto signed_overflow_error if /overflow
// stuff...
signed_overflow_error:
trap // immediately crashes the program
```

You should organize parts of your code into functions.
```js
function do_stuff() {
	// code in here...
	return
}
```

WARNING: functions, as currently implemented, are a very leaky abstraction. To ensure that your code works, make sure to follow these guidelines:
- Define your functions at the bottom of your program, after the exit syscall, as otherwise they will get run without you having ever called them (in a future version this might be done automatically).
- Always make sure to add a `return` statement at the end.
- Don't `goto` into or out of a function or you could f*** up your stack.

Because functions are basically just syntax sugar, these two are equivalent:
```js
function my_function() {
	// stuff...
	return
}
```

```js
my_function:
// stuff...
return
```

You can call a function like so:
```js
my_function(rax = 4, rdi = 93)
```
which is equivalent to this:
```js
rax = 4
rdi = 93
my_function()
```
You can actually pretend that anything is a function and call it.
```js
rcx = my_function
// uncomment this for a segfault
// rcx *= 2
(rcx)(rax = 4, rdi = 93)

(ax)() // invalid: only 64-bit (pointer-sized) places can be called
```

What's the difference between `goto function` and `function()`? The second one pushes the current address of the instruction pointer (RIP) to the stack. Then, when you do `return`, it pops it and continue executing where it left off. If you futz with the stack, this could pop in garbage data and immediately segfault the program.

Make sure you understand the following:
```js
// (1) stack: [], RIP = 0x1122334455667788
my_function()
// (5) stack: [], RIP = 0x112233445566778d

// code in between...

function my_function() {
	// (2) stack: [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11], RIP = 0x34302
	<- 0x3333
	// (3) stack: [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x33, 0x33], RIP = 0x34307
	-> ax
	// (4) stack: [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11], RIP = 0x34309
	return
}
```

### Operations

TODO: add more documentation

# Examples

Note: see /test_files for more examples.

## Print a u64
```js
// rax: integer to be printed
function print_u64() {
    // if the number is 0, just print "0"
    @set_flags(rax & rax)
    {
        break if !/zero
        static zero_string = "0\n"
        @syscall(rax = 1, rdi = 1, rsi = zero_string, rdx = @len(zero_string))
        return
    }

    rcx = 10
    rsi = rsp - 1
    rsi[u8] = "\n"

    {
        @set_flags(rax & rax)
        break if /zero

        rdx = 0
        @unsigned_divmod(rdx:rax, rcx)

        dl += "0"
        rsi--
        *rsi = dl

        continue
    }

    rdx = rsp
    rdx -= rsi
    @syscall(rax = 1, rdi = 1)

    return
}
```

## Allocate and free memory
```js
// rsi: length, pointer is returned in rax
function allocate_memory() {
    @syscall(rax = 9, rdi = 0, rdx = 3, r10 = 0x22, r8 = -1, r9 = 0) // mmap

    // ensure that an error hasn't occurred
    @set_flags(rax & rax)
    goto fail if /sign
    return

    fail:
    static allocation_error = "memory allocation failed!\n"
    @syscall(rax = 1, rdi = 1, rsi = allocation_error, rdx = @len(allocation_error))
    trap
}

// rdi: pointer to be freed, rsi: the original length
function free_memory() {
    @syscall(rax = 11) // munmap
    return
}
```