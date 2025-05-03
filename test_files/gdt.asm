static gdtr = "\0\0\0\0\0\0\0\0\0\0\0\0"

function set_gdt() {
    ax = rsp[4][u16]
    gdtr[0][u16] = ax
    eax = rsp[8][u32]
    eax = gdtr[2][u32]
    @load_gdt(gdtr[0][u32])
}
