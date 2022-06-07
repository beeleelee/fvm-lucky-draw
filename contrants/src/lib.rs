mod blockstore;
mod types;

use crate::blockstore::Blockstore;
use crate::types::*;
use cid::multihash::Code;
use cid::Cid;
use fvm_ipld_encoding::{to_vec, CborStore, RawBytes, DAG_CBOR, from_slice};
use fvm_sdk as sdk;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::ActorID;
use fvm_sdk::message;
use fvm_ipld_hamt as hamt;
use hamt::Hamt;
use fvm_shared::address::Address;
use fvm_sdk::rand::*;


/// A macro to abort concisely.
/// This should be part of the SDK as it's very handy.
macro_rules! abort {
    ($code:ident, $msg:literal $(, $ex:expr)*) => {
        fvm_sdk::vm::abort(
            fvm_shared::error::ExitCode::$code.value(),
            Some(format!($msg, $($ex,)*).as_str()),
        )
    };
}



/// The actor's WASM entrypoint. It takes the ID of the parameters block,
/// and returns the ID of the return value block, or NO_DATA_BLOCK_ID if no
/// return value.
///
/// Should probably have macros similar to the ones on fvm.filecoin.io snippets.
/// Put all methods inside an impl struct and annotate it with a derive macro
/// that handles state serde and dispatch.
#[no_mangle]
pub fn invoke(params_id: u32) -> u32 {
    // Conduct method dispatch. Handle input parameters and return data.
    let ret: Option<RawBytes> = match message::method_number() {
        1 => constructor(params_id),
        2 => add_candidates(params_id),
        3 => ready(params_id),
        4 => lucky_draw(),
        5 => current_state(params_id),
        _ => abort!(USR_UNHANDLED_MESSAGE, "unrecognized method"),
    };

    // Insert the return data block if necessary, and return the correct
    // block ID.
    match ret {
        None => NO_DATA_BLOCK_ID,
        Some(v) => match sdk::ipld::put_block(DAG_CBOR, v.bytes()) {
            Ok(id) => id,
            Err(err) => abort!(USR_SERIALIZATION, "failed to store return value: {}", err),
        },
    }
}

/// The constructor populates the initial state.
///
/// Method num 1. This is part of the Filecoin calling convention.
/// InitActor#Exec will call the constructor on method_num = 1.
pub fn constructor(params_id: u32) -> Option<RawBytes> {
    // This constant should be part of the SDK.
    const INIT_ACTOR_ADDR: ActorID = 1;

    // Should add SDK sugar to perform ACL checks more succinctly.
    // i.e. the equivalent of the validate_* builtin-actors runtime methods.
    // https://github.com/filecoin-project/builtin-actors/blob/master/actors/runtime/src/runtime/fvm.rs#L110-L146
    if message::caller() != INIT_ACTOR_ADDR {
        abort!(USR_FORBIDDEN, "constructor invoked by non-init actor");
    }
    
    let (_, raw) = match message::params_raw(params_id) {
        Ok(tup) => tup,
        Err(err) => abort!(USR_ILLEGAL_ARGUMENT, "failed to receive params: {:?}", err),
    };
    let ip: InitParam = match from_slice(raw.as_slice()){
        Ok(ip) => ip, 
        Err(err) => abort!(USR_ILLEGAL_ARGUMENT, "failed to unmarshal InitParam: {:?}", err),
    };
    let owner = match fvm_sdk::actor::resolve_address(&ip.owner){
        Some(o) => o,
        None => {
            abort!(USR_ILLEGAL_ARGUMENT, "failed to resovle owner to actor: {:?}", ip.owner)
        }
    };
    let candidates: Cid;
    let mut idx: u32 = 0;
    if ip.candidates.len() > 0 {
        let mut bstore:Hamt<Blockstore, Candidate, u32> = Hamt::new(Blockstore);
        ip.candidates.iter().for_each(|addr|{
            if let Err(err) = bstore.set(idx, Candidate { address: addr.clone(), idx: idx, winner: false }) {
                abort!(USR_ILLEGAL_STATE, "failed save candidate: {:?}", err)
            }
            idx = idx+1;
        });
        candidates = match bstore.flush() {
            Ok(cid) => cid,
            Err(err) => {
                abort!(USR_ILLEGAL_STATE, "failed update candidates: {:?}", err)
            }
        };
    } else {
        candidates = Cid::default();
    }
    let state = State{
        owner,
        candidates,
        winners: vec![],
        ready: false,
        finished: false,
        winners_num: ip.winners_num,
        canidx: idx,
    };
    state.save();
    None
}

/// Method num 2.
pub fn current_state(_: u32) -> Option<RawBytes> {
    let state = State::load();


    let res = format!("Owner: {:?} | Ready: {} | Finished: {} | Winners: #{:?}", state.owner, state.ready, state.finished, state.winners);

    let ret = to_vec(res.as_str());
    match ret {
        Ok(ret) => Some(RawBytes::new(ret)),
        Err(err) => {
            abort!(
                USR_ILLEGAL_STATE,
                "failed to serialize return value: {:?}",
                err
            );
        }
    }
}

/// Method num 3.
pub fn lucky_draw() -> Option<RawBytes> {
    let mut state = State::load();
    
    if sdk::message::caller() !=  state.owner {
        abort!(USR_FORBIDDEN, "lucky draw invoked by non-owner actor");
    }
    if !state.ready {
        abort!(USR_ILLEGAL_STATE, "lucky draw is not ready yet");
    }
    if state.winners.len() >= state.winners_num.try_into().unwrap() {
        abort!(USR_ILLEGAL_STATE, "all winners have been drawn");
    }

    let mut candidates: Hamt<Blockstore, Candidate, u32> = match Hamt::load(&state.candidates, Blockstore) {
        Ok(can) => can,
        Err(err) => {
            abort!(USR_ILLEGAL_STATE, "failed load candidates from store: {:?}", err)
        }
    };
    let mut cv: Vec<u32> = Vec::new();
    let mut addrs: Vec<Address> = Vec::new();
    if let Err(err) = candidates.for_each(|_, cand| {
        if !cand.winner {
            cv.push(cand.idx);
            addrs.push(cand.address.clone());
        };
        Ok(())
    }){
        abort!(USR_ILLEGAL_STATE, "failed tranverse candidates: {:?}", err)
    }
    let cvlen: u32 = cv.len().try_into().unwrap();
    if cvlen == 0 {
        abort!(USR_ILLEGAL_STATE, "lack of candidates")
    }
    
    let i = rng_gen_range(0, cvlen);
    let i: usize = i.try_into().unwrap();
    let winner = cv.as_slice()[i];
    state.winners.push(winner);
    
    if let Err(err) = candidates.set(winner, Candidate { address: addrs.as_slice()[i].clone(), winner: true, idx: winner }) {
        abort!(USR_ILLEGAL_STATE, "failed set winner: {:?}", err)
    }
    state.candidates = match candidates.flush() {
        Ok(cid) => cid,
        Err(err) => {
            abort!(USR_ILLEGAL_STATE, "failed update candidates: {:?}", err)
        }
    };
    
    state.save();
    
    let res = format!("Winner: {:?}", addrs.as_slice()[i].to_string());

    let ret = to_vec(res.as_str());
    match ret {
        Ok(ret) => Some(RawBytes::new(ret)),
        Err(err) => {
            abort!(
                USR_ILLEGAL_STATE,
                "failed to serialize return value: {:?}",
                err
            )
        }
    }
}


/// Method num 4.
pub fn ready(_: u32) -> Option<RawBytes> {
    let mut state = State::load();
    
    if sdk::message::caller() !=  state.owner {
        abort!(USR_FORBIDDEN, "ready invoked by non-owner actor");
    }

    state.ready = true;
    state.save();
    None
}


pub fn add_candidates(params_id: u32) -> Option<RawBytes> {
    let (_, raw) = match message::params_raw(params_id) {
        Ok(tup) => tup,
        Err(err) => abort!(USR_ILLEGAL_ARGUMENT, "failed to receive params: {:?}", err),
    };
    
    let p: AddCandidatesParam = match from_slice(raw.as_slice()) {
        Ok(item) => item,
        Err(err) => abort!(USR_ILLEGAL_ARGUMENT, "failed to unmarshal AddCandidatesParam: {:?}", err),
    };
    let mut state: State = State::load();
    let mut candidates :Hamt<Blockstore, Candidate, u32>;
    if state.candidates.eq(&Cid::default()) {
        candidates = Hamt::new(Blockstore);
    } else {
        candidates = match Hamt::load(&state.candidates, Blockstore) {
            Ok(can) => can,
            Err(err) => {
                abort!(USR_ILLEGAL_STATE, "failed load candidates from store: {:?}", err)
            }
        }
    }
    p.addresses.iter().for_each(|addr| {
        if let Err(err) = candidates.set(state.canidx, Candidate { address: addr.clone(), idx: state.canidx, winner: false }) {
            abort!(USR_ILLEGAL_STATE, "failed save candidate: {:?}", err)
        }
        state.canidx = state.canidx + 1;
    });
    let cid = match candidates.flush() {
        Ok(cid) => cid,
        Err(err) => {
            abort!(USR_ILLEGAL_STATE, "failed update candidates: {:?}", err)
        }
    };
    state.candidates = cid;
    state.save();
    None
}

/// We should probably have a derive macro to mark an object as a state object,
/// and have load and save methods automatically generated for them as part of a
/// StateObject trait (i.e. impl StateObject for State).
impl State {
    pub fn load() -> Self {
        // First, load the current state root.
        let root = match sdk::sself::root() {
            Ok(root) => root,
            Err(err) => abort!(USR_ILLEGAL_STATE, "failed to get root: {:?}", err),
        };

        // Load the actor state from the state tree.
        match Blockstore.get_cbor::<Self>(&root) {
            Ok(Some(state)) => state,
            Ok(None) => abort!(USR_ILLEGAL_STATE, "state does not exist"),
            Err(err) => abort!(USR_ILLEGAL_STATE, "failed to get state: {}", err),
        }
    }

    pub fn save(&self) -> Cid {
        let serialized = match to_vec(self) {
            Ok(s) => s,
            Err(err) => abort!(USR_SERIALIZATION, "failed to serialize state: {:?}", err),
        };
        let cid = match sdk::ipld::put(Code::Blake2b256.into(), 32, DAG_CBOR, serialized.as_slice())
        {
            Ok(cid) => cid,
            Err(err) => abort!(USR_SERIALIZATION, "failed to store initial state: {:}", err),
        };
        if let Err(err) = sdk::sself::set_root(&cid) {
            abort!(USR_ILLEGAL_STATE, "failed to set root ciid: {:}", err);
        }
        cid
    }
}

fn rng_gen_range(low: u32, high: u32) -> u32 {
    let d: [u8;32] = [0;32];
    let r = get_beacon_randomness(32, fvm_sdk::network::curr_epoch() - 100, d.as_slice()).unwrap().0;
    let x = r.as_slice();
    let xx: [u8;4] = [x[0], x[1], x[2], x[3]];
    let xxx = u32::from_be_bytes(xx);
    low + xxx % (high - low)
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use base64;
    use std::str::FromStr;

    #[test]
    fn test_address() {
        let addr = "f12zrfpwtuasimdmyuimdravhciaesljapklhd7ea";
        let addr = Address::from_str(&addr).unwrap();
        assert_eq!(addr.protocol(), fvm_shared::address::Protocol::Secp256k1);
        assert_eq!(to_vec(&addr).unwrap(), vec![85, 1, 214, 98, 87, 218, 116, 4, 144, 193, 179, 20, 67, 7, 16, 84, 226, 64, 9, 37, 164, 15]);
        let addr: &str = "f020328";
        let addr = Address::from_str(&addr).unwrap();
        assert_eq!(addr.protocol(), fvm_shared::address::Protocol::ID);
        assert_eq!(to_vec(&addr).unwrap(), vec![68, 0, 232, 158, 1]);
    }

    #[test]
    fn test_add_candidates_param() {
        let addrs = vec!["f12zrfpwtuasimdmyuimdravhciaesljapklhd7ea", "f13arowvbfjgdy3hqmzujfvknuxn2wts77l5ths3q", "f13cp7xurexqvs33h2nh3d5ujzg4mwc4rtrvijw7q"];
        let addrs: Vec<Address> = addrs.into_iter().map(|addr| Address::from_str(addr).unwrap()).collect();
        let p = AddCandidatesParam{
            addresses: addrs,
        };
        let bs = to_vec(&p).unwrap();
        let expect: Vec<u8> = vec![129, 131, 85, 1, 214, 98, 87, 218, 116, 4, 144, 193, 179, 20, 67, 7, 16, 84, 226, 64, 9, 37, 164, 15, 85, 1, 216, 34, 235, 84, 37, 73, 135, 141, 158, 12, 205, 18, 90, 169, 180, 187, 117, 105, 203, 255, 85, 1, 216, 159, 251, 210, 36, 188, 43, 45, 236, 250, 105, 246, 62, 209, 57, 55, 25, 97, 114, 51];
        assert_eq!(bs, expect);

        let p2: AddCandidatesParam = fvm_ipld_encoding::from_slice(&expect).unwrap();
        let addrs2 = p2.addresses;
        assert_eq!(addrs2[0].to_string(), "f12zrfpwtuasimdmyuimdravhciaesljapklhd7ea");
        assert_eq!(addrs2[1].to_string(), "f13arowvbfjgdy3hqmzujfvknuxn2wts77l5ths3q");
        assert_eq!(addrs2[2].to_string(), "f13cp7xurexqvs33h2nh3d5ujzg4mwc4rtrvijw7q");
        assert_eq!(base64::encode(expect), "gYNVAdZiV9p0BJDBsxRDBxBU4kAJJaQPVQHYIutUJUmHjZ4MzRJaqbS7dWnL/1UB2J/70iS8Ky3s+mn2PtE5NxlhcjM=");
    }

    #[test]
    fn test_init_param() {
        let can1 = Address::from_str("f12zrfpwtuasimdmyuimdravhciaesljapklhd7ea").unwrap();
        let can2 = Address::from_str("f13arowvbfjgdy3hqmzujfvknuxn2wts77l5ths3q").unwrap();
        let can3 = Address::from_str("f13cp7xurexqvs33h2nh3d5ujzg4mwc4rtrvijw7q").unwrap();
        let can4 = Address::from_str("f13tgop5lqasp3dbwxjizzkcol5du6avjqtgrvojy").unwrap();
        let can5 = Address::from_str("f14tik37yu7gejv6ifo7r2n4pcaaoyqocd74xv2zq").unwrap();
        let can6 = Address::from_str("f15am4vztyfiu3y4yiyhgawrkyz44lsxgvr3dzqmi").unwrap();
        let p: InitParam = InitParam { 
            owner: Address::from_str("f1joi27fay5otrjkn6r3ak4fwxyolkifbz3dlcwdi").unwrap(),
            winners_num: 3, 
            candidates: vec![can1, can2, can3, can4, can5, can6],
        };
        let bs = to_vec(&p).unwrap();
        assert_eq!(bs, vec![131, 85, 1, 75, 145, 175, 148, 24, 235, 167, 20, 169, 190, 142, 192, 174, 22, 215, 195, 150, 164, 20, 57, 3, 134, 85, 1, 214, 98, 87, 218, 116, 4, 144, 193, 179, 20, 67, 7, 16, 84, 226, 64, 9, 37, 164, 15, 85, 1, 216, 34, 235, 84, 37, 73, 135, 141, 158, 12, 205, 18, 90, 169, 180, 187, 117, 105, 203, 255, 85, 1, 216, 159, 251, 210, 36, 188, 43, 45, 236, 250, 105, 246, 62, 209, 57, 55, 25, 97, 114, 51, 85, 1, 220, 204, 231, 245, 112, 4, 159, 177, 134, 215, 74, 51, 149, 9, 203, 232, 233, 224, 85, 48, 85, 1, 228, 208, 173, 255, 20, 249, 136, 154, 249, 5, 119, 227, 166, 241, 226, 0, 29, 136, 56, 67, 85, 1, 232, 25, 202, 230, 120, 42, 41, 188, 115, 8, 193, 204, 11, 69, 88, 207, 56, 185, 92, 213]);
        assert_eq!(base64::encode(bs), "g1UBS5GvlBjrpxSpvo7ArhbXw5akFDkDhlUB1mJX2nQEkMGzFEMHEFTiQAklpA9VAdgi61QlSYeNngzNElqptLt1acv/VQHYn/vSJLwrLez6afY+0Tk3GWFyM1UB3Mzn9XAEn7GG10ozlQnL6OngVTBVAeTQrf8U+Yia+QV346bx4gAdiDhDVQHoGcrmeCopvHMIwcwLRVjPOLlc1Q==");

        let p = InitParam {
            owner: Address::from_str("f1joi27fay5otrjkn6r3ak4fwxyolkifbz3dlcwdi").unwrap(),
            winners_num: 3,
            candidates: Vec::<Address>::new(),
        };
        let bs: Vec<u8> = to_vec(&p).unwrap();
        assert_eq!(bs, vec![131, 85, 1, 75, 145, 175, 148, 24, 235, 167, 20, 169, 190, 142, 192, 174, 22, 215, 195, 150, 164, 20, 57, 3, 128]);
        assert_eq!(base64::encode(bs), "g1UBS5GvlBjrpxSpvo7ArhbXw5akFDkDgA==");
    }

}