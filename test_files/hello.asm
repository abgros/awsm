static message = "Hello, World!\n"
@syscall(eax = 1, edi = 1, rsi = message, edx = @len(message))
@syscall(eax = 60, edi ^= edi) // EXIT_SUCCESS