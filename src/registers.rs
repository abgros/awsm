use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

#[rustfmt::skip]
#[derive(Clone, Copy, Debug)]
pub enum Register {
	Rax, Eax, Ax, Al, Ah,
	Rcx, Ecx, Cx, Cl, Ch,
	Rdx, Edx, Dx, Dl, Dh,
	Rbx, Ebx, Bx, Bl, Bh,
	Rsp, Esp, Sp, Spl,
	Rbp, Ebp, Bp, Bpl,
	Rsi, Esi, Si, Sil,
	Rdi, Edi, Di, Dil,
	R8, R8d, R8w, R8b,
	R9, R9d, R9w, R9b,
	R10, R10d, R10w, R10b,
	R11, R11d, R11w, R11b,
	R12, R12d, R12w, R12b,
	R13, R13d, R13w, R13b,
	R14, R14d, R14w, R14b,
	R15, R15d, R15w, R15b,
}

use Register::*;

use crate::place::{Displacement, Scale, Size};

#[rustfmt::skip]
impl Display for Register {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let s = match self {
			Rax => "rax",  Eax => "eax",  Ax => "ax",   Al => "al",   Ah => "ah",
			Rcx => "rcx",  Ecx => "ecx",  Cx => "cx",   Cl => "cl",   Ch => "ch",
			Rdx => "rdx",  Edx => "edx",  Dx => "dx",   Dl => "dl",   Dh => "dh",
			Rbx => "rbx",  Ebx => "ebx",  Bx => "bx",   Bl => "bl",   Bh => "bh",
			Rsp => "rsp",  Esp => "esp",  Sp => "sp",   Spl => "spl",
			Rbp => "rbp",  Ebp => "ebp",  Bp => "bp",   Bpl => "bpl",
			Rsi => "rsi",  Esi => "esi",  Si => "si",   Sil => "sil",
			Rdi => "rdi",  Edi => "edi",  Di => "di",   Dil => "dil",
			R8 => "r8",    R8d => "r8d",  R8w => "r8w",  R8b => "r8b",
			R9 => "r9",    R9d => "r9d",  R9w => "r9w",  R9b => "r9b",
			R10 => "r10",  R10d => "r10d",R10w => "r10w",R10b => "r10b",
			R11 => "r11",  R11d => "r11d",R11w => "r11w",R11b => "r11b",
			R12 => "r12",  R12d => "r12d",R12w => "r12w",R12b => "r12b",
			R13 => "r13",  R13d => "r13d",R13w => "r13w",R13b => "r13b",
			R14 => "r14",  R14d => "r14d",R14w => "r14w",R14b => "r14b",
			R15 => "r15",  R15d => "r15d",R15w => "r15w",R15b => "r15b",
		};
		write!(f, "{s}")
	}
}

impl Register {
	pub fn id(&self) -> u8 {
		match self {
			Rax | Eax | Ax | Al => 0,
			Rcx | Ecx | Cx | Cl => 1,
			Rdx | Edx | Dx | Dl => 2,
			Rbx | Ebx | Bx | Bl => 3,
			Ah => 4,
			Ch => 5,
			Dh => 6,
			Bh => 7,
			Rsp | Esp | Sp | Spl => 4,
			Rbp | Ebp | Bp | Bpl => 5,
			Rsi | Esi | Si | Sil => 6,
			Rdi | Edi | Di | Dil => 7,
			R8 | R8d | R8w | R8b => 8,
			R9 | R9d | R9w | R9b => 9,
			R10 | R10d | R10w | R10b => 10,
			R11 | R11d | R11w | R11b => 11,
			R12 | R12d | R12w | R12b => 12,
			R13 | R13d | R13w | R13b => 13,
			R14 | R14d | R14w | R14b => 14,
			R15 | R15d | R15w | R15b => 15,
		}
	}

	pub fn bits(&self) -> usize {
		match self {
			Rax | Rcx | Rdx | Rbx | Rsp | Rbp | Rsi | Rdi => 64,
			Eax | Ecx | Edx | Ebx | Esp | Ebp | Esi | Edi => 32,
			Ax | Cx | Dx | Bx | Sp | Bp | Si | Di => 16,
			Ah | Ch | Dh | Bh => 8,
			Al | Cl | Dl | Bl | Spl | Bpl | Sil | Dil => 8,
			R8 | R9 | R10 | R11 | R12 | R13 | R14 | R15 => 64,
			R8d | R9d | R10d | R11d | R12d | R13d | R14d | R15d => 32,
			R8w | R9w | R10w | R11w | R12w | R13w | R14w | R15w => 16,
			R8b | R9b | R10b | R11b | R12b | R13b | R14b | R15b => 8,
		}
	}

	pub fn size(&self) -> Size {
		match self {
			Rax | Rcx | Rdx | Rbx | Rsp | Rbp | Rsi | Rdi => Size::Qword,
			Eax | Ecx | Edx | Ebx | Esp | Ebp | Esi | Edi => Size::Dword,
			Ax | Cx | Dx | Bx | Sp | Bp | Si | Di => Size::Word,
			Ah | Ch | Dh | Bh => Size::Byte,
			Al | Cl | Dl | Bl | Spl | Bpl | Sil | Dil => Size::Byte,
			R8 | R9 | R10 | R11 | R12 | R13 | R14 | R15 => Size::Qword,
			R8d | R9d | R10d | R11d | R12d | R13d | R14d | R15d => Size::Dword,
			R8w | R9w | R10w | R11w | R12w | R13w | R14w | R15w => Size::Word,
			R8b | R9b | R10b | R11b | R12b | R13b | R14b | R15b => Size::Byte,
		}
	}

	pub fn needs_rex(&self) -> bool {
		matches!(self, Spl | Bpl | Sil | Dil) || self.is_extended_reg()
	}

	pub fn assert_compatible(&self, reg: Self) -> Result<(), &'static str> {
		self.compatible_with(reg).then_some(()).ok_or("cannot use high-byte registers (ah, ch, dh, bh) together with registers that require a REX prefix, like spl, sil, or r8b–r15b")
	}

	pub fn assert_compatible_three_way(
		reg: Self,
		index: Option<Self>,
		base: Option<Self>,
	) -> Result<(), &'static str> {
		match (index, base) {
			(None, None) => Ok(()),
			(None, Some(r)) | (Some(r), None) => reg.assert_compatible(r),
			(Some(r1), Some(r2)) => {
				reg.assert_compatible(r1)?;
				reg.assert_compatible(r2)?;
				r1.assert_compatible(r2)
			}
		}
	}

	pub fn compatible_with(&self, reg: Self) -> bool {
		let no_rex = matches!(self, Ah | Ch | Dh | Bh) || matches!(reg, Ah | Ch | Dh | Bh);
		let rex = self.needs_rex() || reg.needs_rex();
		!(no_rex && rex)
	}

	pub fn is_extended_reg(&self) -> bool {
		self.id() >= 8
	}

	pub fn three_way_rex(
		reg: Option<Self>,
		index: Option<Self>,
		base: Option<Self>,
		size: Size,
		ignore_rexw: bool,
	) -> Option<u8> {
		let rexw = (matches!(size, Size::Qword) && !ignore_rexw) as u8;
		let rexr = reg.filter(Self::is_extended_reg).is_some() as u8;
		let rexx = index.filter(Self::is_extended_reg).is_some() as u8;
		let rexb = base.filter(Self::is_extended_reg).is_some() as u8;

		let rex = 0x40 | (rexw << 3) | (rexr << 2) | (rexx << 1) | rexb;

		let needs_rex = [reg, index, base].iter().flatten().any(Register::needs_rex) || rex != 0x40;
		needs_rex.then_some(rex)
	}

	pub fn assert_64_bits(&self) -> Result<(), &'static str> {
		if self.bits() == 64 {
			Ok(())
		} else {
			Err("only a 64-bit register (e.g. rcx, rbp) is allowed in this context")
		}
	}

	pub fn get_own_modrm(&self, extension: u8) -> u8 {
		0b11_000_000 | (extension << 3) | (self.id() & 7)
	}

	pub fn get_modrm_slash_n(
		base: Register,
		disp: Option<Displacement>,
		n: u8,
		index: Option<(Scale, Register)>,
	) -> u8 {
		let mod_part = match disp {
			Some(Displacement::Dword(_)) => 0b10,
			Some(Displacement::Byte(_)) => 0b01,
			None if base.id() & 7 == 0b101 => 0b01,
			None => 0,
		};

		if index.is_some() {
			(mod_part << 6) | (n << 3) | 0b100
		} else {
			(mod_part << 6) | (n << 3) | (base.id() & 7)
		}
	}

	pub fn get_modrm_slash_r(
		base: Register,
		disp: Option<Displacement>,
		reg: Register,
		index: Option<(Scale, Register)>,
	) -> u8 {
		let mod_part = match disp {
			Some(Displacement::Dword(_)) => 0b10,
			Some(Displacement::Byte(_)) => 0b01,
			None if base.id() & 7 == 0b101 => 0b01,
			None => 0,
		};

		if index.is_some() {
			(mod_part << 6) | ((reg.id() & 7) << 3) | 0b100
		} else {
			(mod_part << 6) | ((reg.id() & 7) << 3) | (base.id() & 7)
		}
	}

	pub fn maybe_sib(base: Option<Register>, index: Option<(Scale, Register)>) -> Option<u8> {
		let (scale_part, index_part) = if let Some((scale, idx_reg)) = index {
			let scale_bits = match scale {
				Scale::One => 0b00,
				Scale::Two => 0b01,
				Scale::Four => 0b10,
				Scale::Eight => 0b11,
			};
			let index_bits = idx_reg.id() & 7;
			(scale_bits, index_bits)
		} else {
			(0, 0b100)
		};

		let base_part = if let Some(base_reg) = base {
			base_reg.id() & 7
		} else {
			0b101
		};

		let sib = (scale_part << 6) | (index_part << 3) | base_part;

		// a SIB is only required if there is an index, *or* if the base is rsp/r13 (i.e. 0b100).
		if index.is_some() || base_part == 0b100 {
			Some(sib)
		} else {
			None
		}
	}

	pub fn get_modrm_reg_reg(lreg: Self, rreg: Self) -> u8 {
		0xc0 | ((rreg.id() & 7) << 3) | (lreg.id() & 7)
	}

	pub fn get_own_modrm_riprelative(&self) -> u8 {
		0x05 | ((self.id() & 7) << 3)
	}

	pub fn modrm_rip_slash_n(extension: u8) -> u8 {
		(extension << 3) | 0b101
	}
}

#[rustfmt::skip]
impl FromStr for Register {
	type Err = &'static str;

	fn from_str(reg: &str) -> Result<Register, Self::Err> {
		match reg {
			"rax" => Ok(Rax), "eax" => Ok(Eax), "ax" => Ok(Ax), "al" => Ok(Al), "ah" => Ok(Ah),
			"rcx" => Ok(Rcx), "ecx" => Ok(Ecx), "cx" => Ok(Cx), "cl" => Ok(Cl), "ch" => Ok(Ch),
			"rdx" => Ok(Rdx), "edx" => Ok(Edx), "dx" => Ok(Dx), "dl" => Ok(Dl), "dh" => Ok(Dh),
			"rbx" => Ok(Rbx), "ebx" => Ok(Ebx), "bx" => Ok(Bx), "bl" => Ok(Bl), "bh" => Ok(Bh),
			"rsp" => Ok(Rsp), "esp" => Ok(Esp), "sp" => Ok(Sp), "spl" => Ok(Spl),
			"rbp" => Ok(Rbp), "ebp" => Ok(Ebp), "bp" => Ok(Bp), "bpl" => Ok(Bpl),
			"rsi" => Ok(Rsi), "esi" => Ok(Esi), "si" => Ok(Si), "sil" => Ok(Sil),
			"rdi" => Ok(Rdi), "edi" => Ok(Edi), "di" => Ok(Di), "dil" => Ok(Dil),
			"r8" => Ok(R8), "r8d" => Ok(R8d), "r8w" => Ok(R8w), "r8b" => Ok(R8b),
			"r9" => Ok(R9), "r9d" => Ok(R9d), "r9w" => Ok(R9w), "r9b" => Ok(R9b),
			"r10" => Ok(R10), "r10d" => Ok(R10d), "r10w" => Ok(R10w), "r10b" => Ok(R10b),
			"r11" => Ok(R11), "r11d" => Ok(R11d), "r11w" => Ok(R11w), "r11b" => Ok(R11b),
			"r12" => Ok(R12), "r12d" => Ok(R12d), "r12w" => Ok(R12w), "r12b" => Ok(R12b),
			"r13" => Ok(R13), "r13d" => Ok(R13d), "r13w" => Ok(R13w), "r13b" => Ok(R13b),
			"r14" => Ok(R14), "r14d" => Ok(R14d), "r14w" => Ok(R14w), "r14b" => Ok(R14b),
			"r15" => Ok(R15), "r15d" => Ok(R15d), "r15w" => Ok(R15w), "r15b" => Ok(R15b),
			_ => Err("invalid register"),
		}
	}
}
