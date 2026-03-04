use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThemeChoice {
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub theme: ThemeChoice,
    pub font_size: f32,
    pub word_wrap: bool,
    pub font_family: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::Dark,
            font_size: 14.0,
            word_wrap: true,
            font_family: None,
        }
    }
}

pub fn load_config() -> Config {
    confy::load("piconote", None).unwrap_or_default()
}

pub fn save_config(config: &Config) {
    let _ = confy::store("piconote", None, config);
}
