use fss::beavertuple::BeaverTuple;
use fss::idpf::*;
use fss::dpf::*;
use fss::RingElm;
use fss::BinElm;
use fss::Group;
use fss::Share;
use fss::prg::PrgSeed;
use fss::{bits_to_u16,bits_Xor};
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


fn write_file<T: serde::ser::Serialize>(path:&str, value:&T){
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
    pub k_share: Vec<DPFKey<RingElm>>, //dpf keys
    pub a_share: Vec<RingElm>,  //alpha
    // pub beavers: Vec<BeaverTuple>,
}

impl BasicOffline{
    pub fn new() -> Self{
        Self{k_share: Vec::new(), a_share: Vec::new()}
    }

    pub fn loadData(&mut self,idx:&u8){
        match read_file(&format!("../data/k{}.bin", idx)) {
            Ok(value) => self.k_share = value,
            Err(e) => println!("Error reading file: {}", e),  // Or handle the error as needed
        }

        match read_file(&format!("../data/a{}.bin", idx)) {
            Ok(value) => self.a_share = value,
            Err(e) => println!("Error reading file: {}", e),  // Or handle the error as needed
        }

        // match read_file(&format!("../data/beaver{}.bin", idx)) {
        //     Ok(value) => self.beavers = value,
        //     Err(e) => println!("Error reading file: {}", e),  // Or handle the error as needed
        // }
    }

    pub fn genData(&self,seed: &PrgSeed,input_bits: usize){
        let mut stream = FixedKeyPrgStream::new();
        stream.set_key(&seed.key);

 //Offline-Step-1. Set DPF Parameters
        let r_bits = stream.next_bits(input_bits);
        //let r_bits0 = stream.next_bits(input_bits);

        let beta = RingElm::from(1u16);

        let mut dpf_0: Vec<DPFKey<RingElm>> = Vec::new();
        let mut dpf_1: Vec<DPFKey<RingElm>> = Vec::new();

        let mut aVec_0: Vec<RingElm> = Vec::new();
        let mut aVec_1: Vec<RingElm> = Vec::new();

        // rnd_alpha and rnd_alpha1 are basically the same
        // that means aVec_0 always has the value 0
        // it finally works because the bits of 0 and the actual r bits are added as ring element in exchange_ring_vec and r is reconstructed for both parties
        // !!This is not secure!!
        let mut rnd_alpha = RingElm::from( bits_to_u16(&r_bits[0..input_bits]));
        let mut rnd_alpha1 = RingElm::from( bits_to_u16(&r_bits[0..input_bits]));
        aVec_1.push(rnd_alpha1.clone());
        aVec_0.push(rnd_alpha - rnd_alpha1);

        //println!("r_bits");
        //rnd_alpha.print();
        //println!("{:?}", aVec_0);
        //println!("{:?}", aVec_1);

        let (dpf_key0, dpf_key1) = DPFKey::gen(&r_bits[0..input_bits], &beta);
        dpf_0.push(dpf_key0);
        dpf_1.push(dpf_key1);
        
        // TODO Generate w (correction bit) -> w=1 if final layer control bit t0=1 else w=-1

//Offline-Step2. SIGMOID TRUTH TABLE
        let mut func_truth_table: Vec<f32> = Vec::new();
        // let mut scaled_values_vec: Vec<f32> = Vec::new();

        for i in 0..TOTAL_NUMBERS {
            let integer_part = i >> FLOAT_BITS;
            let fractional_part = i & ((1 << FLOAT_BITS) - 1);

            let combined_value = (integer_part << FLOAT_BITS) | fractional_part;

            // Calculate the corresponding f32 value
            // FIXME ?? at index 0 and 32768 it is both times 0 (or -0) with sigmoid 0.5 (exact)
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

            // scaled_values_vec.push(f32_value);
            func_truth_table.push(sigmoid(f32_value));
        }

        // println!("Number {} || Sigmoid {}", scaled_values_vec[0], func_truth_table[0]);
        // for k in 32767..32778 {
        //     println!("Number {} || Sigmoid {}", scaled_values_vec[k], func_truth_table[k]);
        // }

        // Split truth table into TOTAL_BITS parts
        for i in 0..TOTAL_BITS {
            let temp_slice = &func_truth_table[
                i*(func_truth_table.len()/TOTAL_BITS)..(i+1)*(func_truth_table.len()/TOTAL_BITS)
            ];
            write_file(&format!("../data/func_database/slice_{}.bin", i), &temp_slice);
        }
    
        write_file("../data/k0.bin", &dpf_0);
        write_file("../data/k1.bin", &dpf_1);

        write_file("../data/a0.bin", &aVec_0);
        write_file("../data/a1.bin", &aVec_1);
    }
}


fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-f32::from(x)).exp())
}
