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
    let mut ret = RingElm::zero();
    // Protocol 2(a) - reconstruct x=r-a(mod2^k) -> r: random val, a: secret sharing of user input
    // first do r-a mod2k then exchange_u16_vec
    let mask = p.netlayer.exchange_u16_vec(p.offlinedata.r_share.to_vec()).await;
    let mut x = mask[0];

    // TODO reduce x_share to u16 before calculations
    x = x.wrapping_sub(10u16);

    // Protocol 2(b) - compute yσ (EvalAll routine -> implement in DPF key)
    let y_vec = p.offlinedata.k_share[0].evalAll();
    // println!("y_vec LENGTH {:?}",y_vec.len());

    let func_database = load_func_db(); // -> load works but store is not done correctly -> load 32 files
    // println!("FUNC DB LENGTH {}", func_database.len());

    let mut u: RingElm = RingElm::from(0u32);
    
    // Protocol 2(c) - compute u
    // FIXME shift_index needs to wrap around 2^k
    // for i in 0..y_vec.len() {
    //     // println!("STEP 2C PROGRESS: {}", i);
    //     // TODO do this in plaintext - i+x.usize
    //     let mut ring_shift_index = RingElm::from(i as u32) + x;
    //     let usize_shift_index = ring_shift_index.to_usize();

    //     // println!("--- STEP 2C PROGRESS: {}", usize_shift_index);
    //     if y_vec[usize_shift_index] {
    //         let mut temp = RingElm::from(func_database[i]);
            
    //         // -1^σ
    //         if !p.netlayer.is_server {
    //             temp.negate();
    //         }

    //         u = u + temp;
    //     }   
    // }

    println!("CURRENT SHARE:");
    x_share.print();
    println!("");

    println!("X SUBBED VALUE:");
    //x.print();
    println!("");

    println!("U VALUE:");
    u.print();
    println!("");

    println!("W SHARE: ");
    p.offlinedata.w_share[0].print();
    println!("");

    println!("R SHARE:");
    println!("{}", p.offlinedata.r_share[0]);
    
    // TODO this should be happening in online
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

// The function database, evalAll and calculation here must happen over 2^k (not 2^l which is the input domain)
fn quantize_input(input_domain_x_share:&RingElm)->RingElm{
    let mut bound_domain_x_share = RingElm::zero();

    // It works by truncation
    // A number m where k <= m <= l-2 is set based on which truncation happens
    // But the way I understand it, they define a dynamic method of truncation (?!) but the bounded domain is always 2^k (k=9? or 16?)

    bound_domain_x_share
}
