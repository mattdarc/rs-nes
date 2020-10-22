mod cpu {

    #[cfg(test)]
    #[allow(unused_mut)]
    mod tests {
        use crate::cpu::*;
        use crate::instructions::AddressingMode::*;
        use crate::instructions::InstrName::*;

        impl Ricoh2A03 {
            pub fn test_program(rom: &[u8]) -> Ricoh2A03 {
                Ricoh2A03::with(crate::cartridge::test::program(rom))
            }
        }

        fn bytes(instr: &Instruction) -> u16 {
            use AddressingMode::*;
            match instr.mode() {
                ZeroPage => 2,    // 2 byte
                ZeroPageX => 2,   // 2 byte
                ZeroPageY => 2,   // 2 byte
                Absolute => 3,    // 3 byte
                AbsoluteX => 3,   // 3 byte
                AbsoluteY => 3,   // 3 byte
                Indirect => 2,    // 2 byte
                IndirectX => 2,   // 2 byte
                IndirectY => 2,   // 2 byte
                Relative => 2,    // 2 byte
                Accumulator => 1, // 1 byte
                Immediate => 2,   // 2 byte
                Invalid => 1, // bytes that have no mode are implicitly 1
            }
        }

        // TODO: Need to verify noop cycles
        macro_rules! verify_op {
	    ($name:ident, $addr_mode:ident,
	     $opcode:literal,
	     [ROM: $($b:expr),*][$(*$addr:literal=$val:literal),*]{$($reg:ident : $pv:expr),*}
	     => [$(*$exp_addr:literal = $exp_b:expr),*]{$($eflg:ident : $ev:expr),*}) => {
		let act_instr = get_from(($opcode).into());
		assert_eq!(act_instr.name(), &$name, "Instruction mismatch for {:?}", &$name);
		assert_eq!(act_instr.mode(), &$addr_mode, "Address mode mismatch for {:?}", &$addr_mode);

		// Set up initial CPU state
		let mut cpu = Ricoh2A03::test_program(&[$opcode, $($b,)*]);
		$(cpu.$reg = $pv;)*
		$(cpu.write($addr, $val);)*

		// Init and keep track of PC
		cpu.init();
		let pc_bef = cpu.pc;

		// Make sure we run for the correct number of no-op cycles
		// and exit normally
		assert_eq!(cpu.run_for(act_instr.cycles() as usize), 0);

		// Verify CPU state
		assert_eq!(cpu.pc - pc_bef, bytes(&act_instr), "PC did not retrieve the correct number of bytes");
		$(assert_eq!(cpu.$eflg, $ev);)*
		$(assert_eq!(cpu.read($exp_addr), $exp_b, "Memory at {:#X} does not match {:#}", $exp_addr, $exp_b);)*

		// Verify one more cycle will increment the PC again
		let pc_bef = cpu.pc;
		assert_eq!(cpu.run_for(1), 0);
		assert_ne!(cpu.pc, pc_bef);
	    }
	}

        #[test]
        fn negative() {
            assert!(is_negative(255));
            assert!(is_negative(128));
            assert!(!is_negative(127));
            assert!(!is_negative(0));
        }

        // TODO: Add flag verification
        #[test]
        fn adc() {
            verify_op!(ADC, Immediate, 0x69, [ROM: 0x03][]{acc: 2} => []{acc: 5});
            verify_op!(ADC, ZeroPage,  0x65, [ROM: 0x00][*0x00=0x01]{acc: 2} => []{acc: 3});
            verify_op!(ADC, ZeroPageX, 0x75, [ROM: 0x01][*0x07=0x01]{acc: 4, x: 6} => []{acc: 5});
            verify_op!(ADC, Absolute,  0x6D, [ROM: 0x00, 0x10][*0x1000=0x01]{acc: 4} => []{acc: 5});
            verify_op!(ADC, AbsoluteX, 0x7D, [ROM: 0x00, 0x10][*0x1006=0x01]{acc: 4, x: 6} => []{acc: 5});
            verify_op!(ADC, AbsoluteY, 0x79, [ROM: 0x00, 0x10][*0x1006=0x01]{acc: 4, y: 6} => []{acc: 5});
            verify_op!(ADC, IndirectX, 0x61, [ROM: 0x1][*0x08=0x10, *0x1000=0x02]{acc: 4, x: 6} => []{acc: 6});
            verify_op!(ADC, IndirectY, 0x71, [ROM: 0x1][*0x2=0x10, *0x1006=0x02]{acc: 4, y: 6} => []{acc: 6});
        }

	#[test]
	fn and() {
            verify_op!(AND, Immediate, 0x29, [ROM: 0x03][]{acc: 2} => []{acc: 2});
            verify_op!(AND, ZeroPage,  0x25, [ROM: 0x00][*0x00=0x01]{acc: 3} => []{acc: 1});
            verify_op!(AND, ZeroPageX, 0x35, [ROM: 0x01][*0x07=0x01]{acc: 5, x: 6} => []{acc: 1});
            verify_op!(AND, Absolute,  0x2D, [ROM: 0x00, 0x10][*0x1000=0x05]{acc: 5} => []{acc: 5});
            verify_op!(AND, AbsoluteX, 0x3D, [ROM: 0x00, 0x10][*0x1006=0x05]{acc: 4, x: 6} => []{acc: 4});
            verify_op!(AND, AbsoluteY, 0x39, [ROM: 0x00, 0x10][*0x1012=0x05]{acc: 4, y: 0x12} => []{acc: 4});
            verify_op!(AND, IndirectX, 0x21, [ROM: 0x1][*0x08=0x10, *0x1000=0x07]{acc: 7, x: 6} => []{acc: 7});
            verify_op!(AND, IndirectY, 0x31, [ROM: 0x1][*0x2=0x10, *0x1006=0x07]{acc: 7, y: 6} => []{acc: 7});
	}

	#[test]
	fn asl() {
            verify_op!(ASL, Accumulator, 0x0A, [ROM:][]{acc: 3} => []{acc: 6});
            verify_op!(ASL, ZeroPage,    0x06, [ROM: 0x00][*0x00=0x01]{} => [*0x00=0x02]{});
            verify_op!(ASL, ZeroPageX,   0x16, [ROM: 0x01][*0x07=0x01]{x: 6} => [*0x07=0x02]{});
            verify_op!(ASL, Absolute,    0x0E, [ROM: 0x00, 0x10][*0x1000=0x05]{} => [*0x1000=0x0A]{});
            verify_op!(ASL, AbsoluteX,   0x1E, [ROM: 0x00, 0x10][*0x1006=0x05]{x: 6} => [*0x1006=0x0A]{});
	}

        macro_rules! test_rom {
            ($name:ident => $rom:literal) => {
                #[test]
                fn $name() {
                    let cart = match Cartridge::load($rom) {
                        Ok(cart) => cart,
                        Err(e) => unreachable!(
                            "Error with \"{:?}\": {:?}",
                            $rom, e
                        ),
                    };
                    let mut cpu = Ricoh2A03::with(cart);
                    cpu.init();
                    cpu.run_for(10_000);
                    assert_eq!(cpu.cycle, 10_000);
                }
            };
        }

        // MMC1
        test_rom!(tetris => "roms/Tetris.nes");

        // MMC4
        // test_rom!(mario => "roms/super_mario_bros3.nes");
    }
}
