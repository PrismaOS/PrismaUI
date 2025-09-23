use rust_embed::RustEmbed;
use anyhow;

#[derive(RustEmbed)]
#[folder = "assets/"]
#[include = "*.png"]
#[include = "*.jpg"]
#[include = "*.jpeg"]
#[include = "*.svg"]
#[include = "*.webp"]
pub struct Assets;

impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow::anyhow!("Asset not found: {}", path))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<std::path::PathBuf>> {
        Ok(Self::iter()
            .filter(|p| p.starts_with(path))
            .map(|p| std::path::PathBuf::from(p.to_string()))
            .collect())
    }
}