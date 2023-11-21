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


pub async fn pika_eval(p: &mut MPCParty<BasicOffline>, x_share:&RingElm)->RingElm{
    let mut ret = RingElm::zero();

    // Protocol 2(a) - reconstruct x=r-a(mod2^k) -> r: random val, a: secret sharing of user input
    // FIXME - Something wrong here
    let mask = p.netlayer.exchange_ring_vec(p.offlinedata.a_share.to_vec()).await;
    let mut x = mask[0];

    println!("MASK VALUE:"); // mask is 3561744742
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
    let y_vec = p.offlinedata.k_share[0].evalAll();
    println!("y_vec LENGTH {:?}",y_vec.len());

    let func_database = load_func_db(); // -> load works but store is not done correctly -> load 16 files
    println!("FUNC DB LENGTH {}", func_database.len());

    // let mut u_vec: Vec<T> = Vec::new();

    // need ring elements that represent the correct domain (16 bits)
    // for i in 0..y_vec.len() {
    //     let shift = i + x.value;
    //     u_vec.push(&y_vec[shift]);
    // }

    // for i in 0..p.offlinedata.k_share.len(){

    //     let mask = p.exchange_ring_vec(cur+alpha);

    //     let cur: RingElm = x_share + i;

    //     let word = dpf_key_list[i].eval(&cur); // -> The input is just an index of the key - NOT ANOTHER LIST (??)
    //     word *= lOCAL_DATABSE[i]; 
    //     ret += word;
    // }
    ret
}

fn load_func_db()->Vec<f32>{
    let mut ret: Vec<f32> = Vec::new();

    for i in 0..16 {
        let mut temp: Vec<f32> = Vec::new();
        match read_file(&format!("../data/func_database/slice_{}.bin", i)) {
            Ok(value) => temp = value,
            Err(e) => println!("Error reading file: {}", e),  // Or handle the error as needed
        }

        ret.append(&mut temp)
    }

    ret
}

// Should go from 16 to 9 or 14 and check how to calculate scale in paper
fn quantize_input(input_domain_x_share:&RingElm)->RingElm{
    let mut bound_domain_x_share = RingElm::zero();

    // It works by truncation
    // A number m where k <= m <= l-2 is set based on which truncation happens
    // But the way I understand it, they define a dynamic method of truncation (?!) but the bounded domain is always 2^k (k=9? or 16?)

    bound_domain_x_share
}
