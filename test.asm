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