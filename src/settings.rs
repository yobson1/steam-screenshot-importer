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

    pub fn from_name(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|filter| filter.name() == value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScreenshotSettings {
    pub jpeg_quality: u8,
    pub resize_filter: ResizeFilter,
    pub check_updates_on_startup: bool,
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        Self {
            jpeg_quality: 95,
            resize_filter: ResizeFilter::Lanczos3,
            check_updates_on_startup: true,
        }
    }
}
