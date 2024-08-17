use ore_api::{
    consts::{BUS_ADDRESSES, TOKEN_DECIMALS},
    state::Bus,
};

use ore_utils::AccountDeserialize;
use solana_client::nonblocking::rpc_client::RpcClient;

    #[no_mangle]
    pub async fn busses(rpc: &RpcClient ) {
        let rpc_client: &RpcClient = rpc;
        for address in BUS_ADDRESSES.iter() {
            let data = rpc_client.get_account_data(address).await.unwrap();
            match Bus::try_from_bytes(&data) {
                Ok(bus) => {
                    let rewards = (bus.rewards as f64) / 10f64.powf(TOKEN_DECIMALS as f64);
                    println!("Bus {}: {:} ORE", bus.id, rewards);
                    println!("Bus {} ORE", address);
                }
                Err(_) => {}
            }
        }
    }

