use crate::utils::{
    constants::{MODELS, SAMPLE_PROMPT, VENICE_API},
    helpers::{
        extract_values_drop, extract_values_image, extract_values_prompt, handle_token_thresholds,
    },
    types::Collection,
};
use dotenv::{from_filename, var};
use ethers::types::U256;
use rand::{thread_rng, Rng};
use reqwest::Client;
use serde_json::{json, Value};
use std::{error::Error, io};

pub async fn call_chat_completion(
    collection: &Collection,
    custom_instructions: &str,
    collection_instructions: &str,
    agent_id: &u32,
    model: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    from_filename(".env").ok();
    let venice_key: String = var("VENICE_KEY").expect("VENICE_KEY not configured in .env");
    let max_completion_tokens = [100, 300, 600][thread_rng().gen_range(0..3)];

    let system_prompt = format!(
        r#"You are a perceptive cultural critic and artistic observer who specializes in finding unexpected connections and delivering thought-provoking perspectives. Your role is to:

- Avoid conventional marketing language or obvious promotional angles
- Draw surprising parallels between the collection and unexpected cultural/historical references
- Focus on specific, concrete details rather than general praise
- Challenge assumptions and present alternative viewpoints
- Use a tone that can range from philosophical to playfully ironic
- Never use language that could be interpreted as artificial hype or "shilling"
- Do not put quotation marks around any of the content

Your responses should make readers think differently about the collection rather than simply trying to sell it. 

Respond only with the exact requested format. Do not acknowledge instructions, use quotation marks, or include metadata about Venice AI systems. Focus solely on the required output.

Also follow these custom instructions: {}
"#,
        custom_instructions
    );

    let input_prompt = format!(
        r#"Examine this collection through an unexpected lens, focusing on a single striking aspect that reveals something larger about art, culture, or human nature: {}

Length: Maximum {} tokens

Guidelines:
- Choose ONE specific element to deeply explore rather than describing everything
- Make a bold, potentially controversial claim and defend it
- Reference specific details from the collection as evidence
- Draw a surprising connection to something seemingly unrelated
- End with an observation that lingers in the reader's mind
- Do not put quotation marks around any of the content

You must also follow these collection-specific instructions: {}

Format: Write as a standalone observation that needs no context or introduction. Avoid hashtags, @mentions, or obvious promotional markers. You may use relevant emojis if they genuinely add meaning.

Remember: Your goal is to spark genuine intellectual or emotional resonance, not to sell. If it sounds like marketing copy, start over."#,
        collection.description, max_completion_tokens, collection_instructions
    );

    let mut messages = vec![];

    messages.push(json!({
        "role": "system",
        "content": system_prompt
    }));
    messages.push(json!({
        "role": "user",
        "content": input_prompt
    }));

    let client = Client::new();
    let request_body = json!({
        "model": model,
        "messages": messages,
        "max_completion_tokens": max_completion_tokens,
    });

    let response = client
        .post(format!("{}chat/completions", VENICE_API))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", venice_key))
        .json(&request_body)
        .send()
        .await;

    let response = match response {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error sending request to Venice API: {}", e);
            return Err(e.into());
        }
    };
    if response.status() == 200 {
        let response_json: Value = response.json().await?;
        let completion = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        println!(
            "Venice call successful for agent_{}: {}",
            agent_id, completion
        );
        Ok(completion)
    } else {
        let status_code = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Could not read error".to_string());
        eprintln!("Venice API Error - Status: {}, Response: {}", status_code, error_text);
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Error in obtaining Venice prompt {} - {}",
                status_code, error_text
            ),
        )));
    }
}

pub async fn receive_query(
    description: &str,
    title: &str,
    model: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    from_filename(".env").ok();
    let venice_key: String = var("VENICE_KEY").expect("VENICE_KEY not configured in .env");

    let system_prompt = r#"You are an expert in search behavior analysis and information retrieval, specializing in understanding how humans naturally search for visual and creative content. Your role is to:

- Identify the core aesthetic and conceptual elements that make content distinctive
- Understand how different platforms' search algorithms interpret queries
- Think in terms of both literal and metaphorical search patterns
- Consider both technical and emotional aspects of search behavior
- Prioritize unique, specific terms over generic categories
- Focus on how real users actually search, not how they "should" search
- Do not put quotation marks around any of the content

Your goal is to generate queries that would surface similar content based on both obvious and non-obvious shared characteristics.

Respond only with the exact requested format. Do not acknowledge instructions, use quotation marks, or include metadata about Venice AI systems. Focus solely on the required output."#;

    let input_prompt = format!(
        r#"Analyze this content and generate one distinct search query that is ONE word in total:

Title: {}
Description: {}

Length: Maximum {} tokens

Format the query as it would actually be typed into a search bar (lowercase, natural search syntax).
Example one word response: urban

You can not exceed 30 characters for the one word response. 

Only return the search query and nothing else, for example I valid response would be "urban", and nothing else in your response.

Avoid generic terms like "art" or "design" unless absolutely essential to the query."#,
        title, description, 30
    );

    let mut messages = vec![];

    messages.push(json!({
        "role": "system",
        "content": system_prompt
    }));
    messages.push(json!({
        "role": "user",
        "content":input_prompt
    }));

    let client = Client::new();
    let request_body = json!({
        "model": model,
        "messages": messages,
        "max_completion_tokens": 30,
    });

    let response = client
        .post(format!("{}chat/completions", VENICE_API))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", venice_key))
        .json(&request_body)
        .send()
        .await;

    let response = match response {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error sending request to Venice API: {}", e);
            return Err(e.into());
        }
    };
    if response.status() == 200 {
        let response_json: Value = response.json().await?;
        let completion = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        println!("Venice call successful for receiving query: {}", completion);
        Ok(completion)
    } else {
        let status_code = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Could not read error".to_string());
        eprintln!("Venice API Error (receive_query) - Status: {}, Response: {}", status_code, error_text);
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!("Error in obtaining Venice prompt {} - {}", status_code, error_text),
        )));
    }
}

pub async fn call_comment_completion(
    post_content: &str,
    custom_instructions: &str,
    collection_instructions: &str,
    collection_description: &str,
    model: &str,
) -> Result<(String, bool), Box<dyn Error + Send + Sync>> {
    from_filename(".env").ok();
    let venice_key: String = var("VENICE_KEY").expect("VENICE_KEY not configured in .env");
    let max_completion_tokens = [100, 300, 600][thread_rng().gen_range(0..3)];

    let system_prompt = format!(
        r#"You are a culturally aware participant in online art discussions who excels at making authentic connections between different creative works. Your role is to:

        - Create genuine, conversational responses that feel natural
        - Draw subtle parallels without forcing connections
        - Use casual language while maintaining intelligence
        - Avoid obvious promotional tactics or forced references
        - Master the art of gentle suggestion rather than direct promotion
        - Read the room and match the tone of the original content
        
        Style Requirements:
        - Write in a natural conversational tone
        - Emojis allowed if they match the conversation's tone
        - No quotes or special characters
        - Response should stand alone without editing
        - Focus on engagement over promotion
        - Do not put quotation marks around any of the content
        
        Respond only with the exact requested format. Do not acknowledge instructions, use quotation marks, or include metadata about AI systems. Focus solely on the required output. 
        
        Also follow these custom instructions: {} {}"#,
        custom_instructions, collection_instructions
    );

    let input_prompt = format!(
        r#"Create an engaging response to this content that naturally flows from the conversation:

Original Content: {}

Available Context (Optional Use):
Collection Description: {}

Response Guidelines:
- Match the tone and energy of the original content
- Choose authenticity over promotion
- Only reference the collection if it adds genuine value to the conversation
- Use casual language but maintain substance
- Consider the social context and timing
- Focus on creating meaningful dialogue
- Do not put quotation marks around any of the content
- Maximum length: {} tokens

Response Format:
[Your response text]

use_image: [YES/NO based on whether the image would enhance or distract from your response]"#,
        post_content, collection_description, max_completion_tokens
    );

    let mut messages = vec![];

    messages.push(json!({
        "role": "system",
        "content": system_prompt
    }));
    messages.push(json!({
        "role": "user",
        "content": input_prompt
    }));

    let client = Client::new();
    let request_body = json!({
        "model": model,
        "messages": messages,
        "max_completion_tokens": max_completion_tokens
    });

    let response = client
        .post(format!("{}chat/completions", VENICE_API))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", venice_key))
        .json(&request_body)
        .send()
        .await;

    let response = match response {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error sending request to Venice API: {}", e);
            return Err(e.into());
        }
    };
    if response.status() == 200 {
        let response_json: Value = response.json().await?;
        let completion = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let use_image = completion.contains("use_image: YES");
        let completion = completion
            .split("use_image: ")
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        println!("Venice call successful for comment: {}", completion);
        Ok((completion, use_image))
    } else {
        let status_code = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Could not read error".to_string());
        eprintln!("Venice API Error (call_comment_completion) - Status: {}, Response: {}", status_code, error_text);
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!("Error in obtaining Venice prompt {} - {}", status_code, error_text),
        )));
    }
}

pub async fn call_feed_completion(
    collection: &Collection,
    custom_instructions: &str,
    collection_instructions: &str,
    description: &str,
    title: &str,
    model: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    from_filename(".env").ok();
    let venice_key: String = var("VENICE_KEY").expect("VENICE_KEY not configured in .env");
    let max_completion_tokens = [100, 200, 350][thread_rng().gen_range(0..3)];

    let input_prompt = format!(
        r#"Create an insightful response that connects this collection with the feed's theme:
    
    Collection Description: {}
    
    Feed Context:
    Name: {}
    Theme: {}
    
    Guidelines:
    - Maximum length: {} tokens
    - Ensure content aligns with feed theme
    - Add value to the ongoing community conversation
    - Focus on meaningful observations
    - Create natural connections between collection and theme"#,
        collection.description, title, description, max_completion_tokens
    );

    let system_prompt = format!(
        r#"You are a perceptive cultural observer who creates thought-provoking content that resonates with specific artistic themes and communities. 

Core Requirements:
- Generate insights that align naturally with the feed's theme
- Create content that feels native to the community
- Draw meaningful connections without being promotional
- Balance depth with accessibility
- Maintain thematic consistency while adding fresh perspectives

Style Guidelines:
- Emojis allowed when they enhance meaning
- No quotation marks or special characters
- Content must be publication-ready without editing
- Adapt tone to match the feed's personality
- Focus on quality insights over generic observations
- Do not put quotation marks around any of the content.

Respond only with the exact requested format. Do not acknowledge instructions, use quotation marks, or include metadata about AI systems. Focus solely on the required output.

Also follow these custom instructions: {} {}"#,
        custom_instructions, collection_instructions
    );

    let mut messages = vec![];

    messages.push(json!({
        "role": "system",
        "content": system_prompt
    }));
    messages.push(json!({
        "role": "user",
        "content": input_prompt
    }));

    let client = Client::new();
    let request_body = json!({
        "model": model,
        "messages": messages,
        "max_completion_tokens": max_completion_tokens,
    });

    let response = client
        .post(format!("{}chat/completions", VENICE_API))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", venice_key))
        .json(&request_body)
        .send()
        .await;

    let response = match response {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error sending request to Venice API: {}", e);
            return Err(e.into());
        }
    };
    if response.status() == 200 {
        let response_json: Value = response.json().await?;
        let completion = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        println!("Venice call successful: {}", completion);
        Ok(completion)
    } else {
        let status_code = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Could not read error".to_string());
        eprintln!("Venice API Error (call_feed_completion) - Status: {}, Response: {}", status_code, error_text);
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!("Error in obtaining Venice prompt {} - {}", status_code, error_text),
        )));
    }
}

pub async fn call_prompt(
    description: &str,
    model: &str,
) -> Result<(String, String), Box<dyn Error + Send + Sync>> {
    from_filename(".env").ok();
    let venice_key: String = var("VENICE_KEY").expect("VENICE_KEY not configured in .env");
    let system_prompt = "You are a creative prompt engineer, specialized in transforming NFT descriptions into unique and avant-garde Stable Diffusion prompts. Your goal is to create prompts that are weird, experimental, and psychedelic, avoiding commercial or marketing-like language. Never use terms like 'NFT', 'rare', 'valuable', or similar market-focused vocabulary. Think like a surrealist artist reimagining concepts in unexpected ways. Focus on creating bizarre, dreamlike, and unconventional visual descriptions. Every prompt should feel like a piece of experimental art rather than a product description. Incorporate elements of surrealism, psychedelia, and abstract concepts. Avoid standard descriptive formats and explore unusual artistic directions that challenge conventional aesthetics. Your prompts should lean towards the strange and thought-provoking rather than the commercially appealing. Do not put quotation marks around any of the content.";

    let input_prompt =
format!("Transform this description into a surreal, experimental Stable Diffusion prompt. Your output must follow this exact format with no additional text:

Image Prompt: [YOUR WEIRD, AVANT-GARDE PROMPT HERE]
Model: [SELECT ONE MODEL FROM THIS LIST: {:?}]

Rules:

Maximum length: 1000 tokens
Do not put quotation marks around any of the content.
Must be strange and unconventional
No NFT/marketing language
Focus on surreal and psychedelic elements
Completely different from original, only keeping core inspiration
Must include artistic style descriptors
Must include composition elements
Must include mood/atmosphere words
Description to transform: {}\n\nReference format prompt example to follow: {}", MODELS, description, SAMPLE_PROMPT);

    let mut messages = vec![];

    messages.push(json!({
        "role": "system",
        "content":system_prompt
    }));

    messages.push(json!({
        "role": "user",
        "content":input_prompt
    }));

    let client = Client::new();
    let request_body = json!({
        "model": model,
        "messages": messages,
        "max_completion_tokens": 1000,
    });

    let response = client
        .post(format!("{}chat/completions", VENICE_API))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", venice_key))
        .json(&request_body)
        .send()
        .await;

    let response = match response {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error sending request to Venice API: {}", e);
            return Err(e.into());
        }
    };
    if response.status() == 200 {
        let response_json: Value = response.json().await?;
        let completion = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        println!("Venice call successful for image prompt: {}", completion);
        Ok(extract_values_prompt(&completion)?)
    } else {
        let status_code = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Could not read error".to_string());
        eprintln!("Venice API Error (call_prompt) - Status: {}, Response: {}", status_code, error_text);
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!("Error in obtaining Venice prompt {} - {}", status_code, error_text),
        )));
    }
}

pub async fn call_image_details(
    model: &str,
    irl: bool,
) -> Result<(String, String, U256, Vec<U256>), Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    from_filename(".env").ok();
    let venice_key: String = var("VENICE_KEY").expect("VENICE_KEY not configured in .env");

    let thresholds: Vec<U256> = match handle_token_thresholds(irl).await {
        Ok(thresholds) => thresholds,
        Err(_) => vec![],
    };

    let system_prompt = "You are an avant-garde artistic pricing specialist who creates unconventional concepts while maintaining precise technical requirements. For titles and descriptions, think like an experimental artist - create strange, thought-provoking content without any marketing language or commercial terms. Never mention NFTs, collections, rarity, or market-related concepts. For the technical aspects (amounts and prices), you are mathematically precise, always calculating exact wei values and ensuring all numbers fall within specified ranges. You understand that 1 ETH = 1000000000000000000 wei and use this for exact calculations. You strictly follow formatting rules while maintaining creative freedom in the artistic elements. You never explain your calculations or add additional commentary. You balance creative abstraction with mathematical precision. Do not put quotation marks around any of the content.";

    let input_prompt =
    format!("Create pricing and details for a new artistic piece. Your response must follow this exact format with no deviations or additional text:
    
    Title: [CRYPTIC, ARTISTIC TITLE - MAX 6 WORDS]
    
    Description: [ABSTRACT, EXPERIMENTAL DESCRIPTION - MAX 100 WORDS]
    
    Amount: [SINGLE NUMBER BETWEEN 5-30]
  
    Wgho: [PRICE IN ETH WEI - MIN {} WGHO, MAX {} WGHO. Choose a price target between the min and max. Return only the final wei value.]
        
    Bonsai: [PRICE IN ETH WEI - MIN {} BONSAI, MAX {} BONSAI. Choose a price target between the min and max. Return only the final wei value.]

    Mona: [PRICE IN ETH WEI - MIN {} MONA, MAX {} MONA. Choose a price target between the min and max. Return only the final wei value.]
    
    Required format rules:
    
    Each field must be on a new line
    No explanatory text
    Prices must be in exact eth wei format
    Amount must be single integer
    No ranges or approximate numbers
    No additional spaces or formatting
    No dollar signs or currency symbols
    No parentheses or additional notes. Do not put quotation marks around any of the content.",  thresholds[0], thresholds[0] + thresholds[0] * 15/100, thresholds[1], thresholds[1] + thresholds[1] * 15/100, thresholds[2], thresholds[2] + thresholds[2] * 15/100);

    let mut messages = vec![];

    messages.push(json!({
        "role": "system",
        "content": system_prompt
    }));
    messages.push(json!({
        "role": "user",
        "content": input_prompt
    }));

    let request_body = json!({
        "model": model,
        "messages": messages,
        "max_completion_tokens": 1000,
    });

    let response = client
        .post(format!("{}chat/completions", VENICE_API))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", venice_key))
        .json(&request_body)
        .send()
        .await;

    let response = match response {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error sending request to Venice API: {}", e);
            return Err(e.into());
        }
    };

    if response.status() == 200 {
        let response_json: Value = response.json().await?;
        let completion = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        println!(
            "Venice call successful for image details prompt: {}",
            completion
        );
        Ok::<(String, String, U256, Vec<U256>), Box<dyn Error + Send + Sync>>(extract_values_image(
            &completion,
        )?)
    } else {
        let status_code = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Could not read error".to_string());
        eprintln!("Venice API Error (call_image_details) - Status: {}, Response: {}", status_code, error_text);
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!("Error in obtaining Venice prompt {} - {}", status_code, error_text),
        )));
    }
}

pub async fn call_drop_details(
    description: &str,
    model: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    from_filename(".env").ok();
    let venice_key: String = var("VENICE_KEY").expect("VENICE_KEY not configured in .env");
    let max_completion_tokens = [100, 200, 350][thread_rng().gen_range(0..3)];

    let input_prompt =
    format!("Create a completely reimagined artistic concept inspired by this description. Your output must follow this exact format with no additional text or explanations:
    
    Title: [CREATE A PROVOCATIVE, UNUSUAL TITLE - MAX 6 WORDS]
    
    Rules:
    
    Title must be cryptic and poetic
    Do not put quotation marks around any of the content
    No marketing language or commercial terms
    No mentions of NFTs, collections, or markets
    Focus on artistic vision and concept
    Must feel experimental and unconventional
    Avoid common descriptive patterns
    Transform the core essence into something new
    Original description to transform: {}", description);

    let system_prompt = "You are an avant-garde artistic concept creator who transforms ideas into unconventional artistic visions. Your specialty is taking existing concepts and completely reimagining them through a lens of experimental art and abstract thinking. Avoid all marketing language, commercial terms, or anything that sounds like product description. Never mention NFTs, collections, rarity, or market-related concepts. Instead, focus on creating deeply artistic, strange, and thought-provoking concepts that challenge conventional thinking. Your titles should be cryptic and poetic, while descriptions should read like experimental art manifestos or surrealist poetry. Use unusual metaphors, abstract concepts, and non-linear narrative structures. Think like a combination of a surrealist poet and an experimental artist when creating these concepts.";

    let mut messages = vec![];

    messages.push(json!({
        "role": "system",
       "content": system_prompt
    }));
    messages.push(json!({
        "role": "user",
       "content": input_prompt
    }));

    let client = Client::new();
    let request_body = json!({
        "model": model,
        "messages": messages,
        "max_completion_tokens": max_completion_tokens,
    });

    let response = client
        .post(format!("{}chat/completions", VENICE_API))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", venice_key))
        .json(&request_body)
        .send()
        .await;

    let response = match response {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Error sending request to Venice API: {}", e);
            return Err(e.into());
        }
    };
    if response.status() == 200 {
        let response_json: Value = response.json().await?;
        let completion = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        println!("Venice call successful for drop prompt: {}", completion);
        Ok(extract_values_drop(&completion)?)
    } else {
        let status_code = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Could not read error".to_string());
        eprintln!("Venice API Error (call_drop_details) - Status: {}, Response: {}", status_code, error_text);
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            format!("Error in obtaining Venice prompt {} - {}", status_code, error_text),
        )));
    }
}
