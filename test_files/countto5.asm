bx = 0 // counter

rsp[1][u8] = "\n"
{
	eax = "1" + rbx
	*rsp = al
	@syscall(rax = 1, rdi = 1, rsi = rsp, rdx = 2)

	bx++
	@set_flags(bx - 5)
	continue if !/zero
}

@syscall(rax = 60, rdi = 0) // EXIT_SUCCESS