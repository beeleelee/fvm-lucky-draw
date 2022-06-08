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
    // get the params raw bytes
    let (_, raw) = match message::params_raw(params_id) {
        Ok(tup) => tup,
        Err(err) => abort!(USR_ILLEGAL_ARGUMENT, "failed to receive params: {:?}", err),
    };
    // unmarshal raw bytes to InitParam
    let ip: InitParam = match from_slice(raw.as_slice()){
        Ok(ip) => ip, 
        Err(err) => abort!(USR_ILLEGAL_ARGUMENT, "failed to unmarshal InitParam: {:?}", err),
    };

    // resolve the owner parameter to actorId
    // only owner can add candidates 
    // only owner can call ready or draw method
    let owner = match fvm_sdk::actor::resolve_address(&ip.owner){
        Some(o) => o,
        None => {
            abort!(USR_ILLEGAL_ARGUMENT, "failed to resovle owner to actor: {:?}", ip.owner)
        }
    };

    // initialize candidates state
    let candidates: Cid;
    let mut idx: u32 = 0;
    if ip.candidates.len() > 0 {
        // using hamt to store candidates
        // assign every candidate an index for lucky draw
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
    } else { // if there aren't candidates from init params, we just use default cid for placeholder
        candidates = Cid::default();
    }
    let state = State{
        owner,
        candidates,
        winners: vec![],
        ready: false,
        finished: false,
        winners_num: ip.winners_num, // this limit the max number of winners for the lucky draw contract
        canidx: idx,
    };
    state.save();
    None
}

/// Method num 2.
/// showing the current state of the contract
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
    // fetch candidates from the state of the contract
    let mut candidates: Hamt<Blockstore, Candidate, u32> = match Hamt::load(&state.candidates, Blockstore) {
        Ok(can) => can,
        Err(err) => {
            abort!(USR_ILLEGAL_STATE, "failed load candidates from store: {:?}", err)
        }
    };
    // set up 2 slice to hold this round candidates indexes and addresses
    let mut cv: Vec<u32> = Vec::new();
    let mut addrs: Vec<Address> = Vec::new();
    // select candidates who have't win a draw
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
    // we randomly choose an index from indexes slice
    let i = rng_gen_range(0, cvlen);
    let i: usize = i.try_into().unwrap();
    // this round winner comes out
    let winner = cv.as_slice()[i];
    state.winners.push(winner);
    // update the candidates hamt
    if let Err(err) = candidates.set(winner, Candidate { address: addrs.as_slice()[i].clone(), winner: true, idx: winner }) {
        abort!(USR_ILLEGAL_STATE, "failed set winner: {:?}", err)
    }
    state.candidates = match candidates.flush() {
        Ok(cid) => cid,
        Err(err) => {
            abort!(USR_ILLEGAL_STATE, "failed update candidates: {:?}", err)
        }
    };
    // check if we reached to limit of max winner number
    if state.winners.len() == state.winners_num.try_into().unwrap() {
        state.finished = true;
    }
    // update contract state
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
        let addr = "f1afrqdycktgvkhpcqvrm4mcky6oyyxabtgavjeii";
        let addr = Address::from_str(&addr).unwrap();
        assert_eq!(addr.protocol(), fvm_shared::address::Protocol::Secp256k1);
        assert_eq!(to_vec(&addr).unwrap(), vec![85, 1, 1, 99, 1, 224, 74, 153, 170, 163, 188, 80, 172, 89, 198, 9, 88, 243, 177, 139, 128, 51]);
        let addr: &str = "f020328";
        let addr = Address::from_str(&addr).unwrap();
        assert_eq!(addr.protocol(), fvm_shared::address::Protocol::ID);
        assert_eq!(to_vec(&addr).unwrap(), vec![68, 0, 232, 158, 1]);
    }

    #[test]
    fn test_add_candidates_param() {
        let addrs = vec![
            "f1afrqdycktgvkhpcqvrm4mcky6oyyxabtgavjeii",
            "f173cjjdlcgclbonefmp3yhi4csbvvtiv327gmbra", 
            "f14tpybg2cxfscpwydxilmsadaanfkss76woksuoy"
        ];
        let addrs: Vec<Address> = addrs.into_iter().map(|addr| Address::from_str(addr).unwrap()).collect();
        let p = AddCandidatesParam{
            addresses: addrs,
        };
        let bs = to_vec(&p).unwrap();
        let expect: Vec<u8> = vec![129, 131, 85, 1, 1, 99, 1, 224, 74, 153, 170, 163, 188, 80, 172, 89, 198, 9, 88, 243, 177, 139, 128, 51, 85, 1, 254, 196, 148, 141, 98, 48, 150, 23, 52, 133, 99, 247, 131, 163, 130, 144, 107, 89, 162, 187, 85, 1, 228, 223, 128, 155, 66, 185, 100, 39, 219, 3, 186, 22, 201, 0, 96, 3, 74, 169, 75, 254];
        assert_eq!(bs, expect);

        let p2: AddCandidatesParam = fvm_ipld_encoding::from_slice(&expect).unwrap();
        let addrs2 = p2.addresses;
        assert_eq!(addrs2[0].to_string(), "f1afrqdycktgvkhpcqvrm4mcky6oyyxabtgavjeii");
        assert_eq!(addrs2[1].to_string(), "f173cjjdlcgclbonefmp3yhi4csbvvtiv327gmbra");
        assert_eq!(addrs2[2].to_string(), "f14tpybg2cxfscpwydxilmsadaanfkss76woksuoy");
        assert_eq!(base64::encode(expect), "gYNVAQFjAeBKmaqjvFCsWcYJWPOxi4AzVQH+xJSNYjCWFzSFY/eDo4KQa1miu1UB5N+Am0K5ZCfbA7oWyQBgA0qpS/4=");
    }

    #[test]
    fn test_init_param() {
        let can1 = Address::from_str("f1afrqdycktgvkhpcqvrm4mcky6oyyxabtgavjeii").unwrap();
        let can2 = Address::from_str("f173cjjdlcgclbonefmp3yhi4csbvvtiv327gmbra").unwrap();
        let can3 = Address::from_str("f14tpybg2cxfscpwydxilmsadaanfkss76woksuoy").unwrap();
        let can4 = Address::from_str("f12tlivbvfurf6neuwsqirsr4hcoi4tiynqldmk3y").unwrap();
        let can5 = Address::from_str("f1gbjayfccagwlpausgzkcs3ss54ou4jgkwu4cana").unwrap();
        let can6 = Address::from_str("f1cpwknapvsfm2zvzbzaxx4l3ce5ugbfzt3ku74sa").unwrap();
        let p: InitParam = InitParam { 
            owner: Address::from_str("f1joi27fay5otrjkn6r3ak4fwxyolkifbz3dlcwdi").unwrap(),
            winners_num: 3, 
            candidates: vec![can1, can2, can3, can4, can5, can6],
        };
        let bs = to_vec(&p).unwrap();
        assert_eq!(bs, vec![131, 85, 1, 75, 145, 175, 148, 24, 235, 167, 20, 169, 190, 142, 192, 174, 22, 215, 195, 150, 164, 20, 57, 3, 134, 85, 1, 1, 99, 1, 224, 74, 153, 170, 163, 188, 80, 172, 89, 198, 9, 88, 243, 177, 139, 128, 51, 85, 1, 254, 196, 148, 141, 98, 48, 150, 23, 52, 133, 99, 247, 131, 163, 130, 144, 107, 89, 162, 187, 85, 1, 228, 223, 128, 155, 66, 185, 100, 39, 219, 3, 186, 22, 201, 0, 96, 3, 74, 169, 75, 254, 85, 1, 212, 214, 138, 134, 165, 164, 75, 230, 146, 150, 148, 17, 25, 71, 135, 19, 145, 201, 163, 13, 85, 1, 48, 82, 12, 20, 66, 1, 172, 183, 130, 146, 54, 84, 41, 110, 82, 239, 29, 78, 36, 202, 85, 1, 19, 236, 166, 129, 245, 145, 89, 172, 215, 33, 200, 47, 126, 47, 98, 39, 104, 96, 151, 51]);
        assert_eq!(base64::encode(bs), "g1UBS5GvlBjrpxSpvo7ArhbXw5akFDkDhlUBAWMB4EqZqqO8UKxZxglY87GLgDNVAf7ElI1iMJYXNIVj94OjgpBrWaK7VQHk34CbQrlkJ9sDuhbJAGADSqlL/lUB1NaKhqWkS+aSlpQRGUeHE5HJow1VATBSDBRCAay3gpI2VCluUu8dTiTKVQET7KaB9ZFZrNchyC9+L2InaGCXMw==");

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