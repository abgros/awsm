rsp += 8
rsp[u64] = 12

atomic_multiply_u64(rdi = rsp, rsi = 12)
print_u64(rax = *rsp)

static message = "ok!\n"
@syscall(eax = 1, edi = 1, rsi = message, edx = @len(message))
@syscall(rax = 60, rdi = 0)

// rdi: pointer to u64, rsi: multiplier
function atomic_multiply_u64() {
	{
		rax = *rdi
		rcx = rax
		rcx *= rsi
		@try_replace(*rdi, rcx, rax) atomically
		break if /zero
		pause
		continue
	}
	return
}

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