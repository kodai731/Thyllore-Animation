/// Bit-for-bit comparison of a Rust oracle against the Slang C ABI over many samples.
#[derive(Default)]
pub struct BitwiseAgreement {
    compared: usize,
    mismatches: usize,
    max_bit_difference: i64,
}

impl BitwiseAgreement {
    pub fn record(&mut self, rust_value: f32, slang_value: f32) {
        self.compared += 1;
        if rust_value.to_bits() != slang_value.to_bits() {
            let difference = rust_value.to_bits() as i64 - slang_value.to_bits() as i64;
            self.mismatches += 1;
            self.max_bit_difference = self.max_bit_difference.max(difference.abs());
        }
    }

    pub fn record_vector(&mut self, rust_value: [f32; 3], slang_value: [f32; 3]) {
        for axis in 0..3 {
            self.record(rust_value[axis], slang_value[axis]);
        }
    }

    pub fn compared(&self) -> usize {
        self.compared
    }

    pub fn assert_identical(&self, quantity: &str) {
        assert_eq!(
            self.mismatches, 0,
            "{quantity}: {}/{} mismatches, max bit difference {}",
            self.mismatches, self.compared, self.max_bit_difference
        );
    }
}
