import { Component } from 'react'
import { Card, Input, Button, message } from 'antd'
import { LotusClient, LotusWalletProvider, HttpJsonRpcConnector } from "filecoin.js"
import * as base64 from "js-base64"
import { newFromString } from '@glif/filecoin-address'
import * as cbor from '@ipld/dag-cbor'
import './App.css';
import 'antd/dist/antd.css';

const { TextArea } = Input


class App extends Component {
  constructor(props) {
    super(props);
    this.lotusClient = null 
    this.walletProvider = null 
    this.candidates = ""
    this.state = {
      step: 1,
      rpcUrl: "",
      rpcToken: "",
      actor: "",
      owner: "",
      candidates: [],
      winners: [],
      ready: false,
      setupInfo: {
        actor: "",
        rpcUrl: "",
        rpcToken: ""
      }
    };
  }
  async checkStep1() {
    let { actor, rpcUrl, rpcToken } = this.state.setupInfo
    if (actor == "" || rpcUrl == "" || rpcToken == "") {
      message.info("please fill the setup info", 3)
      return 
    }
    
    try {
      newFromString(actor)
    } catch(err) {
      message.error("invalid actor: "+err.message, 3)
      return 
    }
    
    let connector = new HttpJsonRpcConnector({
        url: rpcUrl,
        token: rpcToken
      })
    
    this.lotusClient = new LotusClient(connector)
    
    this.walletProvider = new LotusWalletProvider(this.lotusClient)

    let owner 
    try {
      owner = await this.walletProvider.getDefaultAddress()
    } catch(err) {
      message.error("failed to get owner address: " + err.message, 3)
      return 
    }
    this.setState({
      ...this.state, 
      owner: owner,
      actor: actor,
      rpcUrl: rpcUrl,
      rpcToken: rpcToken,
      step: 2,
    })
  }
  async checkStep2() {
    let cds
    // test data
    // f1afrqdycktgvkhpcqvrm4mcky6oyyxabtgavjeii,f173cjjdlcgclbonefmp3yhi4csbvvtiv327gmbra,f14tpybg2cxfscpwydxilmsadaanfkss76woksuoy,f12tlivbvfurf6neuwsqirsr4hcoi4tiynqldmk3y,f1gbjayfccagwlpausgzkcs3ss54ou4jgkwu4cana,f1cpwknapvsfm2zvzbzaxx4l3ce5ugbfzt3ku74sa
    if(this.candidates.trim() === "") {
      message.info("Please select some candidates")
      return 
    }
    cds = this.candidates.trim().split(",")
    cds = cds.filter(item => !!item)
    try {
      let ap = new AddCandidatesParam(cds)
      
      let params = base64.fromUint8Array(ap.encode()).toString()
      // console.log("add candidates params base64: ", params)
      let _message = await this.walletProvider.createMessage({
          To: this.state.actor,
          From: this.state.owner,
          Value: 0,
          Method: 2,
          Params: params
      })
      console.log("going to call add_candidates method, num: 2")
      let signMessage = await this.walletProvider.signMessage(_message);

      let mcid = await this.walletProvider.sendSignedMessage(signMessage)
      let res = await this.lotusClient.state.waitMsg(mcid, 1)

      if (res.Receipt.ExitCode == 0) {
          message.info("add candidates success")
      } else {
          message.error(res.Receipt.Return)
          return 
      }
    } catch (err) {
      message.error(err.message)
      return 
    }
    this.setState({
      ...this.state, 
      candidates: cds,
      step: 3,
    })
  }
  async checkStep3() {
    try {
      let _message = await this.walletProvider.createMessage({
          To: this.state.actor,
          From: this.state.owner,
          Value: 0,
          Method: 3,
          Params: ""
      })
      console.log("going to call ready method, num: 3")
      
      let signMessage = await this.walletProvider.signMessage(_message);

      let mcid = await this.walletProvider.sendSignedMessage(signMessage)
      let res = await this.lotusClient.state.waitMsg(mcid, 1)

      if (res.Receipt.ExitCode == 0) {
          message.info("lucky draw is ready")
      } else {
          message.error(res.Receipt.Return)
      }
    } catch (err) {
      message.error("failed to set state ready: " + err.message)
      return 
    }
    this.setState({
      ...this.state,
      step: 4
    })
  }
  async checkStep4() {
    try {
      let _message = await this.walletProvider.createMessage({
          To: this.state.actor,
          From: this.state.owner,
          Value: 0,
          Method: 4,
          Params: ""
      })
      console.log("going to call lucky draw method, num: 4")
      
      let signMessage = await this.walletProvider.signMessage(_message);

      let mcid = await this.walletProvider.sendSignedMessage(signMessage)
      let res = await this.lotusClient.state.waitMsg(mcid, 1)

      if (res.Receipt.ExitCode === 0) {
          message.info(base64.decode(res.Receipt.Return))
      } else {
          message.error(res.Receipt.Return)
      }
    } catch (err) {
      message.error("failed to set state ready: " + err.message)
      return 
    }
  }
  renderStep1() {
    let { actor, rpcUrl, rpcToken } = this.state.setupInfo
    return (
      <div className="center-row">
        <Card title="Setup"  style={{ width: 400 }}>
          <p>
            <Input addonBefore={"actor:"} value={actor ? actor : ""} onChange={(e)=>{
              this.setState({
                ...this.state,
                setupInfo: {
                  ...this.state.setupInfo,
                  actor: e.target.value
                }
              })
            }}/>
          </p>
          <p>
            <Input addonBefore={"rpc url:"} value={rpcUrl ? rpcUrl : ""} onChange={(e)=>{
              this.setState({
                ...this.state,
                setupInfo: {
                  ...this.state.setupInfo,
                  rpcUrl: e.target.value
                }
              })
            }}/>
          </p>
          <p>
            <Input addonBefore={"rpc token:"} value={rpcToken ? rpcToken : ""} onChange={(e)=>{
              this.setState({
                ...this.state,
                setupInfo: {
                  ...this.state.setupInfo,
                  rpcToken: e.target.value
                }
              })
            }}/>
          </p>
          <p>
            <Button type="primary" onClick={() => this.checkStep1()} >Next</Button>
          </p>
        </Card>
      </div>
    )
  }
  renderStep2() {
    return (
      <div className="center-row">
        <Card title="Add Candidates"  style={{ width: 400 }}>
          <p>
            <TextArea style={{height: 260}}
              onChange={e => {
                this.candidates = e.target.value;
              }}
              />
          </p>
          
          <p>
            <Button type="primary" onClick={() => this.checkStep2()} >Next</Button>
          </p>
        </Card>
      </div>
    )
  }
  renderStep3() {
    return (
      <div className="center-row">
        <Card title="Set state ready"  style={{ width: 400 }}>
          <p>
            click next to set state ready
          </p>
          
          <p>
            <Button type="primary" onClick={() => this.checkStep3()} >Next</Button>
          </p>
        </Card>
      </div>
    )
  }
  renderStep4() {
    return (
      <div className="center-row">
        <Card title="Lucky Draw"  style={{ width: 400 }}>
          <p>
            click Draw button to draw a winner
          </p>
          
          <p>
            <Button type="primary" onClick={() => this.checkStep4()} >Draw</Button>
          </p>
        </Card>
      </div>
    )
  }
  render() {
    return (
      <div className="App">
        <header className="App-header">
          fvm lucky draw 
        </header>
        <div >
            step {this.state.step}
        </div>
        
        <div >
          {this.state.step === 1 ? this.renderStep1() : null}
          {this.state.step === 2 ? this.renderStep2() : null}
          {this.state.step === 3 ? this.renderStep3() : null}
          {this.state.step === 4 ? this.renderStep4() : null}
        </div>
        <div >
          <p/>
          {
            this.state.actor != "" ? <div >actor: {this.state.actor}</div> : null 
          }
          {
            this.state.owner != "" ? <div>owner: {this.state.owner}</div> : null 
          }
          {
            this.state.rpcUrl != "" ? <div>rpc: {this.state.rpcUrl}</div> : null 
          }
          <p/>
        </div>
      </div>
    );
  }
}

export default App;


class AddCandidatesParam {
  constructor(addrs) {
      this.addresses = addrs.map(addr => {
          let a = newFromString(addr)
          return a.str
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
