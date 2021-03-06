const {LotusClient, LotusWalletProvider, HttpJsonRpcConnector} = require("filecoin.js")
const base64 = require("js-base64")
const { newFromString } = require('@glif/filecoin-address')
const cbor = require('@ipld/dag-cbor')
const { CID } = require('multiformats')

const __rpc_url__ = "http://127.0.0.1:1234/rpc/v0"
const __rpc_token__ = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJBbGxvdyI6WyJyZWFkIiwid3JpdGUiLCJzaWduIiwiYWRtaW4iXX0.zQ7XYQ95RFRvvCSqM7eXkgwlSvHDMIsR1SvhDHY7cyY"
const __actor__ = "t01004"
const __init_actor__ = "t01"
const __actor_cid__ = "bafk2bzacebdn2tibnzokjprdzmbaghmr7grv4esaezer6lnn2wubk4rwmhobc"
run()

async function run() {
    const connector = new HttpJsonRpcConnector({
        url: __rpc_url__,
        token: __rpc_token__
    })
    const lotusClient = new LotusClient(connector)
    const walletProvider = new LotusWalletProvider(lotusClient)

    let chainHead = await lotusClient.chain.getHead()
    console.log("current chain height: ", chainHead.Height)

    // owner wallet address
    const owner = await walletProvider.getDefaultAddress()
    console.log("owner address: ", owner)
    // let ip = new InitParam(owner, 3, [
    //     'f1afrqdycktgvkhpcqvrm4mcky6oyyxabtgavjeii',
    //     'f173cjjdlcgclbonefmp3yhi4csbvvtiv327gmbra',
    //     'f14tpybg2cxfscpwydxilmsadaanfkss76woksuoy',
    //     'f12tlivbvfurf6neuwsqirsr4hcoi4tiynqldmk3y',
    //     'f1gbjayfccagwlpausgzkcs3ss54ou4jgkwu4cana',
    //     'f1cpwknapvsfm2zvzbzaxx4l3ce5ugbfzt3ku74sa'
    // ])
    // let ipbs = base64.encode(ip.encode())
    
    // create actor
    //
    //  currently it doesn't work for unmatched cbor serde, it seems the leading major type not match
    //  
    // let ep = new ExecParams(__actor_cid__, base64.toUint8Array(ipbs))
    // let epstr = base64.encode(ep.encode())
    // console.log(epstr)
    // let _message = await walletProvider.createMessage({
    //     To: __init_actor__,
    //     From: owner,
    //     Value: 0,
    //     Method: 2,
    //     Params: epstr,
    // })


    // let signMessage = await walletProvider.signMessage(_message);

    // let mcid = await walletProvider.sendSignedMessage(signMessage)
    // let res = await lotusClient.state.waitMsg(mcid, 1)

    // if (res.Receipt.ExitCode == 0) {
    //     console.log(base64.decode(res.Receipt.Return))
    // } else {
    //     console.log("failed to create actor")
    // }
    

    // add candidates 
    {
        let ap = new AddCandidatesParam([
            'f1afrqdycktgvkhpcqvrm4mcky6oyyxabtgavjeii',
            'f173cjjdlcgclbonefmp3yhi4csbvvtiv327gmbra',
            'f14tpybg2cxfscpwydxilmsadaanfkss76woksuoy',
            'f12tlivbvfurf6neuwsqirsr4hcoi4tiynqldmk3y',
            'f1gbjayfccagwlpausgzkcs3ss54ou4jgkwu4cana',
            'f1cpwknapvsfm2zvzbzaxx4l3ce5ugbfzt3ku74sa'
        ])
        let params = base64.encode(ap.encode())
        console.log("add candidates params base64: ", params)
        let _message = await walletProvider.createMessage({
            To: __actor__,
            From: owner,
            Value: 0,
            Method: 2,
            Params: params
        })
        console.log("going to call add_candidates method, num: 2")
        console.log(_message)
        let signMessage = await walletProvider.signMessage(_message);

        let mcid = await walletProvider.sendSignedMessage(signMessage)
        let res = await lotusClient.state.waitMsg(mcid, 1)

        if (res.Receipt.ExitCode == 0) {
            console.log("add candidates success")
        } else {
            console.log(res.Receipt.Return)
        }
    }

    // set ready
    {
        let _message = await walletProvider.createMessage({
            To: __actor__,
            From: owner,
            Value: 0,
            Method: 3,
            Params: ""
        })
        console.log("going to call ready method, num: 3")
        console.log(_message)
        let signMessage = await walletProvider.signMessage(_message);

        let mcid = await walletProvider.sendSignedMessage(signMessage)
        let res = await lotusClient.state.waitMsg(mcid, 1)

        if (res.Receipt.ExitCode == 0) {
            console.log("lucky draw is ready")
        } else {
            console.log(res.Receipt.Return)
        }
    }

    // show current state
    {
        let _message = await walletProvider.createMessage({
            To: __actor__,
            From: owner,
            Value: 0,
            Method: 5,
            Params: ""
        })
        console.log("going to call show current state method, num: 5")

        let signMessage = await walletProvider.signMessage(_message);

        let mcid = await walletProvider.sendSignedMessage(signMessage)
        let res = await lotusClient.state.waitMsg(mcid, 1)

        if (res.Receipt.ExitCode == 0) {
            console.log(base64.decode(res.Receipt.Return))
        } else {
            console.log("exit whith ", res.Receipt.ExitCode, res.Receipt.Return)
        }
    }

    // do lucky draw
    {
        let _message = await walletProvider.createMessage({
            To: __actor__,
            From: owner,
            Value: 0,
            Method: 4,
            Params: ""
        })
        console.log("going to call lucky draw method, num: 4")

        let signMessage = await walletProvider.signMessage(_message);

        let mcid = await walletProvider.sendSignedMessage(signMessage)
        let res = await lotusClient.state.waitMsg(mcid, 1)

        if (res.Receipt.ExitCode == 0) {
            console.log(base64.decode(res.Receipt.Return))
        } else {
            console.log("exit whith ", res.Receipt.ExitCode, res.Receipt.Return)
        }
    }

    // show current state
    {
        let _message = await walletProvider.createMessage({
            To: __actor__,
            From: owner,
            Value: 0,
            Method: 5,
            Params: ""
        })
        console.log("going to call show current state method, num: 5")

        let signMessage = await walletProvider.signMessage(_message);

        let mcid = await walletProvider.sendSignedMessage(signMessage)
        let res = await lotusClient.state.waitMsg(mcid, 1)

        if (res.Receipt.ExitCode == 0) {
            console.log(base64.decode(res.Receipt.Return))
        } else {
            console.log("exit whith ", res.Receipt.ExitCode, res.Receipt.Return)
        }
    }
    
}


class AddCandidatesParam {
    constructor(addrs) {
        this.addresses = addrs.map(addr => {
            return newFromString(addr).str
        });
    }
    encode() {
        return cbor.encode([
            [...this.addresses]
        ])
    }
}


class InitParam {
    constructor(owner, winners_num, candidates) {
        this.owner = newFromString(owner).str
        this.winners_num = winners_num
        this.candidates = candidates.map(addr => {
            return newFromString(addr).str
        })
    }
    encode() {
        return cbor.encode([
            this.owner, 
            this.winners_num,
            this.candidates
        ])
    }
}

class ExecParams {
    constructor(acid, params) {
        this.code_cid = CID.parse(acid).bytes
        this.constructor_params = params
    }
    encode() {
        return cbor.encode([
            this.code_cid,
            this.constructor_params
        ])
    }
}