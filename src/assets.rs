use std::borrow::Cow;

use anyhow::anyhow;
use gpui::{AssetSource, Result, SharedString};

const APP_ASSET_PREFIX: &str = "assets/";

#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[prefix = "assets/"]
struct AppAssets;

pub struct Assets {
    component_assets: gpui_component_assets::Assets,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            component_assets: gpui_component_assets::Assets::new(""),
        }
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.starts_with(APP_ASSET_PREFIX) {
            return AppAssets::get(path)
                .map(|file| Some(file.data))
                .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""));
        }

        self.component_assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut assets = self.component_assets.list(path)?;
        assets.extend(
            AppAssets::iter()
                .filter(|asset_path| asset_path.starts_with(path))
                .map(|asset_path| SharedString::from(asset_path.into_owned())),
        );
        Ok(assets)
    }
}
