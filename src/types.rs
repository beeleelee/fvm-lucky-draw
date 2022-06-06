use cid::Cid;
use fvm_ipld_encoding::tuple::{Deserialize_tuple, Serialize_tuple};
use fvm_shared::ActorID;
use fvm_shared::address::Address;

/// The state object.
#[derive(Serialize_tuple, Deserialize_tuple, Clone, Debug)]
pub struct State {
    pub owner: ActorID,
    // map canidx => candidate
    pub candidates: Cid,
    pub winners: Vec<u32>,
    pub ready: bool,
    pub finished: bool,
    pub winners_num: u32,
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