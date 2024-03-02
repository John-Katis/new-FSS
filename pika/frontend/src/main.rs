use libmpc::mpc_party::MPCParty;
use libmpc::protocols::pika::*;
use libmpc::mpc_platform::NetInterface;

use fss::{prg::*, RingElm};
use libmpc::offline_data::BasicOffline;
use fss::bits_to_u16;

use std::fs::File;
use std::io::{Write, self, BufRead, BufReader};
use std::env;
use rand::Rng;
use std::time::Instant;
use std::thread::sleep;
use std::time::Duration;
use fss::Group;

const LAN_ADDRESS: &'static str = "127.0.0.1:8088";
const WAN_ADDRESS: &'static str = "192.168.1.1:8088";
pub const TEST_WAN_NETWORK: bool = true;

#[tokio::main]
async fn main() {
    let mut is_server=false;
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // The first command-line argument (index 1) is accessed using args[1]
        let first_argument = args[1].parse::<u8>();

        // Check if the parsing was successful
        match first_argument {
            Ok(value) => {
                match value{
                    0 => is_server = true,
                    1 => {},
                    _ => eprintln!("Error: Party role illegale"),
                }
            }
            Err(_) => {
                eprintln!("Error: Unable to parse the first argument as a u8 value.");
            }
        }
    } else {
        eprintln!("No arguments provided.");
    }

    let index =  if is_server {String::from("0")} else {String::from("1")};
    let index_ID = if is_server{0u8} else {1u8};
//TODO always WAN address - change network
    let mut netlayer = NetInterface::new(is_server,WAN_ADDRESS).await;
    let mut offlinedata = BasicOffline::new();
    let mut p: MPCParty<BasicOffline> = MPCParty::new(offlinedata, netlayer);
    p.setup(10, 10);

    // let mut all_benchmarking_results: Vec<Vec<f32>> = Vec::new();
    // let mut all_protocol_outputs: Vec<Vec<RingElm>> = Vec::new();

    for run in 0..3 {

        let mut input_vec: Vec<Vec<bool>> = Vec::new();
        match read_bool_vectors_from_file("../input/input1.txt") {
            Ok(u32_vector) => { input_vec = u32_vector; }
            Err(e) => { eprintln!("Error: {}", e); }
        }
        
        let offline_time: f32 = gen_offlinedata(input_vec).as_secs_f32();
        
        let mut result: Vec<RingElm> = Vec::new();

        p.offlinedata.loadData(&index_ID);
        let offline_overhead = p.offlinedata.overhead;

        p.netlayer.reset_timer().await;

        if is_server{
            result = pika_eval(&mut p).await;
        }else{
            result = pika_eval(&mut p).await;
        }

        // p.netlayer.print_benchmarking().await;

        // ELEMENTS OF VECTOR
        // index 0: online duration in seconds
        // index 1: rounds
        // index 2: overhead
        // index 3: offline duration in seconds
        // index 4: offline overhead in bytes
        let mut benchmarking_vec: Vec<f32> = p.netlayer.return_benchmarking().await;
        benchmarking_vec.push(offline_time * 1000.0);
        benchmarking_vec.push(offline_overhead);
        // println!("");
        // println!("---------- Benchmarking ---------- ----------");
        println!("\n----- ----- -----\n----- ----- -----\n----- ----- -----\n");
        println!("----- ITERATION {} -----", run);
        println!("\n----- ----- -----\n----- ----- -----\n----- ----- -----\n");
        println!("");
        println!("benchmarking vector: {:?}", benchmarking_vec);
        println!("");
        
        // all_benchmarking_results.push(benchmarking_vec);
        // all_protocol_outputs.push(result)

        // let mut f_benchmarking = File::create(format!( "../results/p{}/benchamrking{}.txt", &index, &run)).expect("create failed");
        // for j in 0..benchmarking_vec.len() {
        //     writeln!(f_benchmarking, "{}", benchmarking_vec[j]);
        // }
        // //f_benchmarking.write_all(&bincode::serialize(&benchmarking_vec).expect("Serialize cmp-bool-share error")).expect("Write cmp-bool-share error.");

        // let mut f_ret = File::create(format!( "../results/numeric_results/p{}/ret{}.bin", &index, &run)).expect("create failed");
        // f_ret.write_all(&bincode::serialize(&result).expect("Serialize cmp-bool-share error")).expect("Write cmp-bool-share error.");

    }

    // for store_index in 0..1000 {
    //     let mut f_benchmarking = File::create(format!( "../results/p{}/benchamrking{}.txt", &index, &store_index)).expect("create failed");
    //     for j in 0..all_benchmarking_results[store_index].len() {
    //         writeln!(f_benchmarking, "{}", all_benchmarking_results[store_index][j]);
    //     }
    //     //f_benchmarking.write_all(&bincode::serialize(&benchmarking_vec).expect("Serialize cmp-bool-share error")).expect("Write cmp-bool-share error.");

    //     let mut f_ret = File::create(format!( "../results/numeric_results/p{}/ret{}.bin", &index, &store_index)).expect("create failed");
    //     f_ret.write_all(&bincode::serialize(&all_protocol_outputs[store_index]).expect("Serialize cmp-bool-share error")).expect("Write cmp-bool-share error.");
    // }

}

fn gen_offlinedata(input_bool_vectors: Vec<Vec<bool>>)->Duration{
    let offline_timer = Instant::now();

    let offline = BasicOffline::new();
    offline.genData(input_bool_vectors);
    let elapsed_time = offline_timer.elapsed();
    println!("Offline key generation time:{:?}", elapsed_time);
    elapsed_time
}

fn read_bool_vectors_from_file(file_path: &str) -> io::Result<Vec<Vec<bool>>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut bool_vector: Vec<Vec<bool>> = Vec::new();

    for line in reader.lines() {
        let value_str = line?;

        match value_str.trim().parse::<i32>() {
            
            Ok(value_i32) => {
                let value_u32 = if value_i32 >= 0 {
                    value_i32 as u32
                } else {
                    // Flip the sign bit for negative numbers
                    (value_i32.abs() as u32) ^ (1 << 31)
                };

                let bools = u32_to_bool_vector(value_u32);
                bool_vector.push(bools);
            }
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Error parsing line '{}': {}", value_str, e)));
            }
        }
    }

    Ok(bool_vector)
}

fn u32_to_bool_vector(value: u32) -> Vec<bool> {
    let bytes = value.to_be_bytes();
    let mut bool_vector = Vec::new();

    for byte in bytes.iter() {
        for i in (0..8).rev() {
            bool_vector.push((byte & (1 << i)) != 0);
        }
    }

    bool_vector
}
