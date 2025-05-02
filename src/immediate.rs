#[derive(Clone, Debug)]
pub enum Immediate {
	Imm8(i8),
	Imm16(i16),
	Imm32(i32),
	Imm64(i64),
	ImmU8(u8),
	ImmU16(u16),
	ImmU32(u32),
	ImmU64(u64),
	Unsized(Vec<u8>),
}

use Immediate::*;

impl Immediate {
	pub fn cast_unsized(self) -> Vec<u8> {
		match self {
			Imm8(val) => val.to_le_bytes().to_vec(),
			Imm16(val) => val.to_le_bytes().to_vec(),
			Imm32(val) => val.to_le_bytes().to_vec(),
			Imm64(val) => val.to_le_bytes().to_vec(),
			ImmU8(val) => val.to_le_bytes().to_vec(),
			ImmU16(val) => val.to_le_bytes().to_vec(),
			ImmU32(val) => val.to_le_bytes().to_vec(),
			ImmU64(val) => val.to_le_bytes().to_vec(),
			Unsized(val) => val,
		}
	}

	pub fn cast_u64(&self) -> Result<u64, &'static str> {
		match *self {
			Imm8(val) => val.try_into().ok(),
			Imm16(val) => val.try_into().ok(),
			Imm32(val) => val.try_into().ok(),
			Imm64(val) => val.try_into().ok(),
			ImmU8(val) => val.try_into().ok(),
			ImmU16(val) => val.try_into().ok(),
			ImmU32(val) => val.try_into().ok(),
			ImmU64(val) => Some(val),
			Unsized(_) => None,
		}
		.ok_or("could not cast immediate to u64")
	}

	pub fn cast_i64(&self) -> Result<i64, &'static str> {
		match *self {
			Imm8(val) => Some(val.into()),
			Imm16(val) => Some(val.into()),
			Imm32(val) => Some(val.into()),
			Imm64(val) => Some(val),
			ImmU8(val) => Some(val.into()),
			ImmU16(val) => Some(val.into()),
			ImmU32(val) => Some(val.into()),
			ImmU64(val) => val.try_into().ok(),
			Unsized(_) => None,
		}
		.ok_or("could not cast immediate to signed i64")
	}

	pub fn cast_u32(&self) -> Result<u32, &'static str> {
		match *self {
			Imm8(val) => val.try_into().ok(),
			Imm16(val) => val.try_into().ok(),
			Imm32(val) => Some(val as u32),
			Imm64(val) => val.try_into().ok(),
			ImmU8(val) => Some(val.into()),
			ImmU16(val) => Some(val.into()),
			ImmU32(val) => Some(val),
			ImmU64(val) => val.try_into().ok(),
			Unsized(_) => None,
		}
		.ok_or("could not cast immediate to u32")
	}

	pub fn cast_i32(&self) -> Result<i32, &'static str> {
		match *self {
			Imm8(val) => Some(val.into()),
			Imm16(val) => Some(val.into()),
			Imm32(val) => Some(val),
			Imm64(val) => val.try_into().ok(),
			ImmU8(val) => Some(val.into()),
			ImmU16(val) => Some(val.into()),
			ImmU32(val) => val.try_into().ok(),
			ImmU64(val) => val.try_into().ok(),
			Unsized(_) => None,
		}
		.ok_or("could not cast immediate to signed i32")
	}

	pub fn cast_i16(&self) -> Result<i16, &'static str> {
		match *self {
			Imm8(val) => Some(val.into()),
			Imm16(val) => Some(val),
			Imm32(val) => val.try_into().ok(),
			Imm64(val) => val.try_into().ok(),
			ImmU8(val) => Some(val.into()),
			ImmU16(val) => val.try_into().ok(),
			ImmU32(val) => val.try_into().ok(),
			ImmU64(val) => val.try_into().ok(),
			Unsized(_) => None,
		}
		.ok_or("could not cast immediate to signed i16")
	}

	pub fn cast_u16(&self) -> Result<u16, &'static str> {
		match *self {
			Imm8(val) => val.try_into().ok(),
			Imm16(val) => Some(val as u16),
			Imm32(val) => val.try_into().ok(),
			Imm64(val) => val.try_into().ok(),
			ImmU8(val) => Some(val.into()),
			ImmU16(val) => Some(val),
			ImmU32(val) => val.try_into().ok(),
			ImmU64(val) => val.try_into().ok(),
			Unsized(_) => None,
		}
		.ok_or("could not cast immediate to u16")
	}

	pub fn cast_i8(&self) -> Result<i8, &'static str> {
		match *self {
			Imm8(val) => Some(val),
			Imm16(val) => val.try_into().ok(),
			Imm32(val) => val.try_into().ok(),
			Imm64(val) => val.try_into().ok(),
			ImmU8(val) => val.try_into().ok(),
			ImmU16(val) => val.try_into().ok(),
			ImmU32(val) => val.try_into().ok(),
			ImmU64(val) => val.try_into().ok(),
			Unsized(_) => None,
		}
		.ok_or("could not cast immediate to signed i8")
	}

	pub fn cast_u8(&self) -> Result<u8, &'static str> {
		match *self {
			Imm8(val) => Some(val as u8),
			Imm16(val) => val.try_into().ok(),
			Imm32(val) => val.try_into().ok(),
			Imm64(val) => val.try_into().ok(),
			ImmU8(val) => Some(val),
			ImmU16(val) => val.try_into().ok(),
			ImmU32(val) => val.try_into().ok(),
			ImmU64(val) => val.try_into().ok(),
			Unsized(_) => None,
		}
		.ok_or("could not cast immediate to u8")
	}

	pub fn strict_cast_u8(&self) -> Result<u8, &'static str> {
		match *self {
			Imm8(val) => val.try_into().ok(),
			Imm16(val) => val.try_into().ok(),
			Imm32(val) => val.try_into().ok(),
			Imm64(val) => val.try_into().ok(),
			ImmU8(val) => Some(val),
			ImmU16(val) => val.try_into().ok(),
			ImmU32(val) => val.try_into().ok(),
			ImmU64(val) => val.try_into().ok(),
			Unsized(_) => None,
		}
		.ok_or("could not cast immediate to strictly positive u8")
	}
}
