mod blockstore;
mod types;

use std::str::FromStr;

use crate::blockstore::Blockstore;
use crate::types::*;
use cid::multihash::Code;
use cid::Cid;
//use fvm_ipld_encoding::tuple::{Deserialize_tuple, Serialize_tuple};
use fvm_ipld_encoding::{to_vec, CborStore, RawBytes, DAG_CBOR};
use fvm_sdk as sdk;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::ActorID;
use fvm_shared::address::Address;
use fvm_sdk::message;
use fvm_ipld_hamt as hamt;
use hamt::{Hamt};
use serde_json;

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
        2 => ready(params_id),
        3 => current_state(params_id),
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
    let addrst = String::from_utf8(raw).unwrap();
    let addr = match Address::from_str(addrst.as_str()) {
        Ok(a) => a, 
        Err(err) => abort!(USR_ILLEGAL_ARGUMENT, "failed to parse address: {:?}", err)
    };
    let owner = match fvm_sdk::actor::resolve_address(&addr) {
        Some(id) => id,
        None => abort!(USR_ILLEGAL_ARGUMENT, "failed to resolve address"),
    };
    let state = State{
        owner,
        candidates: Cid::default(),
        winners: vec![],
        ready: false,
        finished: false,
        winners_num: 1,
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
    
    let p: AddCandidatesParam = match serde_json::from_slice(raw.as_slice()) {
        Ok(item) => item,
        Err(err) => abort!(USR_ILLEGAL_ARGUMENT, "failed to do params convertion: {:?}", err),
    };
    let mut state: State = State::load();
    let mut candidates :Hamt<Blockstore, Candidate, ActorID>;
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
        let aid = match sdk::actor::resolve_address(addr) {
            Some(id) => id,
            None => {
                abort!(USR_ILLEGAL_ARGUMENT, "failed to resolve address: {:?}", addr)
            },
        };
        if let Err(err) = candidates.set(aid, Candidate { address: addr.clone(), actor_id: aid, winner: false }) {
            abort!(USR_ILLEGAL_STATE, "failed save candidate: {:?}", err)
        }
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

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

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
    }

}