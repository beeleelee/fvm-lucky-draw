const { newFromString, encode } = require('@glif/filecoin-address')
const cbor = require('@ipld/dag-cbor')
const base64 = require("js-base64")
const { CID } = require("multiformats")

const addr1 = newFromString('f1afrqdycktgvkhpcqvrm4mcky6oyyxabtgavjeii')
console.log(addr1.str) 

const addr2 = newFromString('f173cjjdlcgclbonefmp3yhi4csbvvtiv327gmbra')
console.log(addr2.str)

const addr3 = newFromString("f14tpybg2cxfscpwydxilmsadaanfkss76woksuoy")
console.log(addr3.str)

const addr4 = newFromString("f12tlivbvfurf6neuwsqirsr4hcoi4tiynqldmk3y")
console.log(addr4.str)

const addr5 = newFromString("f1gbjayfccagwlpausgzkcs3ss54ou4jgkwu4cana")
console.log(addr5.str)

const addr6 = newFromString("f1cpwknapvsfm2zvzbzaxx4l3ce5ugbfzt3ku74sa")
console.log(addr6.str)

const owner = newFromString("f1joi27fay5otrjkn6r3ak4fwxyolkifbz3dlcwdi")

let p1 = [
    [addr1.str, addr2.str, addr3.str, addr4.str, addr5.str, addr6.str]
]
console.log("add candidates params ===================")
let p1e = cbor.encode(p1)
console.log(p1e)
console.log(Uint8Array.from(p1e))
console.log(base64.fromUint8Array(p1e).toString())
console.log("==========================================")

let p2 = [
    owner.str, 
    3,
    [
        addr1.str,
        addr2.str,
        addr3.str,
        addr4.str,
        addr5.str,
        addr6.str
    ]
]

let bs = cbor.encode(p2)
let p2str = base64.fromUint8Array(bs).toString()
console.log(p2str)

console.log(CID.parse("bafk2bzacebdn2tibnzokjprdzmbaghmr7grv4esaezer6lnn2wubk4rwmhobc").bytes)

let p3 = [
    [
    CID.parse("bafk2bzacebdn2tibnzokjprdzmbaghmr7grv4esaezer6lnn2wubk4rwmhobc").bytes,
    bs
    ]
]
let p3bs = cbor.encode(p3)
let p3str = base64.fromUint8Array(p3bs).toString()
console.log(p3str)