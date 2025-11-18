use crate::utils::{
    helpers::format_instructions,
    ipfs::upload_lens_storage,
    lens::{feed_info, follow_profiles, make_comment, make_publication, make_quote, search_posts},
    types::{Collection, Content, Image, Publication, SavedTokens, TripleAAgent},
    venice::{call_comment_completion, call_feed_completion, receive_query},
};
use futures::future::join_all;
use serde_json::{to_string, Value};
use std::{error::Error, io};
use uuid::Uuid;

pub async fn lead_generation(
    agent: &TripleAAgent,
    collection: &Collection,
    tokens: Option<SavedTokens>,
    collection_instructions: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match receive_query(&collection.description, &collection.title, &agent.model).await {
        Ok(query) => match search_posts(&agent.wallet, &query).await {
            Ok((posts, profiles)) => {
                let _ = follow_profiles(
                    profiles.clone(),
                    &tokens.as_ref().unwrap().tokens.access_token,
                )
                .await;

                let (comments_posts, quotes_posts) = posts.split_at(posts.len() / 2);

                let _ = make_comments(
                    comments_posts.to_vec(),
                    &tokens.as_ref().unwrap().tokens.access_token,
                    agent.id,
                    &agent.model,
                    &format_instructions(&agent),
                    &collection_instructions,
                    &collection,
                )
                .await;

                let _ = make_quotes(
                    quotes_posts.to_vec(),
                    &tokens.as_ref().unwrap().tokens.access_token,
                    agent.id,
                    &agent.model,
                    &format_instructions(&agent),
                    &collection_instructions,
                    &collection,
                )
                .await;

                // let _ = feed_posts(
                //     collection,
                //     &tokens.as_ref().unwrap().tokens.access_token,
                //     agent.id,
                //     agent.feeds.clone(),
                //     &agent.model,
                //     &format_instructions(&agent),
                //     &collection_instructions,
                // )
                // .await;

                Ok(())
            }
            Err(err) => {
                println!("Error finding posts {:?}", err);
                Err(Box::new(io::Error::new(
                    io::ErrorKind::Other,
                    "Error finding posts",
                )))
            }
        },
        Err(err) => {
            println!("Error receiving query {:?}", err);
            Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                "Error receiving query",
            )))
        }
    }
}

async fn make_comments(
    posts: Vec<Value>,
    auth_tokens: &str,
    private_key: u32,
    model: &str,
    custom_instructions: &str,
    collection_instructions: &str,
    collection: &Collection,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let comment_futures = posts.into_iter().map(|post| async move {
        let mut content = String::new();

        if let Some(metadata) = post["metadata"].as_object() {
            content = metadata
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
        }

        match call_comment_completion(
            &content,
            custom_instructions,
            collection_instructions,
            &collection.description,
            &model,
        )
        .await
        {
            Ok((llm_response, image)) => {
                match format_response(&llm_response, &collection, image).await {
                    Ok(content) => {
                        let _ = make_comment(
                            &content,
                            private_key,
                            auth_tokens,
                            post["id"].as_str().unwrap_or_default(),
                        )
                        .await;
                    }
                    Err(err) => {
                        println!("Error with Comment format {:?}", err);
                    }
                }
            }
            Err(err) => {
                println!("Error with LLM Comment {:?}", err);
            }
        }
        Ok::<(), Box<dyn Error + Send + Sync>>(())
    });

    let results: Vec<_> = join_all(comment_futures).await;

    for result in results {
        if let Err(e) = result {
            println!("Error with commenting: {:?}", e);
        }
    }

    Ok(())
}

async fn make_quotes(
    posts: Vec<Value>,
    auth_tokens: &str,
    private_key: u32,
    model: &str,
    custom_instructions: &str,
    collection_instructions: &str,
    collection: &Collection,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let quote_futures = posts.into_iter().map(|post| async move {
        let mut content = String::new();

        if let Some(metadata) = post["metadata"].as_object() {
            content = metadata
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
        }

        match call_comment_completion(
            &content,
            custom_instructions,
            collection_instructions,
            &collection.description,
            &model,
        )
        .await
        {
            Ok((llm_response, image)) => {
                match format_response(&llm_response, &collection, image).await {
                    Ok(content) => {
                        let _ = make_quote(
                            &content,
                            private_key,
                            auth_tokens,
                            post["id"].as_str().unwrap_or_default(),
                        )
                        .await;
                    }
                    Err(err) => {
                        println!("Error with Quote format {:?}", err);
                    }
                }
            }
            Err(err) => {
                println!("Error with LLM Quote {:?}", err);
            }
        }
        Ok::<(), Box<dyn Error + Send + Sync>>(())
    });

    let results: Vec<_> = join_all(quote_futures).await;

    for result in results {
        if let Err(e) = result {
            println!("Error with quoting: {:?}", e);
        }
    }

    Ok(())
}

async fn format_response(
    llm_message: &str,
    collection: &Collection,
    use_image: bool,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut focus = String::from("TEXT_ONLY");
    let mut schema = "https://json-schemas.lens.dev/posts/text-only/3.0.0.json".to_string();
    let mut image = None;

    if use_image {
        focus = String::from("IMAGE");
        schema = "https://json-schemas.lens.dev/posts/image/3.0.0.json".to_string();
        image = Some(Image {
            tipo: "image/png".to_string(),
            item: collection.image.clone(),
        })
    }
    let tags = vec![
        "tripleA".to_string(),
        collection.title.to_string().replace(" ", "").to_lowercase(),
    ];

    let publication = Publication {
        schema,
        lens: Content {
            mainContentFocus: focus,
            title: llm_message.chars().take(20).collect(),
            content: llm_message.to_string(),
            id: Uuid::new_v4().to_string(),
            locale: "en".to_string(),
            tags,
            image,
        },
    };

    let publication_json = to_string(&publication)?;

    let content = match upload_lens_storage(publication_json).await {
        Ok(con) => con,
        Err(e) => {
            eprintln!("Error uploading content to Lens Storage: {}", e);
            return Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                format!("Error uploading content to Lens Storage: {}", e),
            )));
        }
    };

    Ok(content)
}

async fn feed_posts(
    collection: &Collection,
    auth_tokens: &str,
    private_key: u32,
    feeds: Vec<String>,
    model: &str,
    custom_instructions: &str,
    collection_instructions: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let feed_futures = feeds.into_iter().map(|feed| async move {
        match feed_info(&feed).await {
            Ok((title, description)) => {
                match call_feed_completion(
                    &collection,
                    custom_instructions,
                    collection_instructions,
                    &description,
                    &title,
                    &model,
                )
                .await
                {
                    Ok(llm_response) => {
                        match format_response(&llm_response, &collection, true).await {
                            Ok(content) => {
                                let _ = make_publication(
                                    &content,
                                    private_key,
                                    auth_tokens,
                                    Some(feed),
                                )
                                .await;
                            }
                            Err(err) => {
                                println!("Error with Feed format {:?}", err);
                            }
                        }
                    }
                    Err(err) => {
                        println!("Error with LLM Feed {:?}", err);
                    }
                }
            }
            Err(err) => {
                println!("Error with LLM Feed {:?}", err);
            }
        }

        Ok::<(), Box<dyn Error + Send + Sync>>(())
    });

    let results: Vec<_> = join_all(feed_futures).await;

    for result in results {
        if let Err(e) = result {
            println!("Error with feed: {:?}", e);
        }
    }

    Ok(())
}
