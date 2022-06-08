# fvm-lucky-draw
An example implementation of fvm contract

## Introduction

It's a simple lottery draw app developed base on ealy stage of [fvm](https://github.com/filecoin-project/ref-fvm). 
The example actor - [fil-hello-world-actor](https://github.com/raulk/fil-hello-world-actor) is the very start point of fvm-lucky-draw.
Therefore, the purpose for this repo is demonstration and experience exchanges with other developers who are interested in developing with fvm.

**App workflow**

- Owner deploy lucky draw contract to fvm with some init state, like owner address, some candidates and the max number of winners. We'll got a contract actor after the deployment.
- Owner can add more candidates to the contract actor.
- Owner set the ready state for the contract actor.
- Owner call the draw method of the contract actor to draw a winer, and update the state of the contract actor.
- Owner can do another round of lucky draw until reach the max winner number limit

**Pain points**

- client - param types serialization on client side (mostly for javascript)
- end point - rpc call from browser been blocked because of cors 
- fvm - the caller is InitActor when creating fvm actor, but not have a way to know the real caller. I need set the actor owner when creating lucky_draw actor, so I have to pass owner address through parameters.
- fvm - tried several ways to get a random number but all failed. Seems the only choices are get_beacon_randomness and get_chain_randomness.
- fvm - have to send message to actor and burning gas to check the actor state. So the read action is time-consuming and expensive. 


**Build tips**
```shell
$ git clone https://github.com/beeleelee/fvm-lucky-draw.git
$ ## contract
$ ## build wasm
$ cd fvm-lucky-draw/contracts
$ rustup target add wasm32-unknown-unknown
$ cargo build --release
$
$ ## app_nodejs
$ cd ../app_nodejs
$ npm install
$ node index.js
$
$ ## app_browser
$ cd ../app_browser
$ npm install
$ npm start
```