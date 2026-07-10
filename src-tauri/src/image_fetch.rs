use log::info;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
const STORE_ITEMS_CHUNK_SIZE: usize = 100;

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
        Err(_) => return HashMap::new(),
    };

    let response = match response.text().await {
        Ok(response) => response,
        Err(_) => return HashMap::new(),
    };

    let json: Value = match serde_json::from_str(&response) {
        Ok(json) => json,
        Err(_) => return HashMap::new(),
    };

    json.get("response")
        .and_then(|response| response.get("store_items"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(library_image_url)
        .inspect(|(app_id, url)| info!("Resolved URL for AppID {}: {}", app_id, url))
        .collect()
}

fn library_image_url(store_item: &Value) -> Option<(u32, String)> {
    let app_id = store_item
        .get("appid")
        .or_else(|| store_item.get("id"))?
        .as_u64()?
        .try_into()
        .ok()?;

    let assets = store_item.get("assets")?;
    let format = assets.get("asset_url_format")?.as_str()?;
    let capsule = assets.get("library_capsule")?.as_str()?;

    let url = format!(
        "https://shared.fastly.steamstatic.com/store_item_assets/{}",
        format.replace("${FILENAME}", capsule)
    );

    Some((app_id, url))
}
