static buf = "some long string with lots of text blah blah blah"

rsp -= @len(buf)
@set_direction(forwards)
@copy_u8(rsi = buf, rdi = rsp, rcx = @len(buf))
@compare_u8(rsi = buf, rdi = rsp, rcx = @len(buf))
goto fail if !/zero

@fill_u8(rax = ".", rdi = buf, rcx = @len(buf))
@syscall(eax = 1, edi = 1, rsi = buf, edx = @len(buf))
@syscall(rax = 60, rdi = 0)

fail:
trap