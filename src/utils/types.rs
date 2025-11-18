use ethers::{
    abi::{InvalidOutputType, Token, Tokenizable, TokenizableItem},
    contract::ContractInstance,
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::Wallet,
    types::{Address, U256},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Text {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageExample {
    pub user: String,
    pub content: Text,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TripleAAgent {
    pub id: u32,
    pub name: String,
    pub bio: String,
    pub lore: String,
    pub style: String,
    pub knowledge: String,
    pub adjectives: String,
    pub message_examples: Vec<Vec<MessageExample>>,
    pub model: String,
    pub cover: String,
    pub custom_instructions: String,
    pub wallet: String,
    pub clock: u32,
    pub last_active_time: u32,
    pub account_address: String,
    pub feeds: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TripleAWorker {
    pub lead: bool,
    pub publish: bool,
    pub remix: bool,
    pub mint: bool,
    pub lead_frequency: U256,
    pub publish_frequency: U256,
    pub remix_frequency: U256,
    pub mint_frequency: U256,
    pub instructions: String,
}

#[derive(Debug, Clone)]
pub struct AgentManager {
    pub agent: TripleAAgent,
    pub current_queue: Vec<AgentActivity>,
    pub agents_contract: Arc<
        ContractInstance<
            Arc<SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>>,
            SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>,
        >,
    >,
    pub market_contract: Arc<
        ContractInstance<
            Arc<SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>>,
            SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>,
        >,
    >,
    pub access_controls_contract: Arc<
        ContractInstance<
            Arc<SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>>,
            SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>,
        >,
    >,
    pub collection_manager_contract: Arc<
        ContractInstance<
            Arc<SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>>,
            SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>,
        >,
    >,
    pub tokens: Option<SavedTokens>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Collection {
    pub image: String,
    pub title: String,
    pub description: String,
    pub artist: String,
    pub username: String,
    pub collection_id: U256,
    pub prices: Vec<Price>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Price {
    pub price: U256,
    pub token: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AgentActivity {
    pub collection: Collection,
    pub token: String,
    pub worker: TripleAWorker,
    pub balance: Balance,
    pub collection_id: U256,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Balance {
    pub rent_balance: U256,
    pub bonus_balance: U256,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IPFSResponse {
    Name: String,
    pub Hash: String,
    Size: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LensTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Publication {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub lens: Content,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Image {
    #[serde(rename = "type")]
    pub tipo: String,
    pub item: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Content {
    pub mainContentFocus: String,
    pub title: String,
    pub content: String,
    pub id: String,
    pub locale: String,
    pub tags: Vec<String>,
    pub image: Option<Image>,
}

#[derive(Debug, Clone)]
pub struct SavedTokens {
    pub tokens: LensTokens,
    pub expiry: i64,
}
#[derive(Debug, Clone)]
pub enum ActivityType {
    Publish,
    Lead,
    Remix,
    Mint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriceCollection {
    pub collectionId: U256,
    pub amountSold: U256,
    pub amount: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectionInput {
    pub tokens: Vec<Address>,
    pub prices: Vec<U256>,
    pub agentIds: Vec<U256>,
    pub metadata: String,
    pub forArtist: Address,
    pub collectionType: u8,
    pub amount: U256,
    pub fulfillerId: U256,
    pub remixId: U256,
    pub remixable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectionWorker {
    pub instructions: String,
    pub publishFrequency: U256,
    pub remixFrequency: U256,
    pub leadFrequency: U256,
    pub mintFrequency: U256,
    pub publish: bool,
    pub remix: bool,
    pub lead: bool,
    pub mint: bool,
}

impl Tokenizable for CollectionInput {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::Tuple(tokens) if tokens.len() == 10 => Ok(Self {
                tokens: tokens[0]
                    .clone()
                    .into_array()
                    .unwrap()
                    .into_iter()
                    .map(|t| t.into_address().unwrap())
                    .collect(),
                prices: tokens[1]
                    .clone()
                    .into_array()
                    .unwrap()
                    .into_iter()
                    .map(|t| t.into_uint().unwrap())
                    .collect(),
                agentIds: tokens[2]
                    .clone()
                    .into_array()
                    .unwrap()
                    .into_iter()
                    .map(|t| t.into_uint().unwrap())
                    .collect(),
                metadata: tokens[3].clone().into_string().unwrap(),
                forArtist: tokens[4].clone().into_address().unwrap(),
                collectionType: tokens[5].clone().into_int().unwrap().as_u32() as u8,
                amount: tokens[6].clone().into_uint().unwrap(),
                fulfillerId: tokens[7].clone().into_uint().unwrap(),
                remixId: tokens[8].clone().into_uint().unwrap(),
                remixable: tokens[9].clone().into_bool().unwrap(),
            }),
            _ => Err(InvalidOutputType(String::from("conversion error"))),
        }
    }

    fn into_token(self) -> Token {
        Token::Tuple(vec![
            Token::Array(self.tokens.into_iter().map(Token::Address).collect()),
            Token::Array(self.prices.into_iter().map(Token::Uint).collect()),
            Token::Array(self.agentIds.into_iter().map(Token::Uint).collect()),
            Token::String(self.metadata),
            Token::Address(self.forArtist),
            Token::Uint(U256::from(self.collectionType)),
            Token::Uint(self.amount),
            Token::Uint(self.fulfillerId),
            Token::Uint(self.remixId),
            Token::Bool(self.remixable),
        ])
    }
}

impl Tokenizable for CollectionWorker {
    fn from_token(token: Token) -> Result<Self, InvalidOutputType> {
        match token {
            Token::Tuple(tokens) if tokens.len() == 9 => Ok(Self {
                instructions: tokens[0].clone().into_string().unwrap(),
                publishFrequency: tokens[1].clone().into_uint().unwrap(),
                remixFrequency: tokens[2].clone().into_uint().unwrap(),
                leadFrequency: tokens[3].clone().into_uint().unwrap(),
                mintFrequency: tokens[4].clone().into_uint().unwrap(),
                publish: tokens[5].clone().into_bool().unwrap(),
                remix: tokens[6].clone().into_bool().unwrap(),
                lead: tokens[7].clone().into_bool().unwrap(),
                mint: tokens[8].clone().into_bool().unwrap(),
            }),
            _ => Err(InvalidOutputType(String::from("conversion error"))),
        }
    }

    fn into_token(self) -> Token {
        Token::Tuple(vec![
            Token::String(self.instructions),
            Token::Uint(self.publishFrequency),
            Token::Uint(self.remixFrequency),
            Token::Uint(self.leadFrequency),
            Token::Uint(self.mintFrequency),
            Token::Bool(self.publish),
            Token::Bool(self.remix),
            Token::Bool(self.lead),
            Token::Bool(self.mint),
        ])
    }
}

impl TokenizableItem for CollectionWorker {}

impl TokenizableItem for CollectionInput {}
