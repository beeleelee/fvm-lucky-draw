use cid::Cid;
use fvm_ipld_encoding::tuple::{Deserialize_tuple, Serialize_tuple};
use fvm_shared::ActorID;
use fvm_shared::address::Address;

/// The state object.
#[derive(Serialize_tuple, Deserialize_tuple, Clone, Debug)]
pub struct State {
    // only owner can call add_candidates | ready | draw methods
    pub owner: ActorID,
    // map canidx => candidate  
    // if a candidate is already a winner will not go to next round lucky draw
    pub candidates: Cid,
    // store indexes of winner candidates
    pub winners: Vec<u32>,
    // indicating if lucky draw contract is ready to draw winners
    pub ready: bool,
    // not used for now
    pub finished: bool,
    // this limit the max number of winners for the lucky draw contract
    pub winners_num: u32,
    // the auto increament index that assign to candidate
    pub canidx: u32,
}

#[derive(Serialize_tuple, Deserialize_tuple, Clone, Debug, PartialEq)]
pub struct Candidate {
    pub address: Address,
    pub winner: bool,
    pub idx: u32,
}


#[derive(Serialize_tuple, Deserialize_tuple, Clone, Debug)]
pub struct AddCandidatesParam {
    pub addresses: Vec<Address>,
}

#[derive(Serialize_tuple, Deserialize_tuple, Clone, Debug)]
pub struct InitParam {
    pub owner: Address,
    pub winners_num: u32,
    pub candidates: Vec<Address>,
}