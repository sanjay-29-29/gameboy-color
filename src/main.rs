use crate::gameboy::GameBoy;

mod gameboy;

fn main() {
    let mut gameboy = GameBoy::new();

    loop {
        gameboy.main();
    }
}

// 0x0 => match second_byte {
//     0x0 => {
//         // NO-OP
//     }
//     0x1 => {
//         // STOP
//     }
//     0x2 => {
//         // JMP
//     }
//     0x3 => {
//         // JMP
//     }
//     _ => error(ins),
// },
// 0x1 => {
//     let val = self.fetch_value() as u16 | (self.fetch_value() as u16) >> 4;
//     let register = self.get_r16(second_byte);
//     *register = val;
// }
// 0x2 => {
//     let register: u16 = match second_byte {
//         0..=1 => *self.get_r16(second_byte),
//         2 => {
//             let old_val = self.hl;
//             self.hl = self.hl.wrapping_add(1);
//             old_val
//         }
//         3 => {
//             let old_val = self.hl;
//             self.hl = self.hl.wrapping_sub(1);
//             old_val
//         }
//         _ => panic!("The program panicked at {ins}"),
//     };
//     let a = self.get_register_a();
//     let memory = self.map_ram(register);
//     *memory = a;
// }
// 0x3 => {
//     let register = self.get_r16(second_byte);
//     let sum = (*register).wrapping_add(1);
//     *register = sum;
// }
// 0x4 => {
//     let (sum, old_val) = match second_byte {
//         0 => {
//             let b = self.get_register_b();
//             let sum = b.wrapping_add(1);
//             self.set_register_b(sum);

//             (sum, b)
//         }
//         1 => {
//             let d = self.get_register_d();
//             let sum = d.wrapping_add(1);
//             self.set_register_d(sum);

//             (sum, d)
//         }
//         2 => {
//             let h = self.get_register_h();
//             let sum = h.wrapping_add(1);
//             self.set_register_h(sum);

//             (sum, h)
//         }
//         3 => {
//             let hl = self.hl;
//             let sum = hl.wrapping_add(1);
//             self.hl = sum;

//             (sum as u8, hl as u8)
//         }
//         _ => panic!("The program panicked at {ins}"),
//     };

//     self.set_zero_flag(sum == 0);
//     self.set_subtraction_flag(false);
//     self.set_half_overflow_flag(old_val & 0x0F == 0x0F);
// }
