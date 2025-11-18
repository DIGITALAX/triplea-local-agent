use crate::IPFSResponse;
use base64::{
    engine::general_purpose::{self, STANDARD},
    Engine as _,
};
use dotenv::{dotenv, from_filename, var};
use reqwest::{
    multipart::{Form, Part},
    Client,
};
use serde_json::{from_str, Value};
use std::{
    error::Error,
    sync::{Arc, Once},
};
use tokio::{
    fs::{remove_file, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};
use uuid::Uuid;

static INIT: Once = Once::new();
static mut CLIENT: Option<Arc<Client>> = None;

pub fn create_client() -> Arc<Client> {
    unsafe {
        INIT.call_once(|| {
            dotenv().ok();
            let client = Client::new();
            CLIENT = Some(Arc::new(client));
        });
        CLIENT.clone().expect("Client not initialized")
    }
}

pub fn authentication() -> String {
    dotenv().ok();
    from_filename(".env").ok();
    let id = var("INFURA_PROJECT_ID").expect("INFURA_PROJECT_ID isn't configured");
    let key = var("INFURA_PROJECT_SECRET").expect("INFURA_PROJECT_SECRET isn't configured");
    let aut = format!("{}:{}", id, key);
    STANDARD.encode(aut)
}

pub async fn upload_image_to_ipfs(
    base64_str: &str,
) -> Result<IPFSResponse, Box<dyn Error + Send + Sync>> {
    let base64_data = base64_str.split(',').last().unwrap_or(base64_str);
    let image_bytes = general_purpose::STANDARD.decode(base64_data)?;
    let path = format!("/var/data/{}.png", Uuid::new_v4());
    // let path = format!("var/data/{}.png", Uuid::new_v4());
    let file_result = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&path)
        .await;
    match file_result {
        Ok(mut file) => {
            if let Err(err) = file.write_all(&image_bytes).await {
                eprintln!("Error writing image: {:?}", err);
                return Err(Box::new(err));
            }
            if let Err(err) = file.flush().await {
                eprintln!("Error flushing file: {:?}", err);
                return Err(Box::new(err));
            }

            drop(file);
            let file_read_result = File::open(&path).await;
            match file_read_result {
                Ok(mut file) => {
                    let mut buffer = Vec::new();
                    if let Err(err) = file.read_to_end(&mut buffer).await {
                        eprintln!("Error reading file: {:?}", err);
                        return Err(Box::new(err));
                    }

                    let client = create_client();
                    let aut_encoded = authentication();
                    let form = Form::new().part("file", Part::bytes(buffer).file_name("image.png"));

                    let response = client
                        .post("https://ipfs.infura.io:5001/api/v0/add")
                        .header("Authorization", format!("Basic {}", aut_encoded))
                        .multipart(form)
                        .send()
                        .await?;

                    let text_response = response.text().await?;
                    let ipfs_response: IPFSResponse = serde_json::from_str(&text_response)?;

                    if let Err(err) = remove_file(&path).await {
                        eprintln!("Error deleting file: {:?}", err);
                    }
                    Ok(ipfs_response)
                }
                Err(err) => {
                    eprintln!("Error opening file for reading: {:?}", err);
                    Err(Box::new(err))
                }
            }
        }
        Err(err) => {
            eprintln!("Error creating file: {:?}", err);
            Err(Box::new(err))
        }
    }
}

pub async fn upload_ipfs(data: String) -> Result<IPFSResponse, Box<dyn Error + Send + Sync>> {
    let client = create_client();
    let aut_encoded = authentication();

    let form: Form = Form::new().part("file", Part::text(data.clone()).file_name("data.json"));

    let response = client
        .post("https://ipfs.infura.io:5001/api/v0/add")
        .header("Authorization", format!("Basic {}", aut_encoded))
        .multipart(form)
        .send()
        .await?;

    let text_response = response.text().await?;
    let ipfs_response: IPFSResponse = from_str(&text_response)?;

    Ok(ipfs_response)
}

pub async fn get_storage_key() -> Result<String, Box<dyn Error>> {
    let client = Client::new();

    let response = client
        .post("https://api.grove.storage/link/new")
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Error obtaining storage_key: {}", error_text).into());
    }

    let text_response = response.text().await?;
    let json_response: Value = from_str(&text_response)?;

    if let Some(storage_key) = json_response
        .get(0)
        .and_then(|item| item.get("storage_key"))
    {
        if let Some(storage_key_str) = storage_key.as_str() {
            return Ok(storage_key_str.to_string());
        }
    }

    Err("Couldn't obtain storage_key.".into())
}

pub async fn upload_lens_storage(data: String) -> Result<String, Box<dyn Error>> {
    let client = create_client();
    // let storage_key = get_storage_key().await?;
    let url = format!("https://api.grove.storage/?chain_id=232");

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(data)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Error uploading to Lens Storage: {}", error_text).into());
    }

    let text_response = response.text().await?;
    let json_response: Value = from_str(&text_response)?;

    if let Some(uri) = json_response.get(0).and_then(|item| item.get("uri")) {
        if let Some(uri_str) = uri.as_str() {
            return Ok(uri_str.to_string());
        }
    }

    Err("Couldn't get URI.".into())
}
