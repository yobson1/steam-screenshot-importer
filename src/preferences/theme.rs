use std::{fmt, str::FromStr};

use gpui_component::ThemeMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeSelection {
    System,
    Light,
    Dark,
}

impl ThemeSelection {
    pub const fn mode(self) -> Option<ThemeMode> {
        match self {
            Self::System => None,
            Self::Light => Some(ThemeMode::Light),
            Self::Dark => Some(ThemeMode::Dark),
        }
    }
}

impl From<ThemeMode> for ThemeSelection {
    fn from(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::Light,
            ThemeMode::Dark => Self::Dark,
        }
    }
}

impl fmt::Display for ThemeSelection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        })
    }
}

impl FromStr for ThemeSelection {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "system" => Ok(Self::System),
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            _ => Err(()),
        }
    }
}
