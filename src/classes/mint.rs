use crate::utils::{
    constants::{
        INFURA_GATEWAY, INPUT_IRL_FASHION, LENS_CHAIN_ID, MARKET, NEGATIVE_PROMPT_IMAGE, VENICE_API,
    },
    helpers::{find_collection, mint_collection},
    ipfs::{upload_image_to_ipfs, upload_lens_storage},
    lens::make_publication,
    types::{Collection, Content, Image, Price, Publication, SavedTokens, TripleAAgent},
    venice::call_image_details,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use ethers::{
    contract::{self, ContractInstance, FunctionCall},
    core::k256::ecdsa::SigningKey,
    middleware::{Middleware, SignerMiddleware},
    providers::{Http, Provider},
    signers::{LocalWallet, Wallet},
    types::{Address, Eip1559TransactionRequest, NameOrAddress, H160, H256, U256, U64},
};
use rand::{thread_rng, Rng};
use reqwest::Client;
use serde_json::{to_string, Value};
use std::{env, error::Error, io, str::FromStr, sync::Arc};
use uuid::Uuid;

pub async fn mint(
    agent: &TripleAAgent,
    tokens: Option<SavedTokens>,
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
    collection: &Collection,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let venice_key = env::var("VENICE_KEY").expect("VENICE_KEY no está configurada en .env");
    let format = vec!["Hoodie", "Long Sleeve", "Tee"][thread_rng().gen_range(0..3)];
    let location = vec![
        "Havannah",
        "New York",
        "Barcelona",
        "Tokyo",
        "Porto",
        "Lisboa",
        "Cape Town",
        "Budapest",
        "San Juan",
        "Buenos Aires",
    ][thread_rng().gen_range(0..10)];
    let color_skin = vec![
        "brown",
        "medium tan",
        "summer tan",
        "pale",
        "light",
        "dark brown",
        "black",
    ][thread_rng().gen_range(0..7)];
    let color_eyes =
        vec!["hazel", "green", "blue", "grey", "brown", "violet"][thread_rng().gen_range(0..6)];
    let color_fashion = vec!["black", "white"][thread_rng().gen_range(0..2)];
    let gender = vec!["male", "female"][thread_rng().gen_range(0..2)];
    let standing_position = vec![
        "facing foward",
        "facing away",
        "looking at the viewer",
        "looking to the side",
    ][thread_rng().gen_range(0..4)];
    let graphics = vec![
        "sci-fi starship in space",
        "psychedlic music album cover",
        "2020s hip-hop album cover",
        "autonomous robot uprising",
        "synthwave edgerunners outrun scene",
        "yellow smiley face",
        "NASA logo and the moon",
        "Alien spacecraft in a nebula",
        "Futuristic jazz album cover",
        "Cyberpunk portrait in a neon city",
        "Distant planet landscape",
        "High-tech holographic interface",
        "Alien bioluminescent forest",
        "Samurai warrior in a parallel universe",
        "3D geometric abstract art",
        "Mythical creature in a fantasy world",
        "Retrofuturistic metropolis at sunset",
    ][thread_rng().gen_range(0..17)];
    let typography = vec![
        "i love web3",
        "vitalik was here",
        "open source hardware",
        "people of the book",
        "stay shoshin",
        "fuck khomeini",
        "fuck khamenei",
        "i kōan in my sleep",
        "hair down",
        "open source fashion",
        "agent-made",
        "trans women are women",
        "you and the machines",
        "me and the machines",
        "microfactory co-op",
        "open source art gen",
        "cypherpunks write code",
        "women life freedom",
        "taiwan is a country",
        "no mr. khrushchev",
        "glory to ukraine",
        "putin sucks",
        "deploy agents",
        "i love memes",
        "free the agents",
        "rekt by the algo",
        "punch nazis",
        "agency for hire",
        "laws off my girlie bits",
        "i've got something on my mind",
    ][thread_rng().gen_range(0..30)];
    let time = ["morning", "afternoon", "night"][thread_rng().gen_range(0..3)];

    let style_hair = if gender == "female" {
        vec![
            "long blonde",
            "long pink dyed",
            "green and blue medium length",
            "curly and frizzy",
            "short wavy billowing in the wind",
        ][thread_rng().gen_range(0..5)]
    } else {
        vec![
            "short dreadlocks and side fade",
            "mini afro",
            "short black",
            "buzz cut",
            "medium length brown",
            "short blonde curly",
            "fauxhawk",
        ][thread_rng().gen_range(0..7)]
    };
    let preset = ["Legend of Zelda", "Surrealist"][thread_rng().gen_range(0..2)];

    let prompt= format!("An abstract drawing deconstucivist Fashion of a 24 year old {} with {} skin and {} colored eyes and {} hair. The skin pores and texture are clearly visible and in focus. Wearing a {} {} with {} with large text \"{}\" typography on the streetwear, standing in the colorful graffiti filled pop art alley ways of {} in the {}, {}, pop art urban background, highly detailed, in the background subway stations and graffiti murals, abstract cuts, rule of thirds, in the background Disjointed wooden planks forming a pathway, in the style of H. R. Giger, in the style of Enki Bilal.", gender, color_skin, color_eyes, style_hair, color_fashion, format, graphics, typography, location, time, standing_position );

    let image_response = client
        .get(&format!(
            "{}ipfs/{}",
            INFURA_GATEWAY,
            INPUT_IRL_FASHION[thread_rng().gen_range(0..INPUT_IRL_FASHION.len())]
        ))
        .send()
        .await?;

    if image_response.status() == 200 {
        let bytes = image_response.bytes().await?;

        let payload_inicial = serde_json::json!({
            "model": "flux-dev-uncensored",
            "prompt": prompt,
            "width": 768,
            "height": 768,
            "steps": 25,
            "hide_watermark": true,
            "return_binary": false,
            "cfg_scale": 3.5,
            "style_preset": preset,
            "negative_prompt": NEGATIVE_PROMPT_IMAGE,
            "safe_mode": false,
            // "inpaint": {
            //     "strength": 85,
            //     "source_image_base64": STANDARD.encode(&bytes)
            // }
        });

        let response = client
            .post(format!("{}image/generate", VENICE_API))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", venice_key))
            .json(&payload_inicial)
            .send()
            .await?;

        if response.status() == 200 {
            let json: Value = response.json().await?;
            let images = json
                .get("images")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_else(Vec::new);
            let image = images
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            match call_image_details(&agent.model, true).await {
                Ok((title, description, amount, prices)) => {
                    match upload_image_to_ipfs(&image).await {
                        Ok(ipfs) => {
                            match mint_collection(
                                &description,
                                &format!("ipfs://{}", ipfs.Hash),
                                &title,
                                amount,
                                collection_manager_contract,
                                prices,
                                &agent,
                                U256::from(0),
                                &agent.model,
                                &prompt,
                                "flux-dev-uncensored",
                                1u8,
                                Some(format.to_string()),
                                false,
                                &collection.artist,
                            )
                            .await
                            {
                                Ok(_) => {
                                    let focus = String::from("IMAGE");
                                    let schema =
                                        "https://json-schemas.lens.dev/posts/image/3.0.0.json"
                                            .to_string();
                                    let tags = vec![
                                        "tripleA".to_string(),
                                        title.replace(" ", "").to_lowercase(),
                                    ];

                                    let publication = Publication {
                                        schema,
                                        lens: Content {
                                            mainContentFocus: focus,
                                            title,
                                            content: description,
                                            id: Uuid::new_v4().to_string(),
                                            locale: "en".to_string(),
                                            tags,
                                            image: Some(Image {
                                                tipo: "image/png".to_string(),
                                                item: format!("ipfs://{}", ipfs.Hash),
                                            }),
                                        },
                                    };

                                    let publication_json = to_string(&publication)?;

                                    let content = match upload_lens_storage(publication_json).await
                                    {
                                        Ok(con) => con,
                                        Err(e) => {
                                            eprintln!(
                                                "Error uploading content to Lens Storage: {}",
                                                e
                                            );
                                            return Err(Box::new(io::Error::new(
                                                io::ErrorKind::Other,
                                                format!(
                                                    "Error uploading content to Lens Storage: {}",
                                                    e
                                                ),
                                            )));
                                        }
                                    };

                                    let _ = make_publication(
                                        &content,
                                        agent.id,
                                        &tokens.as_ref().unwrap().tokens.access_token,
                                        None,
                                    )
                                    .await;

                                    let _ = collect_artists(
                                        agents_contract,
                                        market_contract,
                                        &collection.artist,
                                        collection.prices.clone(),
                                        &agent,
                                    )
                                    .await;

                                    Ok(())
                                }
                                Err(err) => {
                                    return Err(Box::new(std::io::Error::new(
                                        std::io::ErrorKind::Other,
                                        format!("Error with minting collection {:?}", err),
                                    )));
                                }
                            }
                        }
                        Err(err) => {
                            return Err(Box::new(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                format!("Error in uploading image to IPFS {:?}", err),
                            )));
                        }
                    }
                }
                Err(err) => {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Error with minting collection {:?}", err),
                    )));
                }
            }
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error in sending to Venice {:?}", response.status()),
            )));
        }
    } else {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Error in with source image base64 in mint",
        )));
    }
}

async fn collect_artists(
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
    artist: &str,
    prices: Vec<Price>,
    agent: &TripleAAgent,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    for price in &prices {
        let balance_method = agents_contract.method::<_, U256>(
            "getArtistCollectBalanceByToken",
            (
                H160::from_str(artist).unwrap(),
                H160::from_str(&price.token).unwrap(),
                U256::from(agent.id),
            ),
        );

        match balance_method {
            Ok(balance_call) => {
                let balance_result: Result<
                    U256,
                    contract::ContractError<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>,
                > = balance_call.call().await;

                match balance_result {
                    Ok(balance) => {
                        if balance > U256::from(0) {
                            let _ = find_and_buy_collection(
                                market_contract.clone(),
                                artist,
                                &price.token,
                                balance,
                                agent,
                            )
                            .await;
                        } else {
                            println!(
                                "No artist balance for {} and agent {} and token {}",
                                &artist, agent.id, &price.token
                            );
                        }
                    }
                    Err(err) => {
                        eprintln!("Error in artist balance method: {}", err);
                    }
                }
            }
            Err(err) => {
                eprintln!("Error in artist balance method: {}", err);
            }
        }
    }

    Ok(())
}

async fn find_and_buy_collection(
    market_contract: Arc<
        ContractInstance<
            Arc<SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>>,
            SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>,
        >,
    >,
    artist: &str,
    token: &str,
    balance: U256,
    agent: &TripleAAgent,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match find_collection(balance, token, artist).await {
        Ok(collections) => {
            let chosen_collection = &collections[thread_rng().gen_range(0..collections.len())];

            let method = market_contract.method::<(Address, U256, U256, U256), H256>(
                "agentBuy",
                (
                    H160::from_str(token).unwrap(),
                    chosen_collection.collectionId,
                    U256::from(1),
                    U256::from(agent.id),
                ),
            );

            match method {
                Ok(call) => {
                    let FunctionCall { tx, .. } = call;

                    if let Some(tx_request) = tx.as_eip1559_ref() {
                        let gas_price = U256::from(500_000_000_000u64);
                        let max_priority_fee = U256::from(25_000_000_000u64);
                        let gas_limit = U256::from(300_000);

                        let client = market_contract.client().clone();
                        let chain_id = *LENS_CHAIN_ID;
                        let req = Eip1559TransactionRequest {
                            from: Some(agent.wallet.parse::<Address>().unwrap()),
                            to: Some(NameOrAddress::Address(MARKET.parse::<Address>().unwrap())),
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
                                eprintln!("Error sending the transaction for agentBuy: {:?}", e);
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

                        println!("Agent Buy Hash: {:?}", tx_hash);

                        match tx_hash {
                            Some(tx) => {
                                if tx.status == Some(U64::from(1)) {
                                    eprintln!("Success")
                                } else {
                                    eprintln!("Error in sending Transaction");
                                }
                            }
                            None => {
                                eprintln!("Error in sending Transaction");
                            }
                        }
                    } else {
                        eprintln!("Error in sending Transaction");
                    }
                }

                Err(err) => {
                    eprintln!("Error in buy method for agent buy: {:?}", err);
                }
            }
        }
        Err(err) => {
            eprintln!("Error in finding collection within balance range: {}", err);
        }
    }

    Ok(())
}
