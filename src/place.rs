use crate::Program;
use crate::parse_string;
use crate::registers::*;
use Register::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Size {
	Byte,
	Word,
	Dword,
	Qword,
}

#[derive(Clone, Copy, Debug)]
pub enum Scale {
	One,
	Two,
	Four,
	Eight,
}

#[derive(Clone, Copy, Debug)]
pub enum Displacement {
	Byte(i8),
	Dword(i32),
}

#[derive(Clone, Copy, Debug)]
pub struct EffectiveAddress {
	pub base: Option<Register>,
	pub index: Option<(Scale, Register)>,
	pub disp: Option<Displacement>,
}

#[derive(Copy, Clone, Debug)]
pub enum Place<'a> {
	Reg8(Register),
	Reg16(Register),
	Reg32(Register),
	Reg64(Register),
	Memory(EffectiveAddress, Option<Size>),
	RipRelative(&'a str, Displacement, Option<Size>),
}

impl<'a> Place<'a> {
	pub fn size(&self) -> Option<Size> {
		match self {
			Place::Reg8(_) => Some(Size::Byte),
			Place::Reg16(_) => Some(Size::Word),
			Place::Reg32(_) => Some(Size::Dword),
			Place::Reg64(_) => Some(Size::Qword),
			Place::Memory(_, size) | Place::RipRelative(_, _, size) => *size,
		}
	}

	pub fn coerce_size(&self, rhs: Place) -> Result<Size, &'static str> {
		match (self.size(), rhs.size()) {
			(Some(s1), Some(s2)) if s1 != s2 => Err("place size mismatch")?,
			(_, Some(s)) | (Some(s), _) => Ok(s),
			(None, None) => Err("memory-to-memory operations are not possible")?,
		}
	}

	pub fn is_register(&self) -> bool {
		self.register().is_some()
	}

	pub fn register(&self) -> Option<Register> {
		match *self {
			Self::Reg8(reg) | Self::Reg16(reg) | Self::Reg32(reg) | Self::Reg64(reg) => Some(reg),
			_ => None,
		}
	}

	pub fn assert_size(&self) -> Result<Size, &'static str> {
		self.size()
			.ok_or("this place expression must have an explicit size")
	}

	pub fn assert_atomic_compatible(lhs: Self, rhs: Self) -> Result<(), &'static str> {
		if matches!(lhs, Place::Memory(..) | Place::RipRelative(..))
			|| matches!(rhs, Place::Memory(..) | Place::RipRelative(..))
		{
			Ok(())
		} else {
			Err("atomic operations can only be performed when one operand accesses memory")?
		}
	}

	pub fn parse(mut s: &'a str) -> Result<Self, &'static str> {
		if let Ok(reg) = s.parse::<Register>() {
			return Ok(match reg.bits() {
				8 => Self::Reg8(reg),
				16 => Self::Reg16(reg),
				32 => Self::Reg32(reg),
				64 => Self::Reg64(reg),
				_ => Err("unsupported register size")?,
			});
		}

		if let Some(rest) = s.strip_prefix("*") {
			if let Ok(reg) = rest.parse::<Register>() {
				reg.assert_64_bits()?;
				return Ok(Self::Memory(
					EffectiveAddress {
						base: Some(reg),
						index: None,
						disp: None,
					},
					None,
				));
			} else if Program::allowed_identifier(rest) {
				return Ok(Self::RipRelative(rest, Displacement::Byte(0), None));
			} else {
				Err("only a static variable or a register can be dereferenced")?
			}
		}

		let size = if let Some(rest) = s.strip_suffix("[u64]") {
			s = rest;
			Some(Size::Qword)
		} else if let Some(rest) = s.strip_suffix("[u32]") {
			s = rest;
			Some(Size::Dword)
		} else if let Some(rest) = s.strip_suffix("[u16]") {
			s = rest;
			Some(Size::Word)
		} else if let Some(rest) = s.strip_suffix("[u8]") {
			s = rest;
			Some(Size::Byte)
		} else {
			None
		};

		let try_split = s.split_once("[");

		// stuff like `rax[u8]` is allowed
		if try_split.is_none() && size.is_some() {
			if let Ok(reg) = s.parse::<Register>() {
				reg.assert_64_bits()?;
				return Ok(Self::Memory(
					EffectiveAddress {
						base: Some(reg),
						index: None,
						disp: None,
					},
					size,
				));
			} else if Program::allowed_identifier(s) {
				return Ok(Self::RipRelative(s, Displacement::Byte(0), size));
			} else {
				Err("only a static variable or a register can be indexed")?
			}
		}

		let (base, rest) = try_split.ok_or("missing brackets")?;
		let inner = rest.strip_suffix("]").ok_or("missing closing bracket")?;

		if let Ok(base) = base.parse::<Register>() {
			base.assert_64_bits()?;

			// rax[rbx << 2 + 3245]
			let index;
			let disp;

			if let Ok(d) = parse_i8(inner) {
				index = None;
				disp = Some(Displacement::Byte(d));
			} else if let Ok(d) = parse_i32(inner) {
				index = None;
				disp = Some(Displacement::Dword(d));
			} else if let Some((first, rest)) = inner.split_once("+") {
				// We expect one part to be a register + offset, and the other part to be a literal.
				let first = first.trim();
				let rest = rest.trim();
				if let Ok(imm8) = parse_i8(first) {
					index = Some(parse_multiplier_and_register(rest)?);
					disp = Some(Displacement::Byte(imm8));
				} else if let Ok(imm32) = parse_i32(first) {
					index = Some(parse_multiplier_and_register(rest)?);
					disp = Some(Displacement::Dword(imm32));
				} else if let Ok(imm8) = parse_i8(rest) {
					index = Some(parse_multiplier_and_register(first)?);
					disp = Some(Displacement::Byte(imm8));
				} else if let Ok(imm32) = parse_i32(rest) {
					index = Some(parse_multiplier_and_register(first)?);
					disp = Some(Displacement::Dword(imm32));
				} else {
					return Err("could not parse i32")?;
				}
			} else if let Some((first, rest)) = inner.split_once("-") {
				let first = first.trim();
				let rest = rest.trim();
				if let Some(imm8) = parse_i8(rest).ok().and_then(i8::checked_neg) {
					index = Some(parse_multiplier_and_register(first)?);
					disp = Some(Displacement::Byte(imm8));
				} else if let Some(imm32) = parse_i32(rest).ok().and_then(i32::checked_neg) {
					index = Some(parse_multiplier_and_register(first)?);
					disp = Some(Displacement::Dword(imm32));
				} else {
					return Err("could not parse i32")?;
				}
			} else if let Ok(o) = parse_multiplier_and_register(inner) {
				index = Some(o);
				disp = None;
			} else {
				return Err("could not parse i32")?;
			}

			if matches!(index, Some((_, Rsp))) {
				Err("rsp cannot be used in the index")?;
			}

			let base = Some(base);
			Ok(Self::Memory(EffectiveAddress { base, disp, index }, size))
		} else if Program::allowed_identifier(base) {
			// Assume that `base` is a static, this will be checked later
			let disp = if let Ok(disp8) = parse_i8(inner) {
				Displacement::Byte(disp8)
			} else if let Ok(disp32) = parse_i32(inner) {
				Displacement::Dword(disp32)
			} else {
				Err("could not parse displacement")?
			};

			Ok(Self::RipRelative(base, disp, size))
		} else {
			Err("could not parse place expression")?
		}
	}
}

pub fn parse_multiplier_and_register(term: &str) -> Result<(Scale, Register), &'static str> {
	let (scale, register) = if let Some((a, b)) = term.split_once("<<") {
		match (a.trim(), b.trim()) {
			(r, "0") => (Scale::One, r.parse::<Register>()?),
			(r, "1") => (Scale::Two, r.parse()?),
			(r, "2") => (Scale::Four, r.parse()?),
			(r, "3") => (Scale::Eight, r.parse()?),
			_ => Err("invalid scaling factor (shift must be 0, 1, 2, or 3)")?,
		}
	} else if let Some((a, b)) = term.split_once("*") {
		match (a.trim(), b.trim()) {
			("1", r) | (r, "1") => (Scale::One, r.parse()?),
			("2", r) | (r, "2") => (Scale::Two, r.parse()?),
			("4", r) | (r, "4") => (Scale::Four, r.parse()?),
			("8", r) | (r, "8") => (Scale::Eight, r.parse()?),
			_ => Err("invalid scaling factor (shift must be 0, 1, 2, or 3)")?,
		}
	} else if let Ok(reg) = term.parse() {
		(Scale::One, reg)
	} else {
		Err("could not parse multiplier and register")?
	};

	register.assert_64_bits()?;
	Ok((scale, register))
}

fn parse_i8(imm: &str) -> Result<i8, &str> {
	if let Ok(parsed) = imm.parse() {
		Ok(parsed)
	} else if let Ok(parsed) = imm.parse::<u128>() {
		parsed
			.try_into()
			.map_err(|_| "value cannot be represented as an i8")
	} else if let Some(rest) = imm.strip_prefix("0x") {
		u32::from_str_radix(rest, 16)
			.map_err(|_| "invalid hex number")?
			.try_into()
			.map_err(|_| "value cannot be represented as an i8")
	} else if let Ok(parsed) = parse_string(imm) {
		let bytes: [u8; 1] = parsed.try_into().map_err(|_| "string must be 1 byte")?;
		u8::from_le_bytes(bytes)
			.try_into()
			.map_err(|_| "value cannot be represented as an i8")
	} else {
		Err("could not parse imm32")
	}
}

#[expect(unused)]
fn parse_i16(imm: &str) -> Result<i16, &str> {
	if let Ok(parsed) = imm.parse() {
		Ok(parsed)
	} else if let Ok(parsed) = imm.parse::<u128>() {
		parsed
			.try_into()
			.map_err(|_| "value cannot be represented as an i16")
	} else if let Some(rest) = imm.strip_prefix("0x") {
		u16::from_str_radix(rest, 16)
			.map_err(|_| "invalid hex number")?
			.try_into()
			.map_err(|_| "value cannot be represented as an i16")
	} else if let Ok(parsed) = parse_string(imm) {
		let bytes: [u8; 2] = parsed.try_into().map_err(|_| "string must be 2 bytes")?;
		u16::from_le_bytes(bytes)
			.try_into()
			.map_err(|_| "value cannot be represented as an i16")
	} else {
		Err("could not parse imm32")
	}
}

fn parse_i32(imm: &str) -> Result<i32, &str> {
	if let Ok(parsed) = imm.parse() {
		Ok(parsed)
	} else if let Ok(parsed) = imm.parse::<u128>() {
		parsed
			.try_into()
			.map_err(|_| "value cannot be represented as an i32")
	} else if let Some(rest) = imm.strip_prefix("0x") {
		u32::from_str_radix(rest, 16)
			.map_err(|_| "invalid hex number")?
			.try_into()
			.map_err(|_| "value cannot be represented as an i32")
	} else if let Ok(parsed) = parse_string(imm) {
		let bytes: [u8; 4] = parsed.try_into().map_err(|_| "string must be 4 bytes")?;
		u32::from_le_bytes(bytes)
			.try_into()
			.map_err(|_| "value cannot be represented as an i32")
	} else {
		Err("could not parse imm32")
	}
}

#[expect(unused)]
fn parse_i64(imm: &str) -> Result<i64, &str> {
	if let Ok(parsed) = imm.parse() {
		Ok(parsed)
	} else if let Ok(parsed) = imm.parse::<u128>() {
		parsed
			.try_into()
			.map_err(|_| "value cannot be represented as an i32")
	} else if let Some(rest) = imm.strip_prefix("0x") {
		u32::from_str_radix(rest, 16)
			.map_err(|_| "invalid hex number")?
			.try_into()
			.map_err(|_| "value cannot be represented as an i32")
	} else if let Ok(parsed) = parse_string(imm) {
		let bytes: [u8; 4] = parsed.try_into().map_err(|_| "string must be 4 bytes")?;
		u32::from_le_bytes(bytes)
			.try_into()
			.map_err(|_| "value cannot be represented as an i32")
	} else {
		Err("could not parse imm32")
	}
}
