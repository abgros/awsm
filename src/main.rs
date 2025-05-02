use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write, stdin, stdout};
use std::process::exit;

mod registers;
use Register::*;
use registers::*;

mod place;
use place::*;

mod immediate;
use immediate::*;

#[cfg(test)]
mod tests;

struct Program {
	code: Vec<u8>,
	// global_name: (data, references<(location, rip)>)
	globals: HashMap<String, (Vec<u8>, Vec<(usize, usize)>)>,
	labels: HashMap<String, (usize, Vec<(usize, usize)>)>,
	blocks_stack: Vec<(usize, Vec<usize>)>,
}

impl Program {
	fn new() -> Self {
		Self {
			code: Vec::new(),
			globals: HashMap::new(),
			labels: HashMap::new(),
			blocks_stack: Vec::new(),
		}
	}

	fn append_elf_header(&mut self) {
		let code = &mut self.code;

		// EI_MAG
		code.push(0x7f);
		code.extend(b"ELF");

		// EI_CLASS
		code.push(2); // 64-bit

		// EI_DATA
		code.push(1); // little-endian

		// EI_VERSION
		code.push(1);

		// EI_OSABI
		code.push(3); // Linux

		// EI_ABIVERSION
		code.push(0); // ignored

		// EI_PAD
		code.extend(&[0, 0, 0, 0, 0, 0, 0]); // ignored

		// e_type
		code.extend(&[3, 0]); // ET_DYN

		// e_machine
		code.extend(&[0x3e, 0]); // AMD x86-64

		// e_version
		code.extend(&[1, 0, 0, 0]);

		// e_entry
		code.extend(&[0x78, 0, 0, 0, 0, 0, 0, 0]); // immediately after headers

		// e_phoff
		code.extend(&[0x40, 0, 0, 0, 0, 0, 0, 0]); // equal to the size of the header

		// e_shoff
		code.extend(&[0, 0, 0, 0, 0, 0, 0, 0]); // ignored

		// e_flags
		code.extend(&[0, 0, 0, 0]); // ignored

		// e_ehsize
		code.extend(&[0x40, 0]); // length is 64 bytes

		// e_phentsize
		code.extend(&[0x38, 0]); // idk

		// e_phnum
		code.extend(&[1, 0]); // one so far lol

		// e_shentsize
		code.extend(&[0x40, 0]); // length is 64 bytes

		// e_shnum
		code.extend(&[0, 0]); // idk

		// e_shstrndx
		code.extend(&[0, 0]); // idk
	}

	fn append_program_header(&mut self) {
		let code = &mut self.code;
		// p_type
		code.extend(&[1, 0, 0, 0]); // PT_LOAD

		// p_flags
		code.extend(&[7, 0, 0, 0]); // PF_R | PF_W | PF_X (read, write, execute)

		// p_offset
		code.extend(&[0, 0, 0, 0, 0, 0, 0, 0]);

		// p_vaddr
		code.extend(&[0, 0, 0, 0, 0, 0, 0, 0]);

		// p_paddr
		code.extend(&[0, 0, 0, 0, 0, 0, 0, 0]); // ignored

		// p_filesz
		code.extend(&[0, 0, 0, 0, 0, 0, 0, 0]); // written in `finalize()`

		// p_memsz
		code.extend(&[0, 0, 0, 0, 0, 0, 0, 0]); // written in `finalize()`

		// p_align
		code.extend(&[0, 0, 0, 0, 0, 0, 0, 0]); // no alignment requirements
	}

	fn finalize(&mut self) -> Result<(), String> {
		if !self.blocks_stack.is_empty() {
			Err("this file is missing a `}`")?
		}

		// Append all of the constants, and fill in all of the RIP-relative offsets.
		for (data, references) in self.globals.values() {
			let global_offset = self.code.len();
			self.code.extend(data);

			// `references` refers to the location of rip when the instruction is reached.
			// assume we need to write the offset immediately before `rip`.
			for &(location, rip) in references {
				let location_slice = &mut self.code[location - 4..location];
				let local_disp = i32::from_le_bytes(location_slice.try_into().unwrap());

				let global_disp = (global_offset - rip) as i32;

				// No risk of overflow unless the program is more than 2 GB.
				let net_disp = local_disp.checked_add(global_disp).unwrap();

				location_slice.copy_from_slice(&net_disp.to_le_bytes());
			}
		}

		for (location, references) in self.labels.values() {
			for &(reference, rip) in references {
				let location_slice = &mut self.code[reference - 4..reference];
				let disp = (*location as i32) - (rip as i32);
				location_slice.copy_from_slice(&disp.to_le_bytes());
			}
		}

		// Write the actual length in the `p_filesz` and `p_memsz` header fields
		let length = self.code.len().to_le_bytes();
		self.code[96..104].copy_from_slice(&length);
		self.code[104..112].copy_from_slice(&length);

		Ok(())
	}

	fn save_to_file(&self, name: &str) -> Result<(), io::Error> {
		let mut f = File::create(name)?;
		f.write_all(&self.code)?;

		Ok(())
	}

	fn allowed_identifier(ident: &str) -> bool {
		matches!(ident.chars().next(), Some('a'..='z' | 'A'..='Z'))
			&& ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
	}

	fn check_identifier<'a>(&self, ident: &'a str) -> Result<&'a str, String> {
		let ident = ident.trim();
		if !Program::allowed_identifier(ident) {
			Err("global name must contain A-Z, a-z, 0-9, or _ and start with a letter")?;
		}
		if ident.parse::<Register>().is_ok() {
			Err("global name cannot be the same as a register name")?;
		}
		let keywords = [
			"if",
			"function",
			"break",
			"continue",
			"goto",
			"return",
			"trap",
			"static",
			"pause",
			"atomically",
		];
		if keywords.contains(&ident) {
			Err("identifier name clashes with a reserved keyword")?;
		}
		if self.globals.contains_key(ident) || self.labels.contains_key(ident) {
			Err(format!("the identifier {ident} is already defined"))?
		}
		Ok(ident)
	}

	fn parse_push(&mut self, rest: &str) -> Result<(), String> {
		if let Ok(place) = Place::parse(rest.trim()) {
			if let Place::Reg16(reg) | Place::Reg64(reg) = place {
				self.unary(0x50 + (reg.id() & 7), false, None, place, true)
			} else if matches!(place.assert_size()?, Size::Word | Size::Qword) {
				self.unary(0xff, false, Some(6), place, true)
			} else {
				Err("only 16-bit and 64-bit places can be pushed")?
			}
		} else if let Some(imm) = self.try_resolve_imm(rest)? {
			if let Ok(val) = imm.cast_u8() {
				self.code.extend([0x6a, val as u8]);
			} else if let Ok(val) = imm.cast_i16() {
				self.code.extend([0x66, 0x68]);
				self.code.extend(i16::to_le_bytes(val));
			} else if let Ok(val) = imm.cast_i32() {
				self.code.push(0x68);
				self.code.extend(i32::to_le_bytes(val));
			} else {
				Err("only 8-bit, 16-bit, or 32-bit immediates can be pushed")?
			}
			Ok(())
		} else {
			Err("could not parse immediate or place")?
		}
	}

	fn parse_pop(&mut self, rest: &str) -> Result<(), String> {
		let place =
			Place::parse(rest.trim()).map_err(|_| "only a place expression can be popped into")?;

		if let Place::Reg16(reg) | Place::Reg64(reg) = place {
			self.unary(0x58 + (reg.id() & 7), false, None, place, true)
		} else if matches!(place.assert_size()?, Size::Word | Size::Qword) {
			self.unary(0x8f, false, Some(0), place, true)
		} else {
			Err("only 16-bit and 64-bit places can be popped into")?
		}
	}

	fn parse_function_call(line: &str) -> Option<(&str, &str, bool)> {
		let rest = line.strip_suffix(")")?;
		if let Some((rest, inner)) = rest.split_once(")(") {
			let name = rest.strip_prefix("(")?;
			if Program::allowed_identifier(name) {
				return Some((name.trim(), inner.trim(), true));
			}
		}
		let (name, inner) = rest.split_once("(")?;

		if Program::allowed_identifier(name) {
			return Some((name.trim(), inner.trim(), false));
		}
		None
	}

	fn parse_break(&mut self, rest: &str) -> Result<(), String> {
		let rest = rest.trim();
		let references = &mut self
			.blocks_stack
			.last_mut()
			.ok_or("`break` can only be used within a block")?
			.1;

		// We don't know how far ahead the block will end, so always do a rel32.
		let (prefix, opcode) = jmp_opcode(rest, true)?;
		if opcode == 0xe3 {
			Err("/ecx_zero and /rcx_zero are currently only supported with `continue`")?
		}
		if let Some(val) = prefix {
			self.code.push(val);
		}
		self.code.extend([opcode, 0, 0, 0, 0]); // dummy rel32 (to be filled in)
		references.push(self.code.len());

		Ok(())
	}

	fn parse_continue(&mut self, rest: &str) -> Result<(), String> {
		let rest = rest.trim();
		let continue_to = self
			.blocks_stack
			.last()
			.ok_or("`continue` can only be used within a block")?
			.0;

		// Note: we have to include the length of the current instruction.
		// It is 3 bytes for rel32 and 2 bytes for rel8.
		let dist = -i32::try_from((self.code.len() + 3) - continue_to).unwrap();
		let rel8 = i8::try_from(dist + 1);
		let (prefix, opcode) = jmp_opcode(rest, rel8.is_err())?;
		if let Some(val) = prefix {
			self.code.push(val);
		}

		match rel8 {
			Ok(val) => self.code.extend([opcode, val as u8]),
			Err(_) => {
				// Append the override prefix.
				self.code.push(opcode);
				self.code.extend(dist.to_ne_bytes());
			}
		}

		Ok(())
	}

	fn parse_goto(&mut self, rest: &str) -> Result<(), String> {
		let (first, second) = rest.trim().split_once(" ").unwrap_or((rest, ""));
		// Only consider a rel8 if the location is already determined.
		let ident = first.trim();
		let condition = second.trim();

		let &mut (location, ref mut references) = self
			.labels
			.get_mut(ident)
			.ok_or_else(|| format!("`{ident}` is not defined as a label"))?;

		if location == usize::MAX {
			let (prefix, opcode) = jmp_opcode(condition, true)?;
			if opcode == 0xe3 {
				Err("/ecx_zero and /rcx_zero are currently only supported with `continue`")?
			}
			if let Some(val) = prefix {
				self.code.push(val);
			}
			self.code.extend([opcode, 0, 0, 0, 0]);
			references.push((self.code.len(), self.code.len()));
		} else {
			let dist = -i32::try_from((self.code.len() + 3) - location).unwrap();
			let rel8 = i8::try_from(dist + 1);
			let (prefix, opcode) = jmp_opcode(condition, rel8.is_err())?;
			if opcode == 0xe3 {
				Err("/ecx_zero and /rcx_zero are currently only supported with `continue`")?
			}
			if let Some(val) = prefix {
				self.code.push(val);
			}

			match rel8 {
				Ok(val) => self.code.extend([opcode, val as u8]),
				Err(_) => {
					// Append the override prefix.
					self.code.push(opcode);
					self.code.extend(dist.to_ne_bytes());
				}
			}
		}

		Ok(())
	}

	fn unary_inner(
		&mut self,
		opcode: u8,
		ext_opcode: bool,
		extension: Option<u8>,
		place: Place,
		no_rexw: bool,
		size: Size,
	) -> Result<(), String> {
		if size == Size::Word {
			self.code.push(0x66);
		}

		match place {
			Place::Reg8(reg) | Place::Reg16(reg) | Place::Reg32(reg) | Place::Reg64(reg) => {
				Register::three_way_rex(None, None, Some(reg), reg.size(), no_rexw)
			}
			Place::Memory(EffectiveAddress { base, index, .. }, _) => {
				Register::three_way_rex(None, index.map(|(_, val)| val), base, size, no_rexw)
			}
			Place::RipRelative(..) => None,
		}
		.inspect(|&rex_byte| self.code.push(rex_byte));

		if ext_opcode {
			self.code.push(0x0f);
		}
		self.code.push(opcode);

		if let Some(n) = extension {
			let modrm = match place {
				Place::Reg8(reg) | Place::Reg16(reg) | Place::Reg32(reg) | Place::Reg64(reg) => {
					reg.get_own_modrm(n)
				}
				Place::Memory(EffectiveAddress { base, index, disp }, _) => {
					let base_reg = base.expect("unary operation should always have a base");
					if let Some((_, index_reg)) = index {
						index_reg.assert_compatible(base_reg)?;
					}
					Register::get_modrm_slash_n(base_reg, disp, n, index)
				}
				Place::RipRelative(..) => Register::modrm_rip_slash_n(n),
			};
			self.code.push(modrm);
		}

		if let Place::Memory(EffectiveAddress { base, index, .. }, _) = place {
			Register::maybe_sib(base, index).inspect(|&sib| self.code.push(sib));
		}

		match place {
			Place::Memory(
				EffectiveAddress {
					disp: Some(Displacement::Byte(val)),
					..
				},
				_,
			) => self.code.push(val as u8),
			Place::Memory(
				EffectiveAddress {
					disp: Some(Displacement::Dword(val)),
					..
				},
				_,
			) => self.code.extend(i32::to_le_bytes(val)),
			Place::Memory(
				EffectiveAddress {
					base: Some(Rbp | R13),
					disp: None,
					..
				},
				_,
			) => self.code.push(0),
			Place::RipRelative(_, Displacement::Byte(val), _) => {
				self.code.extend(i32::to_le_bytes(val as i32))
			}
			Place::RipRelative(_, Displacement::Dword(val), _) => {
				self.code.extend(i32::to_le_bytes(val))
			}
			_ => {}
		}

		Ok(())
	}

	fn unary_with_size(
		&mut self,
		opcode: u8,
		ext_opcode: bool,
		extension: Option<u8>,
		place: Place,
		no_rexw: bool,
		size: Size,
	) -> Result<(), String> {
		self.unary_inner(opcode, ext_opcode, extension, place, no_rexw, size)?;

		if let Place::RipRelative(ident, _, _) = place {
			let (_, references) = self
				.globals
				.get_mut(ident)
				.ok_or_else(|| format!("{ident} is not defined"))?;
			references.push((self.code.len(), self.code.len()));
		}

		Ok(())
	}

	fn unary(
		&mut self,
		opcode: u8,
		ext_opcode: bool,
		extension: Option<u8>,
		place: Place,
		no_rexw: bool,
	) -> Result<(), String> {
		let size = place.assert_size()?;
		self.unary_inner(opcode, ext_opcode, extension, place, no_rexw, size)?;

		if let Place::RipRelative(ident, _, _) = place {
			let (_, references) = self
				.globals
				.get_mut(ident)
				.ok_or_else(|| format!("{ident} is not defined"))?;
			references.push((self.code.len(), self.code.len()));
		}

		Ok(())
	}

	fn unary_with_imm(
		&mut self,
		opcode: u8,
		extension: Option<u8>,
		place: Place,
		no_rexw: bool,
		imm: &Immediate,
		imm_size: Size,
		signed: bool,
	) -> Result<(), String> {
		let size = place.assert_size()?;
		self.unary_inner(opcode, false, extension, place, no_rexw, size)?;
		let ref_location = self.code.len();

		if signed {
			match imm_size {
				Size::Byte => self.code.push(imm.cast_i8()? as u8),
				Size::Word => self.code.extend(imm.cast_i16()?.to_le_bytes()),
				Size::Dword => self.code.extend(imm.cast_i32()?.to_le_bytes()),
				Size::Qword => self.code.extend(imm.cast_i64()?.to_le_bytes()),
			}
		} else {
			match imm_size {
				Size::Byte => self.code.push(imm.cast_u8()? as u8),
				Size::Word => self.code.extend(imm.cast_u16()?.to_le_bytes()),
				Size::Dword => self.code.extend(imm.cast_u32()?.to_le_bytes()),
				Size::Qword => self.code.extend(imm.cast_u64()?.to_le_bytes()),
			}
		}

		if let Place::RipRelative(ident, _, _) = place {
			let (_, references) = self
				.globals
				.get_mut(ident)
				.ok_or_else(|| format!("{ident} is not defined"))?;
			references.push((ref_location, self.code.len()));
		}
		Ok(())
	}

	fn binary_with_imm(
		&mut self,
		opcode: u8,
		ext_opcode: bool,
		place1: Place,
		place2: Place,
		no_rexw: bool,
		size: Size,
		imm: Immediate,
		imm_size: Size,
	) -> Result<(), String> {
		self.binary_inner(opcode, ext_opcode, place1, place2, no_rexw, size)?;
		let ref_location = self.code.len();

		match imm_size {
			Size::Byte => self.code.push(imm.cast_i8()? as u8),
			Size::Word => self.code.extend(imm.cast_i16()?.to_le_bytes()),
			Size::Dword => self.code.extend(imm.cast_i32()?.to_le_bytes()),
			Size::Qword => self.code.extend(imm.cast_i64()?.to_le_bytes()),
		}

		if let (_, Place::RipRelative(ident, _, _)) | (Place::RipRelative(ident, _, _), _) =
			(place1, place2)
		{
			if let Some((_, references)) = self.globals.get_mut(ident) {
				references.push((ref_location, self.code.len()));
			} else if let Some((_, references)) = self.labels.get_mut(ident) {
				references.push((ref_location, self.code.len()));
			} else {
				Err(format!("{ident} is not defined"))?
			}
		}

		Ok(())
	}

	fn binary_inner(
		&mut self,
		opcode: u8,
		ext_opcode: bool,
		place1: Place,
		place2: Place,
		no_rexw: bool,
		size: Size,
	) -> Result<(), String> {
		// Decide whether to add the 16-bit prefix (0x66):
		if matches!(size, Size::Word) {
			self.code.push(0x66);
		}

		// Append the REX prefix if necessary.
		match (place1, place2) {
			(
				Place::Reg8(reg1) | Place::Reg16(reg1) | Place::Reg32(reg1) | Place::Reg64(reg1),
				Place::Reg8(reg2) | Place::Reg16(reg2) | Place::Reg32(reg2) | Place::Reg64(reg2),
			) => Register::three_way_rex(Some(reg2), None, Some(reg1), reg1.size(), no_rexw),
			(
				Place::Reg8(reg) | Place::Reg16(reg) | Place::Reg32(reg) | Place::Reg64(reg),
				Place::Memory(EffectiveAddress { base, index, .. }, _),
			)
			| (
				Place::Memory(EffectiveAddress { base, index, .. }, _),
				Place::Reg8(reg) | Place::Reg16(reg) | Place::Reg32(reg) | Place::Reg64(reg),
			) => Register::three_way_rex(Some(reg), index.map(|val| val.1), base, size, no_rexw),
			(
				Place::Reg8(reg) | Place::Reg16(reg) | Place::Reg32(reg) | Place::Reg64(reg),
				Place::RipRelative(..),
			)
			| (
				Place::RipRelative(..),
				Place::Reg8(reg) | Place::Reg16(reg) | Place::Reg32(reg) | Place::Reg64(reg),
			) => Register::three_way_rex(Some(reg), None, None, size, no_rexw),
			_ => Err("memory-to-memory operations are not possible")?,
		}
		.inspect(|&rex_byte| self.code.push(rex_byte));

		// Append the opcode.
		if ext_opcode {
			self.code.push(0x0f);
		}
		self.code.push(opcode);

		// Append the modr/m byte which contains the \r part.
		let modrm = match (place1, place2) {
			(
				Place::Reg8(reg1) | Place::Reg16(reg1) | Place::Reg32(reg1) | Place::Reg64(reg1),
				Place::Reg8(reg2) | Place::Reg16(reg2) | Place::Reg32(reg2) | Place::Reg64(reg2),
			) => {
				reg1.assert_compatible(reg2)?;
				Register::get_modrm_reg_reg(reg1, reg2)
			}
			(
				Place::Reg8(reg) | Place::Reg16(reg) | Place::Reg32(reg) | Place::Reg64(reg),
				Place::Memory(EffectiveAddress { base, index, disp }, _),
			)
			| (
				Place::Memory(EffectiveAddress { base, index, disp }, _),
				Place::Reg8(reg) | Place::Reg16(reg) | Place::Reg32(reg) | Place::Reg64(reg),
			) => {
				Register::assert_compatible_three_way(reg, index.map(|val| val.1), base)?;
				Register::get_modrm_slash_r(base.unwrap(), disp, reg, index)
			}
			(
				Place::Reg8(reg) | Place::Reg16(reg) | Place::Reg32(reg) | Place::Reg64(reg),
				Place::RipRelative(..),
			)
			| (
				Place::RipRelative(..),
				Place::Reg8(reg) | Place::Reg16(reg) | Place::Reg32(reg) | Place::Reg64(reg),
			) => reg.get_own_modrm_riprelative(),
			_ => unreachable!(),
		};
		self.code.push(modrm);

		// Append the SIB byte if necessary.
		match (place1, place2) {
			(Place::Memory(EffectiveAddress { base, index, .. }, _), _)
			| (_, Place::Memory(EffectiveAddress { base, index, .. }, _)) => {
				Register::maybe_sib(base, index)
			}
			_ => None,
		}
		.inspect(|&sib| self.code.push(sib));

		// Append the displacement if necessary.
		match (place1, place2) {
			(
				Place::Memory(
					EffectiveAddress {
						disp: Some(Displacement::Byte(val)),
						..
					},
					_,
				),
				_,
			)
			| (
				_,
				Place::Memory(
					EffectiveAddress {
						disp: Some(Displacement::Byte(val)),
						..
					},
					_,
				),
			) => self.code.push(val as u8),
			(
				_,
				Place::Memory(
					EffectiveAddress {
						disp: Some(Displacement::Dword(val)),
						..
					},
					_,
				),
			)
			| (
				Place::Memory(
					EffectiveAddress {
						disp: Some(Displacement::Dword(val)),
						..
					},
					_,
				),
				_,
			) => self.code.extend(i32::to_le_bytes(val)),
			(
				_,
				Place::Memory(
					EffectiveAddress {
						base: Some(Rbp | R13),
						disp: None,
						..
					},
					_,
				),
			)
			| (
				Place::Memory(
					EffectiveAddress {
						base: Some(Rbp | R13),
						disp: None,
						..
					},
					_,
				),
				_,
			) => self.code.push(0),
			(_, Place::RipRelative(_, Displacement::Byte(val), _))
			| (Place::RipRelative(_, Displacement::Byte(val), _), _) => {
				self.code.extend(i32::to_le_bytes(val as i32))
			}
			(_, Place::RipRelative(_, Displacement::Dword(val), _))
			| (Place::RipRelative(_, Displacement::Dword(val), _), _) => {
				self.code.extend(i32::to_le_bytes(val))
			}
			_ => {}
		}

		Ok(())
	}

	fn binary_with_size(
		&mut self,
		opcode: u8,
		ext_opcode: bool,
		place1: Place,
		place2: Place,
		no_rexw: bool,
		size: Size,
	) -> Result<(), String> {
		self.binary_inner(opcode, ext_opcode, place1, place2, no_rexw, size)?;

		if let (_, Place::RipRelative(ident, _, _)) | (Place::RipRelative(ident, _, _), _) =
			(place1, place2)
		{
			if let Some((_, references)) = self.globals.get_mut(ident) {
				references.push((self.code.len(), self.code.len()));
			} else if let Some((_, references)) = self.labels.get_mut(ident) {
				references.push((self.code.len(), self.code.len()));
			} else {
				Err(format!("{ident} is not defined"))?
			}
		}

		Ok(())
	}

	fn binary(
		&mut self,
		opcode: u8,
		ext_opcode: bool,
		place1: Place,
		place2: Place,
		no_rexw: bool,
	) -> Result<(), String> {
		let size = match (place1.size(), place2.size()) {
			(None, None) => Err("memory-to-memory operations are not possible")?,
			(Some(sz1), Some(sz2)) if sz1 != sz2 => Err("place size mismatch")?,
			(Some(size), _) | (_, Some(size)) => size,
		};

		self.binary_with_size(opcode, ext_opcode, place1, place2, no_rexw, size)?;

		Ok(())
	}

	fn parse_virtual_call(&mut self, lvalue: &str, inner: &str) -> Result<(), String> {
		let lhs = Place::parse(lvalue)?;
		if matches!(lhs.assert_size()?, Size::Qword) {
			self.parse_call_setup(inner)?;
			self.unary(0xff, false, Some(2), lhs, true)
		} else {
			Err("only a 64-bit place can be called")?
		}
	}

	fn parse_call(&mut self, name: &str, inner: &str) -> Result<(), String> {
		self.parse_call_setup(inner)?;

		let references = &mut self
			.labels
			.get_mut(name)
			.ok_or_else(|| format!("{name} is not defined"))?
			.1;

		self.code.push(0xe8);
		self.code.extend([0, 0, 0, 0]); // dummy rel32 (to be filled in)
		references.push((self.code.len(), self.code.len()));

		Ok(())
	}

	fn parse_inc(&mut self, lvalue: &str, atomic: bool) -> Result<(), String> {
		self.parse_basic_unary(lvalue, 0xfe, 0, atomic)
	}

	fn parse_dec(&mut self, lvalue: &str, atomic: bool) -> Result<(), String> {
		self.parse_basic_unary(lvalue, 0xfe, 1, atomic)
	}

	fn parse_not(&mut self, lvalue: &str, atomic: bool) -> Result<(), String> {
		self.parse_basic_unary(lvalue, 0xf6, 2, atomic)
	}

	fn parse_neg(&mut self, lvalue: &str, atomic: bool) -> Result<(), String> {
		self.parse_basic_unary(lvalue, 0xf6, 3, atomic)
	}

	fn parse_basic_unary(
		&mut self,
		lvalue: &str,
		opcode: u8,
		ext: u8,
		atomic: bool,
	) -> Result<(), String> {
		let place = Place::parse(lvalue)?;
		let opcode = match place {
			Place::Reg8(_) | Place::Memory(_, Some(Size::Byte)) => opcode,
			_ => opcode + 1,
		};
		if atomic && place.is_register() {
			Err("atomic operations can only be performed when one operand accesses memory")?;
		} else if atomic {
			self.code.push(0xf0);
		}
		self.unary(opcode, false, Some(ext), place, false)
	}

	fn parse_assign(&mut self, lvalue: &str, rvalue: &str) -> Result<(), String> {
		let lvalue = lvalue.trim();
		let lhs = Place::parse(lvalue).map_err(|e| {
			format!("the left-hand side of an assignment must be a place expression ({e})")
		})?;

		if let Ok(rhs) = Place::parse(rvalue) {
			let opcode = match (rhs.is_register(), lhs.size(), rhs.size()) {
				(true, _, Some(Size::Byte)) | (true, Some(Size::Byte), _) => 0x88,
				(true, _, _) => 0x89,
				(false, _, Some(Size::Byte)) | (false, Some(Size::Byte), _) => 0x8a,
				(false, _, _) => 0x8b,
			};
			self.binary(opcode, false, lhs, rhs, false)
		} else if let Some(rhs) = &self.try_resolve_imm(rvalue)? {
			let size = lhs.assert_size()?;

			match (lhs, size) {
				(Place::Reg8(reg), _) => {
					let opcode = 0xb0 + (reg.id() & 7);
					self.unary_with_imm(opcode, None, lhs, false, rhs, Size::Byte, false)
				}
				(Place::Reg16(reg), _) | (Place::Reg32(reg), _) => {
					let opcode = 0xb8 + (reg.id() & 7);
					self.unary_with_imm(opcode, None, lhs, false, rhs, size, false)
				}
				(Place::Reg64(reg), _) if rhs.cast_i32().is_err() => {
					let opcode = 0xb8 + (reg.id() & 7);
					self.unary_with_imm(opcode, None, lhs, false, rhs, size, false)
				}
				(_, Size::Byte) => {
					self.unary_with_imm(0xc6, Some(0), lhs, false, rhs, Size::Byte, false)
				}
				(_, Size::Word | Size::Dword) => {
					self.unary_with_imm(0xc7, Some(0), lhs, false, rhs, size, false)
				}
				(_, Size::Qword) => {
					self.unary_with_imm(0xc7, Some(0), lhs, false, rhs, Size::Dword, true)
				}
			}
		} else if let Ok(rhs) = self.resolve_effective_address(rvalue) {
			// https://www.felixcloutier.com/x86/lea
			if !lhs.is_register() {
				Err("the left-hand side must be a register")?
			}
			if matches!(lhs.size(), Some(Size::Byte)) {
				Err("an 8-bit register cannot be used in this operation")?
			}
			self.binary(0x8d, false, lhs, Place::Memory(rhs, lhs.size()), false)
		} else if self.globals.contains_key(rvalue) || self.labels.contains_key(rvalue) {
			let rhs = Place::RipRelative(rvalue, Displacement::Byte(0), Some(Size::Qword));
			self.binary(0x8d, false, lhs, rhs, false)
		} else if let Some(rest) = rvalue.strip_prefix("0:") {
			// https://www.felixcloutier.com/x86/movzx
			let rhs = Place::parse(rest).map_err(|_| "rhs must be a place expression here")?;
			let opcode = match (lhs, rhs.assert_size()?) {
				(Place::Reg16(_) | Place::Reg32(_) | Place::Reg64(_), Size::Byte) => 0xb6,
				(Place::Reg32(_) | Place::Reg64(_), Size::Word) => 0xb7,
				_ => Err(
					"a zero-extended move must involve a u8 or u16-sized place being moved into a register",
				)?,
			};
			self.binary_with_size(opcode, true, rhs, lhs, false, lhs.size().unwrap())
		} else if let Some(rest) = rvalue.strip_prefix("s:") {
			// https://www.felixcloutier.com/x86/movsx:movsxd
			let rhs = Place::parse(rest).map_err(|_| "rhs must be a place expression here")?;
			let (opcode, is_ext) = match (lhs, rhs.assert_size()?) {
				(Place::Reg16(_) | Place::Reg32(_) | Place::Reg64(_), Size::Byte) => (0xbe, true),
				(Place::Reg32(_) | Place::Reg64(_), Size::Word) => (0xbf, true),
				(Place::Reg64(_), Size::Dword) => (0x63, false),
				_ => Err(
					"a sign-extended move must involve a u8, u16, or u32-sized place being moved into a register",
				)?,
			};
			self.binary_with_size(opcode, is_ext, rhs, lhs, false, lhs.size().unwrap())
		} else if let Some(setcc_opcode) = setcc_opcode(rvalue) {
			// https://www.felixcloutier.com/x86/setcc
			if !matches!(lhs.size(), Some(Size::Byte) | None) {
				Err("this operation requires a 1-byte place")?;
			}
			self.unary_with_size(setcc_opcode, true, Some(7), lhs, true, Size::Byte)
		} else if let Some((place_str, condition)) = rvalue.split_once(" if ") {
			// https://www.felixcloutier.com/x86/cmovcc
			let opcode =
				cmovcc_opcode(condition.trim()).ok_or("invalid condition in conditional move")?;
			if !matches!(lhs, Place::Reg16(_) | Place::Reg32(_) | Place::Reg64(_)) {
				Err(
					"only a 16-bit, 32-bit, or 64-bit register can be used as the target of a conditional move",
				)?;
			}
			let rhs = Place::parse(place_str).map_err(|e| {
				format!("could not parse place expression in conditional move ({e})")
			})?;
			self.binary(opcode, true, rhs, lhs, false)
		} else {
			// Try to match multiplication
			for (pos, _) in rvalue.match_indices(" * ") {
				let (first, rest) = rvalue.split_at(pos);
				let (first, second) = (first.trim(), rest[3..].trim());
				if Place::parse(first).is_ok() || Place::parse(second).is_ok() {
					if !lhs.is_register() {
						Err("this operation requires a 16-bit, 32-bit, or 64-bit register")?;
					}
					// One must be a place, the other must be an immediate
					let (rhs, imm) = if let Ok(p) = Place::parse(first) {
						let msg = || format!("could not resolve immediate: {second}");
						(p, self.try_resolve_imm(second)?.ok_or_else(msg)?)
					} else if let Ok(p) = Place::parse(second) {
						let msg = || format!("could not resolve immediate: {first}");
						(p, self.try_resolve_imm(first)?.ok_or_else(msg)?)
					} else {
						Err(
							"this operation requires a place and an immediate; e.g., rax = rdi * 934",
						)?
					};

					let size = lhs.coerce_size(rhs)?;
					let (opcode, imm_size) = match size {
						_ if imm.cast_u8().is_ok() => (0x6b, Size::Byte),
						Size::Word => (0x69, Size::Word),
						_ => (0x69, Size::Dword),
					};
					return self
						.binary_with_imm(opcode, false, rhs, lhs, false, size, imm, imm_size);
				}
			}
			Err("could not parse assignment expression")?
		}
	}

	fn try_resolve_imm(&self, s: &str) -> Result<Option<Immediate>, String> {
		let s = s.trim();
		if let Some(rest) = s.strip_prefix("@") {
			let (name, rest) = rest.split_once("(").ok_or("missing opening bracket")?;
			let rest = rest.strip_suffix(")").ok_or("missing closing bracket")?;
			match name {
				"f64" => Ok(Some(Immediate::ImmU64(
					rest.parse::<f64>()
						.map_err(|_| "could not parse f64")?
						.to_bits() as _,
				))),
				"f32" => Ok(Some(Immediate::ImmU32(
					rest.parse::<f32>()
						.map_err(|_| "could not parse f32")?
						.to_bits() as _,
				))),
				"len" => {
					let entry = self
						.globals
						.get(rest)
						.ok_or_else(|| format!("{rest} is not defined as a static"))?;

					Ok(Some(Immediate::ImmU64(entry.0.len() as u64)))
				}
				_ => Err("invalid builtin; valid options here are @f64(), @f32(), or @len()")?,
			}
		} else if let Ok(data) = parse_string(s) {
			if let Ok(val) = <[u8; 1]>::try_from(&*data) {
				Ok(Some(Immediate::ImmU8(u8::from_le_bytes(val))))
			} else if let Ok(val) = <[u8; 2]>::try_from(&*data) {
				Ok(Some(Immediate::ImmU16(u16::from_le_bytes(val))))
			} else if let Ok(val) = <[u8; 4]>::try_from(&*data) {
				Ok(Some(Immediate::ImmU32(u32::from_le_bytes(val))))
			} else if let Ok(val) = <[u8; 8]>::try_from(&*data) {
				Ok(Some(Immediate::ImmU64(u64::from_le_bytes(val))))
			} else {
				Ok(Some(Immediate::Unsized(data)))
			}
		} else if let Some(rest) = s.strip_prefix("0x") {
			if let Ok(val) = u8::from_str_radix(rest, 16) {
				Ok(Some(Immediate::ImmU8(val)))
			} else if let Ok(val) = u16::from_str_radix(rest, 16) {
				Ok(Some(Immediate::ImmU16(val)))
			} else if let Ok(val) = u32::from_str_radix(rest, 16) {
				Ok(Some(Immediate::ImmU32(val)))
			} else if let Ok(val) = u64::from_str_radix(rest, 16) {
				Ok(Some(Immediate::ImmU64(val)))
			} else if rest.chars().all(|c| c.is_ascii_hexdigit()) {
				if rest.len() % 2 == 1 {
					Err("hex string must have an even number of digits")?
				}
				let v: Vec<u8> = rest
					.as_bytes()
					.rchunks_exact(2)
					.map(|chunk| std::str::from_utf8(chunk).unwrap())
					.map(|s| u8::from_str_radix(s, 16).unwrap())
					.collect();
				Ok(Some(Immediate::Unsized(v)))
			} else {
				Err("invalid hex string")?
			}
		} else if let Ok(val) = s.parse() {
			Ok(Some(Immediate::Imm8(val)))
		} else if let Ok(val) = s.parse() {
			Ok(Some(Immediate::Imm16(val)))
		} else if let Ok(val) = s.parse() {
			Ok(Some(Immediate::Imm32(val)))
		} else if let Ok(val) = s.parse() {
			Ok(Some(Immediate::Imm64(val)))
		} else if let Ok(val) = s.parse() {
			Ok(Some(Immediate::ImmU64(val)))
		} else {
			Ok(None)
		}
	}

	fn parse_divmod(&mut self, inner: &str, signed: bool) -> Result<(), String> {
		let (lhs, rhs) = inner
			.split_once(", ")
			.ok_or("@divmod must be used with two arguments (e.g. `@divmod(edx:eax, rcx))")?;

		let place = Place::parse(rhs.trim())
			.map_err(|_| "the second argument must be a place expression")?;

		match place.assert_size()? {
			Size::Byte if lhs != "ah:al" => Err("first argument must be ah:al for 8-bit operand")?,
			Size::Word if lhs != "dx:ax" => Err("first argument must be dx:ax for 16-bit operand")?,
			Size::Dword if lhs != "edx:eax" => {
				Err("first argument must be edx:eax for 32-bit operand")?
			}
			Size::Qword if lhs != "rdx:rax" => {
				Err("first argument must be rdx:rax for 64-bit operand")?
			}
			_ => {}
		};

		let ext = if signed { 7 } else { 6 };
		self.parse_basic_unary(rhs, 0xf6, ext, false)
	}

	fn parse_call_setup(&mut self, inner: &str) -> Result<(), String> {
		if !inner.is_empty() {
			for parts in inner.split(", ") {
				if let Some((lvalue, rvalue)) = parts.split_once(" ^= ") {
					self.parse_xor(lvalue, rvalue)?;
				} else if let Some((lvalue, rvalue)) = parts.split_once(" = ") {
					self.parse_assign(lvalue, rvalue)?;
				} else {
					Err(
						"calls must only include assignment or XOR statements separated by commas (e.g, `@syscall(eax = 60, edi ^= edi)`",
					)?
				};
			}
		}
		Ok(())
	}

	fn parse_widen_mul(&mut self, inner: &str, signed: bool) -> Result<(), String> {
		let (first, second) = inner
			.split_once(",")
			.ok_or("this operation requires two comma-separated arguments")?;
		let target = Place::parse(second.trim())
			.map_err(|e| format!("could not parse second argument place expression ({e})"))?;
		let (opcode, size) = match (first.trim(), target.size()) {
			("ah:al", Some(Size::Byte) | None) => (0xf6, Size::Byte),
			("dx:ax", Some(Size::Word) | None) => (0xf7, Size::Word),
			("edx:eax", Some(Size::Dword) | None) => (0xf7, Size::Dword),
			("rdx:rax", Some(Size::Qword) | None) => (0xf7, Size::Qword),
			(_, Some(Size::Byte)) => Err("the first argument must be ah:al for an 8-bit multiply")?,
			(_, Some(Size::Word)) => Err("the first argument must be dx:ax for a 16-bit multiply")?,
			(_, Some(Size::Dword)) => {
				Err("the first argument must be edx:eax for a 32-bit multiply")?
			}
			(_, Some(Size::Qword)) => {
				Err("the first argument must be rdx:rax for a 64-bit multiply")?
			}
			(_, None) => Err(
				"invalid arguments; the first argument must be either ah:al, dx:ax, edx:eax,\
				rdx:rax for an 8-bit, 16-bit, 32-bit, or 64-bit second argument respectively",
			)?,
		};
		let ext = if signed { 5 } else { 4 };
		self.unary_with_size(opcode, false, Some(ext), target, false, size)?;
		Ok(())
	}

	fn parse_builtin_function(
		&mut self,
		inner: &str,
		encoding: &[u8],
		atomic: bool,
	) -> Result<(), String> {
		assert_not_atomic(atomic)?;
		self.parse_call_setup(inner)?;
		self.code.extend(encoding);
		Ok(())
	}

	fn parse_xchg(&mut self, inner: &str) -> Result<(), String> {
		let (first, second) = inner
			.split_once(",")
			.ok_or("this operation requires two comma-separated arguments")?;

		let lhs = Place::parse(first.trim())
			.map_err(|e| format!("could not parse first argument place expression ({e})"))?;
		let rhs = Place::parse(second.trim())
			.map_err(|e| format!("could not parse second argument place expression ({e})"))?;
		let size = lhs.coerce_size(rhs)?;

		match (lhs, rhs) {
			(Place::Reg16(Ax), p)
			| (p, Place::Reg16(Ax))
			| (Place::Reg32(Eax), p)
			| (p, Place::Reg32(Eax))
			| (Place::Reg64(Rax), p)
			| (p, Place::Reg64(Rax))
				if p.is_register() =>
			{
				let opcode = 0x90 + (p.register().unwrap().id() & 7);
				self.unary_with_size(opcode, false, None, rhs, false, size)
			}
			_ if size == Size::Byte => self.binary_with_size(0x86, false, lhs, rhs, false, size),
			_ => self.binary_with_size(0x87, false, lhs, rhs, false, size),
		}
	}

	fn parse_xadd(&mut self, inner: &str, atomic: bool) -> Result<(), String> {
		let (first, second) = inner
			.split_once(",")
			.ok_or("this operation requires two comma-separated arguments")?;

		let lhs = Place::parse(first.trim())
			.map_err(|e| format!("could not parse first argument place expression ({e})"))?;
		let rhs = Place::parse(second.trim())
			.map_err(|e| format!("could not parse second argument place expression ({e})"))?;
		rhs.register()
			.ok_or("the second argument must be a register in this operation")?;
		if atomic {
			Place::assert_atomic_compatible(lhs, rhs)?;
			self.code.push(0xf0);
		}
		let size = lhs.coerce_size(rhs)?;
		let opcode = if size == Size::Byte { 0xc0 } else { 0xc1 };
		self.binary_with_size(opcode, true, lhs, rhs, false, size)
	}

	fn parse_cmpxchg(&mut self, inner: &str, atomic: bool) -> Result<(), String> {
		let (first, rest) = inner
			.split_once(",")
			.ok_or("this operation requires three comma-separated arguments")?;
		let (second, third) = rest
			.split_once(",")
			.ok_or("this operation requires three comma-separated arguments")?;
		let lhs = Place::parse(first.trim())
			.map_err(|e| format!("could not parse first argument place expression ({e})"))?;
		let rhs = Place::parse(second.trim())
			.map_err(|e| format!("could not parse second argument place expression ({e})"))?;
		rhs.register()
			.ok_or("the second argument must be a register in this operation")?;
		if atomic {
			Place::assert_atomic_compatible(lhs, rhs)?;
			self.code.push(0xf0);
		}
		let size = lhs.coerce_size(rhs)?;
		match (size, third.trim()) {
			(Size::Byte, "al")
			| (Size::Word, "ax")
			| (Size::Dword, "eax")
			| (Size::Qword, "rax") => {}
			(Size::Byte, _) => Err("the third argument must be `al` for this 8-bit operation")?,
			(Size::Word, _) => Err("the third argument must be `ax` for this 16-bit operation")?,
			(Size::Dword, _) => Err("the third argument must be `eax` for this 32-bit operation")?,
			(Size::Qword, _) => Err("the third argument must be `rax` for this 64-bit operation")?,
		};
		let opcode = if size == Size::Byte { 0xb0 } else { 0xb1 };
		self.binary_with_size(opcode, true, lhs, rhs, false, size)
	}

	fn parse_builtin(&mut self, name: &str, inner: &str, atomic: bool) -> Result<(), String> {
		let inner = inner.trim();

		match name {
			"syscall" => self.parse_builtin_function(inner, &[0x0f, 0x05], atomic)?,
			"raw" => {
				assert_not_atomic(atomic)?;
				match parse_string(inner) {
					Ok(raw) => self.code.extend(raw),
					Err(e) => Err(format!("@raw() must contain a valid string ({e})"))?,
				}
			}
			"random" => {
				assert_not_atomic(atomic)?;
				if let Ok(reg) = Place::parse(inner) {
					if matches!(reg, Place::Reg16(_) | Place::Reg32(_) | Place::Reg64(_)) {
						return self.unary(0xc7, true, Some(6), reg, false);
					}
				}
				Err("rdrand must be used with a 16-bit, 32-bit, or 64-bit register")?;
			}
			"random_seed" => {
				assert_not_atomic(atomic)?;
				if let Ok(reg) = Place::parse(inner) {
					if matches!(reg, Place::Reg16(_) | Place::Reg32(_) | Place::Reg64(_)) {
						return self.unary(0xc7, true, Some(7), reg, false);
					}
				}
				Err("rdseed must be used with a 16-bit, 32-bit, or 64-bit register")?;
			}
			"negate" => self.parse_neg(inner, atomic)?,
			"not" => self.parse_not(inner, atomic)?,
			"divmod" => {
				assert_not_atomic(atomic)?;
				self.parse_divmod(inner, true)?;
			}
			"unsigned_divmod" => {
				assert_not_atomic(atomic)?;
				self.parse_divmod(inner, false)?;
			}
			"widen_mul" => {
				assert_not_atomic(atomic)?;
				self.parse_widen_mul(inner, true)?;
			}
			"unsigned_widen_mul" => {
				assert_not_atomic(atomic)?;
				self.parse_widen_mul(inner, false)?;
			}
			"swap" => {
				if atomic {
					Err("@swap is always atomic; adding `atomically` is redundant")?;
				}
				self.parse_xchg(inner)?
			}
			"swap_add" => self.parse_xadd(inner, atomic)?,
			"try_replace" => self.parse_cmpxchg(inner, atomic)?,
			"set_flags" => {
				assert_not_atomic(atomic)?;
				// There are two modes: lvalue - rvalue and lvalue & rvalue
				if let Some((first, rest)) = inner.split_once(" & ") {
					return self.parse_test(first, rest);
				}

				// " - " might appear within a place so do trial and error
				for (pos, _) in inner.match_indices(" - ") {
					let (first, rest) = inner.split_at(pos);
					let (first, rest) = (first.trim(), rest[3..].trim());
					if Place::parse(first).is_ok() {
						return self.parse_cmp(first, rest);
					}
				}

				let msg = "@set_flags must be used with two arguments combined with \" - \" or \" & \", e.g. @set_flags(rax - rdi[5])";
				Err(msg)?
			}
			"set_direction" => {
				assert_not_atomic(atomic)?;
				let opcode = match inner {
					"forwards" | "forward" => 0xfc,
					"backwards" | "backward" => 0xfd,
					_ => Err("invalid value; allowed parameters are 'forwards' and 'backwards'")?,
				};
				self.code.push(opcode);
			}
			"fill_u8" => self.parse_builtin_function(inner, &[0xf3, 0xaa], atomic)?, // rep stosb
			"fill_u16" => self.parse_builtin_function(inner, &[0x66, 0xf3, 0xab], atomic)?, // rep stosw
			"fill_u32" => self.parse_builtin_function(inner, &[0xf3, 0xab], atomic)?, // rep stosd
			"fill_u64" => self.parse_builtin_function(inner, &[0xf3, 0x48, 0xaa], atomic)?, // rep stosq
			"copy_u8" => self.parse_builtin_function(inner, &[0xf3, 0xa4], atomic)?, // rep movsb
			"copy_u16" => self.parse_builtin_function(inner, &[0x66, 0xf3, 0xa5], atomic)?, // rep movsw
			"copy_u32" => self.parse_builtin_function(inner, &[0xf3, 0xa5], atomic)?, // rep movsd
			"copy_u64" => self.parse_builtin_function(inner, &[0xf3, 0x48, 0xa5], atomic)?, // rep movsq
			"compare_u8" => self.parse_builtin_function(inner, &[0xf3, 0xa6], atomic)?, // repe cmpsb
			"compare_u16" => self.parse_builtin_function(inner, &[0x66, 0xf3, 0xa7], atomic)?, // repe cmpsw
			"compare_u32" => self.parse_builtin_function(inner, &[0xf3, 0xa7], atomic)?, // repe cmpsd
			"compare_u64" => self.parse_builtin_function(inner, &[0xf3, 0xf8, 0xa7], atomic)?, // repe cmpsq
			_ => Err(format!("invalid command: {name}"))?,
		}
		Ok(())
	}

	fn parse_basic(
		&mut self,
		lvalue: &str,
		rvalue: &str,
		opcode1: u8,
		ext: u8,
		opcode2: u8,
	) -> Result<(), String> {
		let lvalue = lvalue.trim();
		let rvalue = rvalue.trim();
		let lhs = Place::parse(lvalue).map_err(|_| "could not parse lvalue place expression")?;
		// basic binary expression can be used atomically
		let (rvalue, atomic) = extract_atomic_suffix(rvalue);

		if let Some(imm) = self.try_resolve_imm(rvalue)? {
			if atomic && lhs.is_register() {
				Err("atomic operations can only be performed when one operand accesses memory")?;
			} else if atomic {
				self.code.push(0xf0);
			}
			// If the immediate can fit in one byte, we can use special encodings to save bytes.
			let imm_is_1_byte = imm.cast_i8().is_ok();
			if matches!(lhs, Place::Reg8(Al)) {
				let val = imm.cast_u8()?;
				self.code.extend([opcode1, val as u8]);
			} else if matches!(lhs, Place::Reg16(Ax)) && !imm_is_1_byte {
				let val = imm.cast_u16()?;
				self.code.extend([0x66, opcode1 + 1]);
				self.code.extend(val.to_le_bytes());
			} else if matches!(lhs, Place::Reg32(Eax)) && !imm_is_1_byte {
				let val = imm.cast_u32()?;
				self.code.push(0x35);
				self.code.extend(val.to_le_bytes());
			} else if matches!(lhs, Place::Reg64(Rax)) && !imm_is_1_byte {
				let val = imm.cast_i32()?;
				self.code.extend([0x48, opcode1 + 1]);
				self.code.extend(val.to_le_bytes());
			} else {
				match (lhs.assert_size()?, imm_is_1_byte) {
					(Size::Byte, _) => {
						self.unary_with_imm(0x80, Some(ext), lhs, false, &imm, Size::Byte, false)?
					}
					(Size::Word, true) => {
						self.unary_with_imm(0x83, Some(ext), lhs, false, &imm, Size::Byte, true)?
					}
					(Size::Word, false) => {
						self.unary_with_imm(0x81, Some(ext), lhs, false, &imm, Size::Word, false)?
					}
					(Size::Dword, true) => {
						self.unary_with_imm(0x83, Some(ext), lhs, false, &imm, Size::Byte, true)?
					}
					(Size::Dword, false) => {
						self.unary_with_imm(0x81, Some(ext), lhs, false, &imm, Size::Dword, false)?
					}
					(Size::Qword, true) => {
						self.unary_with_imm(0x83, Some(ext), lhs, false, &imm, Size::Byte, true)?
					}
					(Size::Qword, false) => {
						self.unary_with_imm(0x81, Some(ext), lhs, false, &imm, Size::Dword, true)?
					}
				}
			}
		} else if let Ok(rhs) = Place::parse(rvalue) {
			if atomic {
				Place::assert_atomic_compatible(lhs, rhs)?;
				self.code.push(0xf0);
			}
			let size = lhs.coerce_size(rhs)?;
			let opcode = match (rhs.is_register(), size) {
				(true, Size::Byte) => opcode2,
				(true, _) => opcode2 + 1,
				(false, Size::Byte) => opcode2 + 2,
				(false, _) => opcode2 + 3,
			};
			self.binary_with_size(opcode, false, lhs, rhs, false, size)?
		} else {
			Err("could not parse rvalue place expression")?
		}
		Ok(())
	}

	fn parse_basic_shift(&mut self, lvalue: &str, rvalue: &str, ext: u8) -> Result<(), String> {
		let (lvalue, rvalue) = (lvalue.trim(), rvalue.trim());
		let lhs = Place::parse(lvalue)
			.map_err(|e| format!("could not parse place expression: {lvalue} ({e})"))?;

		let size = lhs.assert_size()?;
		let is_cl = matches!(rvalue.parse::<Register>(), Ok(Cl));
		let maybe_imm8 = self.try_resolve_imm(rvalue)?;
		let is_1 = matches!(maybe_imm8.as_ref().and_then(|i| i.cast_u8().ok()), Some(1));

		if let Some(imm) = &maybe_imm8 {
			imm.strict_cast_u8()?;
		}

		let opcode = if is_1 {
			0xd0
		} else if is_cl {
			0xd2
		} else if maybe_imm8.is_some() {
			0xc0
		} else {
			Err("the right-hand side must either be an immediate or the cl register")?
		};

		match (size, maybe_imm8, is_1) {
			(Size::Byte, Some(imm), false) => {
				self.unary_with_imm(opcode, Some(ext), lhs, false, &imm, Size::Byte, false)
			}
			(_, Some(imm), false) => {
				self.unary_with_imm(opcode + 1, Some(ext), lhs, false, &imm, Size::Byte, false)
			}
			(Size::Byte, _, _) => self.unary(opcode, false, Some(ext), lhs, false),
			(_, _, _) => self.unary(opcode + 1, false, Some(ext), lhs, false),
		}
	}

	fn parse_imul_inplace(&mut self, lvalue: &str, rvalue: &str) -> Result<(), String> {
		let lvalue = lvalue.trim();
		let rvalue = rvalue.trim();
		if let Ok(lhs) = Place::parse(lvalue) {
			if matches!(lhs, Place::Reg16(_) | Place::Reg32(_) | Place::Reg64(_)) {
				let rhs = Place::parse(rvalue)
					.map_err(|e| format!("could not parse rvalue place expression ({e})"))?;
				let size = lhs.coerce_size(rhs)?;
				return self.binary_with_size(0xaf, true, rhs, lhs, false, size);
			}
		}
		Err("only 16-bit, 32-bit, or 64-bit registers can be used as the target of this operation")?
	}

	fn parse_xor(&mut self, lvalue: &str, rvalue: &str) -> Result<(), String> {
		self.parse_basic(lvalue, rvalue, 0x34, 6, 0x30)
	}

	fn parse_and(&mut self, lvalue: &str, rvalue: &str) -> Result<(), String> {
		self.parse_basic(lvalue, rvalue, 0x24, 4, 0x20)
	}

	fn parse_or(&mut self, lvalue: &str, rvalue: &str) -> Result<(), String> {
		self.parse_basic(lvalue, rvalue, 0x0c, 1, 0x08)
	}

	fn parse_cmp(&mut self, lvalue: &str, rvalue: &str) -> Result<(), String> {
		self.parse_basic(lvalue, rvalue, 0x3c, 7, 0x38)
	}

	fn parse_test(&mut self, lvalue: &str, rvalue: &str) -> Result<(), String> {
		let opcode1 = 0xa8;
		let ext = 0;
		let opcode2 = 0x84;

		let lvalue = lvalue.trim();
		let rvalue = rvalue.trim();
		let lhs = Place::parse(lvalue).map_err(|_| "could not parse lvalue place expression")?;

		if let Some(imm) = self.try_resolve_imm(rvalue)? {
			if matches!(lhs, Place::Reg8(Al)) {
				let val = imm.cast_u8()?;
				self.code.extend([opcode1, val as u8]);
			} else if matches!(lhs, Place::Reg16(Ax)) {
				let val = imm.cast_u16()?;
				self.code.extend([0x66, opcode1 + 1]);
				self.code.extend(val.to_le_bytes());
			} else if matches!(lhs, Place::Reg32(Eax)) {
				let val = imm.cast_u32()?;
				self.code.push(0x35);
				self.code.extend(val.to_le_bytes());
			} else if matches!(lhs, Place::Reg64(Rax)) {
				let val = imm.cast_i32()?;
				self.code.extend([0x48, opcode1 + 1]);
				self.code.extend(val.to_le_bytes());
			} else {
				let size = lhs.assert_size()?;
				match size {
					Size::Byte => {
						self.unary_with_imm(0x80, Some(ext), lhs, false, &imm, size, false)?
					}
					Size::Word | Size::Dword => {
						self.unary_with_imm(0x81, Some(ext), lhs, false, &imm, size, false)?
					}
					Size::Qword => {
						self.unary_with_imm(0x81, Some(ext), lhs, false, &imm, Size::Dword, true)?
					}
				}
			}
		} else if let Ok(rhs) = Place::parse(rvalue) {
			match (rhs.is_register(), rhs.assert_size()?) {
				(true, Size::Byte) => self.binary(opcode2, false, lhs, rhs, false)?,
				(true, _) => self.binary(opcode2 + 1, false, lhs, rhs, false)?,
				(false, _) => Err(
					"in this expression, the right-hand value must be either a register or immediate",
				)?,
			};
		} else {
			Err("could not parse rvalue place expression")?
		}
		Ok(())
	}

	fn parse_add(&mut self, lvalue: &str, rvalue: &str) -> Result<(), String> {
		self.parse_basic(lvalue, rvalue, 0x04, 0, 0x00)
	}

	fn parse_sub(&mut self, lvalue: &str, rvalue: &str) -> Result<(), String> {
		self.parse_basic(lvalue, rvalue, 0x2c, 5, 0x28)
	}

	#[expect(unused)]
	fn parse_adc(&mut self, lvalue: &str, rvalue: &str) -> Result<(), String> {
		self.parse_basic(lvalue, rvalue, 0x14, 2, 0x10)
	}

	#[expect(unused)]
	fn parse_sbb(&mut self, lvalue: &str, rvalue: &str) -> Result<(), String> {
		self.parse_basic(lvalue, rvalue, 0x1C, 3, 0x18)
	}

	fn parse_static(&mut self, rest: &str) -> Result<(), String> {
		let (first, rest) = rest
			.split_once('=')
			.ok_or("could not identify variable name")?;
		let name = self.check_identifier(first)?.to_owned();
		let data = self.try_resolve_imm(rest)?.ok_or("invalid value")?;
		self.globals.insert(name, (data.cast_unsized(), vec![]));
		Ok(())
	}

	fn parse_function(&mut self, rest: &str) -> Result<(), String> {
		let rest = rest.strip_suffix("{").ok_or("missing \"{\"")?.trim();
		let rest = rest.strip_suffix("()").ok_or("missing \"()\"")?.trim();
		let ident = self.check_identifier(rest)?.to_owned();
		self.labels.insert(ident, (usize::MAX, Vec::new())); // dummy value
		Ok(())
	}

	fn parse_label(&mut self, rest: &str) -> Result<(), String> {
		let ident = self.check_identifier(rest.trim())?.to_owned();
		self.labels.insert(ident, (usize::MAX, Vec::new())); // dummy value
		Ok(())
	}

	fn parse_block_label(&mut self, rest: &str) -> Result<(), String> {
		let label = rest.trim().strip_suffix(":").ok_or("unexpected {")?;
		self.parse_label(label)
	}

	fn parse_global(&mut self, line: &str) -> Result<(), String> {
		if line == "{" {
			// nothing
		} else if let Some(rest) = line.strip_prefix("static ") {
			self.parse_static(rest)?;
		} else if let Some(rest) = line.strip_prefix("function ") {
			self.parse_function(rest)?;
		} else if let Some(rest) = line.strip_suffix(":") {
			self.parse_label(rest)?;
		} else if let Some(rest) = line.strip_suffix("{") {
			self.parse_block_label(rest)?;
		}
		Ok(())
	}

	fn parse_line(&mut self, line: &str) -> Result<(), String> {
		let line = line.trim();

		if line == "{" {
			self.blocks_stack.push((self.code.len(), Vec::new()));
		} else if line == "}" {
			let (_, references) = self.blocks_stack.pop().ok_or("encountered trailing }")?;
			let global_offset = self.code.len();
			for location in references {
				let location_slice = &mut self.code[location - 4..location];
				let disp = i32::try_from(global_offset - location).unwrap();
				location_slice.copy_from_slice(&disp.to_le_bytes());
			}
		} else if line == "trap" {
			self.code.extend([0x0f, 0x0b]); // ud2
		} else if line == "pause" {
			self.code.extend([0xf3, 0x90]); // pause
		} else if line == "return" {
			self.code.push(0xc3); // ret
		} else if let Some(rest) = line.strip_prefix("@") {
			let (rest, atomic) = extract_atomic_suffix(rest);
			let (name, rest) = rest.split_once("(").ok_or("invalid command")?;
			let inner = rest.strip_suffix(")").ok_or("missing closing bracket")?;
			self.parse_builtin(name, inner, atomic)?;
		} else if let Some(_) = line.strip_prefix("static ") {
			// already handled in `parse_global`...
		} else if let Some(rest) = line.strip_prefix("function ") {
			// assume that a { exists
			self.blocks_stack.push((self.code.len(), Vec::new()));
			// The function line should already be correctly parsed. Now we just need
			// to record its location within the code.
			let name = rest.split_once("(").unwrap().0;
			self.labels.get_mut(name).unwrap().0 = self.code.len();
		} else if let Some(rest) = line.strip_suffix("{") {
			self.blocks_stack.push((self.code.len(), Vec::new()));
			// The label line should already be correctly parsed.
			let name = rest.trim().strip_suffix(":").unwrap().trim();
			self.labels.get_mut(name).unwrap().0 = self.code.len();
		} else if let Some(rest) = line.strip_suffix(":") {
			// The label line should already be correctly parsed.
			let name = rest.trim();
			self.labels.get_mut(name).unwrap().0 = self.code.len();
		} else if let Some(rest) = line.strip_prefix("<- ") {
			error_if_atomic(line)?;
			self.parse_push(rest)?;
		} else if let Some(rest) = line.strip_prefix("-> ") {
			error_if_atomic(line)?;
			self.parse_pop(rest)?;
		} else if let Some(rest) = line.strip_prefix("goto") {
			self.parse_goto(rest)?;
		} else if let Some(rest) = line.strip_prefix("continue") {
			self.parse_continue(rest)?;
		} else if let Some(rest) = line.strip_prefix("break") {
			self.parse_break(rest)?;
		} else if let Some(rest) = line.strip_suffix("++") {
			self.parse_inc(rest, false)?;
		} else if let Some(rest) = line.strip_suffix("--") {
			self.parse_dec(rest, false)?;
		} else if let Some((name, inner, is_virtual)) = Program::parse_function_call(line) {
			if is_virtual {
				self.parse_virtual_call(name, inner)?;
			} else {
				self.parse_call(name, inner)?;
			}
		} else if let Some((lvalue, rvalue)) = line.split_once(" <<= ") {
			error_if_atomic(line)?;
			self.parse_basic_shift(lvalue, rvalue, 4)?;
		} else if let Some((lvalue, rvalue)) = line.split_once(" >>= ") {
			error_if_atomic(line)?;
			self.parse_basic_shift(lvalue, rvalue, 7)?;
		} else if let Some((lvalue, rvalue)) = line.split_once(" >>>= ") {
			error_if_atomic(line)?;
			self.parse_basic_shift(lvalue, rvalue, 5)?;
		} else if let Some((lvalue, rvalue)) = line.split_once(" *= ") {
			error_if_atomic(line)?;
			self.parse_imul_inplace(lvalue, rvalue)?;
		} else if let Some((lvalue, rvalue)) = line.split_once(" ^= ") {
			self.parse_xor(lvalue, rvalue)?;
		} else if let Some((lvalue, rvalue)) = line.split_once(" |= ") {
			self.parse_or(lvalue, rvalue)?;
		} else if let Some((lvalue, rvalue)) = line.split_once(" &= ") {
			self.parse_and(lvalue, rvalue)?;
		} else if let Some((lvalue, rvalue)) = line.split_once(" += ") {
			self.parse_add(lvalue, rvalue)?;
		} else if let Some((lvalue, rvalue)) = line.split_once(" -= ") {
			self.parse_sub(lvalue, rvalue)?;
		} else if let Some((lvalue, rvalue)) = line.split_once(" = ") {
			error_if_atomic(line)?;
			self.parse_assign(lvalue, rvalue)?;
		} else if let Some(rest) = line.strip_suffix(" atomically") {
			if let Some(rest) = rest.strip_suffix("++") {
				self.parse_inc(rest, true)?;
			} else if let Some(rest) = rest.strip_suffix("--") {
				self.parse_dec(rest, true)?;
			} else {
				Err("unexpected `atomically` keyword")?;
			}
		} else {
			Err("invalid command")?;
		}
		Ok(())
	}

	fn resolve_effective_address(&self, mut s: &str) -> Result<EffectiveAddress, String> {
		let mut term1 = None::<(Scale, Register)>;
		let mut term2 = None::<(Scale, Register)>;
		let mut disp = None;
		let mut curr_negative = false;

		// leading negative sign is a special case
		while !s.is_empty() {
			if s.as_bytes()[0] == b'-' {
				curr_negative = !curr_negative;
				s = s[1..].trim();
			}

			let (curr, next_is_negative) = if let Some((first, rest)) = s.split_once(&['+', '-']) {
				let matched_char = s.as_bytes()[first.len()];
				s = rest.trim();
				(first.trim(), matched_char == b'-')
			} else {
				let rest = s;
				s = "";
				(rest.trim(), false)
			};

			if curr_negative {
				let imm = self
					.try_resolve_imm(curr)?
					.ok_or("the `-` symbol cannot be used with a register")?;
				if disp.is_some() {
					Err("only one immediate is allowed in an expression")?;
				} else if let Ok(val) = imm.cast_i8() {
					disp = Some(Displacement::Byte(val.wrapping_neg()));
				} else if let Ok(val) = imm.cast_i32() {
					disp = Some(Displacement::Dword(val.wrapping_neg()));
				} else {
					Err("immediate cannot be more than 32 bits")?;
				}
			} else if let Some(imm) = self.try_resolve_imm(curr)? {
				if disp.is_some() {
					Err("only one immediate is allowed in an expression")?;
				} else if let Ok(val) = imm.cast_i8() {
					disp = Some(Displacement::Byte(val));
				} else if let Ok(val) = imm.cast_i32() {
					disp = Some(Displacement::Dword(val));
				} else {
					Err("immediate cannot be more than 32 bits")?;
				}
			} else {
				match parse_multiplier_and_register(curr) {
					Ok((scale, reg)) => {
						match (term1, term2) {
							(None, _) => term1 = Some((scale, reg)),
							(_, None) => term2 = Some((scale, reg)),
							_ => Err("only up to two registers are allowed")?,
						};
					}
					Err(e) => Err(format!("could not parse `{curr}` ({e})"))?,
				}
			}

			curr_negative = next_is_negative;
		}

		let (base, index) = match (term1, term2) {
			(None, None) => Err("at least one register must be specified")?,
			(Some((_, Rsp)), Some((_, Rsp))) => Err("rsp cannot be used twice")?,
			(Some((Scale::One, Rsp)), idx) | (idx, Some((Scale::One, Rsp))) => (Some(Rsp), idx),
			(Some((_, Rsp)), _) | (_, Some((_, Rsp))) => {
				Err("rsp cannot be shifted or multiplied")?
			}
			(Some((Scale::One, base)), idx) | (idx, Some((Scale::One, base))) => (Some(base), idx),
			(Some((scale, reg)), None) | (None, Some((scale, reg))) => (None, Some((scale, reg))),
			_ => Err("only one register can be scaled and multiplied")?,
		};

		Ok(EffectiveAddress { base, index, disp })
	}

	fn parse_from_file(filename: &str) -> Result<Self, String> {
		let mut program = Program::new();
		program.append_elf_header();
		program.append_program_header();

		let text = {
			let mut file =
				File::open(filename).map_err(|e| format!("Error opening {filename}: {e}"))?;
			let mut buf = String::new();
			file.read_to_string(&mut buf)
				.map_err(|e| format!("File could not be read: {e}"))?;
			buf
		};

		let lines = text.lines().map(|l| strip_comments(l).trim());

		// Preprocess
		for (n, line) in lines.clone().enumerate() {
			if !line.is_empty() {
				program.parse_global(line).map_err(|msg: String| {
					format!("Could not parse line {}:\n\t{line}\nError: {msg}", n + 1)
				})?;
			}
		}

		for (n, line) in lines.enumerate() {
			if !line.is_empty() {
				program.parse_line(line).map_err(|msg: String| {
					format!("Could not parse line {}:\n\t{line}\nError: {msg}", n + 1)
				})?;
			}
		}

		program.finalize()?;

		Ok(program)
	}
}

fn jmp_opcode(condition: &str, is_32_bit: bool) -> Result<(Option<u8>, u8), String> {
	// https://www.felixcloutier.com/x86/jcc
	let allowed = "if [/carry, !/carry, /parity, !/parity, /zero, \
		!/zero, /sign, !/sign, /overflow, !/overflow, /less, !/less, \
		/less_or_equal, !/less_or_equal, /carry_or_zero, \
		!/carry_or_zero, /rcx_zero, /ecx_zero]";

	let opcode = match (condition, is_32_bit) {
		("", false) => 0xeb,
		("", true) => 0xe9,
		("if /carry", false) => 0x72,
		("if /carry", true) => 0x82,
		("if !/carry", false) => 0x73,
		("if !/carry", true) => 0x83,
		("if /parity", false) => 0x7a,
		("if /parity", true) => 0x8a,
		("if !/parity", false) => 0x7b,
		("if !/parity", true) => 0x8b,
		("if /zero", false) => 0x74,
		("if /zero", true) => 0x84,
		("if !/zero", false) => 0x75,
		("if !/zero", true) => 0x85,
		("if /sign", false) => 0x78,
		("if /sign", true) => 0x88,
		("if !/sign", false) => 0x79,
		("if !/sign", true) => 0x89,
		("if /overflow", false) => 0x70,
		("if /overflow", true) => 0x80,
		("if !/overflow", false) => 0x71,
		("if !/overflow", true) => 0x81,
		("if /less", false) => 0x7C,
		("if /less", true) => 0x8C,
		("if !/less", false) => 0x7D,
		("if !/less", true) => 0x8D,
		("if /less_or_equal", false) => 0x7E,
		("if /less_or_equal", true) => 0x8E,
		("if !/less_or_equal", false) => 0x7F,
		("if !/less_or_equal", true) => 0x8F,
		("if /carry_or_zero", false) => 0x76,
		("if /carry_or_zero", true) => 0x86,
		("if !/carry_or_zero", false) => 0x77,
		("if !/carry_or_zero", true) => 0x87,
		("if /rcx_zero", true) => 0xe3,
		("if /ecx_zero", true) => 0xe3,
		("if /ecx_zero" | "if /rcx_zero", false) => Err(
			"due to x86 limitations, the /ecx_zero and /rcx_zero conditions can only be used \
			with 8-bit offsets, and the required offset would overflow; consider using a \
			different instruction, or reorganizing your code to reduce the length of the jump",
		)?,
		_ => Err(format!(
			"unrecognized condition: {condition}; allowed values: {allowed}"
		))?,
	};

	let prefix = if opcode == 0xe9 {
		None
	} else if condition == "if /ecx_zero" {
		Some(0x67)
	} else if is_32_bit {
		Some(0x0f)
	} else {
		None
	};

	Ok((prefix, opcode))
}

fn cmovcc_opcode(condition: &str) -> Option<u8> {
	let opcode = match condition {
		"/overflow" => 0x40,
		"!/overflow" => 0x41,
		"/carry" => 0x42,
		"!/carry" => 0x43,
		"/zero" => 0x44,
		"!/zero" => 0x45,
		"/carry_or_zero" => 0x46,
		"!/carry_or_zero" => 0x47,
		"/sign" => 0x48,
		"!/sign" => 0x49,
		"/parity" => 0x4A,
		"!/parity" => 0x4B,
		"/less" => 0x4C,
		"!/less" => 0x4D,
		"/less_or_equal" => 0x4E,
		"!/less_or_equal" => 0x4F,
		_ => return None,
	};
	return Some(opcode);
}

fn setcc_opcode(condition: &str) -> Option<u8> {
	let opcode = match condition {
		"/carry" => 0x92,
		"!/carry" => 0x93,
		"/parity" => 0x9a,
		"!/parity" => 0x9b,
		"/zero" => 0x94,
		"!/zero" => 0x95,
		"/sign" => 0x98,
		"!/sign" => 0x99,
		"/overflow" => 0x90,
		"!/overflow" => 0x91,
		"/less" => 0x9c,
		"!/less" => 0x9d,
		"/less_or_equal" => 0x9e,
		"!/less_or_equal" => 0x9f,
		"/carry_or_zero" => 0x96,
		"!/carry_or_zero" => 0x97,
		_ => return None,
	};
	return Some(opcode);
}

fn parse_string(str: &str) -> Result<Vec<u8>, String> {
	let bytes = str
		.trim()
		.strip_prefix("\"")
		.ok_or("missing leading quotation mark")?
		.as_bytes();

	let mut out = Vec::with_capacity(bytes.len());
	let mut i = 0;

	loop {
		let end = i + bytes
			.get(i..)
			.unwrap_or_default()
			.iter()
			.position(|&c| c == b'"' || c == b'\\' || c.is_ascii_control())
			.ok_or("missing closing quotation mark")?;

		out.extend(&bytes[i..end]);
		i = end;

		out.push(match (bytes[i], bytes.get(i + 1)) {
			(b'"', Some(_)) => Err("unexpected quotation mark within string")?,
			(b'"', None) => break,
			(b'\\', Some(b'"')) => b'"',
			(b'\\', Some(b'0')) => b'\0',
			(b'\\', Some(b'\\')) => b'\\',
			(b'\\', Some(b'n')) => b'\n',
			(b'\\', Some(b'r')) => b'\r',
			(b'\\', Some(b't')) => b'\t',
			(b'\\', Some(b'x')) => bytes
				.get(i + 2)
				.and_then(|&c| char::from(c).to_digit(16))
				.zip(bytes.get(i + 3).and_then(|&c| char::from(c).to_digit(16)))
				.map(|(digit1, digit2)| (digit1 * 16 + digit2) as u8)
				.inspect(|_| i += 2)
				.ok_or("bad hex string")?,
			(b'\\', Some(c)) => Err(format!("invalid escape sequence: \\{}", *c as char))?,
			(b'\\', None) => Err("missing escape sequence")?,
			(c, _) => Err(format!("illegal control character: 0x{c:x}"))?,
		});
		i += 2;
	}
	Ok(out)
}

fn assert_not_atomic(atomic: bool) -> Result<(), &'static str> {
	if atomic {
		Err("this operation cannot be performed atomically")?
	}
	Ok(())
}

fn error_if_atomic(line: &str) -> Result<(), &'static str> {
	if line.ends_with(" atomically") {
		Err("this operation cannot be performed atomically")?
	}
	Ok(())
}

fn extract_atomic_suffix(s: &str) -> (&str, bool) {
	match s.strip_suffix(" atomically") {
		Some(rest) => (rest.trim(), true),
		None => (s, false),
	}
}

fn strip_comments(line: &str) -> &str {
	// idea: loop over the line and count quotation marks
	// if we ever reach "//" while not inside a string,
	// strip the comment and return immediately

	let mut inside_string = false;
	let mut bytes = line.as_bytes();

	while let Some(i) = bytes.iter().position(|c| b"\"\\/".contains(c)) {
		if bytes[i] == b'"' {
			inside_string = !inside_string;
			bytes = &bytes[i + 1..];
		} else if matches!(bytes.get(i..i + 2), Some(b"\\\"" | b"\\\\")) {
			bytes = &bytes[i + 2..]; // ignore escaped
		} else if matches!(bytes.get(i..i + 2), Some(b"//")) && !inside_string {
			return &line[..line.len() - bytes.len() + i];
		} else {
			bytes = &bytes[i + 1..];
		}
	}
	line
}

fn main() {
	let mut buf = String::new();
	print!("Enter the file name: ");
	stdout().flush().unwrap();
	stdin().read_line(&mut buf).unwrap();
	let filename = buf.trim();
	let Some(binary_name) = filename.strip_suffix(".asm") else {
		println!("File name should end with .asm.");
		exit(1234);
	};

	let program = Program::parse_from_file(&filename).unwrap_or_else(|e| {
		println!("{filename} could not be assembled.\n{e}");
		exit(69);
	});

	println!("Successfully assembled program: {:?}", &program.code[120..]);
	println!("Saved to {binary_name}");
	println!("Note: this program must be run on Linux with a x86-64 CPU.");
	program.save_to_file(binary_name).unwrap();
}
