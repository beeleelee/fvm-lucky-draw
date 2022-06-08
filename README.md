# fvm-lucky-draw
An example implementation of fvm contract

## Introduction

It's a simple lottery draw app developed base on ealy stage of [fvm](https://github.com/filecoin-project/ref-fvm).
Therefore, the purpose for this project is demonstration and experience exchanges with other developers who are interested in developing with fvm.

**App workflow**

- Owner deploy lucky draw contract to fvm with some init state, like owner address, some candidates and the max number of winners. We'll got a contract actor after the deployment.
- Owner can add more candidates to the contract actor.
- Owner set the ready state for the contract actor.
- Owner call the draw method of the contract actor to draw a winer, and update the state of the contract actor.
- Owner can do another round of lucky draw until reach the max winner number limit
