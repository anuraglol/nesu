use crate::bus::Mem;
use crate::cpu::{AddressingMode, CPU, CpuFlags};

impl<'a> CPU<'a> {
    pub(crate) fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(&mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    pub(crate) fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(&mode);
        let value = self.mem_read(addr);

        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_x);
    }

    pub(crate) fn ldy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(&mode);
        let value = self.mem_read(addr);

        self.register_y = value;
        self.update_zero_and_negative_flags(self.register_y);
    }

    pub(crate) fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    pub(crate) fn stx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_x);
    }

    pub(crate) fn sty(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_y);
    }

    pub(crate) fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }

    pub(crate) fn txa(&mut self) {
        self.register_a = self.register_x;
        self.update_zero_and_negative_flags(self.register_x);
    }

    pub(crate) fn tay(&mut self) {
        self.register_y = self.register_a;
        self.update_zero_and_negative_flags(self.register_y);
    }

    pub(crate) fn tya(&mut self) {
        self.register_a = self.register_y;
        self.update_zero_and_negative_flags(self.register_y);
    }

    pub(crate) fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    pub(crate) fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_x);
    }

    pub(crate) fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    pub(crate) fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_negative_flags(self.register_y);
    }

    pub(crate) fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a &= value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    pub(crate) fn ora(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a |= value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    pub(crate) fn eor(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a ^= value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    pub(crate) fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        let and = self.register_a & data;
        if and == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        self.status.set(CpuFlags::NEGATIV, data & 0b10000000 > 0);
        self.status.set(CpuFlags::OVERFLOW, data & 0b01000000 > 0);
    }

    pub(crate) fn sei(&mut self) {
        self.status.insert(CpuFlags::INTERRUPT_DISABLE);
    }

    pub(crate) fn compare(&mut self, mode: &AddressingMode, compare_with: u8) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        if data <= compare_with {
            self.status.insert(CpuFlags::CARRY);
        } else {
            self.status.remove(CpuFlags::CARRY);
        }

        self.update_zero_and_negative_flags(compare_with.wrapping_sub(data));
    }

    pub(crate) fn adc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.add_to_register_a(value);
    }

    pub(crate) fn sbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(&mode);
        let data = self.mem_read(addr);
        self.add_to_register_a(((data as i8).wrapping_neg().wrapping_sub(1)) as u8);
    }

    pub(crate) fn inc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr).wrapping_add(1);

        self.mem_write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    pub(crate) fn dec(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr).wrapping_sub(1);

        self.mem_write(addr, value);
        self.update_zero_and_negative_flags(value);
    }

    pub(crate) fn asl_accumulator(&mut self) {
        let mut data = self.register_a;
        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        data = data << 1;
        self.set_register_a(data)
    }

    pub(crate) fn asl(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        data = data << 1;
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }

    pub(crate) fn lsr_accumulator(&mut self) {
        let mut data = self.register_a;
        if data & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        data = data >> 1;
        self.set_register_a(data)
    }

    pub(crate) fn lsr(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        if data & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        data = data >> 1;
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }

    pub(crate) fn rol(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        let old_carry = self.status.contains(CpuFlags::CARRY);

        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        data = data << 1;
        if old_carry {
            data = data | 1;
        }
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }

    pub(crate) fn rol_accumulator(&mut self) {
        let mut data = self.register_a;
        let old_carry = self.status.contains(CpuFlags::CARRY);

        if data >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        data = data << 1;
        if old_carry {
            data = data | 1;
        }
        self.set_register_a(data);
    }

    pub(crate) fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        let old_carry = self.status.contains(CpuFlags::CARRY);

        if data & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        data = data >> 1;
        if old_carry {
            data = data | 0b10000000;
        }
        self.mem_write(addr, data);
        self.update_zero_and_negative_flags(data);
        data
    }

    pub(crate) fn ror_accumulator(&mut self) {
        let mut data = self.register_a;
        let old_carry = self.status.contains(CpuFlags::CARRY);

        if data & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
        }
        data = data >> 1;
        if old_carry {
            data = data | 0b10000000;
        }
        self.set_register_a(data);
    }

    pub(crate) fn jmp(&mut self, mode: &AddressingMode) {
        match mode {
            AddressingMode::Absolute => {
                self.program_counter = self.mem_read_u16(self.program_counter);
            }
            AddressingMode::Indirect => {
                let addr = self.mem_read_u16(self.program_counter);
                self.program_counter = self.mem_read_u16(addr);
            }
            _ => panic!("Unsupported addressing mode for JMP"),
        }
    }

    pub(crate) fn beq(&mut self) {
        self.branch(self.status.contains(CpuFlags::ZERO));
    }

    pub(crate) fn bne(&mut self) {
        self.branch(!self.status.contains(CpuFlags::ZERO));
    }

    pub(crate) fn bcc(&mut self) {
        self.branch(!self.status.contains(CpuFlags::CARRY));
    }

    pub(crate) fn bcs(&mut self) {
        self.branch(self.status.contains(CpuFlags::CARRY));
    }

    pub(crate) fn bmi(&mut self) {
        self.branch(self.status.contains(CpuFlags::NEGATIV));
    }

    pub(crate) fn bpl(&mut self) {
        self.branch(!self.status.contains(CpuFlags::NEGATIV));
    }

    pub(crate) fn bvc(&mut self) {
        self.branch(!self.status.contains(CpuFlags::OVERFLOW));
    }

    pub(crate) fn bvs(&mut self) {
        self.branch(self.status.contains(CpuFlags::OVERFLOW));
    }

    pub(crate) fn pha(&mut self) {
        self.stack_push(self.register_a);
    }

    pub(crate) fn php(&mut self) {
        self.stack_push((self.status.clone() | CpuFlags::BREAK | CpuFlags::BREAK2).bits());
    }

    pub(crate) fn pla(&mut self) {
        self.register_a = self.stack_pop();
        self.update_zero_and_negative_flags(self.register_a);
    }

    pub(crate) fn plp(&mut self) {
        let status = self.stack_pop();
        self.status = CpuFlags::from_bits_truncate(status);
    }

    pub(crate) fn tsx(&mut self) {
        self.register_x = self.stack_pointer
    }

    pub(crate) fn txs(&mut self) {
        self.stack_pointer = self.register_x
    }

    pub(crate) fn clc(&mut self) {
        self.clear_carry_flag();
    }

    pub(crate) fn sec(&mut self) {
        self.set_carry_flag();
    }

    pub(crate) fn clv(&mut self) {
        self.status.remove(CpuFlags::OVERFLOW);
    }

    pub(crate) fn cld(&mut self) {
        self.status.remove(CpuFlags::DECIMAL_MODE);
    }

    pub(crate) fn sed(&mut self) {
        self.status.insert(CpuFlags::DECIMAL_MODE);
    }

    pub(crate) fn nop(&mut self) {}

    pub(crate) fn jsr(&mut self) {
        let target = self.mem_read_u16(self.program_counter);

        let return_addr = self.program_counter + 1;

        self.stack_push((return_addr >> 8) as u8);
        self.stack_push(return_addr as u8);

        self.program_counter = target;
    }

    pub(crate) fn rts(&mut self) {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;

        let return_addr = (hi << 8) | lo;

        self.program_counter = return_addr + 1;
    }

    pub(crate) fn brk(&mut self) {
        self.program_counter = self.program_counter.wrapping_add(1);

        self.stack_push((self.program_counter >> 8) as u8);
        self.stack_push(self.program_counter as u8);

        let status = self.status | CpuFlags::BREAK | CpuFlags::BREAK2;
        self.stack_push(status.bits());

        self.status.insert(CpuFlags::INTERRUPT_DISABLE);

        self.program_counter = self.mem_read_u16(0xFFFE);
    }

    pub(crate) fn rti(&mut self) {
        let status = self.stack_pop();
        self.status = CpuFlags::from_bits_truncate(status);

        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;

        self.program_counter = (hi << 8) | lo;
    }
}
