use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResizeFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

impl ResizeFilter {
    pub const ALL: [Self; 5] = [
        Self::Nearest,
        Self::Triangle,
        Self::CatmullRom,
        Self::Gaussian,
        Self::Lanczos3,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Nearest => "Nearest",
            Self::Triangle => "Triangle",
            Self::CatmullRom => "CatmullRom",
            Self::Gaussian => "Gaussian",
            Self::Lanczos3 => "Lanczos3",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Nearest => "Nearest Neighbor: fastest, blockiest",
            Self::Triangle => "Triangle: bilinear",
            Self::CatmullRom => "Catmull-Rom: bicubic",
            Self::Gaussian => "Gaussian",
            Self::Lanczos3 => "Lanczos3: best quality, slowest",
        }
    }
}

impl fmt::Display for ResizeFilter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

impl FromStr for ResizeFilter {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|filter| filter.name() == value)
            .ok_or(())
    }
}
