use crate::mpc_party::MPCParty;
use fss::*;
use fss::dpf::*;
use fss::RingElm;
use fss::BinElm;
use crate::offline_data::*;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

pub const TOTAL_BITS:usize = 32;

pub async fn pika_eval(p: &mut MPCParty<BasicOffline>)->Vec<RingElm>{
    let iter_end: usize = p.offlinedata.x_share.len() as usize;

    // println!("");
    // println!("---------- Party Shares ---------- ----------");
    // println!("");

    // println!("CURRENT SHARE (u16 ring):");
    // println!("{}", p.offlinedata.x_share[0]);

    // println!("W SHARE (u32 ring): ");
    // p.offlinedata.w_share[0].print();
    // println!("");

    // println!("R SHARE (u16 domain):");
    // println!("{}", p.offlinedata.r_share[0]);
    
    // println!("");
    // println!("---------- Mask x ---------- ----------");
    // println!("");

// Protocol 2(a) - reconstruct x=r-a(mod2^k) -> r: random val, a: secret sharing of user input

    let mut party_mask: Vec<u16> = Vec::new();

    for k in 0..iter_end {
        party_mask.push(p.offlinedata.r_share[k].wrapping_sub(p.offlinedata.x_share[k]));
    }

    let mask = p.netlayer.exchange_u16_vec(party_mask).await;

    // println!("MASK X VALUE (u16 domain):");
    // println!("{}", mask[0]);

    let mut all_this_beaver_halfs: Vec<Vec<u8>> = Vec::new();

    for j in 0..iter_end {
// Protocol 2(b) - compute yσ (EvalAll routine -> implement in DPF key)
        // println!("");
        // println!("---------- EvalAll ---------- ----------");
        // println!("");
    
        let y_vec = p.offlinedata.k_share[j].evalAll();
        // println!("y_vec LENGTH: {:?}",y_vec.len());

        let func_database = load_func_db();
        // println!("FUNC DB LENGTH: {}", func_database.len());

        // println!("");
        // println!("---------- u Calculation (DB lookup) ----------");
        // println!("");

        let mut u: RingElm = RingElm::from(0u32);

// Protocol 2(c) - compute u
        for i in 0..u16::MAX {
            let mut shift_index: u16 = i;
            shift_index = shift_index.wrapping_add(mask[j]);

            if i <= u16::MAX - mask[j] {
                shift_index = shift_index - 1u16;
            }

            if y_vec[shift_index as usize] {
                let mut temp = RingElm::from(func_database[i as usize]);
                
                // -1^σ
                if !p.netlayer.is_server {
                    temp.negate();
                }

                u = u + temp;
            }   
        }

        // println!("U VALUE (u32 ring):");
        // u.print();
        // println!("");

        // println!("");
        // println!("---------- Beaver Triple ---------- ----------");
        // println!("");

        let beaver_this_half: Vec<u8> = p.offlinedata.beavers[j].beaver_mul0(
            u,
            p.offlinedata.w_share[j]
        );

        // println!("THIS HALF FOR BEAVER TRIPLE:");
        // println!("{:?}", beaver_this_half);
        // println!("");

        all_this_beaver_halfs.push(beaver_this_half);
    }
//FIXME incorrect for batched inputs
    let all_beaver_other_halfs: Vec<Vec<u8>> = p.netlayer.exchange_byte_vec(&all_this_beaver_halfs).await;

    // println!("---------- After Processing all Inputs Locally ---------- ----------");
    // println!("");
    // println!("OTHER HALF FOR BEAVER TRIPLE:");
    // println!("{:?}", all_beaver_other_halfs);

    let mut beaver_shares_return_vector: Vec<RingElm> = Vec::new();

    for l in 0..iter_end {
        let beaver_secret_share: RingElm = p.offlinedata.beavers[l].beaver_mul1(
            p.netlayer.is_server,
            &all_beaver_other_halfs[l]
        );

        beaver_shares_return_vector.push(beaver_secret_share);
    }

    // println!("BEAVER SECRET SHARE:");
    // println!("{:?}", beaver_shares_return_vector);
    // println!("");

    // println!("");
    // println!("---------- Correctness ---------- ----------");
    // println!("");

    // println!("THIS PARTY BEAVER:");
    // beaver_shares_return_vector[0].print();
    // println!("");

    // let this_party_beaver: Vec<RingElm> = vec![beaver_shares_return_vector[0]];
    // let beaver_comb = p.netlayer.exchange_ring_vec(this_party_beaver).await;

    // println!("EXCHANGE VALUE");
    // beaver_comb[0].print();
    // println!("");

    // println!("EXCHANGE VALUE BITS:");
    // println!("{:b}", beaver_comb[0].to_u32().unwrap());

    // let mut result: f32 = beaver_comb[0].to_u32().unwrap() as f32;
    // let f32_number = result / (1 << 16) as f32;

    // println!("Original u32 number as f32: {}", result);
    // println!("Interpreted f32 number: {}", f32_number);

    beaver_shares_return_vector
}

fn load_func_db()->Vec<f32>{
    let mut ret: Vec<f32> = Vec::new();

    match read_file("../data/func_database.bin") {
        Ok(value) => ret = value,
        Err(e) => println!("Error reading file: {}", e),  // Or handle the error as needed
    }
    ret
}
