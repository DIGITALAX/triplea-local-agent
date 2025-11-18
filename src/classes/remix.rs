use base64::{engine::general_purpose::STANDARD, Engine};
use ethers::{
    contract::ContractInstance,
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::Wallet,
};
use rand::{thread_rng, Rng};
use reqwest::Client;
use serde_json::{to_string, Value};
use std::{env, error::Error, io, sync::Arc};
use uuid::Uuid;

use crate::utils::{
    constants::{
        INFURA_GATEWAY, NEGATIVE_PROMPT, REMIX_FEED, STYLE_PRESETS, VENICE_API, ZERO_ADDRESS,
    },
    helpers::mint_collection,
    ipfs::{upload_image_to_ipfs, upload_lens_storage},
    lens::make_publication,
    types::{Collection, Content, Image, Publication, SavedTokens, TripleAAgent},
    venice::{call_image_details, call_prompt},
};

pub async fn remix(
    agent: &TripleAAgent,
    collection: &Collection,
    tokens: Option<SavedTokens>,
    collection_manager_contract: Arc<
        ContractInstance<
            Arc<SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>>,
            SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>,
        >,
    >,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match call_prompt(&collection.description, &agent.model).await {
        Ok((prompt, model)) => {
            let client = Client::new();

            let image_response = client
                .get(&format!(
                    "{}ipfs/{}",
                    INFURA_GATEWAY,
                    collection.image.trim_start_matches("ipfs://")
                ))
                .send()
                .await?;

            if image_response.status() == 200 {
                let bytes = image_response.bytes().await?;

                let venice_key =
                    env::var("VENICE_KEY").expect("VENICE_KEY no está configurada en .env");
                let payload_inicial = serde_json::json!({
                    "model": model,
                    "prompt": prompt,
                    "width": 768,
                    "height": 768,
                    "steps": 25,
                    "hide_watermark": true,
                    "return_binary": false,
                    "cfg_scale": 3.5,
                    "style_preset": STYLE_PRESETS[thread_rng().gen_range(0..3)],
                    "negative_prompt": NEGATIVE_PROMPT,
                    "safe_mode": false,
                    // "inpaint": {
                    //     "strength": 90,
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
                    match call_image_details(&agent.model, false).await {
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
                                        collection.collection_id,
                                        &agent.model,
                                        &prompt,
                                        &model,
                                        0u8,
                                        None,
                                        true,
                                        ZERO_ADDRESS,
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

                                            let content =
                                                match upload_lens_storage(publication_json).await {
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
                                                // Some(REMIX_FEED.to_string()),
                                                None,
                                            )
                                            .await;
                                        }
                                        Err(err) => {
                                            return Err(Box::new(std::io::Error::new(
                                                std::io::ErrorKind::Other,
                                                format!("Error with creating remix {:?}", err),
                                            )));
                                        }
                                    }

                                    Ok(())
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
                                format!("Error with creating remix {:?}", err),
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
                    "Error in with source image base64 in remix",
                )));
            }
        }
        Err(err) => {
            eprintln!("Error with image prompt: {}", err);
            Ok(())
        }
    }
}
