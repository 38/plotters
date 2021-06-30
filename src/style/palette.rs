use super::{color::PaletteColor, full_palette};

// Helper to quickly convert colors into tuples
macro_rules! color_to_tuple {
    ($name:path) => {
        ($name.0, $name.1, $name.2);
    };
}

pub trait Palette {
    const COLORS: &'static [(u8, u8, u8)];
    fn pick(idx: usize) -> PaletteColor<Self>
    where
        Self: Sized,
    {
        PaletteColor::<Self>::pick(idx)
    }
}

/// The palette of 99% accessibility
pub struct Palette99;
/// The palette of 99.99% accessibility
pub struct Palette9999;
/// The palette of 100% accessibility
pub struct Palette100;

/// A palette of light colors, suitable for backgrounds.
pub struct PaletteLight;
/// A palette of vivid colors.
pub struct PaletteVivid;

impl Palette for PaletteLight {
    const COLORS: &'static [(u8, u8, u8)] = &[
        color_to_tuple!(full_palette::RED_100),
        color_to_tuple!(full_palette::GREEN_100),
        color_to_tuple!(full_palette::BLUE_100),
        color_to_tuple!(full_palette::ORANGE_100),
        color_to_tuple!(full_palette::TEAL_100),
        color_to_tuple!(full_palette::YELLOW_100),
        color_to_tuple!(full_palette::CYAN_100),
        color_to_tuple!(full_palette::DEEPORANGE_100),
        color_to_tuple!(full_palette::BLUEGREY_100),
        color_to_tuple!(full_palette::PURPLE_100),
        color_to_tuple!(full_palette::LIME_100),
        color_to_tuple!(full_palette::INDIGO_100),
        color_to_tuple!(full_palette::DEEPPURPLE_100),
        color_to_tuple!(full_palette::PINK_100),
        color_to_tuple!(full_palette::LIGHTBLUE_100),
        color_to_tuple!(full_palette::LIGHTGREEN_100),
        color_to_tuple!(full_palette::AMBER_100),
    ];
}

impl Palette for PaletteVivid {
    const COLORS: &'static [(u8, u8, u8)] = &[
        color_to_tuple!(full_palette::RED_A400),
        color_to_tuple!(full_palette::GREEN_A400),
        color_to_tuple!(full_palette::BLUE_A400),
        color_to_tuple!(full_palette::ORANGE_A400),
        color_to_tuple!(full_palette::TEAL_A400),
        color_to_tuple!(full_palette::YELLOW_A400),
        color_to_tuple!(full_palette::CYAN_A400),
        color_to_tuple!(full_palette::DEEPORANGE_A400),
        color_to_tuple!(full_palette::BLUEGREY_A400),
        color_to_tuple!(full_palette::PURPLE_A400),
        color_to_tuple!(full_palette::LIME_A400),
        color_to_tuple!(full_palette::INDIGO_A400),
        color_to_tuple!(full_palette::DEEPPURPLE_A400),
        color_to_tuple!(full_palette::PINK_A400),
        color_to_tuple!(full_palette::LIGHTBLUE_A400),
        color_to_tuple!(full_palette::LIGHTGREEN_A400),
        color_to_tuple!(full_palette::AMBER_A400),
    ];
}

impl Palette for Palette99 {
    const COLORS: &'static [(u8, u8, u8)] = &[
        (230, 25, 75),
        (60, 180, 75),
        (255, 225, 25),
        (0, 130, 200),
        (245, 130, 48),
        (145, 30, 180),
        (70, 240, 240),
        (240, 50, 230),
        (210, 245, 60),
        (250, 190, 190),
        (0, 128, 128),
        (230, 190, 255),
        (170, 110, 40),
        (255, 250, 200),
        (128, 0, 0),
        (170, 255, 195),
        (128, 128, 0),
        (255, 215, 180),
        (0, 0, 128),
        (128, 128, 128),
        (0, 0, 0),
    ];
}

impl Palette for Palette9999 {
    const COLORS: &'static [(u8, u8, u8)] = &[
        (255, 225, 25),
        (0, 130, 200),
        (245, 130, 48),
        (250, 190, 190),
        (230, 190, 255),
        (128, 0, 0),
        (0, 0, 128),
        (128, 128, 128),
        (0, 0, 0),
    ];
}

impl Palette for Palette100 {
    const COLORS: &'static [(u8, u8, u8)] =
        &[(255, 225, 25), (0, 130, 200), (128, 128, 128), (0, 0, 0)];
}
