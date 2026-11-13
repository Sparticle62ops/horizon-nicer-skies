//! 1:1 port of Gradient: TreeMap<Double,int[]> gradient with floor/ceiling
//! interpolation. Rust: BTreeMap with IEEE-754 total-order keys (same
//! ordering java's Double.compareTo gives for finite values).

use std::collections::BTreeMap;

pub struct Gradient {
    gradient: BTreeMap<u64, [i32; 3]>,
}

fn gradient_key(value: f64) -> u64 {
    let bits = value.to_bits();
    if bits & (1u64 << 63) != 0 { !bits } else { bits | (1u64 << 63) }
}

fn gradient_unkey(bits: u64) -> f64 {
    f64::from_bits(if bits & (1u64 << 63) != 0 { bits & !(1u64 << 63) } else { !bits })
}

impl Gradient {
    pub fn new() -> Self {
        Gradient { gradient: BTreeMap::new() }
    }

    pub fn add(&mut self, index: f64, red: i32, green: i32, blue: i32) {
        if !(0.0..=1.0).contains(&index) {
            panic!("Index must be between 0 and 1");
        }

        let color: [i32; 3] = [red, green, blue];
        self.gradient.insert(gradient_key(index), color);
    }

    pub fn remove(&mut self, index: f64) {
        self.gradient.remove(&gradient_key(index));
    }

    pub fn clear(&mut self) {
        self.gradient.clear();
    }

    pub fn get_at(&self, value: f64) -> [i32; 3] {
        if !(0.0..=1.0).contains(&value) {
            panic!("Value must be between 0 and 1");
        }

        let v = gradient_key(value);

        // java: floorEntry(value); under the lowest, return the lowest.
        let floor_entry = self.gradient.range(..=v).next_back().map(|(k, c)| (*k, *c));
        let ceiling_entry = self.gradient.range(v..).next().map(|(k, c)| (*k, *c));

        let floor_entry = match floor_entry {
            Some(e) => e,
            None => return *self.gradient.iter().next().unwrap().1,
        };
        let ceiling_entry = match ceiling_entry {
            Some(e) => e,
            None => return *self.gradient.iter().next_back().unwrap().1,
        };

        let (floor_key, floor_color) = floor_entry;
        let (ceiling_key, ceiling_color) = ceiling_entry;

        let ratio = (value - gradient_unkey(floor_key))
            / (gradient_unkey(ceiling_key) - gradient_unkey(floor_key));
        let inv_ratio = 1.0 - ratio;

        let first_color = floor_color;
        let second_color = ceiling_color;

        let red = second_color[0] as f64 * ratio + first_color[0] as f64 * inv_ratio;
        let green = second_color[1] as f64 * ratio + first_color[1] as f64 * inv_ratio;
        let blue = second_color[2] as f64 * ratio + first_color[2] as f64 * inv_ratio;

        [
            red.round() as i32,
            green.round() as i32,
            blue.round() as i32,
        ]
    }
}
