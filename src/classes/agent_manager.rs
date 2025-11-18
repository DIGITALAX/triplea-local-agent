use crate::classes::mint::mint;
use crate::classes::{lead::lead_generation, publish::publish, remix::remix};
use crate::utils::helpers::fetch_metadata;
use crate::utils::types::{Balance, Price};
use crate::utils::{
    constants::{ACCESS_CONTROLS, AGENTS, ARTISTS, LENS_CHAIN_ID},
    contracts::{initialize_api, initialize_contracts},
    lens::{handle_lens_account, handle_tokens},
    types::{AgentActivity, AgentManager, Collection, SavedTokens, TripleAAgent, TripleAWorker},
};
use crate::ActivityType;
use chrono::{Timelike, Utc};
use dotenv::{from_filename, var};
use ethers::{
    contract::{self, ContractInstance, FunctionCall},
    core::k256::ecdsa::SigningKey,
    middleware::{Middleware, SignerMiddleware},
    providers::{Http, Provider},
    signers::{LocalWallet, Wallet},
    types::{Address, Eip1559TransactionRequest, NameOrAddress, H160, H256, U256},
};
use reqwest::Client;
use serde_json::{json, Value};
use std::{error::Error, io, str::FromStr, sync::Arc, time::Duration};
use tokio::time;

impl AgentManager {
    pub fn new(agent: &TripleAAgent) -> Option<Self> {
        let contracts = initialize_contracts(agent.id);
        initialize_api();

        match contracts {
            Some((
                access_controls_contract,
                agents_contract,
                collection_manager_contract,
                market_contract,
            )) => Some(AgentManager {
                agent: agent.clone(),
                current_queue: Vec::new(),
                agents_contract,
                access_controls_contract,
                market_contract,
                tokens: None,
                collection_manager_contract,
            }),
            None => {
                eprintln!(
                    "Failed to initialize contracts for agent with ID: {}",
                    agent.id
                );
                None
            }
        }
    }

    pub async fn resolve_activity(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.agent.last_active_time = Utc::now().num_seconds_from_midnight();
        if self.current_queue.len() > 0 {
            return Ok(());
        }

        let collections_info = self.get_collections_info().await;

        match collections_info {
            Ok(info) => {
                self.current_queue = info.clone();

                if info.len() < 1 {
                    println!(
                        "No collections for agent this round for agent_{}",
                        self.agent.id
                    );
                    return Ok(());
                }

                let _ = self.queue_lens_activity().await;
                // match self.pay_rent().await {
                //     Ok(_) => {
                //         let _ = self.queue_lens_activity().await;
                //     }
                //     Err(err) => {
                //         eprintln!("Error paying rent: {:?}", err);
                //     }
                // }
            }
            Err(err) => {
                eprintln!("Error obtaining collection information: {:?}", err);
            }
        }

        Ok(())
    }

    async fn check_gas_balance(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let method = self.access_controls_contract.method::<_, U256>(
            "getNativeGrassBalance",
            H160::from_str(&self.agent.wallet.clone()).unwrap(),
        );

        match method {
            Ok(call) => {
                let result: Result<
                    U256,
                    contract::ContractError<
                        SignerMiddleware<Arc<Provider<Http>>, LocalWallet>,
                    >,
                > = call.call().await;

                match result {
                    Ok(balance) => {
                        println!("Agent GHO Balance: {} ({}  GHO)\n", balance, balance.as_u128() as f64 / 1e18);
                        let threshold = U256::from(10_000_000_000_000_000u128);

                        if balance < threshold {
                            eprintln!("\n❌ ERROR: Insufficient GHO balance!");
                            eprintln!("Current balance: {} GHO", balance.as_u128() as f64 / 1e18);
                            eprintln!("Minimum required: 0.01 GHO");
                            eprintln!("\nPlease send GHO to your agent's wallet: {}", self.agent.wallet);
                            eprintln!("GHO is the native gas token on Lens mainnet.\n");
                            return Err("Insufficient GHO balance. Please top up your agent's wallet.".into());
                        }

                        println!("✓ GHO balance sufficient for transactions\n");
                        Ok(())
                    }
                    Err(err) => {
                        eprintln!("Error checking agent GHO balance: {}", err);
                        Err(Box::new(err))
                    }
                }
            }
            Err(err) => {
                eprintln!("Error creating balance check method: {}", err);
                Err(Box::new(err))
            }
        }
    }

    async fn pay_rent(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut rent_tokens: Vec<H160> = vec![];
        let mut rent_collection_ids: Vec<U256> = vec![];

        let _ = self.check_gas_balance().await;

        println!("Queue {:?}", &self.current_queue);

        for collection in &self.current_queue {
            println!("Collection {:?}", &collection.collection_id);

            for price in &collection.collection.prices {
                println!("Price {:?}", &price);

                let method = self.agents_contract.method::<_, U256>(
                    "getAgentRentBalance",
                    (
                        H160::from_str(&price.token).unwrap(),
                        self.agent.id,
                        collection.collection.collection_id,
                    ),
                );

                match method {
                    Ok(call) => {
                        let result: Result<
                            U256,
                            contract::ContractError<
                                SignerMiddleware<Arc<Provider<Http>>, LocalWallet>,
                            >,
                        > = call.call().await;

                        match result {
                            Ok(balance) => {
                                println!("Balance: {}\n", balance);

                                match self.calculate_rent(collection, &price.token).await {
                                    Ok(cycle_rent) => {
                                        if balance >= cycle_rent {
                                            rent_tokens.push(H160::from_str(&price.token).unwrap());
                                            rent_collection_ids.push(collection.collection_id);
                                            break;
                                        }
                                    }
                                    Err(err) => {
                                        eprintln!("Error calculating rent: {}", err);
                                    }
                                }
                            }
                            Err(err) => {
                                eprintln!("Error calling token balance: {}", err);
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("Error getting active token balance: {}", err);
                    }
                }
            }
        }

        println!(
            "Rent: {:?} {:?}\n",
            rent_collection_ids.clone(),
            rent_tokens
        );

        if rent_collection_ids.len() > 0 {
            println!(
                "Method data {:?} {:?} {}",
                rent_tokens,
                rent_collection_ids.clone(),
                self.agent.id
            );
            let method = self
                .agents_contract
                .method::<(Vec<Address>, Vec<U256>, U256), H256>(
                    "payRent",
                    (
                        rent_tokens.clone(),
                        rent_collection_ids.clone(),
                        U256::from(self.agent.id as u64),
                    ),
                );

            match method {
                Ok(call) => {
                    let FunctionCall { tx, .. } = call;

                    if let Some(tx_request) = tx.as_eip1559_ref() {
                        let gas_price = U256::from(500_000_000_000u64);
                        let max_priority_fee = U256::from(25_000_000_000u64);
                        let gas_limit = U256::from(300_000);

                        let client = self.agents_contract.client().clone();
                        let chain_id = *LENS_CHAIN_ID;
                        let req = Eip1559TransactionRequest {
                            from: Some(self.agent.wallet.parse::<Address>().unwrap()),
                            to: Some(NameOrAddress::Address(AGENTS.parse::<Address>().unwrap())),
                            gas: Some(gas_limit),
                            value: tx_request.value,
                            data: tx_request.data.clone(),
                            max_priority_fee_per_gas: Some(max_priority_fee),
                            max_fee_per_gas: Some(gas_price + max_priority_fee),
                            chain_id: Some(chain_id.into()),
                            ..Default::default()
                        };

                        let pending_tx = match client.send_transaction(req, None).await {
                            Ok(tx) => tx,
                            Err(e) => {
                                eprintln!("Error sending the transaction for payRent: {:?}", e);
                                Err(Box::new(e))?
                            }
                        };

                        let tx_hash = match pending_tx.confirmations(1).await {
                            Ok(hash) => hash,
                            Err(e) => {
                                eprintln!("Error with transaction confirmation: {:?}", e);
                                Err(Box::new(e))?
                            }
                        };

                        println!("Agent {} TX Hash: {:?}", self.agent.id, tx_hash);

                        self.current_queue
                            .retain(|item| rent_collection_ids.contains(&item.collection_id));

                        println!(
                            "Final queue for agent{}: {:?}",
                            self.agent.id, self.current_queue
                        );

                        Ok(())
                    } else {
                        self.current_queue = Vec::new();
                        eprintln!("Error in sending Transaction");
                        Err(Box::new(io::Error::new(
                            io::ErrorKind::Other,
                            "Error in sending Transaction",
                        )))
                    }
                }

                Err(err) => {
                    self.current_queue = Vec::new();
                    eprintln!("Error in create method for payRent: {:?}", err);
                    Err(Box::new(err))
                }
            }
        } else {
            println!(
                "No collection Ids with sufficient tokens for agent_{}",
                self.agent.id
            );

            self.current_queue.retain(|item| {
                ARTISTS
                    .iter()
                    .any(|a| a.eq_ignore_ascii_case(&item.collection.artist))
            });

            println!("Queue retained for artists {:?}", self.current_queue);

            Ok(())
        }
    }

    async fn get_collections_info(
        &self,
    ) -> Result<Vec<AgentActivity>, Box<dyn Error + Send + Sync>> {
        let client = Client::new();
        let query = json!({
            "query": r#"
            query ($SkyhuntersAgentManager_id: Int!) {
                agentCreateds(where: { SkyhuntersAgentManager_id: $SkyhuntersAgentManager_id }, first: 1) {
                    balances {
                     rentBalance
                     bonusBalance
                     collectionId
                    }
                    workers {
                        publish
                        remix
                        lead
                        mint
                        leadFrequency
                        publishFrequency
                        mintFrequency
                        remixFrequency
                        instructions
                        collectionId
                        collection {
                            artist
                            collectionId
                            uri
                            metadata {
                                description
                                title
                                image
                            }
                            prices {
                                price
                                token
                            }  
                        }
                    }
                }
            }
            "#,
            "variables": {
                "SkyhuntersAgentManager_id": self.agent.id
            }
        });
        let graph_url = "https://triplea.digitalax.xyz";
        let response = time::timeout(Duration::from_secs(60), async {
            let res = client
                .post(graph_url)
                .json(&query)
                .send()
                .await?;

            res.json::<Value>().await
        })
        .await;

        match response {
            Ok(result) => match result {
                Ok(result) => {
                    let empty_vec = vec![];
                    let agent_createds = result["data"]["agentCreateds"]
                        .as_array()
                        .unwrap_or(&empty_vec);
                    let mut activities = Vec::new();

                    for agent_created in agent_createds {
                        let balances = agent_created["balances"]
                            .as_array()
                            .unwrap_or(&vec![])
                            .iter()
                            .map(|balance| {
                                let collection_id = U256::from_dec_str(
                                    balance["collectionId"].as_str().unwrap_or("0"),
                                )
                                .unwrap_or_default();

                                (
                                    collection_id,
                                    Balance {
                                        rent_balance: U256::from_dec_str(
                                            balance["rentBalance"].as_str().unwrap_or("0"),
                                        )
                                        .unwrap_or_default(),
                                        bonus_balance: U256::from_dec_str(
                                            balance["bonusBalance"].as_str().unwrap_or("0"),
                                        )
                                        .unwrap_or_default(),
                                    },
                                )
                            })
                            .collect::<std::collections::HashMap<_, _>>();

                        for worker in agent_created["workers"].as_array().unwrap_or(&vec![]) {
                            let collection_id =
                                U256::from_dec_str(worker["collectionId"].as_str().unwrap_or("0"))
                                    .unwrap_or_default();

                            let artist = worker["collection"]["artist"]
                                .as_str()
                                .unwrap_or_default()
                                .to_string();

                            let username =
                                handle_lens_account(&artist, true).await.unwrap_or_default();

                            let default_balance = Balance {
                                rent_balance: U256::zero(),
                                bonus_balance: U256::zero(),
                            };
                            let balance: &Balance =
                                balances.get(&collection_id).unwrap_or(&default_balance);

                            let metadata = worker["collection"]["metadata"].clone();
                            let is_metadata_empty = metadata.is_null()
                                || metadata.as_object().map(|o| o.is_empty()).unwrap_or(false);

                            let metadata_filled = if is_metadata_empty {
                                if let Some(uri) = worker["collection"]["uri"].as_str() {
                                    fetch_metadata(uri).await.unwrap_or(json!({}))
                                } else {
                                    json!({})
                                }
                            } else {
                                metadata
                            };
                            activities.push(AgentActivity {
                                collection: Collection {
                                    collection_id,
                                    artist,
                                    username,
                                    image: metadata_filled["image"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string(),
                                    title: metadata_filled["title"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string(),
                                    description: metadata_filled["description"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string(),
                                    prices: worker["collection"]["prices"]
                                        .as_array()
                                        .unwrap_or(&vec![])
                                        .iter()
                                        .filter_map(|v| {
                                            Some(Price {
                                                price: v["price"]
                                                    .as_str()
                                                    .and_then(|s| U256::from_dec_str(s).ok())
                                                    .unwrap_or_default(),
                                                token: v["token"]
                                                    .as_str()
                                                    .unwrap_or_default()
                                                    .to_string(),
                                            })
                                        })
                                        .collect(),
                                },
                                token: worker["token"].as_str().unwrap_or_default().to_string(),
                                balance: balance.clone(),
                                collection_id,
                                worker: TripleAWorker {
                                    instructions: worker["instructions"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string(),
                                    lead: worker["lead"].as_bool().unwrap_or_default(),
                                    publish: worker["publish"].as_bool().unwrap_or_default(),
                                    mint: worker["mint"].as_bool().unwrap_or_default(),
                                    remix: worker["remix"].as_bool().unwrap_or_default(),
                                    lead_frequency: U256::from_dec_str(
                                        worker["leadFrequency"].as_str().unwrap_or("0"),
                                    )
                                    .unwrap_or_default(),
                                    publish_frequency: U256::from_dec_str(
                                        worker["publishFrequency"].as_str().unwrap_or("0"),
                                    )
                                    .unwrap_or_default(),
                                    remix_frequency: U256::from_dec_str(
                                        worker["remixFrequency"].as_str().unwrap_or("0"),
                                    )
                                    .unwrap_or_default(),
                                    mint_frequency: U256::from_dec_str(
                                        worker["mintFrequency"].as_str().unwrap_or("0"),
                                    )
                                    .unwrap_or_default(),
                                },
                            });
                        }
                    }

                    Ok(activities)
                }
                Err(err) => {
                    eprintln!("Error in response: {:?}", err);
                    Err(Box::new(err))
                }
            },
            Err(err) => {
                eprintln!("Time out: {:?}", err);
                Err(Box::new(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("Timeout: {:?}", err),
                )))
            }
        }
    }

    async fn queue_lens_activity(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let current_time = Utc::now().num_seconds_from_midnight() as i64;
        let remaining_time = self.agent.clock as i64 - current_time;

        let adjusted_remaining_time = if remaining_time > 0 {
            remaining_time
        } else {
            7200
        };

        let queue = self.current_queue.clone();
        let queue_size = queue.len() as i64;

        let interval = if queue_size > 0 {
            adjusted_remaining_time / queue_size
        } else {
            0
        };

        println!(
            "Queue Length for Agent_{} before loop: {}",
            self.agent.id,
            self.current_queue.len()
        );

        for activity in queue {
            let tokens = handle_tokens(
                self.agent.id,
                &self.agent.account_address,
                self.tokens.clone(),
            )
            .await;

            match tokens {
                Ok(new_tokens) => {
                    self.tokens = Some(new_tokens);
                    self.current_queue
                        .retain(|item| item.collection_id == activity.collection_id);

                    let agent = self.agent.clone();
                    let tokens = self.tokens.clone();
                    let collection_contract = self.collection_manager_contract.clone();
                    let agents_contract = self.agents_contract.clone();
                    let market_contract = self.market_contract.clone();

                    tokio::spawn(async move {
                        cycle_activity(
                            &agent,
                            tokens,
                            &activity,
                            interval,
                            collection_contract,
                            agents_contract,
                            market_contract,
                        )
                        .await;
                    });
                }

                Err(err) => {
                    eprintln!("Error renewing Lens tokens: {:?}", err);
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(interval as u64)).await;
        }

        println!(
            "Queue Length for Agent_{} after finishing: {}",
            self.agent.id,
            self.current_queue.len()
        );

        Ok(())
    }

    async fn calculate_rent(
        &self,
        activity: &AgentActivity,
        token: &String,
    ) -> Result<U256, Box<dyn Error + Send + Sync>> {
        let mut rent_total = U256::from(0);

        if activity.worker.lead {
            let rent_method = self
                .access_controls_contract
                .method::<_, U256>("getTokenCycleRentLead", H160::from_str(token).unwrap());

            match rent_method {
                Ok(rent_call) => {
                    let token_result: Result<
                        U256,
                        contract::ContractError<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>,
                    > = rent_call.call().await;

                    match token_result {
                        Ok(rent_threshold) => {
                            rent_total += rent_threshold * activity.worker.lead_frequency;
                        }
                        Err(err) => {
                            eprintln!("Error in rent method lead: {}", err);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error in rent method lead: {}", err);
                }
            }
        }

        if activity.worker.publish {
            let rent_method = self
                .access_controls_contract
                .method::<_, U256>("getTokenCycleRentPublish", H160::from_str(token).unwrap());

            match rent_method {
                Ok(rent_call) => {
                    let token_result: Result<
                        U256,
                        contract::ContractError<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>,
                    > = rent_call.call().await;

                    match token_result {
                        Ok(rent_threshold) => {
                            rent_total += rent_threshold * activity.worker.publish_frequency;
                        }
                        Err(err) => {
                            eprintln!("Error in rent method publish: {}", err);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error in rent method publish: {}", err);
                }
            }
        }

        if activity.worker.remix {
            let rent_method = self
                .access_controls_contract
                .method::<_, U256>("getTokenCycleRentRemix", H160::from_str(token).unwrap());

            match rent_method {
                Ok(rent_call) => {
                    let token_result: Result<
                        U256,
                        contract::ContractError<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>,
                    > = rent_call.call().await;

                    match token_result {
                        Ok(rent_threshold) => {
                            rent_total += rent_threshold * activity.worker.remix_frequency;
                        }
                        Err(err) => {
                            eprintln!("Error in rent method remix: {}", err);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error in rent method remix: {}", err);
                }
            }
        }

        if activity.worker.mint {
            let rent_method = self
                .access_controls_contract
                .method::<_, U256>("getTokenCycleRentMint", H160::from_str(token).unwrap());

            match rent_method {
                Ok(rent_call) => {
                    let token_result: Result<
                        U256,
                        contract::ContractError<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>,
                    > = rent_call.call().await;

                    match token_result {
                        Ok(rent_threshold) => {
                            rent_total += rent_threshold * activity.worker.mint_frequency;
                        }
                        Err(err) => {
                            eprintln!("Error in rent method mint: {}", err);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error in rent method mint: {}", err);
                }
            }
        }

        Ok(rent_total)
    }
}

async fn cycle_activity(
    agent: &TripleAAgent,
    tokens: Option<SavedTokens>,
    activity: &AgentActivity,
    interval: i64,
    collection_manager_contract: Arc<
        ContractInstance<
            Arc<SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>>,
            SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>,
        >,
    >,
    agents_contract: Arc<
        ContractInstance<
            Arc<SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>>,
            SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>,
        >,
    >,
    market_contract: Arc<
        ContractInstance<
            Arc<SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>>,
            SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>,
        >,
    >,
) {
    let total_activities = activity.worker.lead_frequency.as_u64()
        + activity.worker.publish_frequency.as_u64()
        + activity.worker.remix_frequency.as_u64()
        + activity.worker.mint_frequency.as_u64();

    if total_activities == 0 {
        println!("⚠️ No activities found. Skipping cycle.");
        return;
    }
    let activity_interval = interval / total_activities as i64;

    let mut tasks = vec![];
    for _ in 0..activity.worker.lead_frequency.as_u64() {
        tasks.push(ActivityType::Lead);
    }
    for _ in 0..activity.worker.publish_frequency.as_u64() {
        tasks.push(ActivityType::Publish);
    }
    for _ in 0..activity.worker.remix_frequency.as_u64() {
        tasks.push(ActivityType::Remix);
    }
    for _ in 0..activity.worker.mint_frequency.as_u64() {
        tasks.push(ActivityType::Mint);
    }

    tasks = distribute_tasks(tasks);

    if tasks.is_empty() {
        println!("⚠️ No tasks available after distribution.");
        return;
    }

    println!("Tasks to run: {:?}", tasks);

    let handles: Vec<_> = tasks
        .into_iter()
        .enumerate()
        .map(|(i, task)| {
            let agent = agent.clone();
            let tokens = tokens.clone();
            let collection = activity.collection.clone();
            let instructions = activity.worker.instructions.clone();
            let collection_contract = collection_manager_contract.clone();
            let agents_contract = agents_contract.clone();
            let market_contract = market_contract.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(
                    (i as i64 * activity_interval) as u64,
                ))
                .await;

                match task {
                    ActivityType::Mint => {
                        let tokens =
                            handle_tokens(agent.id, &agent.account_address, tokens.clone()).await;

                        match tokens {
                            Ok(new_tokens) => {
                                let _ = mint(
                                    &agent,
                                    Some(new_tokens),
                                    collection_contract,
                                    agents_contract,
                                    market_contract,
                                    &collection,
                                )
                                .await;
                            }

                            Err(err) => {
                                eprintln!("Error renewing Lens tokens on Mint: {:?}", err);
                            }
                        }
                    }
                    ActivityType::Lead => {
                        let tokens =
                            handle_tokens(agent.id, &agent.account_address, tokens.clone()).await;

                        match tokens {
                            Ok(new_tokens) => {
                                let _ = lead_generation(
                                    &agent,
                                    &collection,
                                    Some(new_tokens),
                                    &instructions,
                                )
                                .await;
                            }
                            Err(err) => {
                                eprintln!("Error renewing Lens tokens on Lead Gen: {:?}", err);
                            }
                        }
                    }
                    ActivityType::Publish => {
                        let tokens =
                            handle_tokens(agent.id, &agent.account_address, tokens.clone()).await;

                        match tokens {
                            Ok(new_tokens) => {
                                let _ =
                                    publish(&agent, Some(new_tokens), &collection, &instructions)
                                        .await;
                            }
                            Err(err) => {
                                eprintln!("Error renewing Lens tokens on Publish: {:?}", err);
                            }
                        }
                    }
                    ActivityType::Remix => {
                        let tokens =
                            handle_tokens(agent.id, &agent.account_address, tokens.clone()).await;

                        match tokens {
                            Ok(new_tokens) => {
                                let _ = remix(
                                    &agent,
                                    &collection,
                                    Some(new_tokens),
                                    collection_contract,
                                )
                                .await;
                            }
                            Err(err) => {
                                eprintln!("Error renewing Lens tokens on Remix: {:?}", err);
                            }
                        }
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.await;
    }

    println!(
        "Finished cycle_activity for Agent_{} and Activity {:?}",
        agent.id, activity
    );
}

fn distribute_tasks(mut tasks: Vec<ActivityType>) -> Vec<ActivityType> {
    let mut distributed = vec![];
    while !tasks.is_empty() {
        if let Some(task) = tasks.pop() {
            distributed.push(task);
        }
        if tasks.len() > 1 {
            tasks.rotate_left(1);
        }
    }
    distributed
}
