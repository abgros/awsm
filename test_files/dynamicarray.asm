static length = 8192

rdx = 0:length[u16]

<- rdx
allocate_memory(rsi = rdx)
-> rbx

// loop over memory and verify that everything is zeroed
rcx = 0
{
	@set_flags(rcx - rbx)
	break if /zero

	rdx = rax[rcx]
	rcx += 8
	@set_flags(rdx & rdx)
	continue if /zero

	trap
}

free_memory(rdi = rax, rsi = rbx)

static msg = "ok\n"
@syscall(rax = 1, rdi = 1, rsi = msg, rdx = @len(msg))
@syscall(rax = 60, rdi = 1)

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