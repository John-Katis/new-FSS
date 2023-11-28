use crate::mpc_party::MPCParty;
use fss::*;
use fss::dpf::*;
use fss::RingElm;
use fss::BinElm;
use crate::offline_data::*;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

pub async fn pika_eval(p: &mut MPCParty<BasicOffline>, x_share:&RingElm)->RingElm{
    let mut ret = RingElm::zero();

    // Protocol 2(a) - reconstruct x=r-a(mod2^k) -> r: random val, a: secret sharing of user input
    // FIXME - Something wrong here - shares are the number and 0
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

    let mut u: RingElm = RingElm::from(0);
    // let mut u: f32 = 0.0;

    // Protocol 2(c) - compute uσ then u
    for i in 0..y_vec.len() {
        // ##### ##### WAY 1 - u as RingElm ##### #####
        let shift_index = RingElm::from(i as u16) + x;
        let y_elem = y_vec[shift_index.to_u16().unwrap_or_default() as usize];
        // CURRENTLY: converting float values from func_database to ring elements as u16 (not good)
        // REQUIRED: Inner product of function database and evalAll output
        u = u + y_vec[shift_index.to_u16().unwrap_or_default() as usize] * RingElm::from(func_database[i]); // need the -1^σ

        // // ##### ##### WAY 2 - u as float ##### #####
        // let shift_index = RingElm::from(i as u16) + x;
        // let y_elem = y_vec[shift_index.to_u16().unwrap_or_default() as usize];
        // // CURRENTLY: converting float values from func_database to ring elements as u16 (not good)
        // // REQUIRED: Inner product of function database and evalAll output
        // u = u + y_elem.to_u16().unwrap_or_default() as f32 * func_database[i]; // need the -1^σ
    }

    println!("u VALUE (WITHOUT -1^σ)");
    u.print();
    println!("");

    // vvv QUESTIONS vvv
    // 1. See dpf, I have the bits in isolation but which one defines t0(v)
    // 2. How to do -1^σ (always has output of -1)
    // 3. See step 2(d)
    // 4. For finding u, I need to multiply -1^σ (by static casting?) with a RingElm and a f32 -> how can this be done? Should ring elements be a different type instead?
    // 5. Implemented From<f32> in ring.rs -> should all ring element values be floats?
    // 6. Also ring.rs -> i return the random number as u16 (not u32 as in Rust implementation), is that ok?
    // ^^^ QUESTIONS ^^^

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

// The function database, evalAll and calculation here must happen of 2^k (not 2^l which is the input domain)
fn quantize_input(input_domain_x_share:&RingElm)->RingElm{
    let mut bound_domain_x_share = RingElm::zero();

    // It works by truncation
    // A number m where k <= m <= l-2 is set based on which truncation happens
    // But the way I understand it, they define a dynamic method of truncation (?!) but the bounded domain is always 2^k (k=9? or 16?)

    bound_domain_x_share
}
