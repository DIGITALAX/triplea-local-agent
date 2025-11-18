use crate::utils::constants::{
    ACCESS_CONTROLS, AGENTS, COLLECTION_MANAGER, LENS_CHAIN_ID, LENS_RPC_URL, MARKET,
};
use aes_gcm::{
    aead::{Aead, KeyInit},
    {Aes256Gcm, Nonce},
};
use base64::{engine::general_purpose, Engine};
use dotenv::{dotenv, from_filename, var};
use ethers::{
    abi::{Abi, Address},
    contract::Contract,
    middleware::{SignerMiddleware, Middleware},
    providers::{Http, Provider, ProviderExt},
    signers::{LocalWallet, Signer},
    types::Chain,
};
use reqwest::Client;
use serde_json::{from_str, Value};
use std::{
    error::Error,
    sync::{Arc, Mutex, Once},
};

static INIT_PROVIDER: Once = Once::new();
static INIT_LENS: Once = Once::new();
static ACCESS_CONTROLS_CONTRACT: Mutex<
    Option<Arc<Contract<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>>>,
> = Mutex::new(None);
static AGENTS_CONTRACT: Mutex<
    Option<Arc<Contract<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>>>,
> = Mutex::new(None);
static COLLECTION_MANAGER_CONTRACT: Mutex<
    Option<Arc<Contract<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>>>,
> = Mutex::new(None);
static MARKET_CONTRACT: Mutex<
    Option<Arc<Contract<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>>>,
> = Mutex::new(None);
static PROVIDER: Mutex<Option<Arc<Provider<Http>>>> = Mutex::new(None);
static LENS_CLIENT: Mutex<Option<Arc<Client>>> = Mutex::new(None);
static WALLET: Mutex<Option<LocalWallet>> = Mutex::new(None);

pub fn initialize_provider() -> Arc<Provider<Http>> {
    INIT_PROVIDER.call_once(|| {
        dotenv().ok();
        let chain_id = *LENS_CHAIN_ID;
        let chain: Chain = unsafe { std::mem::transmute(chain_id as u64) };
        let mut provider =
            Provider::<Http>::try_from(LENS_RPC_URL).expect("Error in creating the provider");
        let provider = provider.set_chain(chain).clone();
        *PROVIDER.lock().unwrap() = Some(Arc::new(provider));
    });

    PROVIDER
        .lock()
        .unwrap()
        .clone()
        .expect("Provider not initialized")
}

pub fn initialize_api() -> Arc<Client> {
    INIT_LENS.call_once(|| {
        *LENS_CLIENT.lock().unwrap() = Some(Arc::new(Client::new()));
    });

    LENS_CLIENT
        .lock()
        .unwrap()
        .clone()
        .expect("Client not initialized")
}

pub fn initialize_access_wallet() -> LocalWallet {
    from_filename(".env").ok();
    let wallet = match var("FAUCET_KEY") {
        Ok(key) => match key.parse::<LocalWallet>() {
            Ok(mut wallet) => {
                let chain_id = *LENS_CHAIN_ID;
                wallet = wallet.with_chain_id(chain_id);
                wallet
            }
            Err(e) => panic!("Error in parsing faucet private key: {:?}", e),
        },
        Err(e) => panic!("FAUCET_KEY not found: {:?}", e),
    };

    *WALLET.lock().unwrap() = Some(wallet.clone());
    wallet
}

pub fn initialize_wallet(private_key: u32) -> Option<LocalWallet> {
    from_filename(".env").ok();
    match var(format!("ID_{}", private_key.to_string())) {
        Ok(key) => match key.parse::<LocalWallet>() {
            Ok(mut wallet) => {
                let chain_id = *LENS_CHAIN_ID;
                wallet = wallet.with_chain_id(chain_id);
                *WALLET.lock().unwrap() = Some(wallet.clone());
                Some(wallet)
            }
            Err(e) => {
                eprintln!("Error in parsing private key: {:?}", e);
                None
            }
        },
        Err(_) => {
            eprintln!(
                "PRIVATE_KEY not found in .env for agent_{}, looking in /var/data/data.json...",
                private_key
            );

            match std::fs::
            read_to_string("/var/data/data.json")
            // read_to_string("var/data/data.json")
            {
                Ok(json_data) => {
                    let parsed_json: Value =
                        from_str(&json_data).expect("Failed to parse data.json");
                    if let Some(encrypted_data) =
                        parsed_json[format!("ID_{}", private_key)].as_str()
                    {
                        match configure_key(encrypted_data) {
                            Ok(decrypted_private_key) => {
                                match decrypted_private_key.parse::<LocalWallet>() {
                                    Ok(mut wallet) => {
                                        let chain_id = *LENS_CHAIN_ID;
                                        wallet = wallet.with_chain_id(chain_id);
                                        *WALLET.lock().unwrap() = Some(wallet.clone());
                                        println!("Key Found for ID_{} in var/data", private_key);
                                        Some(wallet)
                                    }
                                    Err(e) => {
                                        eprintln!("Error parsing decrypted private key: {:?}", e);
                                        read_from_secret(private_key);
                                        None
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error decrypting private key: {:?}", e);
                                read_from_secret(private_key);
                                None
                            }
                        }
                    } else {
                        eprintln!("ID_{} not found in data.json", private_key);
                        read_from_secret(private_key);
                        None
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read /var/data/data.json: {:?}", e);
                    read_from_secret(private_key);
                    None
                }
            }
        }
    }
}

fn read_from_secret(private_key: u32) -> Option<LocalWallet> {
    eprintln!(
        "PRIVATE_KEY not found in .env or /var/data/data.json, looking in /etc/secrets/data.txt..."
    );

    match std::fs::read_to_string("/etc/secrets/data.txt") {
        Ok(data) => {
            let entries: Vec<&str> = data.lines().collect();
            for entry in entries {
                if entry.starts_with(&format!("ID_{}=", private_key)) {
                    if let Some(encrypted_data) = entry.split('=').nth(1) {
                        match configure_key(encrypted_data) {
                            Ok(decrypted_private_key) => {
                                match decrypted_private_key.parse::<LocalWallet>() {
                                    Ok(mut wallet) => {
                                        let chain_id = *LENS_CHAIN_ID;
                                        wallet = wallet.with_chain_id(chain_id);
                                        *WALLET.lock().unwrap() = Some(wallet.clone());
                                        println!("Key Found for ID_{} in etc/secrets", private_key);
                                        return Some(wallet);
                                    }
                                    Err(e) => {
                                        eprintln!("Error parsing decrypted private key: {:?}", e);
                                        return None;
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error decrypting private key: {:?}", e);
                                return None;
                            }
                        }
                    } else {
                        eprintln!(
                            "Details not found for ID_{} in /etc/secrets/data.txt",
                            private_key
                        );
                        return None;
                    }
                }
            }
            eprintln!("ID_{} not found in /etc/secrets/data.txt", private_key);
            return None;
        }
        Err(e) => {
            eprintln!("Failed to read /etc/secrets/data.txt: {:?}", e);
            return None;
        }
    }
}

pub fn initialize_contracts(
    private_key: u32,
) -> Option<(
    Arc<Contract<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>>,
    Arc<Contract<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>>,
    Arc<Contract<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>>,
    Arc<Contract<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>>,
)> {
    dotenv().ok();
    let provider = initialize_provider();


    let wallet = match initialize_wallet(private_key) {
        Some(wallet) => wallet,
        None => {
            eprintln!("Wallet initialization failed. Skipping agent creation.");
            return None;
        }
    };

    let client = Arc::new(SignerMiddleware::new(provider.clone(), wallet));

    let access_wallet = initialize_access_wallet();


    let access_client = Arc::new(SignerMiddleware::new(provider.clone(), access_wallet));

    let access_controls_address = ACCESS_CONTROLS
        .parse::<Address>()
        .expect("Error in parsing ACCESS_CONTROLS");
    let access_controls_abi: Abi =
        from_str(include_str!("./../../abis/TripleAAccessControls.json"))
            .expect("Error in loading AccessControls ABI");
    let access_controls_contract = Contract::new(
        access_controls_address,
        access_controls_abi,
        access_client.clone(),
    );
    *ACCESS_CONTROLS_CONTRACT.lock().unwrap() = Some(Arc::new(access_controls_contract));

    let agents_address = AGENTS.parse::<Address>().expect("Error in parsing AGENTS");
    let agents_abi: Abi = from_str(include_str!("./../../abis/TripleAAgents.json"))
        .expect("Error in loading Agents ABI");
    let agents_contract = Contract::new(agents_address, agents_abi, client.clone());
    *AGENTS_CONTRACT.lock().unwrap() = Some(Arc::new(agents_contract));

    let collection_manager_address = COLLECTION_MANAGER
        .parse::<Address>()
        .expect("Error in parsing COLLECTION_MANAGER");
    let collection_manager_abi: Abi =
        from_str(include_str!("./../../abis/TripleACollectionManager.json"))
            .expect("Error in loading CollectionManager ABI");
    let collection_manager_contract = Contract::new(
        collection_manager_address,
        collection_manager_abi,
        client.clone(),
    );
    *COLLECTION_MANAGER_CONTRACT.lock().unwrap() = Some(Arc::new(collection_manager_contract));

    let market_address = MARKET.parse::<Address>().expect("Error in parsing MARKET");
    let market_abi: Abi = from_str(include_str!("./../../abis/TripleAMarket.json"))
        .expect("Error in loading Market ABI");
    let market_contract = Contract::new(market_address, market_abi, client.clone());
    *MARKET_CONTRACT.lock().unwrap() = Some(Arc::new(market_contract));

    Some((
        ACCESS_CONTROLS_CONTRACT
            .lock()
            .unwrap()
            .clone()
            .expect("ACCESS_CONTROLS_CONTRACT not initialized"),
        AGENTS_CONTRACT
            .lock()
            .unwrap()
            .clone()
            .expect("AGENTS_CONTRACT not initialized"),
        COLLECTION_MANAGER_CONTRACT
            .lock()
            .unwrap()
            .clone()
            .expect("COLLECTION_MANAGER_CONTRACT not initialized"),
        MARKET_CONTRACT
            .lock()
            .unwrap()
            .clone()
            .expect("MARKET_CONTRACT not initialized"),
    ))
}

pub fn configure_key(encryption_details: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let encryption_details_parse: Value =
        from_str(encryption_details).expect("Failed to parse encryption_details JSON");
    let encrypted_private_key = encryption_details_parse["encrypted"].as_str().unwrap();
    let iv = encryption_details_parse["iv"].as_str().unwrap();
    let auth_tag = encryption_details_parse["authTag"].as_str().unwrap();
    let encryption_key = var("ENCRYPTION_KEY").expect("ENCRYPTION_KEY isn't configured.");

    let key = encryption_key.as_bytes();
    let cipher = Aes256Gcm::new_from_slice(key).expect("Invalid Key");

    let slice = &general_purpose::STANDARD
        .decode(iv)
        .expect("Error with slice");
    let nonce = Nonce::from_slice(slice);
    let ciphertext = general_purpose::STANDARD.decode(encrypted_private_key)?;
    let mut combined = ciphertext.clone();
    combined.extend_from_slice(&general_purpose::STANDARD.decode(auth_tag)?);

    let decrypted_data = cipher
        .decrypt(nonce, combined.as_ref())
        .expect("Invalid Data to Decrypt");
    let private_key = String::from_utf8(decrypted_data).expect("UTF8 Error in key");

    Ok(private_key)
}
