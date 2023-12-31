use libmpc::mpc_party::MPCParty;
use libmpc::protocols::pika::*;
use libmpc::mpc_platform::NetInterface;

use fss::{prg::*, RingElm};
use libmpc::offline_data::BasicOffline;
use fss::bits_to_u32;

use std::fs::File;
use std::io::Write;
use std::env;
use rand::Rng;
use std::time::Instant;
use std::thread::sleep;
use std::time::Duration;
use fss::Group;

pub const INPUT_BITS:usize = 32;
const LAN_ADDRESS: &'static str = "127.0.0.1:8888";
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
    
    gen_offlinedata();

    let seed = if is_server {PrgSeed::zero()} else {PrgSeed::one()};//Guarantee same input bits to ease the debug process
    let mut stream = FixedKeyPrgStream::new();
    stream.set_key(&seed.key);

    // TODO Implement batch here (10^0 - 10^5 used in Pika paper with bad scalability)
    // TODO generate correct shares (r1=rand from bits, r2=1-r1 or r2=negate(1)-r1 as RingElms) for x and w
    let x_share_bits = stream.next_bits(INPUT_BITS*2);
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
        let mut cur_share = RingElm::from( bits_to_u32(&x_share_bits[..INPUT_BITS]));
        result = pika_eval(&mut p, &cur_share).await;
    }else{
        let mut cur_share = RingElm::from( bits_to_u32(&x_share_bits[INPUT_BITS..]));
        result = pika_eval(&mut p, &cur_share).await;
    }

    let mut f_ret = File::create(format!( "../test/ret{}.bin", &index)).expect("create failed");
    f_ret.write_all(&bincode::serialize(&result).expect("Serialize cmp-bool-share error")).expect("Write cmp-bool-share error.");
}

fn gen_offlinedata(){
    let offline_timer = Instant::now();

    let offline = BasicOffline::new();
    offline.genData(&PrgSeed::zero(), INPUT_BITS);

    println!("Offline key generation time:{:?}", offline_timer.elapsed());
}
