use crate::mpc_party::MPCParty;
use fss::*;
use fss::dpf::*;
use fss::RingElm;
use fss::BinElm;
use crate::offline_data::*;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

pub const TOTAL_BITS:usize = 32;

pub async fn pika_eval(p: &mut MPCParty<BasicOffline>, x_share:&u16)->RingElm{
    println!("");
    println!("---------- Input ---------- ----------");
    println!("");

    println!("CURRENT SHARE (u16 ring):");
    println!("{}", x_share);

    println!("W SHARE (u32 ring): ");
    p.offlinedata.w_share[0].print();
    println!("");

    println!("R SHARE (u16 domain):");
    println!("{}", p.offlinedata.r_share[0]);
    
    println!("");
    println!("---------- Mask x ---------- ----------");
    println!("");

    // Protocol 2(a) - reconstruct x=r-a(mod2^k) -> r: random val, a: secret sharing of user input

    let mut party_mask: Vec<u16> = Vec::new();
    party_mask.push(p.offlinedata.r_share[0].wrapping_sub(*x_share));
    let mask = p.netlayer.exchange_u16_vec(party_mask).await;

    println!("MASK X VALUE (u16 domain):");
    println!("{}", mask[0]);

    println!("");
    println!("---------- EvalAll ---------- ----------");
    println!("");

    // Protocol 2(b) - compute yσ (EvalAll routine -> implement in DPF key)
    let y_vec = p.offlinedata.k_share[0].evalAll();
    println!("y_vec LENGTH: {:?}",y_vec.len());

    let func_database = load_func_db();
    println!("FUNC DB LENGTH: {}", func_database.len());

    println!("");
    println!("---------- u Calculation (DB lookup) ----------");
    println!("");

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

    println!("");
    println!("---------- Beaver Triple ---------- ----------");
    println!("");
    
    let beaver_this_half: Vec<u8> = p.offlinedata.beavers[0].beaver_mul0(
        u,
        p.offlinedata.w_share[0]
    );

    println!("THIS HALF FOR BEAVER TRIPLE:");
    println!("{:?}", beaver_this_half);

    let beaver_other_half: Vec<u8> = p.netlayer.exchange_byte_vec(&beaver_this_half).await;

    println!("OTHER HALF FOR BEAVER TRIPLE:");
    println!("{:?}", beaver_other_half);

    let beaver_secret_share: RingElm = p.offlinedata.beavers[0].beaver_mul1(
        p.netlayer.is_server,
        &beaver_other_half
    );

    println!("BEAVER SECRET SHARE:");
    beaver_secret_share.print();
    println!("");

    beaver_secret_share

    // TODO check correctness
}

fn load_func_db()->Vec<f32>{
    let mut ret: Vec<f32> = Vec::new();

    match read_file("../data/func_database.bin") {
        Ok(value) => ret = value,
        Err(e) => println!("Error reading file: {}", e),  // Or handle the error as needed
    }
    ret
}
