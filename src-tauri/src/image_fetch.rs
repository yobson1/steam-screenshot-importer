use log::{error, info};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
const STORE_ITEMS_CHUNK_SIZE: usize = 100;

#[derive(Deserialize)]
struct StoreBrowseResponse {
    response: StoreBrowseResult,
}

#[derive(Deserialize)]
struct StoreBrowseResult {
    #[serde(default)]
    store_items: Vec<StoreItem>,
}

#[derive(Deserialize)]
struct StoreItem {
    appid: Option<u32>,
    id: Option<u32>,
    assets: Option<StoreItemAssets>,
}

#[derive(Deserialize)]
struct StoreItemAssets {
    asset_url_format: Option<String>,
    library_capsule: Option<String>,
}

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(reqwest::Client::new)
}

pub async fn get_library_images(app_ids: &[u32]) -> HashMap<u32, String> {
    let mut image_urls = HashMap::new();

    for app_ids in app_ids.chunks(STORE_ITEMS_CHUNK_SIZE) {
        image_urls.extend(get_library_images_chunk(app_ids).await);
    }

    image_urls
}

async fn get_library_images_chunk(app_ids: &[u32]) -> HashMap<u32, String> {
    if app_ids.is_empty() {
        return HashMap::new();
    }

    let input = serde_json::json!({
        "ids": app_ids
            .iter()
            .map(|app_id| serde_json::json!({ "appid": app_id }))
            .collect::<Vec<_>>(),
        "context": {
            "language": "english",
            "country_code": "US"
        },
        "data_request": {
            "include_assets": true
        }
    });

    let response = match http_client()
        .get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/")
        .query(&[("input_json", input.to_string())])
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            error!(
                "Failed to request Steam library images for {} apps: {}",
                app_ids.len(),
                error
            );
            return HashMap::new();
        }
    };

    if !response.status().is_success() {
        error!(
            "Steam Store API returned HTTP {} while fetching library images for {} apps",
            response.status(),
            app_ids.len()
        );
        return HashMap::new();
    }

    let response_body = match response.text().await {
        Ok(response_body) => response_body,
        Err(error) => {
            error!(
                "Failed to read Steam Store API response for {} apps: {}",
                app_ids.len(),
                error
            );
            return HashMap::new();
        }
    };

    let response: StoreBrowseResponse = match serde_json::from_str(&response_body) {
        Ok(response) => response,
        Err(error) => {
            error!(
                "Failed to parse Steam Store API response for {} apps: {}",
                app_ids.len(),
                error
            );
            return HashMap::new();
        }
    };

    response
        .response
        .store_items
        .into_iter()
        .filter_map(library_image_url)
        .inspect(|(app_id, url)| info!("Resolved URL for AppID {}: {}", app_id, url))
        .collect()
}

fn library_image_url(store_item: StoreItem) -> Option<(u32, String)> {
    let app_id = store_item.appid.or(store_item.id)?;
    let assets = store_item.assets?;
    let format = assets.asset_url_format?;
    let capsule = assets.library_capsule?;

    let url = format!(
        "https://shared.fastly.steamstatic.com/store_item_assets/{}",
        format.replace("${FILENAME}", &capsule)
    );

    Some((app_id, url))
}
