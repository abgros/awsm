use crate::*;

#[test]
fn test_resolve_effective_address_exhaustive() {
	let program = Program::new();

	assert!(matches!(
		program.resolve_effective_address("rax").unwrap(),
		EffectiveAddress {
			base: Some(Rax),
			index: None,
			disp: None,
		}
	));

	assert!(matches!(
		program.resolve_effective_address("rsp").unwrap(),
		EffectiveAddress {
			base: Some(Rsp),
			index: None,
			disp: None,
		}
	));

	assert!(matches!(
		program.resolve_effective_address("rax + rdi").unwrap(),
		EffectiveAddress {
			base: Some(Rax),
			index: Some((Scale::One, Rdi)),
			disp: None,
		}
	));

	assert!(matches!(
		program.resolve_effective_address("rax + rdi*4").unwrap(),
		EffectiveAddress {
			base: Some(Rax),
			index: Some((Scale::Four, Rdi)),
			disp: None,
		}
	));

	assert!(matches!(
		program.resolve_effective_address("rcx*8").unwrap(),
		EffectiveAddress {
			base: None,
			index: Some((Scale::Eight, Rcx)),
			disp: None,
		}
	));

	assert!(matches!(
		program.resolve_effective_address("rax + 7").unwrap(),
		EffectiveAddress {
			base: Some(Rax),
			index: None,
			disp: Some(Displacement::Byte(7)),
		}
	));

	assert!(matches!(
		program.resolve_effective_address("rax - 4").unwrap(),
		EffectiveAddress {
			base: Some(Rax),
			index: None,
			disp: Some(Displacement::Byte(-4)),
		}
	));

	assert!(matches!(
		program
			.resolve_effective_address("rax + 0x12345678")
			.unwrap(),
		EffectiveAddress {
			base: Some(Rax),
			index: None,
			disp: Some(Displacement::Dword(0x12345678)),
		}
	));

	assert!(matches!(
		program.resolve_effective_address("rbx + rsp + -8").unwrap(),
		EffectiveAddress {
			base: Some(Rsp),
			index: Some((Scale::One, Rbx)),
			disp: Some(Displacement::Byte(-8)),
		}
	));

	assert!(matches!(
		program
			.resolve_effective_address("rax + rdi + 0x4343")
			.unwrap(),
		EffectiveAddress {
			base: Some(Rax),
			index: Some((Scale::One, Rdi)),
			disp: Some(Displacement::Dword(0x4343)),
		}
	));

	assert!(matches!(
		program
			.resolve_effective_address("rcx + rcx << 1 - 999")
			.unwrap(),
		EffectiveAddress {
			base: Some(Rcx),
			index: Some((Scale::Two, Rcx)),
			disp: Some(Displacement::Dword(-999)),
		}
	));

	assert!(matches!(
		program
			.resolve_effective_address("-999 + rcx * 2 + rcx")
			.unwrap(),
		EffectiveAddress {
			base: Some(Rcx),
			index: Some((Scale::Two, Rcx)),
			disp: Some(Displacement::Dword(-999)),
		}
	));

	assert!(program.resolve_effective_address("rsp*4").is_err());
	assert!(program.resolve_effective_address("rsp + rsp").is_err());
	assert!(
		program
			.resolve_effective_address("rax + rbx + rcx")
			.is_err()
	);
	assert!(program.resolve_effective_address("eax").is_err());
	assert!(program.resolve_effective_address("0x10").is_err());
	assert!(program.resolve_effective_address("-5 - rax").is_err());
	assert!(program.resolve_effective_address("rax + 4 + 8").is_err());
	assert!(
		program
			.resolve_effective_address("rax + 0x1_0000_0000")
			.is_err()
	);
	assert!(program.resolve_effective_address("rax*2 + rbx*4").is_err());
	assert!(program.resolve_effective_address("-rax").is_err());
}

#[test]
fn test_parse_string() {
	assert_eq!(parse_string("\"hello\"").unwrap(), b"hello");
	assert!(parse_string("\"hello").is_err());
	assert!(parse_string("hello\"").is_err());
	assert!(parse_string("hello").is_err());
	assert!(parse_string("\"hello\" a").is_err());
}

#[test]
fn test_parse_assign() {
	let mut program = Program::new();
	program.parse_line("*rax = al").unwrap();
	assert_eq!(program.code, vec![0x88, 0x00]);

	let mut program = Program::new();
	program.parse_line("rax[rdi + 1] = cl").unwrap();
	assert_eq!(program.code, vec![0x88, 0x4C, 0x38, 0x01]);

	let mut program = Program::new();
	program.parse_line("rsp[rcx - 4] = r8b").unwrap();
	assert_eq!(program.code, vec![0x44, 0x88, 0x44, 0x0C, 0xFC]);

	let mut program = Program::new();
	program
		.parse_line("r15[rax * 8 + 0x7fffffff] = r8b")
		.unwrap();
	assert_eq!(
		program.code,
		vec![0x45, 0x88, 0x84, 0xC7, 0xFF, 0xFF, 0xFF, 0x7F]
	);

	let mut program = Program::new();
	program.parse_line("*rbp = al").unwrap();
	assert_eq!(program.code, vec![0x88, 0x45, 0x00]);

	let mut program = Program::new();
	program.parse_line("*r13 = al").unwrap();
	assert_eq!(program.code, vec![0x41, 0x88, 0x45, 0x00]);

	let mut program = Program::new();
	program.parse_line("*r12 = al").unwrap();
	assert_eq!(program.code, vec![0x41, 0x88, 0x04, 0x24]);

	let mut program = Program::new();
	program.parse_line("r8b = r9b").unwrap();
	assert_eq!(program.code, vec![0x45, 0x88, 0xC8]);

	let mut program = Program::new();
	assert!(program.parse_line("r8b = ah").is_err());

	let mut program = Program::new();
	assert!(program.parse_line("ah = r8b").is_err());

	let mut program = Program::new();
	assert!(program.parse_line("rax[rsp] = al").is_err());

	let mut program = Program::new();
	program.parse_line("rax[rbp] = al").unwrap();
	assert_eq!(program.code, vec![0x88, 0x04, 0x28]);

	let mut program = Program::new();
	program.parse_line("r12[rbp * 4 - 8] = r9b").unwrap();
	assert_eq!(program.code, vec![0x45, 0x88, 0x4C, 0xAC, 0xF8]);

	let mut program = Program::new();
	program.parse_line("rax[0x12345678] = al").unwrap();
	assert_eq!(program.code, vec![0x88, 0x80, 0x78, 0x56, 0x34, 0x12]);

	let mut program = Program::new();
	program.parse_line("r13[0x12345678] = al").unwrap();
	assert_eq!(program.code, vec![0x41, 0x88, 0x85, 0x78, 0x56, 0x34, 0x12]);

	let mut program = Program::new();
	program
		.parse_line("r12[rbx * 4 + 0x11223344] = cl")
		.unwrap();
	assert_eq!(
		program.code,
		vec![0x41, 0x88, 0x8c, 0x9C, 0x44, 0x33, 0x22, 0x11]
	);

	let mut program = Program::new();
	assert!(
		program
			.parse_line("rbp[r15 * 2 + 0xcafebabe] = r8b")
			.is_err()
	);

	let mut program = Program::new();
	program
		.parse_line("r14[r9 * 8 + 0x01020304] = r10b")
		.unwrap();
	assert_eq!(
		program.code,
		vec![0x47, 0x88, 0x94, 0xCE, 0x04, 0x03, 0x02, 0x01]
	);

	let mut program = Program::new();
	program
		.parse_line("rsi[rdx * 4 - 0x12345678] = dh")
		.unwrap();
	assert_eq!(program.code, vec![0x88, 0xB4, 0x96, 0x88, 0xA9, 0xCB, 0xED]);
}

#[test]
fn test_parse_assign_register_movs() {
	let mut program = Program::new();
	program.parse_line("ax = cx").unwrap();
	assert_eq!(program.code, vec![0x66, 0x89, 0xC8]);

	let mut program = Program::new();
	program.parse_line("bx = dx").unwrap();
	assert_eq!(program.code, vec![0x66, 0x89, 0xD3]);

	let mut program = Program::new();
	program.parse_line("r8w = r9w").unwrap();
	assert_eq!(program.code, vec![0x66, 0x45, 0x89, 0xC8]);

	let mut program = Program::new();
	program.parse_line("r15w = r12w").unwrap();
	assert_eq!(program.code, vec![0x66, 0x45, 0x89, 0xE7]);

	let mut program = Program::new();
	program.parse_line("r11w = sp").unwrap();
	assert_eq!(program.code, vec![0x66, 0x41, 0x89, 0xE3]);

	let mut program = Program::new();
	program.parse_line("eax = ecx").unwrap();
	assert_eq!(program.code, vec![0x89, 0xC8]);

	let mut program = Program::new();
	program.parse_line("ebx = edx").unwrap();
	assert_eq!(program.code, vec![0x89, 0xD3]);

	let mut program = Program::new();
	program.parse_line("r8d = r9d").unwrap();
	assert_eq!(program.code, vec![0x45, 0x89, 0xC8]);

	let mut program = Program::new();
	program.parse_line("r15d = r12d").unwrap();
	assert_eq!(program.code, vec![0x45, 0x89, 0xE7]);

	let mut program = Program::new();
	program.parse_line("r11d = esp").unwrap();
	assert_eq!(program.code, vec![0x41, 0x89, 0xE3]);

	let mut program = Program::new();
	program.parse_line("rax = rcx").unwrap();
	assert_eq!(program.code, vec![0x48, 0x89, 0xC8]);

	let mut program = Program::new();
	program.parse_line("rbx = rdx").unwrap();
	assert_eq!(program.code, vec![0x48, 0x89, 0xD3]);

	let mut program = Program::new();
	program.parse_line("r8 = r9").unwrap();
	assert_eq!(program.code, vec![0x4D, 0x89, 0xC8]);

	let mut program = Program::new();
	program.parse_line("r15 = r12").unwrap();
	assert_eq!(program.code, vec![0x4D, 0x89, 0xE7]);

	let mut program = Program::new();
	program.parse_line("r11 = rsp").unwrap();
	assert_eq!(program.code, vec![0x49, 0x89, 0xE3]);
}

#[test]
fn test_parse_assign_imm() {
	let mut program = Program::new();
	program.parse_line("rdx = 21").unwrap();
	assert_eq!(program.code, vec![0x48, 0xC7, 0xC2, 0x15, 0x00, 0x00, 0x00]);
}

#[test]
fn bare_syscall() {
	let mut program = Program::new();
	program.parse_line("@syscall()").unwrap();
	assert_eq!(program.code, vec![0x0f, 0x05]);
}

#[test]
fn test_xor_all_64() {
	let mut p = Program::new();
	p.parse_line("rax ^= rax").unwrap();
	assert_eq!(p.code, vec![0x48, 0x31, 0xC0]);

	p = Program::new();
	p.parse_line("rax ^= rcx").unwrap();
	assert_eq!(p.code, vec![0x48, 0x31, 0xC8]);

	p = Program::new();
	p.parse_line("rcx ^= rax").unwrap();
	assert_eq!(p.code, vec![0x48, 0x31, 0xC1]);

	p = Program::new();
	p.parse_line("r8 ^= rax").unwrap();
	assert_eq!(p.code, vec![0x49, 0x31, 0xC0]);

	p = Program::new();
	p.parse_line("rax ^= r8").unwrap();
	assert_eq!(p.code, vec![0x4C, 0x31, 0xC0]);

	p = Program::new();
	p.parse_line("r8 ^= r8").unwrap();
	assert_eq!(p.code, vec![0x4D, 0x31, 0xC0]);

	p = Program::new();
	p.parse_line("r15 ^= r15").unwrap();
	assert_eq!(p.code, vec![0x4D, 0x31, 0xFF]);

	p = Program::new();
	p.parse_line("r9 ^= r10").unwrap();
	assert_eq!(p.code, vec![0x4D, 0x31, 0xD1]);

	p = Program::new();
	p.parse_line("r10 ^= r9").unwrap();
	assert_eq!(p.code, vec![0x4D, 0x31, 0xCA]);
}

#[test]
fn test_xor_all_32() {
	let mut p = Program::new();
	p.parse_line("eax ^= eax").unwrap();
	assert_eq!(p.code, vec![0x31, 0xC0]);

	p = Program::new();
	p.parse_line("eax ^= ecx").unwrap();
	assert_eq!(p.code, vec![0x31, 0xC8]);

	p = Program::new();
	p.parse_line("eax ^= r8d").unwrap();
	assert_eq!(p.code, vec![0x44, 0x31, 0xC0]);

	p = Program::new();
	p.parse_line("eax ^= r15d").unwrap();
	assert_eq!(p.code, vec![0x44, 0x31, 0xF8]);

	p = Program::new();
	p.parse_line("r8d ^= eax").unwrap();
	assert_eq!(p.code, vec![0x41, 0x31, 0xC0]);

	p = Program::new();
	p.parse_line("r8d ^= r8d").unwrap();
	assert_eq!(p.code, vec![0x45, 0x31, 0xC0]);

	p = Program::new();
	p.parse_line("r8d ^= r15d").unwrap();
	assert_eq!(p.code, vec![0x45, 0x31, 0xF8]);

	p = Program::new();
	p.parse_line("r15d ^= eax").unwrap();
	assert_eq!(p.code, vec![0x41, 0x31, 0xC7]);

	p = Program::new();
	p.parse_line("r15d ^= r8d").unwrap();
	assert_eq!(p.code, vec![0x45, 0x31, 0xC7]);

	p = Program::new();
	p.parse_line("r15d ^= r15d").unwrap();
	assert_eq!(p.code, vec![0x45, 0x31, 0xFF]);

	p = Program::new();
	p.parse_line("ecx ^= ebx").unwrap();
	assert_eq!(p.code, vec![0x31, 0xD9]);

	p = Program::new();
	p.parse_line("r9d ^= r11d").unwrap();
	assert_eq!(p.code, vec![0x45, 0x31, 0xD9]);

	p = Program::new();
	p.parse_line("r10d ^= edx").unwrap();
	assert_eq!(p.code, vec![0x41, 0x31, 0xD2]);

	p = Program::new();
	p.parse_line("edi ^= r14d").unwrap();
	assert_eq!(p.code, vec![0x44, 0x31, 0xF7]);

	p = Program::new();
	p.parse_line("r12d ^= esi").unwrap();
	assert_eq!(p.code, vec![0x41, 0x31, 0xF4]);
}

#[test]
fn test_xor_all_16() {
	let mut p = Program::new();
	p.parse_line("ax ^= ax").unwrap();
	assert_eq!(p.code, vec![0x66, 0x31, 0xC0]);

	p = Program::new();
	p.parse_line("ax ^= cx").unwrap();
	assert_eq!(p.code, vec![0x66, 0x31, 0xC8]);

	p = Program::new();
	p.parse_line("ax ^= r8w").unwrap();
	assert_eq!(p.code, vec![0x66, 0x44, 0x31, 0xC0]);

	p = Program::new();
	p.parse_line("ax ^= r15w").unwrap();
	assert_eq!(p.code, vec![0x66, 0x44, 0x31, 0xF8]);

	p = Program::new();
	p.parse_line("r8w ^= ax").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0x31, 0xC0]);

	p = Program::new();
	p.parse_line("r8w ^= r8w").unwrap();
	assert_eq!(p.code, vec![0x66, 0x45, 0x31, 0xC0]);

	p = Program::new();
	p.parse_line("r8w ^= r15w").unwrap();
	assert_eq!(p.code, vec![0x66, 0x45, 0x31, 0xF8]);

	p = Program::new();
	p.parse_line("r15w ^= ax").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0x31, 0xC7]);

	p = Program::new();
	p.parse_line("r15w ^= r8w").unwrap();
	assert_eq!(p.code, vec![0x66, 0x45, 0x31, 0xC7]);

	p = Program::new();
	p.parse_line("r15w ^= r15w").unwrap();
	assert_eq!(p.code, vec![0x66, 0x45, 0x31, 0xFF]);

	p = Program::new();
	p.parse_line("dx ^= bx").unwrap();
	assert_eq!(p.code, vec![0x66, 0x31, 0xDA]);

	p = Program::new();
	p.parse_line("r9w ^= r11w").unwrap();
	assert_eq!(p.code, vec![0x66, 0x45, 0x31, 0xD9]);

	p = Program::new();
	p.parse_line("r10w ^= cx").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0x31, 0xCA]);

	p = Program::new();
	p.parse_line("di ^= r14w").unwrap();
	assert_eq!(p.code, vec![0x66, 0x44, 0x31, 0xF7]);

	p = Program::new();
	p.parse_line("r13w ^= si").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0x31, 0xF5]);
}

#[test]
fn test_xor_8_low() {
	let mut p = Program::new();
	p.parse_line("al ^= al").unwrap();
	assert_eq!(p.code, vec![0x30, 0xC0]);

	let mut p = Program::new();
	p.parse_line("al ^= cl").unwrap();
	assert_eq!(p.code, vec![0x30, 0xC8]);

	let mut p = Program::new();
	p.parse_line("al ^= dl").unwrap();
	assert_eq!(p.code, vec![0x30, 0xD0]);

	let mut p = Program::new();
	p.parse_line("al ^= bl").unwrap();
	assert_eq!(p.code, vec![0x30, 0xD8]);

	let mut p = Program::new();
	p.parse_line("al ^= spl").unwrap();
	assert_eq!(p.code, vec![0x40, 0x30, 0xE0]);

	let mut p = Program::new();
	p.parse_line("al ^= bpl").unwrap();
	assert_eq!(p.code, vec![0x40, 0x30, 0xE8]);

	let mut p = Program::new();
	p.parse_line("al ^= sil").unwrap();
	assert_eq!(p.code, vec![0x40, 0x30, 0xF0]);

	let mut p = Program::new();
	p.parse_line("al ^= dil").unwrap();
	assert_eq!(p.code, vec![0x40, 0x30, 0xF8]);

	let mut p = Program::new();
	p.parse_line("al ^= r8b").unwrap();
	assert_eq!(p.code, vec![0x44, 0x30, 0xC0]);

	let mut p = Program::new();
	p.parse_line("al ^= r9b").unwrap();
	assert_eq!(p.code, vec![0x44, 0x30, 0xC8]);

	let mut p = Program::new();
	p.parse_line("r10b ^= r15b").unwrap();
	assert_eq!(p.code, vec![0x45, 0x30, 0xFA]);

	let mut p = Program::new();
	p.parse_line("r15b ^= r10b").unwrap();
	assert_eq!(p.code, vec![0x45, 0x30, 0xD7]);

	let mut p = Program::new();
	p.parse_line("r8b ^= al").unwrap();
	assert_eq!(p.code, vec![0x41, 0x30, 0xC0]);

	let mut p = Program::new();
	p.parse_line("r9b ^= cl").unwrap();
	assert_eq!(p.code, vec![0x41, 0x30, 0xC9]);

	let mut p = Program::new();
	p.parse_line("r10b ^= dl").unwrap();
	assert_eq!(p.code, vec![0x41, 0x30, 0xD2]);

	let mut p = Program::new();
	p.parse_line("r11b ^= bl").unwrap();
	assert_eq!(p.code, vec![0x41, 0x30, 0xDB]);

	let mut p = Program::new();
	p.parse_line("r12b ^= spl").unwrap();
	assert_eq!(p.code, vec![0x41, 0x30, 0xE4]);

	let mut p = Program::new();
	p.parse_line("r13b ^= dil").unwrap();
	assert_eq!(p.code, vec![0x41, 0x30, 0xFD]);
}

#[test]
fn test_xor_8_high() {
	let mut p = Program::new();
	p.parse_line("ah ^= ah").unwrap();
	assert_eq!(p.code, vec![0x30, 0xE4]);

	let mut p = Program::new();
	p.parse_line("ah ^= al").unwrap();
	assert_eq!(p.code, vec![0x30, 0xC4]);

	let mut p = Program::new();
	p.parse_line("ah ^= cl").unwrap();
	assert_eq!(p.code, vec![0x30, 0xCC]);

	let mut p = Program::new();
	p.parse_line("ah ^= dl").unwrap();
	assert_eq!(p.code, vec![0x30, 0xD4]);

	let mut p = Program::new();
	p.parse_line("ah ^= bl").unwrap();
	assert_eq!(p.code, vec![0x30, 0xDC]);

	let mut p = Program::new();
	p.parse_line("ch ^= ah").unwrap();
	assert_eq!(p.code, vec![0x30, 0xE5]);

	let mut p = Program::new();
	p.parse_line("dh ^= bh").unwrap();
	assert_eq!(p.code, vec![0x30, 0xFE]);

	let mut p = Program::new();
	p.parse_line("bh ^= ch").unwrap();
	assert_eq!(p.code, vec![0x30, 0xEF]);

	let mut p = Program::new();
	p.parse_line("bh ^= al").unwrap();
	assert_eq!(p.code, vec![0x30, 0xC7]);

	let mut p = Program::new();
	p.parse_line("dl ^= ah").unwrap();
	assert_eq!(p.code, vec![0x30, 0xE2]);
}
#[test]
fn test_xor_should_fail() {
	let mut p = Program::new();

	assert!(p.parse_line("eax ^= rax").is_err());
	assert!(p.parse_line("r14 ^= r13d").is_err());
	assert!(p.parse_line("al ^= ax").is_err());
	assert!(p.parse_line("r10w ^= r8b").is_err());
	assert!(p.parse_line("ah ^= spl").is_err());
	assert!(p.parse_line("ch ^= r8b").is_err());
	assert!(p.parse_line("r11b ^= bh").is_err());
	assert!(p.parse_line("r15b ^= ah").is_err());
	assert!(p.parse_line("r9d ^= r9w").is_err());
	assert!(p.parse_line("spl ^= spll").is_err());
	assert!(p.parse_line("dil ^= dill").is_err());
}

#[test]
fn test_xor_r64_imm_valid() {
	let mut p = Program::new();
	p.parse_line("rax ^= 0").unwrap();
	assert_eq!(p.code, vec![0x48, 0x83, 0xF0, 0x00]);

	let mut p = Program::new();
	p.parse_line("rdi ^= 1").unwrap();
	assert_eq!(p.code, vec![0x48, 0x83, 0xF7, 0x01]);

	let mut p = Program::new();
	p.parse_line("rsp ^= 255").unwrap();
	assert_eq!(p.code, vec![0x48, 0x81, 0xF4, 0xFF, 0x00, 0x00, 0x00]);

	let mut p = Program::new();
	p.parse_line("rbp ^= 42").unwrap();
	assert_eq!(p.code, vec![0x48, 0x83, 0xF5, 0x2A]);

	let mut p = Program::new();
	p.parse_line("r12 ^= 7").unwrap();
	assert_eq!(p.code, vec![0x49, 0x83, 0xF4, 0x07]);

	let mut p = Program::new();
	p.parse_line("r13 ^= 123").unwrap();
	assert_eq!(p.code, vec![0x49, 0x83, 0xF5, 0x7B]);

	let mut p = Program::new();
	p.parse_line("rax ^= -1").unwrap();
	assert_eq!(p.code, vec![0x48, 0x83, 0xF0, 0xFF]);

	let mut p = Program::new();
	p.parse_line("rsp ^= 9999").unwrap();
	assert_eq!(p.code, vec![0x48, 0x81, 0xF4, 0x0F, 0x27, 0x00, 0x00]);

	let mut p = Program::new();
	p.parse_line("r12 ^= 256").unwrap();
	assert_eq!(p.code, vec![0x49, 0x81, 0xF4, 0x00, 0x01, 0x00, 0x00]);

	let mut p = Program::new();
	p.parse_line("r13 ^= -256").unwrap();
	assert_eq!(p.code, vec![0x49, 0x81, 0xF5, 0x00, 0xFF, 0xFF, 0xFF]);
}

#[test]
fn test_xor_registers_with_immediate() {
	let mut p = Program::new();
	p.parse_line("rax ^= 0").unwrap();
	assert_eq!(p.code, vec![0x48, 0x83, 0xF0, 0x00]);

	let mut p = Program::new();
	p.parse_line("rbx ^= 1").unwrap();
	assert_eq!(p.code, vec![0x48, 0x83, 0xF3, 0x01]);

	let mut p = Program::new();
	p.parse_line("rdi ^= -1").unwrap();
	assert_eq!(p.code, vec![0x48, 0x83, 0xF7, 0xFF]);

	let mut p = Program::new();
	p.parse_line("rsp ^= 9999").unwrap();
	assert_eq!(p.code, vec![0x48, 0x81, 0xF4, 0x0F, 0x27, 0x00, 0x00]);

	let mut p = Program::new();
	p.parse_line("r8 ^= 42").unwrap();
	assert_eq!(p.code, vec![0x49, 0x83, 0xF0, 0x2A]);

	let mut p = Program::new();
	p.parse_line("r15 ^= 128").unwrap();
	assert_eq!(p.code, vec![0x49, 0x81, 0xF7, 0x80, 0x00, 0x00, 0x00]);

	let mut p = Program::new();
	p.parse_line("eax ^= 0").unwrap();
	assert_eq!(p.code, vec![0x83, 0xF0, 0x00]);

	let mut p = Program::new();
	p.parse_line("ebp ^= 1").unwrap();
	assert_eq!(p.code, vec![0x83, 0xF5, 0x01]);

	let mut p = Program::new();
	p.parse_line("edi ^= 1234").unwrap();
	assert_eq!(p.code, vec![0x81, 0xF7, 0xD2, 0x04, 0x00, 0x00]);

	let mut p = Program::new();
	p.parse_line("r9d ^= 4").unwrap();
	assert_eq!(p.code, vec![0x41, 0x83, 0xF1, 0x04]);

	let mut p = Program::new();
	p.parse_line("r13d ^= 300").unwrap();
	assert_eq!(p.code, vec![0x41, 0x81, 0xF5, 0x2C, 0x01, 0x00, 0x00]);

	let mut p = Program::new();
	p.parse_line("ax ^= 1").unwrap();
	assert_eq!(p.code, vec![0x66, 0x83, 0xF0, 0x01]);

	let mut p = Program::new();
	p.parse_line("dx ^= 256").unwrap();
	assert_eq!(p.code, vec![0x66, 0x81, 0xF2, 0x00, 0x01]);

	let mut p = Program::new();
	p.parse_line("r12w ^= 5").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0x83, 0xF4, 0x05]);

	let mut p = Program::new();
	p.parse_line("r8w ^= 500").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0x81, 0xF0, 0xF4, 0x01]);

	let mut p = Program::new();
	p.parse_line("al ^= 7").unwrap();
	assert_eq!(p.code, vec![0x34, 0x07]);

	let mut p = Program::new();
	p.parse_line("bl ^= -128").unwrap();
	assert_eq!(p.code, vec![0x80, 0xF3, 0x80]);

	let mut p = Program::new();
	p.parse_line("r9b ^= 1").unwrap();
	assert_eq!(p.code, vec![0x41, 0x80, 0xF1, 0x01]);

	let mut p = Program::new();
	p.parse_line("r13b ^= -1").unwrap();
	assert_eq!(p.code, vec![0x41, 0x80, 0xF5, 0xFF]);
}

#[test]
fn test_unsigned_divmod_registers() {
	let mut p = Program::new();
	p.parse_line("@unsigned_divmod(ah:al, cl)").unwrap();
	assert_eq!(p.code, vec![0xF6, 0xF1]);

	let mut p = Program::new();
	p.parse_line("@unsigned_divmod(ah:al, r8b)").unwrap();
	assert_eq!(p.code, vec![0x41, 0xF6, 0xF0]);

	let mut p = Program::new();
	p.parse_line("@unsigned_divmod(dx:ax, cx)").unwrap();
	assert_eq!(p.code, vec![0x66, 0xF7, 0xF1]);

	let mut p = Program::new();
	p.parse_line("@unsigned_divmod(dx:ax, r12w)").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0xF7, 0xF4]);

	let mut p = Program::new();
	p.parse_line("@unsigned_divmod(edx:eax, ebx)").unwrap();
	assert_eq!(p.code, vec![0xF7, 0xF3]);

	let mut p = Program::new();
	p.parse_line("@unsigned_divmod(edx:eax, r13d)").unwrap();
	assert_eq!(p.code, vec![0x41, 0xF7, 0xF5]);

	let mut p = Program::new();
	p.parse_line("@unsigned_divmod(rdx:rax, rcx)").unwrap();
	assert_eq!(p.code, vec![0x48, 0xF7, 0xF1]);

	let mut p = Program::new();
	p.parse_line("@unsigned_divmod(rdx:rax, r9)").unwrap();
	assert_eq!(p.code, vec![0x49, 0xF7, 0xF1]);

	let mut p = Program::new();
	p.parse_line("@divmod(ah:al, cl)").unwrap();
	assert_eq!(p.code, vec![0xF6, 0xF9]);

	let mut p = Program::new();
	p.parse_line("@divmod(ah:al, r8b)").unwrap();
	assert_eq!(p.code, vec![0x41, 0xF6, 0xF8]);

	let mut p = Program::new();
	p.parse_line("@divmod(dx:ax, cx)").unwrap();
	assert_eq!(p.code, vec![0x66, 0xF7, 0xF9]);

	let mut p = Program::new();
	p.parse_line("@divmod(dx:ax, r12w)").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0xF7, 0xFC]);

	let mut p = Program::new();
	p.parse_line("@divmod(edx:eax, ebx)").unwrap();
	assert_eq!(p.code, vec![0xF7, 0xFB]);

	let mut p = Program::new();
	p.parse_line("@divmod(edx:eax, r13d)").unwrap();
	assert_eq!(p.code, vec![0x41, 0xF7, 0xFD]);

	let mut p = Program::new();
	p.parse_line("@divmod(rdx:rax, rcx)").unwrap();
	assert_eq!(p.code, vec![0x48, 0xF7, 0xF9]);

	let mut p = Program::new();
	p.parse_line("@divmod(rdx:rax, r9)").unwrap();
	assert_eq!(p.code, vec![0x49, 0xF7, 0xF9]);
}

#[test]

fn test_raw() {
	let mut program = Program::new();
	program.parse_line("@raw(\"\\x0f\\x05\")").unwrap();
	assert_eq!(program.code, vec![0x0f, 0x05]);
}

#[test]
fn test_divmod_should_fail() {
	let mut p = Program::new();

	assert!(p.parse_line("@unsigned_divmod(ax:dx, cx)").is_err());
	assert!(p.parse_line("divmod(eax:edx, ecx)").is_err());
	assert!(p.parse_line("@unsigned_divmod(r8:rax, rcx)").is_err());
	assert!(p.parse_line("@divmod(dx:ax, ecx)").is_err());
	assert!(p.parse_line("@divmod(rdx:rax, r8d)").is_err());
	assert!(p.parse_line("@divmod(ah:ax, 5)").is_err());
	assert!(p.parse_line("@unsigned_divmod()").is_err());
	assert!(p.parse_line("@divmod(garbage, junk)").is_err());
	assert!(p.parse_line("@unsigned_divmod(edx:eax ecx)").is_err());
	assert!(p.parse_line("@unsigned_divmod(edx:eax)").is_err());
	assert!(p.parse_line("@divmod(edx:eax,)").is_err());
	assert!(p.parse_line("@divmod(edx:eax, ecx").is_err());
}

#[test]
fn reg_inc() {
	let mut p = Program::new();
	p.parse_line("rsi++").unwrap();
	assert_eq!(p.code, vec![0x48, 0xFF, 0xC6]);

	let mut p = Program::new();
	p.parse_line("eax++").unwrap();
	assert_eq!(p.code, vec![0xFF, 0xC0]);

	let mut p = Program::new();
	p.parse_line("ax++").unwrap();
	assert_eq!(p.code, vec![0x66, 0xFF, 0xC0]);

	let mut p = Program::new();
	p.parse_line("al++").unwrap();
	assert_eq!(p.code, vec![0xFE, 0xC0]);

	let mut p = Program::new();
	p.parse_line("r8++").unwrap();
	assert_eq!(p.code, vec![0x49, 0xFF, 0xC0]);

	let mut p = Program::new();
	p.parse_line("r8b++").unwrap();
	assert_eq!(p.code, vec![0x41, 0xFE, 0xC0]);

	let mut p = Program::new();
	p.parse_line("r9b++").unwrap();
	assert_eq!(p.code, vec![0x41, 0xFE, 0xC1]);
}

#[test]
fn mem_inc_disp32() {
	let mut p = Program::new();
	p.parse_line("rax[-23 + rcx << 2][u32]++").unwrap();
	assert_eq!(p.code, vec![0xFF, 0x44, 0x88, 0xe9]);

	let mut p = Program::new();
	p.parse_line("rsp[0x9999][u64]++").unwrap();
	assert_eq!(p.code, vec![0x48, 0xFF, 0x84, 0x24, 0x99, 0x99, 0x00, 0x00]);
}

#[test]
fn mem_inc_disp8() {
	let mut p = Program::new();
	p.parse_line("rbp[u64]++").unwrap();
	assert_eq!(p.code, vec![0x48, 0xFF, 0x45, 0x00]);

	let mut p = Program::new();
	p.parse_line("r13[u16]++").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0xFF, 0x45, 0x00]);
}

#[test]
fn test_strip_comments() {
	assert_eq!(strip_comments("abc // comment"), "abc ");
	assert_eq!(strip_comments("abc def"), "abc def");
	assert_eq!(strip_comments("// only comment"), "");
	assert_eq!(strip_comments("   // comment"), "   ");
	assert_eq!(
		strip_comments("let s = \"hi // not comment\";"),
		"let s = \"hi // not comment\";"
	);
	assert_eq!(
		strip_comments("let x = \"abc // still in string"),
		"let x = \"abc // still in string"
	);
	assert_eq!(
		strip_comments("let x = \"abc\" // trailing comment"),
		"let x = \"abc\" "
	);
	assert_eq!(
		strip_comments("let x = \\\"abc\\\" // comment"),
		"let x = \\\"abc\\\" "
	);
	assert_eq!(
		strip_comments("let s = \"abc\\\\\\\"def\" // real comment"),
		"let s = \"abc\\\\\\\"def\" "
	);
	assert_eq!(strip_comments("\"a\" + \"b\" // comment"), "\"a\" + \"b\" ");
	assert_eq!(strip_comments("abc \\\\ // comment"), "abc \\\\ ");
	assert_eq!(
		strip_comments("let x = \"\\\\\" // comment"),
		"let x = \"\\\\\" "
	);
	assert_eq!(strip_comments("// comment\""), "");
	assert_eq!(strip_comments("\""), "\"");
	assert_eq!(strip_comments("abc \\\""), "abc \\\"");
	assert_eq!(strip_comments("abc \\\" // comment"), "abc \\\" ");
	assert_eq!(strip_comments("abc \\\\ // comment"), "abc \\\\ ");
}

#[test]
fn test_parse_place() {
	assert!(matches!(Place::parse("rax"), Ok(Place::Reg64(Rax))));
	assert!(matches!(
		Place::parse("*rax"),
		Ok(Place::Memory(
			EffectiveAddress {
				base: Some(Rax),
				index: None,
				disp: None,
			},
			None
		))
	));

	assert!(matches!(
		Place::parse("rax[1]"),
		Ok(Place::Memory(
			EffectiveAddress {
				base: Some(Rax),
				index: None,
				disp: Some(Displacement::Byte(1)),
			},
			None
		))
	));

	assert!(matches!(
		Place::parse("rbp[128]"),
		Ok(Place::Memory(
			EffectiveAddress {
				base: Some(Rbp),
				index: None,
				disp: Some(Displacement::Dword(128)),
			},
			None
		))
	));

	assert!(matches!(
		Place::parse("rbp[rax + 128]"),
		Ok(Place::Memory(
			EffectiveAddress {
				base: Some(Rbp),
				index: Some((Scale::One, Rax)),
				disp: Some(Displacement::Dword(128)),
			},
			None
		))
	));

	assert!(matches!(
		Place::parse("rdx[4*rcx + -1]"),
		Ok(Place::Memory(
			EffectiveAddress {
				base: Some(Rdx),
				index: Some((Scale::Four, Rcx)),
				disp: Some(Displacement::Byte(-1)),
			},
			None
		))
	));

	assert!(matches!(
		Place::parse("rdx[49 +r15<<0][u8]"),
		Ok(Place::Memory(
			EffectiveAddress {
				base: Some(Rdx),
				index: Some((Scale::One, R15)),
				disp: Some(Displacement::Byte(49))
			},
			Some(Size::Byte)
		))
	));

	assert!(matches!(
		Place::parse("rax[-1]"),
		Ok(Place::Memory(
			EffectiveAddress {
				base: Some(Rax),
				index: None,
				disp: Some(Displacement::Byte(-1)),
			},
			None
		))
	));

	assert!(matches!(
		Place::parse("r13[rdx - 34][u32]"),
		Ok(Place::Memory(
			EffectiveAddress {
				base: Some(R13),
				index: Some((Scale::One, Rdx)),
				disp: Some(Displacement::Byte(-34)),
			},
			Some(Size::Dword)
		))
	));

	assert!(Place::parse("rax[]").is_err());
	assert!(Place::parse("rax[eax + 1]").is_err());
	assert!(Place::parse("rax[rax * 3]").is_err());
	assert!(Place::parse("rax[49 - rax]").is_err());
	assert!(Place::parse("ecx[0]").is_err());
	assert!(Place::parse("rax[rax << 4]").is_err());
	assert!(Place::parse("rax[rax >> 1]").is_err());
	assert!(Place::parse("* rax").is_err());
	assert!(Place::parse("*rax[u64]").is_err());
}

#[test]
fn test_parse_mov_m8_imm8() {
	let mut p = Program::new();
	p.parse_line("rax[u8] = 0x12").unwrap();
	assert_eq!(p.code, vec![0xc6, 0x00, 0x12]);

	p = Program::new();
	p.parse_line("rsp[u8] = 0x34").unwrap();
	assert_eq!(p.code, vec![0xc6, 0x04, 0x24, 0x34]);

	p = Program::new();
	p.parse_line("rsp[rcx][u8] = 0x56").unwrap();
	assert_eq!(p.code, vec![0xc6, 0x04, 0x0c, 0x56]);

	p = Program::new();
	p.parse_line("rsp[rcx*2][u8] = 0x78").unwrap();
	assert_eq!(p.code, vec![0xc6, 0x04, 0x4c, 0x78]);

	p = Program::new();
	p.parse_line("rsp[rcx*4][u8] = 0x9a").unwrap();
	assert_eq!(p.code, vec![0xc6, 0x04, 0x8c, 0x9a]);

	p = Program::new();
	p.parse_line("rsp[rcx*8][u8] = 0xbc").unwrap();
	assert_eq!(p.code, vec![0xc6, 0x04, 0xcc, 0xbc]);

	p = Program::new();
	p.parse_line("rsp[rcx + 5][u8] = 0xde").unwrap();
	assert_eq!(p.code, vec![0xc6, 0x44, 0x0c, 0x05, 0xde]);

	p = Program::new();
	p.parse_line("rsp[rcx - 10][u8] = 0xef").unwrap();
	assert_eq!(p.code, vec![0xc6, 0x44, 0x0c, 0xf6, 0xef]);

	p = Program::new();
	p.parse_line("rsp[rcx*2 + 7][u8] = 0x01").unwrap();
	assert_eq!(p.code, vec![0xc6, 0x44, 0x4c, 0x07, 0x01]);

	p = Program::new();
	p.parse_line("rsp[rcx*4 - 8][u8] = 0x23").unwrap();
	assert_eq!(p.code, vec![0xc6, 0x44, 0x8c, 0xf8, 0x23]);

	p = Program::new();
	p.parse_line("rsp[rcx + 512][u8] = 0x45").unwrap();
	assert_eq!(p.code, vec![0xc6, 0x84, 0x0c, 0x00, 0x02, 0x00, 0x00, 0x45]);

	p = Program::new();
	p.parse_line("r8[u8] = 0x67").unwrap();
	assert_eq!(p.code, vec![0x41, 0xc6, 0x00, 0x67]);

	p = Program::new();
	p.parse_line("r8[rcx][u8] = 0x89").unwrap();
	assert_eq!(p.code, vec![0x41, 0xc6, 0x04, 0x08, 0x89]);

	p = Program::new();
	p.parse_line("r8[rcx*4 + 5][u8] = 0xab").unwrap();
	assert_eq!(p.code, vec![0x41, 0xc6, 0x44, 0x88, 0x05, 0xab]);
}

#[test]
fn test_pop() {
	let mut p = Program::new();
	p.parse_line("-> rax").unwrap();
	assert_eq!(p.code, &[0x58]);

	let mut p = Program::new();
	p.parse_line("-> rax[u64]").unwrap();
	assert_eq!(p.code, &[0x8f, 0]);

	let mut p = Program::new();
	p.parse_line("-> ax").unwrap();
	assert_eq!(p.code, &[0x66, 0x58]);

	let mut p = Program::new();
	p.parse_line("-> r9").unwrap();
	assert_eq!(p.code, &[0x41, 0x59]);

	let mut p = Program::new();
	p.parse_line("-> r9[u64]").unwrap();
	assert_eq!(p.code, &[0x41, 0x8f, 0x01]);

	let mut p = Program::new();
	p.parse_line("-> r12[u16]").unwrap();
	assert_eq!(p.code, &[0x66, 0x41, 0x8F, 0x04, 0x24]);

	let mut p = Program::new();
	p.parse_line("-> r13[rcx*8+0x239][u16]").unwrap();
	assert_eq!(
		p.code,
		&[0x66, 0x41, 0x8F, 0x84, 0xCD, 0x39, 0x02, 0x00, 0x00]
	);

	let mut p = Program::new();
	p.parse_line("-> rsp[u64]").unwrap();
	assert_eq!(p.code, &[0x8F, 0x04, 0x24]);

	let mut p = Program::new();
	p.parse_line("-> rsp[0x10][u64]").unwrap();
	assert_eq!(p.code, &[0x8F, 0x44, 0x24, 0x10]);

	let mut p = Program::new();
	p.parse_line("-> rbp[u64]").unwrap();
	assert_eq!(p.code, &[0x8F, 0x45, 0x00]);

	let mut p = Program::new();
	p.parse_line("-> rsp[rcx * 2 - 34][u16]").unwrap();
	assert_eq!(p.code, &[0x66, 0x8F, 0x44, 0x4C, 0xDE]);
}

#[test]
fn test_push() {
	let mut p = Program::new();
	p.parse_line("<- rcx").unwrap();
	assert_eq!(p.code, &[0x51]);

	let mut p = Program::new();
	p.parse_line("<- rsp[u64]").unwrap();
	assert_eq!(p.code, &[0xFF, 0x34, 0x24]);

	let mut p = Program::new();
	p.parse_line("<- rsp[r9 * 8 - 0x1234][u16]").unwrap();
	assert_eq!(
		p.code,
		&[0x66, 0x42, 0xFF, 0xB4, 0xCC, 0xCC, 0xED, 0xFF, 0xFF]
	);

	let mut p = Program::new();
	p.parse_line("<- rsp[r12][u64]").unwrap();
	assert_eq!(p.code, &[0x42, 0xFF, 0x34, 0x24]);

	let mut p = Program::new();
	p.parse_line("<- 100").unwrap();
	assert_eq!(p.code, &[0x6A, 0x64]);

	let mut p = Program::new();
	p.parse_line("<- 1000").unwrap();
	assert_eq!(p.code, &[0x66, 0x68, 0xE8, 0x03]);
}

#[test]
fn test_zeroex() {
	let mut p = Program::new();
	p.parse_line("rax = 0:r8[u8]").unwrap();
	assert_eq!(p.code, &[0x49, 0x0F, 0xB6, 0x00]);

	let mut p = Program::new();
	p.parse_line("cx = 0:r8[u8]").unwrap();
	assert_eq!(p.code, &[0x66, 0x41, 0x0F, 0xB6, 0x08]);

	let mut p = Program::new();
	p.parse_line("r11d = 0:r9b").unwrap();
	assert_eq!(p.code, &[0x45, 0x0F, 0xB6, 0xD9]);

	let mut p = Program::new();
	p.parse_line("edx = 0:r12b").unwrap();
	assert_eq!(p.code, &[0x41, 0x0F, 0xB6, 0xD4]);

	let mut p = Program::new();
	p.parse_line("ebx = 0:rsi[u8]").unwrap();
	assert_eq!(p.code, &[0x0F, 0xB6, 0x1E]);

	let mut p = Program::new();
	p.parse_line("r15d = 0:r15b").unwrap();
	assert_eq!(p.code, &[0x45, 0x0F, 0xB6, 0xFF]);

	let mut p = Program::new();
	p.parse_line("eax = 0:al").unwrap();
	assert_eq!(p.code, &[0x0F, 0xB6, 0xC0]);

	let mut p = Program::new();
	p.parse_line("edi = 0:rbx[u8]").unwrap();
	assert_eq!(p.code, &[0x0F, 0xB6, 0x3B]);

	let mut p = Program::new();
	p.parse_line("rax = 0:rcx[45][u16]").unwrap();
	assert_eq!(p.code, &[0x48, 0x0F, 0xB7, 0x41, 0x2D]);
}

#[test]
fn test_zeroex_error() {
	let mut p = Program::new();
	assert!(p.parse_line("rax = 0:edx").is_err());
	assert!(p.parse_line("rax[u64] = 0:edx").is_err());
}

#[test]
fn test_signex() {
	let mut p = Program::new();
	p.parse_line("rax = s:r8[u32]").unwrap();
	assert_eq!(p.code, &[0x49, 0x63, 0x00]);

	let mut p = Program::new();
	p.parse_line("cx = s:r8[u8]").unwrap();
	assert_eq!(p.code, &[0x66, 0x41, 0x0F, 0xBE, 0x08]);

	let mut p = Program::new();
	p.parse_line("r11d = s:r9b").unwrap();
	assert_eq!(p.code, &[0x45, 0x0F, 0xBE, 0xD9]);

	let mut p = Program::new();
	p.parse_line("edx = s:r12b").unwrap();
	assert_eq!(p.code, &[0x41, 0x0F, 0xBE, 0xD4]);

	let mut p = Program::new();
	p.parse_line("r15d = s:r15b").unwrap();
	assert_eq!(p.code, &[0x45, 0x0F, 0xBE, 0xFF]);

	let mut p = Program::new();
	p.parse_line("eax = s:al").unwrap();
	assert_eq!(p.code, &[0x0F, 0xBE, 0xC0]);

	let mut p = Program::new();
	p.parse_line("edi = s:rbx[r15 << 1 + 0xabcde][u16]")
		.unwrap();
	assert_eq!(
		p.code,
		&[0x42, 0x0F, 0xBF, 0xBC, 0x7B, 0xDE, 0xBC, 0x0A, 0x00]
	);
}

#[test]
fn test_setcc() {
	let mut p = Program::new();
	p.parse_line("al = /carry").unwrap();
	assert_eq!(p.code, &[0x0F, 0x92, 0xf8]);

	let mut p = Program::new();
	p.parse_line("r15[rcx * 2 - 34545] = /overflow").unwrap();
	assert_eq!(
		p.code,
		&[0x41, 0x0F, 0x90, 0xbc, 0x4F, 0x0F, 0x79, 0xFF, 0xFF]
	);
}

#[test]
fn test_setcc_error() {
	let mut p = Program::new();
	assert!(p.parse_line("*rax = /wtf").is_err());
	assert!(p.parse_line("rax[u64] = /overflow").is_err());
}

#[test]
fn test_cmovcc() {
	let mut p = Program::new();
	p.parse_line("rax = rbx if /overflow").unwrap();
	assert_eq!(p.code, &[0x48, 0x0F, 0x40, 0xC3]);

	let mut p = Program::new();
	p.parse_line("rcx = rdx if !/overflow").unwrap();
	assert_eq!(p.code, &[0x48, 0x0F, 0x41, 0xCA]);

	let mut p = Program::new();
	p.parse_line("rdx = rsi if /carry").unwrap();
	assert_eq!(p.code, &[0x48, 0x0F, 0x42, 0xD6]);

	let mut p = Program::new();
	p.parse_line("r8 = r9 if /zero").unwrap();
	assert_eq!(p.code, &[0x4D, 0x0F, 0x44, 0xC1]);

	let mut p = Program::new();
	p.parse_line("r10 = r11 if !/sign").unwrap();
	assert_eq!(p.code, &[0x4D, 0x0F, 0x49, 0xD3]);

	let mut p = Program::new();
	p.parse_line("r12 = r13 if /sign").unwrap();
	assert_eq!(p.code, &[0x4D, 0x0F, 0x48, 0xE5]);

	let mut p = Program::new();
	p.parse_line("r14 = r15 if !/less_or_equal").unwrap();
	assert_eq!(p.code, &[0x4D, 0x0F, 0x4F, 0xF7]);

	let mut p = Program::new();
	p.parse_line("rsi = rdi[r14 << 3 - 0xf23] if !/zero")
		.unwrap();
	assert_eq!(
		p.code,
		&[0x4A, 0x0F, 0x45, 0xB4, 0xF7, 0xDD, 0xF0, 0xFF, 0xFF]
	);
}

#[test]
fn test_cmovcc_err() {
	let mut p = Program::new();
	assert!(p.parse_line("rax = rdx if /wtf").is_err());
	assert!(p.parse_line("rax[u64] = rdx if /overflow").is_err());
}

#[test]
fn test_widen_mul() {
	let mut p = Program::new();
	p.parse_line("@widen_mul(edx:eax, *r15)").unwrap();
	assert_eq!(p.code, vec![0x41, 0xF7, 0x2F]);

	p = Program::new();
	p.parse_line("@unsigned_widen_mul(ah:al, r14[35])").unwrap();
	assert_eq!(p.code, vec![0x41, 0xF6, 0x66, 0x23]);

	p = Program::new();
	p.parse_line("@widen_mul(dx:ax, r13w)").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0xF7, 0xED]);

	p = Program::new();
	p.parse_line("@widen_mul(rdx:rax, rcx)").unwrap();
	assert_eq!(p.code, vec![0x48, 0xF7, 0xE9]);

	p = Program::new();
	p.parse_line("@unsigned_widen_mul(edx:eax, rsi[rbx << 1 + 8])")
		.unwrap();
	assert_eq!(p.code, vec![0xF7, 0x64, 0x5E, 0x08]);

	p = Program::new();
	p.parse_line("@unsigned_widen_mul(rdx:rax, r8)").unwrap();
	assert_eq!(p.code, vec![0x49, 0xF7, 0xE0]);

	p = Program::new();
	p.parse_line("@widen_mul(dx:ax, rbp[-4])").unwrap();
	assert_eq!(p.code, vec![0x66, 0xF7, 0x6D, 0xFC]);

	p = Program::new();
	p.parse_line("@unsigned_widen_mul(ah:al, rsp[rcx*4])")
		.unwrap();
	assert_eq!(p.code, vec![0xF6, 0x24, 0x8C]);
}

#[test]
fn test_random() {
	let mut p = Program::new();
	p.parse_line("@random(rcx)").unwrap();
	assert_eq!(p.code, &[0x48, 0x0F, 0xC7, 0xF1]);

	let mut p = Program::new();
	p.parse_line("@random_seed(r13d)").unwrap();
	assert_eq!(p.code, &[0x41, 0x0F, 0xC7, 0xFD]);

	let mut p = Program::new();
	p.parse_line("@random_seed(sp)").unwrap();
	assert_eq!(p.code, &[0x66, 0x0F, 0xC7, 0xFC]);
}

#[test]
fn test_random_error() {
	let mut p = Program::new();
	assert!(p.parse_line("@random(rax[u64])").is_err());
	assert!(p.parse_line("@random_seed(al)").is_err());
}

#[test]
fn test_imul() {
	let mut p = Program::new();
	p.parse_line("rcx *= rax").unwrap();
	assert_eq!(p.code, &[0x48, 0x0F, 0xAF, 0xC8]);

	let mut p = Program::new();
	p.parse_line("bp *= si").unwrap();
	assert_eq!(p.code, &[0x66, 0x0F, 0xAF, 0xEE]);

	let mut p = Program::new();
	p.parse_line("r10d *= rax[r9 << 2 - 0x123]").unwrap();
	assert_eq!(
		p.code,
		&[0x46, 0x0F, 0xAF, 0x94, 0x88, 0xDD, 0xFE, 0xFF, 0xFF]
	);
}

#[test]
fn test_imul_error() {
	let mut p = Program::new();
	assert!(p.parse_line("rcx *= 45").is_err());
	assert!(p.parse_line("edx *= cx").is_err());
	assert!(p.parse_line("rbp[u64] *= rax").is_err());
}

#[test]
fn test_mul_imm() {
	let mut p = Program::new();
	p.parse_line("rcx = rax * 567").unwrap();
	assert_eq!(p.code, vec![0x48, 0x69, 0xC8, 0x37, 0x02, 0x00, 0x00]);

	let mut p = Program::new();
	p.parse_line("ax = *rdx * -35").unwrap();
	assert_eq!(p.code, vec![0x66, 0x6B, 0x02, 0xDD]);

	let mut p = Program::new();
	p.parse_line("ebx = -1 * rax[rcx * 8]").unwrap();
	assert_eq!(p.code, vec![0x6B, 0x1C, 0xC8, 0xFF]);

	let mut p = Program::new();
	p.parse_line("r8 = r15 * 3").unwrap();
	assert_eq!(p.code, vec![0x4D, 0x6B, 0xC7, 0x03]);

	let mut p = Program::new();
	p.parse_line("r9 = *r10 * 0x7F").unwrap();
	assert_eq!(p.code, vec![0x4D, 0x6B, 0x0A, 0x7F]);

	let mut p = Program::new();
	p.parse_line("edi = rsp[4] * 0x1234").unwrap();
	assert_eq!(p.code, vec![0x69, 0x7C, 0x24, 0x04, 0x34, 0x12, 0x00, 0x00]);

	let mut p = Program::new();
	p.parse_line("dx = -128 * rbp[rsi * 2]").unwrap();
	assert_eq!(p.code, vec![0x66, 0x6B, 0x54, 0x75, 0x00, 0x80]);

	let mut p: Program = Program::new();
	p.parse_line("dx = rbp[rsi * 2] * -128").unwrap();
	assert_eq!(p.code, vec![0x66, 0x6B, 0x54, 0x75, 0x00, 0x80]);
}

#[test]
fn test_mul_imm_err() {
	let mut p = Program::new();
	assert!(p.parse_line("dx = ax * 0xffffffff").is_err());
	assert!(p.parse_line("edx = cx * 5").is_err());
	assert!(p.parse_line("rbp = ax * ax").is_err());
}

#[test]
fn test_sal() {
	let mut p = Program::new();
	p.parse_line("rax <<= 5").unwrap();
	assert_eq!(p.code, vec![0x48, 0xC1, 0xE0, 0x05]);

	let mut p = Program::new();
	p.parse_line("rdi <<= cl").unwrap();
	assert_eq!(p.code, vec![0x48, 0xD3, 0xE7]);

	let mut p = Program::new();
	p.parse_line("rcx <<= 1").unwrap();
	assert_eq!(p.code, vec![0x48, 0xD1, 0xE1]);

	let mut p = Program::new();
	p.parse_line("r8 <<= 3").unwrap();
	assert_eq!(p.code, vec![0x49, 0xC1, 0xE0, 0x03]);

	let mut p = Program::new();
	p.parse_line("r15 <<= cl").unwrap();
	assert_eq!(p.code, vec![0x49, 0xD3, 0xE7]);

	let mut p = Program::new();
	p.parse_line("eax <<= 2").unwrap();
	assert_eq!(p.code, vec![0xC1, 0xE0, 0x02]);

	p = Program::new();
	p.parse_line("edx <<= cl").unwrap();
	assert_eq!(p.code, vec![0xD3, 0xE2]);

	let mut p = Program::new();
	p.parse_line("bx <<= 4").unwrap();
	assert_eq!(p.code, vec![0x66, 0xC1, 0xE3, 0x04]);

	let mut p = Program::new();
	p.parse_line("r9w <<= cl").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0xD3, 0xE1]);

	p = Program::new();
	p.parse_line("al <<= 1").unwrap();
	assert_eq!(p.code, vec![0xD0, 0xE0]);

	let mut p = Program::new();
	p.parse_line("r12b <<= 7").unwrap();
	assert_eq!(p.code, vec![0x41, 0xC0, 0xE4, 0x07]);

	let mut p = Program::new();
	p.parse_line("rcx[rdx * 2 - 0xfff][u16] <<= 200").unwrap();
	assert_eq!(
		p.code,
		vec![0x66, 0xC1, 0xA4, 0x51, 0x01, 0xF0, 0xFF, 0xFF, 0xC8]
	);
}

#[test]
fn test_sal_err() {
	let mut p = Program::new();
	assert!(p.parse_line("dx <<= -1").is_err());
	assert!(p.parse_line("r15 <<= dl").is_err());
	assert!(p.parse_line("*rax <<= cl").is_err());
}

#[test]
fn test_sar() {
	let mut p = Program::new();
	p.parse_line("rax >>= 5").unwrap();
	assert_eq!(p.code, vec![0x48, 0xC1, 0xF8, 0x05]);

	let mut p = Program::new();
	p.parse_line("rdi >>= cl").unwrap();
	assert_eq!(p.code, vec![0x48, 0xD3, 0xFF]);

	let mut p = Program::new();
	p.parse_line("rcx >>= 1").unwrap();
	assert_eq!(p.code, vec![0x48, 0xD1, 0xF9]);

	let mut p = Program::new();
	p.parse_line("r8 >>= 3").unwrap();
	assert_eq!(p.code, vec![0x49, 0xC1, 0xF8, 0x03]);

	let mut p = Program::new();
	p.parse_line("r15 >>= cl").unwrap();
	assert_eq!(p.code, vec![0x49, 0xD3, 0xFF]);

	let mut p = Program::new();
	p.parse_line("eax >>= 2").unwrap();
	assert_eq!(p.code, vec![0xC1, 0xF8, 0x02]);

	let mut p = Program::new();
	p.parse_line("edx >>= cl").unwrap();
	assert_eq!(p.code, vec![0xD3, 0xFA]);

	let mut p = Program::new();
	p.parse_line("bx >>= 4").unwrap();
	assert_eq!(p.code, vec![0x66, 0xC1, 0xFB, 0x04]);

	let mut p = Program::new();
	p.parse_line("r9w >>= cl").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0xD3, 0xF9]);

	let mut p = Program::new();
	p.parse_line("al >>= 1").unwrap();
	assert_eq!(p.code, vec![0xD0, 0xF8]);

	let mut p = Program::new();
	p.parse_line("r12b >>= 7").unwrap();
	assert_eq!(p.code, vec![0x41, 0xC0, 0xFC, 0x07]);

	let mut p = Program::new();
	p.parse_line("rcx[rdx * 2 - 0xfff][u16] >>= 200").unwrap();
	assert_eq!(
		p.code,
		vec![0x66, 0xC1, 0xBC, 0x51, 0x01, 0xF0, 0xFF, 0xFF, 0xC8]
	);
}

#[test]
fn test_sar_err() {
	let mut p = Program::new();
	assert!(p.parse_line("dx >>= -1").is_err());
	assert!(p.parse_line("r15 >>= dl").is_err());
	assert!(p.parse_line("*rax >>= cl").is_err());
}

#[test]
fn test_shr() {
	let mut p = Program::new();
	p.parse_line("rax >>>= 5").unwrap();
	assert_eq!(p.code, vec![0x48, 0xC1, 0xE8, 0x05]);

	let mut p = Program::new();
	p.parse_line("rdi >>>= cl").unwrap();
	assert_eq!(p.code, vec![0x48, 0xD3, 0xEF]);

	let mut p = Program::new();
	p.parse_line("rcx >>>= 1").unwrap();
	assert_eq!(p.code, vec![0x48, 0xD1, 0xE9]);

	let mut p = Program::new();
	p.parse_line("r8 >>>= 3").unwrap();
	assert_eq!(p.code, vec![0x49, 0xC1, 0xE8, 0x03]);

	let mut p = Program::new();
	p.parse_line("r15 >>>= cl").unwrap();
	assert_eq!(p.code, vec![0x49, 0xD3, 0xEF]);

	let mut p = Program::new();
	p.parse_line("eax >>>= 2").unwrap();
	assert_eq!(p.code, vec![0xC1, 0xE8, 0x02]);

	let mut p = Program::new();
	p.parse_line("edx >>>= cl").unwrap();
	assert_eq!(p.code, vec![0xD3, 0xEA]);

	let mut p = Program::new();
	p.parse_line("bx >>>= 4").unwrap();
	assert_eq!(p.code, vec![0x66, 0xC1, 0xEB, 0x04]);

	let mut p = Program::new();
	p.parse_line("r9w >>>= cl").unwrap();
	assert_eq!(p.code, vec![0x66, 0x41, 0xD3, 0xE9]);

	let mut p = Program::new();
	p.parse_line("al >>>= 1").unwrap();
	assert_eq!(p.code, vec![0xD0, 0xE8]);

	let mut p = Program::new();
	p.parse_line("r12b >>>= 7").unwrap();
	assert_eq!(p.code, vec![0x41, 0xC0, 0xEC, 0x07]);

	let mut p = Program::new();
	p.parse_line("rcx[rdx * 2 - 0xfff][u16] >>>= 200").unwrap();
	assert_eq!(
		p.code,
		vec![0x66, 0xC1, 0xAC, 0x51, 0x01, 0xF0, 0xFF, 0xFF, 0xC8]
	);
}

#[test]
fn test_shr_err() {
	let mut p = Program::new();
	assert!(p.parse_line("dx >>>= -1").is_err());
	assert!(p.parse_line("r15 >>>= dl").is_err());
	assert!(p.parse_line("*rax >>>= cl").is_err());
}

#[test]
fn test_pause() {
	let mut p = Program::new();
	p.parse_line("pause").unwrap();
	assert_eq!(p.code, vec![0xf3, 0x90]);
}

#[test]
fn test_direction() {
	let mut p = Program::new();
	p.parse_line("@set_direction(  forwards  )").unwrap();
	assert_eq!(p.code, vec![0xfc]);

	let mut p = Program::new();
	p.parse_line("@set_direction( forward)").unwrap();
	assert_eq!(p.code, vec![0xfc]);

	let mut p = Program::new();
	p.parse_line("@set_direction(backwards  )").unwrap();
	assert_eq!(p.code, vec![0xfd]);

	let mut p = Program::new();
	p.parse_line("@set_direction(   backward )").unwrap();
	assert_eq!(p.code, vec![0xfd]);
}

#[test]
fn test_swap() {
	let mut p = Program::new();
	p.parse_line("@swap(rax, rdi)").unwrap();
	assert_eq!(p.code, vec![0x48, 0x97]);

	let mut p = Program::new();
	p.parse_line("@swap(*rsp, r14)").unwrap();
	assert_eq!(p.code, vec![0x4C, 0x87, 0x34, 0x24]);

	let mut p = Program::new();
	p.parse_line("@swap(r8w, rdi[rcx * 2 - 0x44])").unwrap();
	assert_eq!(p.code, vec![0x66, 0x44, 0x87, 0x44, 0x4F, 0xBC]);

	let mut p = Program::new();
	p.parse_line("@swap(cl, r15b)").unwrap();
	assert_eq!(p.code, vec![0x44, 0x86, 0xF9]);

	let mut p = Program::new();
	p.parse_line("@swap(r9d, r10d)").unwrap();
	assert_eq!(p.code, vec![0x45, 0x87, 0xD1]);

	let mut p = Program::new();
	p.parse_line("@swap(eax, ebx)").unwrap();
	assert_eq!(p.code, vec![0x93]);

	let mut p = Program::new();
	p.parse_line("@swap(al, sil)").unwrap();
	assert_eq!(p.code, vec![0x40, 0x86, 0xF0]);

	let mut p = Program::new();
	p.parse_line("@swap(eax, eax)").unwrap();
	assert_eq!(p.code, vec![0x90]);
}

#[test]
fn test_swap_err() {
	let mut p = Program::new();
	assert!(p.parse_line("@swap(rax, esi)").is_err());
	assert!(p.parse_line("@swap(*rdi, rsi[u64])").is_err());
	assert!(p.parse_line("@swap(eax, edi, esi)").is_err());
	assert!(p.parse_line("@swap(r9b, ah)").is_err());
}

#[test]
fn test_swap_add() {
	let mut p = Program::new();
	p.parse_line("@swap_add(rax, rdi)").unwrap();
	assert_eq!(p.code, vec![0x48, 0x0F, 0xC1, 0xF8]);

	let mut p = Program::new();
	p.parse_line("@swap_add(*rsp, r14)").unwrap();
	assert_eq!(p.code, vec![0x4C, 0x0F, 0xC1, 0x34, 0x24]);

	let mut p = Program::new();
	p.parse_line("@swap_add(rdi[rcx * 2 - 0x44], r8w)").unwrap();
	assert_eq!(p.code, vec![0x66, 0x44, 0x0F, 0xC1, 0x44, 0x4F, 0xBC]);

	let mut p = Program::new();
	p.parse_line("@swap_add(cl, r15b)").unwrap();
	assert_eq!(p.code, vec![0x44, 0x0F, 0xC0, 0xF9]);

	let mut p = Program::new();
	p.parse_line("@swap_add(r10d, r9d)").unwrap();
	assert_eq!(p.code, vec![0x45, 0x0F, 0xC1, 0xCA]);

	let mut p = Program::new();
	p.parse_line("@swap_add(eax, ebx)").unwrap();
	assert_eq!(p.code, vec![0x0F, 0xC1, 0xD8]);

	let mut p = Program::new();
	p.parse_line("@swap_add(al, sil)").unwrap();
	assert_eq!(p.code, vec![0x40, 0x0F, 0xC0, 0xF0]);

	let mut p = Program::new();
	p.parse_line("@swap_add(eax, eax)").unwrap();
	assert_eq!(p.code, vec![0x0F, 0xC1, 0xC0]);
}

#[test]
fn test_swap_add_err() {
	let mut p = Program::new();
	assert!(p.parse_line("@swap_add(rax, esi)").is_err());
	assert!(p.parse_line("@swap_add(*rdi, rsi[u64])").is_err());
	assert!(p.parse_line("@swap_add(eax, edi, esi)").is_err());
	assert!(p.parse_line("@swap_add(r9b, ah)").is_err());
	assert!(p.parse_line("@swap_add(rax, rsp[u64])").is_err());
}

#[test]
fn test_try_replace() {
	let mut p = Program::new();
	p.parse_line("@try_replace(rdi, rax, rax)").unwrap();
	assert_eq!(p.code, vec![0x48, 0x0F, 0xB1, 0xC7]);

	let mut p = Program::new();
	p.parse_line("@try_replace(*rsp, r14, rax)").unwrap();
	assert_eq!(p.code, vec![0x4C, 0x0F, 0xB1, 0x34, 0x24]);

	let mut p = Program::new();
	p.parse_line("@try_replace(rdi[rcx * 2 - 0x44], r8w, ax)")
		.unwrap();
	assert_eq!(p.code, vec![0x66, 0x44, 0x0F, 0xB1, 0x44, 0x4F, 0xBC]);

	let mut p = Program::new();
	p.parse_line("@try_replace(r15b, cl, al)").unwrap();
	assert_eq!(p.code, vec![0x41, 0x0F, 0xB0, 0xCF]);

	let mut p = Program::new();
	p.parse_line("@try_replace(r10d, r9d, eax)").unwrap();
	assert_eq!(p.code, vec![0x45, 0x0F, 0xB1, 0xCA]);

	let mut p = Program::new();
	p.parse_line("@try_replace(ebx, eax, eax)").unwrap();
	assert_eq!(p.code, vec![0x0F, 0xB1, 0xC3]);

	let mut p = Program::new();
	p.parse_line("@try_replace(sil, al, al)").unwrap();
	assert_eq!(p.code, vec![0x40, 0x0F, 0xB0, 0xC6]);

	let mut p = Program::new();
	p.parse_line("@try_replace(eax, eax, eax)").unwrap();
	assert_eq!(p.code, vec![0x0F, 0xB1, 0xC0]);
}

#[test]
fn test_try_replace_err() {
	let mut p = Program::new();
	assert!(p.parse_line("@try_replace(rax, esi, rax)").is_err());
	assert!(p.parse_line("@try_replace(*rdi, rsi[u64], rax)").is_err());
	assert!(p.parse_line("@try_replace(eax, edi, esi)").is_err());
	assert!(p.parse_line("@try_replace(r9b, ah, al)").is_err());
	assert!(p.parse_line("@try_replace(eax, eax)").is_err());
	assert!(p.parse_line("@try_replace(eax, eax)").is_err());
}

#[test]
fn test_atomic() {
	let mut p = Program::new();
	p.parse_line("rax[u8]++ atomically").unwrap();
	assert_eq!(p.code, vec![0xF0, 0xFE, 0x00]);

	p = Program::new();
	p.parse_line("@try_replace(*rdi, rcx, rax) atomically")
		.unwrap();
	assert_eq!(p.code, vec![0xF0, 0x48, 0x0F, 0xB1, 0x0F]);

	p = Program::new();
	p.parse_line("*rsp ^= al atomically").unwrap();
	assert_eq!(p.code, vec![0xF0, 0x30, 0x04, 0x24]);

	p = Program::new();
	p.parse_line("rdx[rcx + 4] &= al atomically").unwrap();
	assert_eq!(p.code, vec![0xF0, 0x20, 0x44, 0x0A, 0x04]);

	p = Program::new();
	p.parse_line("@negate(rcx[u32]) atomically").unwrap();
	assert_eq!(p.code, vec![0xF0, 0xF7, 0x19]);
}

#[test]
fn test_atomic_err() {
	let mut p = Program::new();
	assert!(
		p.parse_line("@try_replace(rax, esi, rax) atomically")
			.is_err()
	);
	assert!(p.parse_line("@swap(rax, *rsp) atomically").is_err());
	assert!(p.parse_line("rax += 1 atomically").is_err());
	assert!(p.parse_line("@negate(r15) atomically").is_err());
	assert!(p.parse_line("rax = *rdi atomically").is_err());
}

#[test]
fn test_asm_files() {
	use std::fs;
	use std::path::Path;
	use std::process::Command;

	let env_file = fs::read_to_string(".env").unwrap();
	let env: HashMap<&str, &str> = env_file
		.lines()
		.flat_map(|line| line.split_once("="))
		.collect();
	let ssh_host = env["host"];
	let identity = env["identity"];
	let remote_dir = "projects/awsm_test";

	let test_cases = vec![
		("hello", "Hello, World!\n"),
		("multiple_strings", "Part 1, Part 2\n"),
		("countto5", "1\n2\n3\n4\n5\n"),
		("printnum", "32459387\n"),
		("goosechase", "ok\n"),
		("staticmut", "ok!\n"),
		("dynamicarray", "ok\n"),
		("atomicmul", "144\nok!\n"),
		(
			"copystring",
			".................................................",
		),
	];

	for (name, expected) in test_cases {
		let asm_path = format!("test_files/{name}.asm");
		let exe_path = format!("test_files/{name}");

		let program = Program::parse_from_file(&asm_path).unwrap();
		program.save_to_file(&exe_path).unwrap();
		assert!(Path::new(&exe_path).exists());

		// Upload ELF
		let scp_status = Command::new("scp")
			.args([
				"-i",
				identity,
				&exe_path,
				&format!("{ssh_host}:{remote_dir}/"),
			])
			.status()
			.expect("scp failed");
		assert!(scp_status.success(), "scp failed for {name}");

		// Run remotely
		let ssh_output = Command::new("ssh")
			.args([
				"-i",
				identity,
				ssh_host,
				&format!("chmod +x ~/{remote_dir}/{name} && ~/{remote_dir}/{name}"),
			])
			.output()
			.expect("ssh failed");

		let stdout = String::from_utf8_lossy(&ssh_output.stdout);
		let stderr = String::from_utf8_lossy(&ssh_output.stderr);

		assert_eq!(
			stdout, expected,
			"\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}\n--- expected ---\n{expected}"
		);
	}
}
