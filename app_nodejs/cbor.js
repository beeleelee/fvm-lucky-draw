const { newFromString, encode } = require('@glif/filecoin-address')
const cbor = require('@ipld/dag-cbor')
const base64 = require("js-base64")

const addr1 = newFromString('f12zrfpwtuasimdmyuimdravhciaesljapklhd7ea')
console.log(addr1.str) 

const addr2 = newFromString('f13arowvbfjgdy3hqmzujfvknuxn2wts77l5ths3q')
console.log(addr2.str)

const addr3 = newFromString("f13cp7xurexqvs33h2nh3d5ujzg4mwc4rtrvijw7q")
console.log(addr3.str)

const addr4 = newFromString("f13tgop5lqasp3dbwxjizzkcol5du6avjqtgrvojy")
console.log(addr4.str)

const addr5 = newFromString("f14tik37yu7gejv6ifo7r2n4pcaaoyqocd74xv2zq")
console.log(addr5.str)

const addr6 = newFromString("f15am4vztyfiu3y4yiyhgawrkyz44lsxgvr3dzqmi")
console.log(addr6.str)

const owner = newFromString("f1joi27fay5otrjkn6r3ak4fwxyolkifbz3dlcwdi")

let p1 = [
    [addr1.str, addr2.str, addr3.str]
]

console.log(cbor.encode(p1))

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