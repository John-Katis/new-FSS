use fss::beavertuple::BeaverTuple;
use fss::idpf::*;
use fss::dpf::*;
use fss::RingElm;
use fss::BinElm;
use fss::Group;
use fss::Share;
use fss::prg::PrgSeed;
use fss::{bits_to_u32,bits_to_u16,bits_Xor};
use fss::prg::FixedKeyPrgStream;
use bincode::Error;
use std::fs::File;
use std::io::Write;
use std::io::Read;
use serde::de::DeserializeOwned;

pub const INTERVALS_AMOUNT:usize = 1000;
pub const TOTAL_NUMBERS:i32 = 1 << 16;
pub const INTEGER_BITS:i32 = 7;
pub const FLOAT_BITS:i32 = 9;
pub const TOTAL_BITS:usize = INTEGER_BITS as usize + FLOAT_BITS as usize;


pub fn write_file<T: serde::ser::Serialize>(path:&str, value:&T){
    let mut file = File::create(path).expect("create failed");
    file.write_all(&bincode::serialize(&value).expect("Serialize value error")).expect("Write key error.");
}

// Changed to public (Jannis) -> use in pika.rs too
pub fn read_file<T: DeserializeOwned>(path: &str) -> Result<T, Error> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let value = bincode::deserialize(&buf)?;
    Ok(value)
}

pub struct BasicOffline {
    // seed: PrgSeed,
    pub k_share: Vec<DPFKey<bool>>, //dpf keys
    pub r_share: Vec<u16>,  //alpha
    pub w_share: Vec<RingElm>,
    pub beavers: Vec<BeaverTuple>,
}

impl BasicOffline{
    pub fn new() -> Self{
        Self{k_share: Vec::new(), r_share: Vec::new(), w_share: Vec::new(), beavers: Vec::new()}
    }

    pub fn loadData(&mut self,idx:&u8){
        match read_file(&format!("../data/k{}.bin", idx)) {
            Ok(value) => self.k_share = value,
            Err(e) => println!("Error reading key file: {}", e),  // Or handle the error as needed
        }

        match read_file(&format!("../data/r{}.bin", idx)) {
            Ok(value) => self.r_share = value,
            Err(e) => println!("Error reading a share file: {}", e),  // Or handle the error as needed
        }

        match read_file(&format!("../data/w{}.bin", idx)) {
            Ok(value) => self.w_share = value,
            Err(e) => println!("Error reading w share file: {}", e),  // Or handle the error as needed
        }

        match read_file(&format!("../data/bvt{}.bin", idx)) {
            Ok(value) => self.beavers = value,
            Err(e) => println!("Error reading beaver tuple file: {}", e),  // Or handle the error as needed
        }
    }

    pub fn genData(&self,seed: &PrgSeed, input_bits: usize, bounded_domain_bits: usize){
        let mut stream = FixedKeyPrgStream::new();
        stream.set_key(&seed.key);

        //Offline-Step-1. Set DPF Parameters: k, a, w
        let a_bits = stream.next_bits(bounded_domain_bits*2+input_bits);

        let beta: bool = true; // RingElm::from(1u32);

        let mut dpf_0: Vec<DPFKey<bool>> = Vec::new();
        let mut dpf_1: Vec<DPFKey<bool>> = Vec::new();

        let mut rVec_0: Vec<u16> = Vec::new();
        let mut rVec_1: Vec<u16> = Vec::new();

        let mut wVec_0: Vec<RingElm> = Vec::new();
        let mut wVec_1: Vec<RingElm> = Vec::new();

        // TODO need to generate r shares with u16 representation
        let r1: &[bool] = &a_bits[..bounded_domain_bits];
        let r2: &[bool] = &a_bits[bounded_domain_bits..bounded_domain_bits*2];
        let binding = r1.iter().zip(r2.iter()).map(|(&x, &y)| x || y).collect::<Vec<_>>();
        let r: &[bool] = binding.as_slice();
        
        let r1_int: u16 = bits_to_u16(r1);
        let r2_int: u16 = bits_to_u16(r2);
        rVec_0.push(r1_int);
        rVec_1.push(r2_int);

        let (dpf_key0, dpf_key1, control_bit) = DPFKey::gen(&r, &beta);
        dpf_0.push(dpf_key0);
        dpf_1.push(dpf_key1);

        let w0: RingElm = RingElm::from(bits_to_u32(&a_bits[bounded_domain_bits*2..bounded_domain_bits*2+input_bits]));
        let mut w_bit: RingElm = RingElm::from(1u32);

        if !control_bit {
            w_bit.negate();
        }

        let w1: RingElm = w_bit - w0;
        wVec_0.push(w0);
        wVec_1.push(w1);

        //Offline-Step2. Function truth table
        let mut func_truth_table: Vec<f32> = Vec::new();

        for i in 0..TOTAL_NUMBERS {
            let integer_part = i >> FLOAT_BITS;
            let fractional_part = i & ((1 << FLOAT_BITS) - 1);

            let combined_value = (integer_part << FLOAT_BITS) | fractional_part;

            // FIXME ?? at index 0 and 32768 it is both times 0 (or -0) with sigmoid 0.5 (exact)
            // It should be (-2^k-1, 2^k-1], index 0 shouldn't come into play
            let scaled_value = combined_value as f32 / (1 << FLOAT_BITS) as f32;
            let f32_value = if i < TOTAL_NUMBERS / 2 {
                scaled_value
            } else {
                -(scaled_value / (1 << INTEGER_BITS-1) as f32) + 1.0
            };

            // println!(
            //     "f32 value: {} (binary representation: {:07b} | {:09b})",
            //     f32_value, integer_part, fractional_part
            // );

            func_truth_table.push(sigmoid(f32_value));
        }

        // println!("Number {} || Sigmoid {}", scaled_values_vec[0], func_truth_table[0]);
        // for k in 32767..32778 {
        //     println!("Number {} || Sigmoid {}", scaled_values_vec[k], func_truth_table[k]);
        // }

        let size: usize = 1;
        let mut beavertuples0 = Vec::new();
        let mut beavertuples1 = Vec::new();

        BeaverTuple::genBeaver(&mut beavertuples0, &mut beavertuples1, &seed, size);

        for i in 0..TOTAL_BITS {
            let temp_slice = &func_truth_table[
                i*(func_truth_table.len()/TOTAL_BITS)..(i+1)*(func_truth_table.len()/TOTAL_BITS)
            ];
            write_file(&format!("../data/func_database/slice_{}.bin", i), &temp_slice);
        }
    
        write_file("../data/k0.bin", &dpf_0);
        write_file("../data/k1.bin", &dpf_1);

        write_file("../data/r0.bin", &rVec_0);
        write_file("../data/r1.bin", &rVec_1);

        write_file("../data/w0.bin", &wVec_0);
        write_file("../data/w1.bin", &wVec_1);

        write_file("../data/bvt0.bin", &beavertuples0);
        write_file("../data/bvt1.bin", &beavertuples1);
    }
}


fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-f32::from(x)).exp())
}
