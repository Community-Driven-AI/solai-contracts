/*
 ____                ___       ______      ______     
/\  _`\             /\_ \     /\  _  \    /\__  _\    
\ \,\L\_\     ___   \//\ \    \ \ \L\ \   \/_/\ \/    
 \/_\__ \    / __`\   \ \ \    \ \  __ \     \ \ \    
   /\ \L\ \ /\ \L\ \   \_\ \_   \ \ \/\ \     \_\ \__ 
   \ `\____\\ \____/   /\____\   \ \_\ \_\    /\_____\
    \/_____/ \/___/    \/____/    \/_/\/_/    \/_____/
                                                      
This program is for creating SolAI NFTs
... powered by Solana Grizzlython ... powered by Solana

- Mint local model NFTs and distribute them to the model owners.
- Obtain scores from the users.
- Aggregate the scores to create a global model and mint global model NFTs.

(last updated: 14 Mar. 2023)
*/

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    program_pack::{IsInitialized, Pack, Sealed},
    sysvar,
    sysvar::{rent::Rent, Sysvar},
    system_instruction::{create_account},
    system_instruction,
    transaction::Transaction,
};
use std::mem::size_of;
use spl_token::{
    self,
    instruction::{initialize_account},
    state::{Account as TokenAccount, Mint as TokenMint},
};
use spl_token::instruction::TokenInstruction::MintTo;
use crate::spl_token::instruction::transfer;
use std::convert::TryInto;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ipfs_api::IpfsApi;

#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct NFTData {
    pub is_initialized: bool,
    pub name: [u8; 32],
    pub symbol: [u8; 10],
    pub token_uri: [u8; 128],
    pub owner: Pubkey,
}

struct Payload {
    data: String,
}

struct LocalModel {
    weights: Vec<f32>,
    num_samples: usize,
    // Add any other metadata needed for FedAvg
}

fn verify_local_model(model: LocalModel) -> bool {
    // TODO - SolAI
    // Should verify that the model meets certain criteria, 
    // such as having a minimum accuracy threshold
    // or not containing any malicious code
    true
}

// Define the program entrypoint
entrypoint!(process_instruction);

// Process the program instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
    local_model: LocalModel,
    model_scores: ModelScores,
) -> ProgramResult {

    /*
    * LOCAL MODEL
     */
    
    // Verify Submitted Local Model
    if !verify_model(local_model.clone()) {
        return Err(ProgramError::InvalidArgument);
    }

    // Mint Local Model NFT and Distribute it to the recipient account
    mint_nft_and_distribute(
        &program_id,
        &mut accounts,
        "SolAI".to_string(),
        "SolAI".to_string(),
        "https://solai.s3.amazonaws.com/local-trained-models/4a83e7fa37391919c9940b144e8232d35fd99cc15a0f56ce723020aa8b05c29c.pt".to_string(),
        &recipient_account_key,
    )?;



    /*
    * EVALUATE 
    * TODO-SolAI: test_scores_account_data
     */

    // Create a new account to hold the model's test scores
    let test_scores_account_info = next_account_info(accounts.iter())?;
    let rent = &Rent::get()?;
    let size = model_scores.scores.len() * 4;
    let rent_exempt_balance = rent.minimum_balance(size);
    let ix = system_instruction::create_account(
        &test_scores_account_info.key,
        &global_model_info.key,
        rent_exempt_balance,
        size as u64,
        &spl_token::id(),
    );
    let accounts = vec![
        AccountMeta::new_readonly(global_model_info.key, false),
        AccountMeta::new(test_scores_account_info.key, false),
        AccountMeta::new_readonly(model_scores.submitter_address, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(rent_sysvar::ID, false),
    ];
    solana_program::program::invoke(&ix, &accounts)?;

    // Store the model's test scores in the new account
    let mut test_scores_account_data = test_scores_account_info.data.borrow_mut();
    let scores_bytes = model_scores.scores.to_le_bytes();
    test_scores_account_data[0..size].copy_from_slice(&scores_bytes);



    /*
    * GLOBAL MODEL
    * TODO-SolAI: Convert this into a periodic update, rather than an on-demand update
     */

    //
    // Aggregate by FedAvg ... and Upload it to IPFS
    //

    // Get the global model state account
    let global_model_info = next_account_info(accounts.iter())?;

    // Load the global model state from the account data
    let mut global_model_data = global_model_info.data.borrow_mut();
    let mut global_model: GlobalModel = bincode::deserialize(&global_model_data[..]).unwrap();

    // Update the global model with the local model using FedAvg
    let learning_rate = 0.01; // Set the learning rate
    let batch_size = 32; // Set the batch size
    let num_samples = global_model.num_samples + local_model.num_samples;
    for (global_weights, local_weights) in global_model.weights.iter_mut().zip(local_model.weights.iter()) {
        *global_weights += (local_weights - *global_weights) * (learning_rate / batch_size as f32);
    }
    global_model.num_samples = num_samples;

    // Serialize the updated global model and write it back to the account data
    global_model_data.copy_from_slice(&bincode::serialize(&global_model).unwrap());
    upload_to_ipfs(program_id, accounts, &payload.data)?;

    //
    // Mint Global Model NFTs
    //

    // Calculate the number of NFTs each participant should receive
    let mut nfts_per_participant: HashMap<Pubkey, u64> = HashMap::new();
    let total_participation_percentage: f32 = global_model
        .participants
        .iter()
        .map(|p| p.participation_percentage)
        .sum();
    for participant in global_model.participants.iter() {
        let nfts_for_participant = (num_nfts_to_mint as f32 * participant.participation_percentage
            / total_participation_percentage)
            .round() as u64;
        nfts_per_participant.insert(participant.address, nfts_for_participant);
    }

    // Mint the NFTs and distribute them to the participants
    let token_program_info = next_account_info(accounts.iter())?;
    let mut mint_info = next_account_info(accounts.iter())?;
    let mint_authority_info = next_account_info(accounts.iter())?;
    let mut participant_token_infos: Vec<AccountInfo> = Vec::new();
    for participant in global_model.participants.iter() {
        let participant_token_info = next_account_info(accounts.iter())?;
        participant_token_infos.push(participant_token_info.clone());
    }
    let cpi_accounts = vec![
        AccountMeta::new_readonly(token_program_info.key, false),
        mint_info.clone().into(),
        mint_authority_info.clone().into(),
    ];
    let mut instructions: Vec<Instruction> = Vec::new();
    for (participant, nfts_for_participant) in nfts_per_participant {
        let participant_token_info_index = global_model
            .participants
            .iter()
            .position(|p| p.address == participant)
            .unwrap();
        let participant_token_info = &participant_token_infos[participant_token_info_index];
        instructions.push(spl_token::instruction::mint_to(
            token_program_info.key,
            mint_info.key,
            participant_token_info.key,
            mint_authority_info.key,
            &[],
            nfts_for_participant,
        )?);
    }
    let signers = &[&mint_authority_info];
    invoke_signed(&instructions, &cpi_accounts, signers)?;

    
    /*
    * CONCLUDE
     */
    
    // Sign and send the transaction
    let (recent_blockhash, fee_calculator) = client.get_recent_blockhash()?;
    let signers = vec![payer_account_key];
    let mut transaction = Transaction::new_signed_with_payer(
        &[],
        Some(&payer_account_key),
        &signers,
        recent_blockhash,
    );
    transaction.sign(&signers, fee_calculator.target_signatures);
    let result = client.send_transaction_with_config(
        &transaction,
        RpcSendTransactionConfig {
            skip_preflight: false,
            preflight_commitment: None,
            encoding: None,
        },
    )?;

    Ok(())
}

#[inline(never)]
fn upload_to_ipfs(program_id: &Pubkey, accounts: &[AccountInfo], payload_json: &str) -> ProgramResult {
    // Get the account info for the payer account
    let account_info_iter = &mut accounts.iter();
    let payer_account_info = next_account_info(account_info_iter)?;

    // Get the rent sysvar and calculate the rent exemption
    let rent = &Rent::get()?;
    let size = payload_json.as_bytes().len();
    let rent_exempt_balance = rent.minimum_balance(size);

    // Create a new account to hold the uploaded data
    let data_account_key = Pubkey::new_unique();
    let ix = system_instruction::create_account(
        &payer_account_info.key,
        &data_account_key,
        rent_exempt_balance,
        size as u64,
        program_id,
    );
    let accounts = vec![
        AccountMeta::new_readonly(payer_account_info.key, true),
        AccountMeta::new(data_account_key, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];
    solana_program::program::invoke(&ix, &accounts)?;

    // Write the data to the data account
    let mut data_account_data = data_account_info.data.borrow_mut();
    data_account_data.copy_from_slice(payload_json.as_bytes());

    // Upload the data to IPFS
    let ipfs = IpfsApi::new("127.0.0.1", 5001);
    let result = ipfs.add(data_account_data);
    let cid = result.unwrap().hash;

    // Return the CID of the uploaded data
    msg!("Uploaded data to IPFS with CID {}", cid);
    Ok(())
}

impl NFT {
    // Mint a new NFT and distribute it to the specified account
    pub fn mint_nft_and_distribute(
        program_id: &Pubkey,
        accounts: &mut [AccountInfo],
        name: [u8; 32],
        symbol: [u8; 10],
        token_uri: [u8; 128],
        recipient_account: &Pubkey,
    ) -> ProgramResult {
        // Get the account info for the NFT account, the metadata account, the payer account, and the rent sysvar
        let account_info_iter = &mut accounts.iter();
        let nft_account_info = next_account_info(account_info_iter)?;
        let metadata_account_info = next_account_info(account_info_iter)?;
        let payer_account_info = next_account_info(account_info_iter)?;
        let rent_sysvar_info = next_account_info(account_info_iter)?;
    
        // Get the rent sysvar and calculate the rent exemption
        let rent = Rent::get()?;
        let nft_account_size = NFTData::LEN;
        let rent_exempt_balance = rent
            .minimum_balance(nft_account_size)
            .max(nft_account_info.lamports());
    
        // Create the NFT account and associate it with the metadata account and the payer account
        let owner_account = recipient_account.clone();
        let mut nft_data = NFTData::default();
        nft_data.is_initialized = true;
        nft_data.name = name;
        nft_data.symbol = symbol;
        nft_data.token_uri = token_uri;
        nft_data.owner = owner_account;
    
        let ix = system_instruction::create_account(
            payer_account_info.key,
            nft_account_info.key,
            rent_exempt_balance,
            nft_account_size as u64,
            program_id,
        );
    
        let accounts = vec![
            AccountMeta::new(*payer_account_info.key, true),
            AccountMeta::new(*nft_account_info.key, false),
            AccountMeta::new(*metadata_account_info.key, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ];
    
        invoke(&ix, &accounts)?;
    
        // Store the NFT data in the NFT account
        nft_data.pack_into_slice(&mut nft_account_info.data.borrow_mut());
        Ok(())
    }

    pub fn unpack_unchecked(data: &[u8]) -> Result<Self, ProgramError> {
        Self::unpack(data).map_err(|_| ProgramError::InvalidAccountData)
    }
}