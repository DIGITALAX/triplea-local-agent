use serde_json::to_string;
use std::{error::Error, io};
use uuid::Uuid;

use crate::utils::{
    helpers::format_instructions,
    ipfs::upload_lens_storage,
    lens::make_publication,
    types::{Collection, Content, Image, Publication, SavedTokens, TripleAAgent},
    venice::call_chat_completion,
};

pub async fn publish(
    agent: &TripleAAgent,
    tokens: Option<SavedTokens>,
    collection: &Collection,
    collection_instructions: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match call_chat_completion(
        collection,
        &format_instructions(&agent),
        collection_instructions,
        &agent.id,
        &agent.model,
    )
    .await
    {
        Ok(llm_message) => match format_publication(agent, tokens, &llm_message, &collection).await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                eprintln!(
                    "Error in making lens post for agent_{}: {:?}",
                    agent.id, err
                );
                Ok(())
            }
        },
        Err(err) => {
            eprintln!("Error with Venice completion: {:?}", err);
            Ok(())
        }
    }
}

async fn format_publication(
    agent: &TripleAAgent,
    tokens: Option<SavedTokens>,
    llm_message: &str,
    collection: &Collection,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let focus = String::from("IMAGE");
    let schema = "https://json-schemas.lens.dev/posts/image/3.0.0.json".to_string();
    let tags = vec![
        "tripleA".to_string(),
        collection.title.to_string().replace(" ", "").to_lowercase(),
    ];

    let publication = Publication {
        schema,
        lens: Content {
            mainContentFocus: focus,
            title: llm_message.chars().take(20).collect(),
            content: format!(
                "{}\n\n Collect on TripleA here:\nhttps://triplea.agentmeme.xyz/nft/{}/{}/",
                llm_message.to_string(),
                collection.username,
                collection.collection_id
            ),
            id: Uuid::new_v4().to_string(),
            locale: "en".to_string(),
            tags,
            image: Some(Image {
                tipo: "image/png".to_string(),
                item: collection.image.clone(),
            }),
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

    let res = make_publication(
        &content,
        agent.id,
        &tokens.as_ref().unwrap().tokens.access_token,
        None,
    )
    .await
    .map_err(|e| Box::new(e.to_string()));

    println!("Lens response for agent_{}: {:?}", agent.id, res);

    match res {
        Ok(success) => {
            eprintln!("Post success: {:?}", success);
            Ok(())
        }
        Err(e) => {
            eprintln!("Error processing message for agent_{}: {:?}", agent.id, e);
            Err(Box::new(io::Error::new(
                io::ErrorKind::Other,
                "Error sending message",
            )))
        }
    }
}
