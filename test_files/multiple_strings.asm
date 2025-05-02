static message1 = "Part 1, "
@syscall(eax = 1, edi = 1, rsi = message1, rdx = @len(message1))

static message2 = "Part 2\n"
@syscall(eax = 1, edi = 1, rsi = message2, rdx = @len(message2))

// exit(EXIT_SUCCESS)
@syscall(eax = 60, edi = 0)