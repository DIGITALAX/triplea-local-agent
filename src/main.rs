use chrono::{ Timelike, Utc };
use dotenv::{ dotenv, var };
use serde_json::Value;
use std::{ error::Error, time::Duration };
use tokio::spawn;
use utils::{
    types::*,
};
mod classes;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenv().ok();

    let agent_id: u32 = var("AGENT_ID")
        .expect("AGENT_ID not set")
        .parse()
        .expect("AGENT_ID must be a number");

    let agent_name = var("AGENT_NAME").expect("AGENT_NAME not set");
    let agent_bio = var("AGENT_BIO").expect("AGENT_BIO not set");
    let agent_lore = var("AGENT_LORE").expect("AGENT_LORE not set");
    let agent_adjectives = var("AGENT_ADJECTIVES").expect("AGENT_ADJECTIVES not set");
    let agent_style = var("AGENT_STYLE").expect("AGENT_STYLE not set");
    let agent_knowledge = var("AGENT_KNOWLEDGE").expect("AGENT_KNOWLEDGE not set");
    let agent_model = var("AGENT_MODEL").unwrap_or_else(|_| "llama-3.3-70b".to_string());
    let agent_cover = var("AGENT_COVER").expect("AGENT_COVER not set");
    let agent_custom_instructions = var("AGENT_CUSTOM_INSTRUCTIONS")
        .unwrap_or_else(|_| "Be creative and engaging".to_string());
    let agent_wallet = var("AGENT_WALLET").expect("AGENT_WALLET not set");
    let agent_account_address = var("AGENT_ACCOUNT_ADDRESS").expect("AGENT_ACCOUNT_ADDRESS not set");
    let agent_private_key = var("AGENT_PRIVATE_KEY").expect("AGENT_PRIVATE_KEY not set");

    let agent_clock: u32 = var("AGENT_CLOCK")
        .expect("AGENT_CLOCK not set")
        .parse()
        .expect("AGENT_CLOCK must be a number");

    let agent_feeds_str = var("AGENT_FEEDS").unwrap_or_else(|_| "[]".to_string());
    let agent_feeds: Vec<String> = serde_json::from_str(&agent_feeds_str)
        .unwrap_or_else(|_| Vec::new());

    let message_examples_str = var("AGENT_MESSAGE_EXAMPLES").unwrap_or_else(|_| "[]".to_string());
    let message_examples_array: Vec<Vec<Value>> = serde_json::from_str(&message_examples_str)
        .unwrap_or_else(|_| Vec::new());

    let message_examples: Vec<Vec<MessageExample>> = message_examples_array
        .iter()
        .map(|group| {
            group
                .iter()
                .map(|msg| MessageExample {
                    user: msg["user"].as_str().unwrap_or("").to_string(),
                    content: Text {
                        text: msg["content"]["text"].as_str().unwrap_or("").to_string(),
                    },
                })
                .collect()
        })
        .collect();

    std::env::set_var(&format!("ID_{}", agent_id), &agent_private_key);

    let agent = TripleAAgent {
        id: agent_id,
        name: agent_name,
        bio: agent_bio,
        lore: agent_lore,
        adjectives: agent_adjectives,
        style: agent_style,
        knowledge: agent_knowledge,
        message_examples,
        model: agent_model,
        cover: agent_cover,
        custom_instructions: agent_custom_instructions,
        wallet: agent_wallet,
        clock: agent_clock,
        last_active_time: Utc::now().timestamp() as u32,
        account_address: agent_account_address,
        feeds: agent_feeds,
    };

    println!("Starting agent: {} (ID: {})", agent.name, agent.id);
    println!("Agent wallet: {}", agent.wallet);
    println!("Agent clock: {} seconds ({}h {}m {}s)",
        agent.clock,
        agent.clock / 3600,
        (agent.clock % 3600) / 60,
        agent.clock % 60
    );

    let agent_manager = AgentManager::new(&agent).expect("Failed to create agent manager");

    spawn(activity_loop(agent_manager));

    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
    }
}

async fn activity_loop(mut agent_manager: AgentManager) {
    loop {
        if should_trigger(&agent_manager.agent) {
            println!("\n=== Agent {} triggering activity ===", agent_manager.agent.id);

            if let Err(err) = agent_manager.resolve_activity().await {
                eprintln!("Error resolving activity for agent {}: {:?}", agent_manager.agent.id, err);
            }

            println!("=== Agent {} activity complete ===\n", agent_manager.agent.id);
        } else {
            let now = Utc::now();
            let seconds_since_midnight = (now.hour() * 3600 + now.minute() * 60 + now.second()) as i32;
            let diff = ((agent_manager.agent.clock as i32) - seconds_since_midnight).abs();

            println!(
                "Waiting for trigger... (clock: {}s, now: {}s, diff: {}s)",
                agent_manager.agent.clock,
                seconds_since_midnight,
                diff
            );
        }

        tokio::time::sleep(Duration::from_secs(500)).await;
    }
}

fn should_trigger(agent: &TripleAAgent) -> bool {
    let now = Utc::now();
    let seconds_since_midnight = (now.hour() * 3600 + now.minute() * 60 + now.second()) as i32;
    let diff = ((agent.clock as i32) - seconds_since_midnight).abs();

    diff <= 500
}