use crate::mpc_party::MPCParty;
use fss::*;
use fss::dpf::*;
use fss::RingElm;
use fss::BinElm;
use crate::offline_data::*;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

// Import read_file() func from offline_data.rs
use crate::offline_data::read_file;

pub const TOTAL_BITS:usize = 32;


pub async fn pika_eval(p: &mut MPCParty<BasicOffline>, x_share:&RingElm)->RingElm{
    let mut ret = RingElm::zero();

    // Protocol 2(a) - reconstruct x=r-a(mod2^k) -> r: random val, a: secret sharing of user input
    let mask = p.netlayer.exchange_ring_vec(p.offlinedata.a_share.to_vec()).await;
    let mut x = mask[0];

    println!("MASK VALUE:");
    x.print();
    println!("");

    println!("X VALUE:");
    x_share.print();
    println!("");

    x.sub(x_share);
    println!("X SUBBED VALUE:");
    x.print();
    println!("");

    // Protocol 2(b) - compute yσ (EvalAll routine -> implement in DPF key)
    // FIXME
    let y_vec = p.offlinedata.k_share[0].evalAll();
    println!("y_vec LENGTH {:?}",y_vec.len());
    println!("y_vec elem1 {:?}",y_vec[0]);

    let func_database = load_func_db(); // -> load works but store is not done correctly -> load 32 files
    println!("FUNC DB LENGTH {}", func_database.len());

    // let mut u: RingElm = RingElm::from(0);

    // // Protocol 2(c) - compute uσ then u
    // for i in 0..y_vec.len() {
    //     let progress = i / y_vec.len();
    //     println!("STEP 2C PROGRESS: {}", progress);
    //     let shift_index = i + x.to_u32().unwrap_or_default() as usize;
    //     let mut temp = RingElm::from(func_database[i]) * y_vec[shift_index];
        
    //     if !p.netlayer.is_server {
    //         temp.negate();
    //     }

    //     u = u + temp;
    // }

    // // TODO u = -u if not is_server || can be modelled in the main.rs as well as a subtraction?? otherwise I have type mismatches

    // println!("u VALUE (WITHOUT -1^σ)");
    // u.print();
    // println!("");

    // TODOs 
    // 1. Test evalAll correctness
    // 2. Test a_share and w_share correctness
    // 3. Beaver triple with w and u -> ret
    // 4. Test correctness of returned results

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

fn quantize_input(input_domain_x_share:&RingElm)->RingElm{
    let mut bound_domain_x_share = RingElm::zero();

    // It works by truncation
    // A number m where k <= m <= l-2 is set based on which truncation happens
    // But the way I understand it, they define a dynamic method of truncation (?!) but the bounded domain is always 2^k (k=9? or 16?)

    bound_domain_x_share
}
