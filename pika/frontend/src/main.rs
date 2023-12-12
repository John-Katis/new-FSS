use libmpc::mpc_party::MPCParty;
use libmpc::protocols::pika::*;
use libmpc::mpc_platform::NetInterface;

use fss::{prg::*, RingElm};
use libmpc::offline_data::BasicOffline;
use fss::bits_to_u16;

use std::fs::File;
use std::io::Write;
use std::env;
use rand::Rng;
use std::time::Instant;
use std::thread::sleep;
use std::time::Duration;
use fss::Group;

const LAN_ADDRESS: &'static str = "127.0.0.1:8088";
const WAN_ADDRESS: &'static str = "45.63.6.86:8088";
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
    
    let offline_time: f32 = gen_offlinedata().as_secs_f32();
    let index =  if is_server {String::from("0")} else {String::from("1")};
    let index_ID = if is_server{0u8} else {1u8};

    let mut result:RingElm = RingElm::zero();
    let mut netlayer = NetInterface::new(is_server,LAN_ADDRESS).await;

    let mut offlinedata = BasicOffline::new();
    offlinedata.loadData(&index_ID);
    netlayer.reset_timer().await;
    let mut p: MPCParty<BasicOffline> = MPCParty::new(offlinedata, netlayer);
    p.setup(10, 10);

    if is_server{
        result = pika_eval(&mut p).await;
    }else{
        result = pika_eval(&mut p).await;
    }

    println!("");
    println!("---------- Benchmarking ---------- ----------");
    println!("");

    p.netlayer.print_benchmarking().await;

    // ELEMENTS OF VECTOR
    // index 0: online duration in seconds
    // index 1: rounds
    // index 2: overhead
    // index 3: offline duration in seconds
    let mut benchmarking_vec: Vec<f32> = p.netlayer.return_benchmarking().await;
    benchmarking_vec.push(offline_time);
    println!("benchmarking vector: {:?}", benchmarking_vec);
    println!("");

    let mut f_benchmarking = File::create(format!( "../test/results/p0/benchamrking_{}", &index)).expect("create failed");
    f_benchmarking.write_all(&bincode::serialize(&benchmarking_vec).expect("Serialize cmp-bool-share error")).expect("Write cmp-bool-share error.");

    let mut f_ret = File::create(format!( "../test/ret{}.bin", &index)).expect("create failed");
    f_ret.write_all(&bincode::serialize(&result).expect("Serialize cmp-bool-share error")).expect("Write cmp-bool-share error.");
}

fn gen_offlinedata()->Duration{
    let offline_timer = Instant::now();

    let offline = BasicOffline::new();
    offline.genData(&PrgSeed::zero());
    let elapsed_time = offline_timer.elapsed();
    println!("Offline key generation time:{:?}", elapsed_time);
    elapsed_time
}
