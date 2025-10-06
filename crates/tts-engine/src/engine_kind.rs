use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    F5,
    IndexTts,
    Shimmy,
}

impl EngineKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            EngineKind::F5 => "f5",
            EngineKind::IndexTts => "index_tts",
            EngineKind::Shimmy => "shimmy",
        }
    }
}

impl fmt::Display for EngineKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EngineKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "f5" => Ok(EngineKind::F5),
            "index_tts" | "index-tts" | "indextts" => Ok(EngineKind::IndexTts),
            "shimmy" => Ok(EngineKind::Shimmy),
            _ => Err(()),
        }
    }
}
