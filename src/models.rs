use serde::{
    Deserialize,
    Serialize,
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModInfo {
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "GameBananaId")]
    pub game_banana_id: u32,
    #[serde(rename = "GameBananaFileId")]
    pub game_banana_file_id: Option<u32>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModState {
    pub version: String,
    pub is_explicit: bool,
}
