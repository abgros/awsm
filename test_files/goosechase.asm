rsp[u8] = "o"

one: {
	goto two

	four:
	rsp[1][u8] = "k"
	goto five

	three:
	goto four

	two:
	rsp[2][u8] = "\n"
	goto three
}

five:
@syscall(rax = 1, rdi = 1, rsi = rsp, rdx = 3)

rax = quit
(rax)(rdi = 0)

// rdi: exit code
function quit() {
	@syscall(rax = 60)
}