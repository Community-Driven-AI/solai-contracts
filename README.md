<p align="center">
  <a href="https://solana.com">
    <img alt="Solana" src="https://user-images.githubusercontent.com/26410791/224995691-c79a548c-53bb-4fbf-b5a7-d29c04eb8c41.png" /width="200" height="200">
  </a>
</p>

# SolAI Program

> **NOTE**: the SolAI program is currently under construction. We are actively building the system and updating the program code, so please stay tuned to SolAI for updates


The SolAI program is an operating system for SolAI. SolAI acts as a coordinator, illusion generator, and standard library for Solana's first operating Federated Learning engine. For more information, visit [https://solaiprotocol.vercel.app/](https://solaiprotocol.vercel.app/)




## Program Structure

### **Rewarding Local Models and Minting NFTs**

The **`verify_model`** function verifies submitted local models. The **`mint_nft_and_distribute`** function mints Local Model NFTs and distributes them to the recipient account.

### **Peer Evaluation of Local Models**

After training, local models are uploaded and opened to the public for evaluation. There can be three malicious actions:

1. Submitting invalid models.
2. Submitting malicious models that compromise the integrity of the global model.
3. Providing incorrect evaluations of models.

To prevent these malicious actions, we create a new account to hold the models' test scores.

### **Aggregating into Global Model and Minting NFTs**

First, we update the global model with the local model using FedAvg. Then, we serialize the updated global model, write it back to the account data, and upload it to IPFS using the **`upload_to_ipfs`** function. Finally, we mint global model NFTs and distribute them to the contributors. To do this, we calculate the number of NFTs each participant should receive and then mint the NFTs and distribute them to the participants.

## Sample NFT

You can find a sample SolAI NFT that has been minted by submitting a local model in the context of federated learning at the following URL: [https://solscan.io/token/eZHzfWBPjkEVdsb9fzJGZ58SohadGr5snQDWJ2yVHBZ?cluster=devnet#metadata](https://solscan.io/token/eZHzfWBPjkEVdsb9fzJGZ58SohadGr5snQDWJ2yVHBZ?cluster=devnet#metadata)
