use crate::mpc_party::MPCParty;
use fss::*;
use fss::dpf::*;
use fss::RingElm;
use fss::BinElm;
use crate::offline_data::*;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

pub const TOTAL_BITS:usize = 32;

pub async fn pika_eval(p: &mut MPCParty<BasicOffline>, x_share:&RingElm)->RingElm{

    println!("---------- Input ---------- ----------");

    println!("CURRENT SHARE (u32 ring):");
    x_share.print();
    println!("");

    println!("W SHARE (u32 ring): ");
    p.offlinedata.w_share[0].print();
    println!("");

    println!("R SHARE (u16 domain):");
    println!("{}", p.offlinedata.r_share[0]);

    let mut ret = RingElm::zero();
    
    println!("---------- Mask x ---------- ----------");

    // Protocol 2(a) - reconstruct x=r-a(mod2^k) -> r: random val, a: secret sharing of user input
    let quantized_x_share = x_share.quantize_16();

    println!("QUANTIZED INPUT SHARE (u16 domain):");
    println!("{}", quantized_x_share);

    let mut party_mask: Vec<u16> = Vec::new();
    party_mask.push(p.offlinedata.r_share[0].wrapping_sub(quantized_x_share));
    let mask = p.netlayer.exchange_u16_vec(party_mask).await;

    println!("MASK X VALUE (u16 domain):");
    println!("{}", mask[0]);

    println!("---------- EvalAll ---------- ----------");

    // Protocol 2(b) - compute yσ (EvalAll routine -> implement in DPF key)
    let y_vec = p.offlinedata.k_share[0].evalAll();
    println!("y_vec LENGTH: {:?}",y_vec.len());

    // FIXME very slow - no need to split into 16 files - fix in offline_data
    let func_database = load_func_db(); // -> load works but store is not done correctly -> load 32 files
    println!("FUNC DB LENGTH: {}", func_database.len());

    println!("---------- u Calculation (DB lookup) ----------");

    let mut u: RingElm = RingElm::from(0u32);
    
    // Protocol 2(c) - compute u
    for i in 0..y_vec.len() {
        let mut shift_index: u16 = i as u16;
        shift_index = shift_index.wrapping_add(mask[0]);

        if y_vec[shift_index as usize] {
            let mut temp = RingElm::from(func_database[i]);
            
            // -1^σ
            if !p.netlayer.is_server {
                temp.negate();
            }

            u = u + temp;
        }   
    }

    println!("U VALUE (u32 ring):");
    u.print();
    println!("");

    println!("---------- Beaver Triple ---------- ----------");
    
    // TODO this should be happening in online
    // TODO exchange values a*uw or b*uw
    // FIXME I am using the wrong beaver triple?
    ret = p.offlinedata.beavers[0].mul_compute(
        p.netlayer.is_server,
        &u,
        &p.offlinedata.w_share[0]
    );

    println!("BEAVER TRIPLE RESULT");
    ret.print();
    println!();

    ret
}

fn load_func_db()->Vec<f32>{
    let mut ret: Vec<f32> = Vec::new();

    for i in 0..TOTAL_BITS {
        let mut temp: Vec<f32> = Vec::new();
        match read_file(&format!("../data/func_database/slice_{}.bin", i)) {
            Ok(value) => temp = value,
            Err(e) => println!("Error reading file: {}", e),  // Or handle the error as needed
        }

        ret.append(&mut temp)
    }
    ret
}
