use log::info;
use serde_json::Value;
use std::sync::OnceLock;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(reqwest::Client::new)
}

#[tauri::command]
pub async fn get_library_image(app_id: u32) -> Option<String> {
    let input = serde_json::json!({
        "ids": [
            {
                "appid": app_id
            }
        ],
        "context": {
            "language": "english",
            "country_code": "US"
        },
        "data_request": {
            "include_assets": true
        }
    });

    let response = http_client()
        .get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/")
        .query(&[("input_json", input.to_string())])
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;

    let json: Value = serde_json::from_str(&response).ok()?;

    let assets = json
        .get("response")?
        .get("store_items")?
        .get(0)?
        .get("assets")?;

    let format = assets.get("asset_url_format")?.as_str()?;

    let capsule = assets.get("library_capsule")?.as_str()?;

    let url = format!(
        "https://shared.fastly.steamstatic.com/store_item_assets/{}",
        format.replace("${FILENAME}", capsule)
    );

    info!("Resolved URL for AppID {}: {}", app_id, url);

    Some(url)
}
