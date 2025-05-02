static buf = "\0\0\0\0"

buf[u32] = "ok!\n"

@syscall(rax = 1, rdi = 1, rsi = buf, rdx = @len(buf))

// exit(EXIT_SUCCESS)
@syscall(eax = 60, edi = 0)