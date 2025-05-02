print_u64(rax = 32459387)

@syscall(rax = 60, rdi = 0) // EXIT_SUCCESS

// rax: integer to be printed
function print_u64() {
	// if the number is 0, just print "0"
	{
		@set_flags(rax & rax)
		break if !/zero
		static zero_string = "0\n"
		@syscall(rax = 1, rdi = 1, rsi = zero_string, rdx = @len(zero_string))
		return
	}

	rsi = rsp - 1
	rsi[u8] = "\n"

	rcx = 10

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