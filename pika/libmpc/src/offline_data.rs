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
pub const INPUT_DOMAIN:usize = 32;
pub const BOUNDED_DOMAIN:usize = 16;
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
    pub x_share: Vec<u16>, //share of input x
    pub r_share: Vec<u16>, //alpha
    pub w_share: Vec<RingElm>,
    pub beavers: Vec<BeaverTuple>,
}

impl BasicOffline{
    pub fn new() -> Self{
        Self{k_share: Vec::new(), x_share: Vec::new(), r_share: Vec::new(), w_share: Vec::new(), beavers: Vec::new()}
    }

    pub fn loadData(&mut self,idx:&u8){
        match read_file(&format!("../data/k{}.bin", idx)) {
            Ok(value) => self.k_share = value,
            Err(e) => println!("Error reading key file: {}", e),
        }

        match read_file(&format!("../data/x{}.bin", idx)) {
            Ok(value) => self.x_share = value,
            Err(e) => println!("Error reading a share file: {}", e)
        }

        match read_file(&format!("../data/r{}.bin", idx)) {
            Ok(value) => self.r_share = value,
            Err(e) => println!("Error reading a share file: {}", e)
        }

        match read_file(&format!("../data/w{}.bin", idx)) {
            Ok(value) => self.w_share = value,
            Err(e) => println!("Error reading w share file: {}", e)
        }

        match read_file(&format!("../data/bvt{}.bin", idx)) {
            Ok(value) => self.beavers = value,
            Err(e) => println!("Error reading beaver tuple file: {}", e),  // Or handle the error as needed
        }
    }

    pub fn genData(&self,seed: &PrgSeed){
        let mut stream = FixedKeyPrgStream::new();
        stream.set_key(&seed.key);

        let share_gen_bits = stream.next_bits(4*BOUNDED_DOMAIN+INPUT_DOMAIN);

        let beta: bool = true; // RingElm::from(1u32);

// INPUT X GENERATION AND SHARES
        let mut xVec0: Vec<u16> = Vec::new();
        let mut xVec1: Vec<u16> = Vec::new();
        // TODO Implement batch here (10^0 - 10^5 used in Pika paper with bad scalability)
        let x: &[bool] = &share_gen_bits[0*BOUNDED_DOMAIN..1*BOUNDED_DOMAIN];
        let x0: &[bool] = &share_gen_bits[1*BOUNDED_DOMAIN..2*BOUNDED_DOMAIN];
        let binding_x = x.iter().zip(x0.iter()).map(|(&x, &y)| x && !y).collect::<Vec<_>>();
        let x1 = binding_x.as_slice();

        xVec0.push(bits_to_u16(x0));
        xVec1.push(bits_to_u16(x1));

// R NON-ZERO INDEX GENERATION AND SHARES
        let mut rVec_0: Vec<u16> = Vec::new();
        let mut rVec_1: Vec<u16> = Vec::new();

        let r: &[bool] = &share_gen_bits[2*BOUNDED_DOMAIN..3*BOUNDED_DOMAIN];
        let r0: &[bool] = &share_gen_bits[3*BOUNDED_DOMAIN..4*BOUNDED_DOMAIN];
        let binding_r = r.iter().zip(r0.iter()).map(|(&x, &y)| x && !y).collect::<Vec<_>>();
        let r1: &[bool] = binding_r.as_slice();

        rVec_0.push(bits_to_u16(r0));
        rVec_1.push(bits_to_u16(r1));

// DPF KEYS BASED ON R - EXTRACT CONTROL BIT
        let mut dpf_0: Vec<DPFKey<bool>> = Vec::new();
        let mut dpf_1: Vec<DPFKey<bool>> = Vec::new();
        // FIXME the generation and use of x and r can cause issues
        // namely, generating less bits in line share_gen_bits, reduces accuracy
        // Also tried x as input for the dpf and had less accuracy than the current configuration
        let (dpf_key0, dpf_key1, control_bit) = DPFKey::gen(&r, &beta);
        dpf_0.push(dpf_key0);
        dpf_1.push(dpf_key1);

// W BIT BASED ON CONTROL BIT
        let mut wVec_0: Vec<RingElm> = Vec::new();
        let mut wVec_1: Vec<RingElm> = Vec::new();

        let w0: RingElm = RingElm::from(bits_to_u32(&share_gen_bits[4*BOUNDED_DOMAIN..4*BOUNDED_DOMAIN+INPUT_DOMAIN]));
        let mut w_bit: RingElm = RingElm::from(1u32);

        if !control_bit {
            w_bit.negate();
        }

        let w1: RingElm = w_bit - w0;
        wVec_0.push(w0);
        wVec_1.push(w1);

// FUNCTION TRUTH TABLE
        // let mut positive_encoding: Vec<f32> = Vec::new();
        // let mut negative_encoding: Vec<f32> = Vec::new();

        // for i in 0..TOTAL_NUMBERS/2 {
        //     let integer_part = i >> FLOAT_BITS;
        //     let fractional_part = i & ((1 << FLOAT_BITS) - 1);
        //     let f32_value = ((integer_part << FLOAT_BITS) | fractional_part) as f32 / (1 << FLOAT_BITS) as f32;
            
        //     if i > 0 {
        //         positive_encoding.push(sigmoid(f32_value));
        //         negative_encoding.push(sigmoid(-f32_value));
        //     } else {
        //         // Here, 0 is only processed once (as -0)
        //         // This is done to have equal length postitive and negative encodings
        //         negative_encoding.push(sigmoid(-f32_value));
        //     }
        // }
        // // In the paper the database is generated for numbers (-2^(k-1), 2^(k-1)]
        // // The inclusion at the right end of the group is accounted for here
        // // Thus 0 is assigned to the negative encoding 
        // positive_encoding.push(sigmoid(64f32));

        // let func_truth_table: Vec<f32> = [&positive_encoding[..], &negative_encoding[..]].concat();
        let mut func_truth_table: Vec<f32> = Vec::new();

        for i in 0..u16::MAX {
            let sign = i & (1 << 15) != 0;
            let rest_bits = i & !(1 << 15);

            let mut f32_number = if sign {
                -(rest_bits as f32) / (1 << FLOAT_BITS) as f32
            } else {
                rest_bits as f32 / (1 << FLOAT_BITS) as f32
            };

            if i == (u16::MAX / 2) + 1 {
                f32_number = 64f32
            };

            let sigmoid_val = sigmoid(f32_number);
            func_truth_table.push(sigmoid_val);
            // if i == (u16::MAX / 2) + 1 {
            //     println!(
            //         "u16: {:016b} || Sign: {} || Rest: {:015b} || f32: {} || sigmoid: {}",
            //         i, sign, rest_bits, f32_number, sigmoid_val
            //     );
            // }            
        }

// BEAVER TRIPLE
        let size: usize = 1;
        let mut beavertuples0 = Vec::new();
        let mut beavertuples1 = Vec::new();

        BeaverTuple::genBeaver(&mut beavertuples0, &mut beavertuples1, &seed, size);

        // ENCODE - DECODE INPUT FOR DEBUGGING!!!
        println!("");
        println!("----- ORIGINAL INPUTS -----");
        println!("");

        let mut encoded_number: u16 = 0;
        for &bit in x {
            encoded_number = (encoded_number << 1) | bit as u16;
        }

        // Extract the sign, integer part, and fractional part
        let sign = ((encoded_number >> 15) & 1) == 1;
        let integer_part = (encoded_number >> 9) & 0b111111;
        let fractional_part = encoded_number & 0b111_1111_1111;

        // Combine into an f32 number
        let f32_number = if sign {
            -((integer_part as f32) + (fractional_part as f32) / 512.0)
        } else {
            (integer_part as f32) + (fractional_part as f32) / 512.0
        };
        println!("IN u16 {}", bits_to_u16(x));
        println!("Original boolean vector: {:?}", x);
        println!("Encoded number (u16): {}", encoded_number);
        println!("Decoded f32 number: {}", f32_number);
        println!("");
// END DEBUGGING
    
        write_file("../data/k0.bin", &dpf_0);
        write_file("../data/k1.bin", &dpf_1);

        write_file("../data/x0.bin", &xVec0);
        write_file("../data/x1.bin", &xVec1);

        write_file("../data/r0.bin", &rVec_0);
        write_file("../data/r1.bin", &rVec_1);

        write_file("../data/w0.bin", &wVec_0);
        write_file("../data/w1.bin", &wVec_1);

        write_file("../data/bvt0.bin", &beavertuples0);
        write_file("../data/bvt1.bin", &beavertuples1);

        write_file("../data/func_database.bin", &func_truth_table);
    }
}


fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-f32::from(x)).exp())
}
