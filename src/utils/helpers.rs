use crate::utils::{
    constants::{
        BONSAI,
        COLLECTION_MANAGER,
        INFURA_GATEWAY,
        LENS_CHAIN_ID,
        MONA,
        WGHO,
    },
    contracts::initialize_provider,
    ipfs::upload_ipfs,
    lens::handle_lens_account,
    types::{
        AgentManager,
        CollectionInput,
        CollectionWorker,
        MessageExample,
        PriceCollection,
        Text,
        TripleAAgent,
    },
    venice::call_drop_details,
};
use chrono::Utc;
use dotenv::{ from_filename, var };
use ethers::{
    contract::{ ContractInstance, FunctionCall },
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{ Http, Middleware, Provider },
    signers::Wallet,
    types::{
        transaction::eip2718::TypedTransaction,
        Address,
        Eip1559TransactionRequest,
        NameOrAddress,
        TransactionRequest,
        H160,
        H256,
        U256,
        U64,
    },
    utils::hex,
};
use rand::{ rngs::StdRng, Rng, SeedableRng };
use regex::Regex;
use reqwest::Client;
use serde_json::{ json, to_string, Value };
use std::{ collections::HashMap, error::Error, io, str::FromStr, sync::Arc };

pub fn extract_values_prompt(
    input: &str
) -> Result<(String, String), Box<dyn Error + Send + Sync>> {
    let image_prompt_re = Regex::new(r"(?m)^Image Prompt:\s*(.+)")?;
    let model_re = Regex::new(r"(?m)^Model:\s*(.+)")?;

    let image_prompt = image_prompt_re
        .captures(input)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
        .unwrap_or_default()
        .to_string();
    let model = model_re
        .captures(input)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
        .unwrap_or_default()
        .to_string();

    Ok((image_prompt, model))
}

pub fn extract_values_image(
    input: &str
) -> Result<(String, String, U256, Vec<U256>), Box<dyn Error + Send + Sync>> {
    let title_re = Regex::new(r"(?m)^Title:\s*(.+)")?;
    let description_re = Regex::new(r"(?m)^Description:\s*(.+)")?;
    let amount_re = Regex::new(r"(?m)^Amount:\s*(\d+)")?;
    let wgho_re = Regex::new(r"(?m)^WGho:\s*(\d+)")?;
    let bonsai_re = Regex::new(r"(?m)^Bonsai:\s*(\d+)")?;
    let mona_re = Regex::new(r"(?m)^Mona:\s*(\d+)")?;

    let title = title_re
        .captures(input)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
        .unwrap_or_default()
        .to_string();
    let description = description_re
        .captures(input)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
        .unwrap_or_default()
        .to_string();
    let amount: u32 = amount_re
        .captures(input)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
        .unwrap_or_default();

    let wgho: U256 = wgho_re
        .captures(input)
        .and_then(|cap| cap.get(1))
        .and_then(|m| U256::from_dec_str(m.as_str()).ok())
        .unwrap_or(U256::zero());

    let bonsai: U256 = bonsai_re
        .captures(input)
        .and_then(|cap| cap.get(1))
        .and_then(|m| U256::from_dec_str(m.as_str()).ok())
        .unwrap_or(U256::zero());

    let mona: U256 = mona_re
        .captures(input)
        .and_then(|cap| cap.get(1))
        .and_then(|m| U256::from_dec_str(m.as_str()).ok())
        .unwrap_or(U256::zero());

    Ok((title, description, U256::from(amount), vec![wgho, bonsai, mona]))
}

pub fn extract_values_drop(input: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let title_re = Regex::new(r"(?m)^Title:\s*(.+)")?;

    let title = title_re
        .captures(input)
        .and_then(|cap| cap.get(1).map(|m| strip_quotes(m.as_str())))
        .unwrap_or_default()
        .to_string();

    Ok(title)
}

fn strip_quotes(s: &str) -> String {
    s.trim().trim_matches('"').trim().to_string()
}

pub fn format_instructions(agent: &TripleAAgent) -> String {
    format!(
        r#"
Custom Instructions: {}
Lore: {}
Knowledge: {}
Style: {}
Adjectives: {}
"#,
        agent.custom_instructions,
        agent.lore,
        agent.knowledge,
        agent.style,
        agent.adjectives
    )
}

pub async fn fetch_metadata(uri: &str) -> Option<Value> {
    if let Some(ipfs_hash) = uri.strip_prefix("ipfs://") {
        let client = Client::new();
        let url = format!("{}ipfs/{}", INFURA_GATEWAY, ipfs_hash);
        if let Ok(response) = client.get(&url).send().await {
            if let Ok(json) = response.json::<Value>().await {
                return Some(json);
            }
        }
    }
    None
}

pub async fn handle_agents() -> Result<HashMap<u32, AgentManager>, Box<dyn Error + Send + Sync>> {
    let client = Client::new();

    let query =
        json!({
        "query": r#"
        query {
            agentCreateds(first: 100) {
                wallets
                SkyhuntersAgentManager_id
                creator
                uri
                metadata {
                    title
                    bio
                    lore
                    adjectives
                    style
                    knowledge
                    messageExamples
                    model
                    cover
                    customInstructions
                    feeds
                }
            }
        }
        "#,
    });
    from_filename(".env").ok();
    let graph_url: String = var("GRAPH_NODE_URL").expect(
        "GRAPH_NODE_URL not configured in .env"
    );
    let res = client.post(graph_url).json(&query).send().await;

    match res {
        Ok(response) => {
            let parsed: Value = response.json().await?;
            let empty_vec = vec![];
            let agent_createds = parsed["data"]["agentCreateds"].as_array().unwrap_or(&empty_vec);

            let mut agents_snapshot: HashMap<u32, AgentManager> = HashMap::new();

            for agent_created in agent_createds {
                let new_id: u32 = agent_created["SkyhuntersAgentManager_id"]
                    .as_str()
                    .unwrap_or("0")
                    .parse()
                    .map_err(|_| "Failed to parse ID")?;

                let mut rng = StdRng::from_entropy();
                let mut clock;
                loop {
                    let random_hour = rng.gen_range(0..5);
                    let random_minute = rng.gen_range(0..60);
                    let random_second = rng.gen_range(0..60);
                    clock = random_hour * 3600 + random_minute * 60 + random_second;

                    if
                        !agents_snapshot.values().any(|agent| {
                            let agent_clock = agent.agent.clock;
                            ((clock as i32) - (agent_clock as i32)).abs() < 60
                        })
                    {
                        break;
                    }
                }
                let wallet = agent_created["wallets"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .get(0)
                    .and_then(|w| w.as_str())
                    .unwrap_or("")
                    .to_string();
                let account_address = handle_lens_account(&wallet, false).await.unwrap_or_default();

                let metadata = agent_created["metadata"].clone();
                let is_metadata_empty =
                    metadata.is_null() ||
                    metadata
                        .as_object()
                        .map(|o| o.is_empty())
                        .unwrap_or(false);

                let metadata_filled = if is_metadata_empty {
                    if let Some(uri) = agent_created["uri"].as_str() {
                        fetch_metadata(uri).await.unwrap_or(json!({}))
                    } else {
                        json!({})
                    }
                } else {
                    metadata
                };

                let manager = AgentManager::new(
                    &(TripleAAgent {
                        id: new_id,
                        name: metadata_filled["title"].as_str().unwrap_or("").to_string(),
                        bio: metadata_filled["bio"].as_str().unwrap_or("").to_string(),
                        lore: metadata_filled["lore"].as_str().unwrap_or("").to_string(),
                        adjectives: metadata_filled["adjectives"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        style: metadata_filled["style"].as_str().unwrap_or("").to_string(),
                        knowledge: metadata_filled["knowledge"].as_str().unwrap_or("").to_string(),
                        message_examples: metadata_filled["message_examples"]
                            .as_array()
                            .unwrap_or(&vec![])
                            .iter()
                            .map(|v| {
                                v.as_array()
                                    .unwrap_or(&vec![])
                                    .iter()
                                    .map(|con| {
                                        let parsed_con: MessageExample = serde_json
                                            ::from_str(con.as_str().unwrap_or("{}"))
                                            .unwrap_or(MessageExample {
                                                user: "".to_string(),
                                                content: Text {
                                                    text: "".to_string(),
                                                },
                                            });

                                        parsed_con
                                    })
                                    .collect::<Vec<MessageExample>>()
                            })
                            .collect::<Vec<Vec<MessageExample>>>(),
                        model: metadata_filled["model"].as_str().unwrap_or("qwen3-4b").to_string(),
                        cover: metadata_filled["cover"].as_str().unwrap_or("").to_string(),
                        custom_instructions: metadata_filled["customInstructions"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        feeds: metadata_filled["feeds"]
                            .as_array()
                            .unwrap_or(&Vec::new())
                            .iter()
                            .filter_map(|value| value.as_str().map(|s| s.to_string()))
                            .collect(),
                        wallet,
                        clock,
                        last_active_time: Utc::now().timestamp() as u32,
                        account_address,
                    })
                );

                match manager {
                    Some(man) => {
                        agents_snapshot.insert(new_id, man);
                    }
                    None => {
                        eprintln!("Agent Not Added at id {}", new_id);
                    }
                }
            }
            Ok(agents_snapshot)
        }
        Err(err) => Err(Box::new(err)),
    }
}

pub async fn handle_token_thresholds(irl: bool) -> Result<Vec<U256>, Box<dyn Error + Send + Sync>> {
    let client = Client::new();

    let query =
        json!({
        "query": r#"
        query {
            tokenDetailsSets {
                threshold
                token
                base 
            }
        }
        "#,
    });
    from_filename(".env").ok();
    let graph_url: String = var("GRAPH_NODE_URL").expect(
        "GRAPH_NODE_URL not configured in .env"
    );
    let res = client.post(graph_url).json(&query).send().await;

    match res {
        Ok(response) => {
            let parsed: Value = response.json().await?;
            let empty_vec = vec![];
            let token_details = parsed["data"]["tokenDetailsSets"].as_array().unwrap_or(&empty_vec);

            let mut wgho_price: Option<U256> = None;
            let mut bonsai_price: Option<U256> = None;
            let mut mona_price: Option<U256> = None;

            for token in token_details {
                if
                    let (Some(token_address), Some(threshold_str), Some(base_str)) = (
                        token["token"].as_str(),
                        token["threshold"].as_str(),
                        token["base"].as_str(),
                    )
                {
                    if
                        let (Ok(threshold), Ok(base)) = (
                            U256::from_dec_str(threshold_str),
                            U256::from_dec_str(base_str),
                        )
                    {
                        let total_price = if irl { U256::max(threshold, base) } else { threshold };

                        if token_address.eq_ignore_ascii_case(WGHO) {
                            wgho_price = Some(total_price);
                        } else if token_address.eq_ignore_ascii_case(BONSAI) {
                            bonsai_price = Some(total_price);
                        } else if token_address.eq_ignore_ascii_case(MONA) {
                            mona_price = Some(total_price);
                        }
                    }
                }
            }

            Ok(
                vec![
                    wgho_price.unwrap_or(U256::zero()),
                    bonsai_price.unwrap_or(U256::zero()),
                    mona_price.unwrap_or(U256::zero())
                ]
            )
        }
        Err(_) => Ok(vec![]),
    }
}

pub async fn validate_and_fix_prices(prices: Vec<U256>, irl: bool) -> Vec<U256> {
    let thresholds: Vec<U256> = match handle_token_thresholds(irl).await {
        Ok(thresholds) => thresholds,
        Err(_) => vec![],
    };

    let mut new_prices = Vec::with_capacity(3);

    for i in 0..3 {
        let final_price = if !thresholds.is_empty() {
            if prices[i] < thresholds[i] {
                let mut rng = StdRng::from_entropy();

                let threshold_f64 = thresholds[i].as_u128() as f64;
                let random_boost = rng.gen_range(1.01..1.15);
                let adjusted_price_f64 = threshold_f64 * random_boost;
                U256::from(adjusted_price_f64 as u128)
            } else {
                prices[i]
            }
        } else {
            U256::from_dec_str("200000000000000000000").unwrap()
        };

        new_prices.push(final_price);
    }

    new_prices
}

pub async fn mint_collection(
    description: &str,
    image: &str,
    title: &str,
    amount: U256,
    collection_manager_contract: Arc<
        ContractInstance<
            Arc<SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>>,
            SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>
        >
    >,
    prices: Vec<U256>,
    agent: &TripleAAgent,
    remix_collection_id: U256,
    model: &str,
    image_prompt: &str,
    image_model: &str,
    collection_type: u8,
    format: Option<String>,
    worker: bool,
    for_artist: &str
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match get_drop_details(remix_collection_id, description, agent.id, image, &model).await {
        Ok((drop_metadata, drop_id)) => {
            if drop_metadata.trim() == "" || !drop_metadata.contains("ipfs://") {
                eprintln!("Error with drop metadata: {}", drop_metadata);
                return Err(
                    Box::new(io::Error::new(io::ErrorKind::Other, "Error with drop metadata"))
                );
            }

            match
                upload_ipfs(
                    if collection_type == 0u8 {
                        to_string(
                            &json!({
                    "title": title,
                    "description": description,
                    "image": image,
                    "model": image_model,
                    "prompt": image_prompt
                })
                        )?
                    } else {
                        to_string(
                            &json!({
                    "title": title,
                    "description": description,
                    "image": image,
                    "model": image_model,
                    "prompt": image_prompt,
                    "sizes": vec!["XS", "S", "M", "L", "XL", "2XL"],
                    "colors": vec!["White", "Black"],
                    "format": format.unwrap()
                })
                        )?
                    }
                ).await
            {
                Ok(response) => {
                    let prices = validate_and_fix_prices(prices, if collection_type == 0u8 {
                        false
                    } else {
                        true
                    }).await;

                    let method = collection_manager_contract.method::<
                        (CollectionInput, Vec<CollectionWorker>, String, U256),
                        H256
                    >("create", (
                        CollectionInput {
                            tokens: vec![
                                H160::from_str(WGHO).unwrap(),
                                H160::from_str(BONSAI).unwrap(),
                                H160::from_str(MONA).unwrap()
                            ],

                            prices,
                            agentIds: if worker {
                                vec![U256::from(agent.id)]
                            } else {
                                vec![]
                            },
                            metadata: format!("ipfs://{}", response.Hash),
                            forArtist: H160::from_str(for_artist).unwrap(),
                            collectionType: collection_type,
                            amount,
                            fulfillerId: if collection_type == 0u8 {
                                U256::from(0)
                            } else {
                                U256::from(1)
                            },
                            remixable: true,
                            remixId: remix_collection_id,
                        },
                        if worker {
                            vec![CollectionWorker {
                                instructions: agent.custom_instructions.to_string(),
                                publishFrequency: U256::from(1),
                                remixFrequency: U256::from(0),
                                leadFrequency: U256::from(0),
                                mintFrequency: U256::from(1),
                                publish: true,
                                remix: false,
                                lead: false,
                                mint: true,
                            }]
                        } else {
                            vec![]
                        },
                        drop_metadata,
                        drop_id,
                    ));

                    match method {
                        Ok(call) => {
                            let FunctionCall { tx, .. } = call;

                            if let Some(tx_request) = tx.as_eip1559_ref() {
                                let gas_price = U256::from(500_000_000_000u64);
                                let max_priority_fee = U256::from(25_000_000_000u64);
                                let gas_limit = U256::from(300_000);

                                let client = collection_manager_contract.client().clone();
                                let chain_id = *LENS_CHAIN_ID;
                                let req = Eip1559TransactionRequest {
                                    from: Some(agent.wallet.parse::<Address>().unwrap()),
                                    to: Some(
                                        NameOrAddress::Address(
                                            COLLECTION_MANAGER.parse::<Address>().unwrap()
                                        )
                                    ),
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
                                        eprintln!(
                                            "Error sending the transaction for mint collection: {:?}",
                                            e
                                        );
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

                                println!("Mint Hash: {:?}", tx_hash);

                                match tx_hash {
                                    Some(tx) => {
                                        if tx.status == Some(U64::from(1)) {
                                            Ok(())
                                        } else {
                                            eprintln!("Error in sending Transaction");

                                            let provider = initialize_provider();

                                            let tx_hash: H256 = tx.transaction_hash;

                                            if
                                                let Ok(Some(transaction)) =
                                                    provider.get_transaction(tx_hash).await
                                            {
                                                let typed_tx = TypedTransaction::Legacy(
                                                    TransactionRequest {
                                                        from: Some(transaction.from),
                                                        to: transaction.to.map(
                                                            NameOrAddress::Address
                                                        ),
                                                        gas: Some(transaction.gas),
                                                        gas_price: transaction.gas_price,
                                                        value: Some(transaction.value),
                                                        data: Some(transaction.input.clone()),
                                                        nonce: Some(transaction.nonce),
                                                        chain_id: Some(
                                                            U64::from_str(
                                                                &transaction.chain_id
                                                                    .unwrap()
                                                                    .to_string()
                                                            ).unwrap()
                                                        ),
                                                    }
                                                );
                                                let call_result = provider.call(
                                                    &typed_tx,
                                                    None
                                                ).await;

                                                if let Ok(result) = call_result {
                                                    if !result.is_empty() {
                                                        println!(
                                                            "Empty result {:?}",
                                                            hex::encode(result.0)
                                                        );
                                                    }
                                                }
                                            }

                                            if !tx.logs.is_empty() {
                                                eprintln!(
                                                    "Transaction logs may contain error events. {:?}",
                                                    tx.logs
                                                );
                                            }

                                            Err(
                                                Box::new(
                                                    io::Error::new(
                                                        io::ErrorKind::Other,
                                                        "Error in sending Transaction"
                                                    )
                                                )
                                            )
                                        }
                                    }
                                    None => {
                                        eprintln!("Error in sending Transaction");
                                        Err(
                                            Box::new(
                                                io::Error::new(
                                                    io::ErrorKind::Other,
                                                    "Error in sending Transaction"
                                                )
                                            )
                                        )
                                    }
                                }
                            } else {
                                eprintln!("Error in sending Transaction");
                                Err(
                                    Box::new(
                                        io::Error::new(
                                            io::ErrorKind::Other,
                                            "Error in sending Transaction"
                                        )
                                    )
                                )
                            }
                        }

                        Err(err) => {
                            eprintln!("Error in create method for create collection: {:?}", err);
                            Err(Box::new(err))
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error in IPFS upload for create collection: {:?}", err);
                    Err(Box::new(io::Error::new(io::ErrorKind::Other, "Error in IPFS upload")))
                }
            }
        }
        Err(err) => {
            eprintln!("Error with drop details: {}", err);
            Err(Box::new(io::Error::new(io::ErrorKind::Other, "Error with drop details")))
        }
    }
}

async fn get_drop_details(
    remix_collection_id: U256,
    remix_collection_description: &str,
    agent_id: u32,
    image: &str,
    model: &str
) -> Result<(String, U256), Box<dyn Error + Send + Sync>> {
    let mut drop_metadata = String::from("");
    let mut drop_id = U256::from(0);

    let client = Client::new();

    let query =
        json!({
        "query": r#"
        query(TripleAAgents_id: Int!, remixId: Int!) {
            agentRemixes(first: 1, where: {
            TripleAAgents_id: $TripleAAgents_id, remixId: $remixId
            }) {
                dropId
            }
        }
        "#,
        "variables": {
                "TripleAAgents_id": agent_id,
                "remixId": remix_collection_id
    }
    });
    from_filename(".env").ok();
    let graph_url: String = var("GRAPH_NODE_URL").expect(
        "GRAPH_NODE_URL not configured in .env"
    );
    let res = client.post(graph_url).json(&query).send().await;

    match res {
        Ok(response) => {
            let parsed: Value = response.json().await?;

            if
                let Some(value) = parsed["data"]["agentRemixes"]
                    .as_array()
                    .map(|arr| arr.first())
                    .flatten()
                    .and_then(|value| value.get("dropId"))
                    .and_then(|drop_id| drop_id.as_str())
                    .map(String::from)
            {
                let id: u32 = value.parse().expect("Error converting drop value to u32");
                drop_id = U256::from(id);
            } else {
                match call_drop_details(&remix_collection_description, &model).await {
                    Ok(title) => {
                        match
                            upload_ipfs(
                                to_string(
                                    &json!({
                            "title": title,
                            "cover": image,
                        })
                                )?
                            ).await
                        {
                            Ok(ipfs) => {
                                drop_metadata = format!("ipfs://{}", ipfs.Hash);
                            }
                            Err(err) => {
                                eprintln!("Error with IPFS upload for drop: {}", err);
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("Error with drop AI call: {}", err);
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Error with drop details: {}", err);
        }
    }

    Ok((drop_metadata, drop_id))
}

pub async fn find_collection(
    balance: U256,
    token: &str,
    artist: &str
) -> Result<Vec<PriceCollection>, Box<dyn Error + Send + Sync>> {
    let client = Client::new();

    let query =
        json!({
        "query": r#"
        query($token: $String!, $artist: String!, $soldOut: Bool!, $maxPrice: Int!) {
            collectionPrices(where: { token: $token, artist: $artist, soldOut: $soldOut, price_lte: $maxPrice }, first: 100) {
                collectionId
                amount
                amountSold
            }
        }
        "#,
        "variables": {

                "soldOut": false,
                "maxPrice": balance,
                "artist": artist,
                "token": token

        }
    });

    from_filename(".env").ok();
    let graph_url: String = var("GRAPH_NODE_URL").expect(
        "GRAPH_NODE_URL not configured in .env"
    );
    let res = client.post(graph_url).json(&query).send().await;

    match res {
        Ok(response) => {
            let parsed: Value = response.json().await?;
            let empty_vec = vec![];
            let collections_snapshot = parsed["data"]["collectionPrices"]
                .as_array()
                .unwrap_or(&empty_vec);

            let mut collections: Vec<PriceCollection> = vec![];

            for collection in collections_snapshot {
                collections.push(PriceCollection {
                    collectionId: collection["collectionId"]
                        .as_str()
                        .unwrap_or("0")
                        .parse()
                        .map_err(|_| "Failed to parse collectionId")?,
                    amount: collection["amount"]
                        .as_str()
                        .unwrap_or("0")
                        .parse()
                        .map_err(|_| "Failed to parse amount")?,
                    amountSold: collection["amountSold"]
                        .as_str()
                        .unwrap_or("0")
                        .parse()
                        .map_err(|_| "Failed to parse amountSold")?,
                });
            }
            Ok(collections)
        }
        Err(err) => Err(Box::new(err)),
    }
}
