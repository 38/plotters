use super::{FontData, FontDataInternal};
use crate::style::{Color, LayoutBox, TextStyle};

use std::convert::From;

/// The error type for the font implementation
pub type FontError = <FontDataInternal as FontData>::ErrorType;

/// The type we used to represent a result of any font operations
pub type FontResult<T> = Result<T, FontError>;

/// Specifying text transformations
#[derive(Clone)]
pub enum FontTransform {
    None,
    Rotate90,
    Rotate180,
    Rotate270,
}

impl FontTransform {
    /// Compute the offset of the "top-left" cornor of the text.
    /// "Top-left" defined as the first char's top-left in reading orientation.
    pub fn offset(&self, layout: LayoutBox) -> (i32, i32) {
        match self {
            FontTransform::None => (0, 0),
            FontTransform::Rotate90 => ((layout.1).1 - (layout.0).1, 0),
            FontTransform::Rotate180 => ((layout.1).0 - (layout.0).0, (layout.1).1 - (layout.0).1),
            FontTransform::Rotate270 => (0, (layout.1).0 - (layout.0).0),
        }
    }

    /// Transform the coordinate to performe the rotation
    pub fn transform(&self, x: i32, y: i32) -> (i32, i32) {
        match self {
            FontTransform::None => (x, y),
            FontTransform::Rotate90 => (-y, x),
            FontTransform::Rotate180 => (-x, -y),
            FontTransform::Rotate270 => (y, -x),
        }
    }
}

/// Describes a font
#[derive(Clone)]
pub struct FontDesc<'a> {
    size: f64,
    family: FontFamily<'a>,
    data: FontResult<FontDataInternal>,
    transform: FontTransform,
}

/// Describes font family
#[derive(Clone, Copy)]
pub enum FontFamily<'a> {
    Sans,
    SansSerif,
    Monospace,
    Name(&'a str),
}

impl<'a> FontFamily<'a> {
    pub fn as_str(&self) -> &str {
        match self {
            FontFamily::Sans => "sans",
            FontFamily::SansSerif => "sans-serif",
            FontFamily::Monospace => "monospace",
            FontFamily::Name(face) => face,
        }
    }
}

impl<'a> From<&'a str> for FontFamily<'a> {
    fn from(from: &'a str) -> FontFamily<'a> {
        match from {
            "sans" => FontFamily::Sans,
            "sans-serif" => FontFamily::SansSerif,
            "monospace" => FontFamily::Monospace,
            _ => FontFamily::Name(from),
        }
    }
}

impl<'a> From<&'a str> for FontDesc<'a> {
    fn from(from: &'a str) -> FontDesc<'a> {
        FontDesc::new(from.into(), 1.0)
    }
}

impl<'a> From<FontFamily<'a>> for FontDesc<'a> {
    fn from(family: FontFamily<'a>) -> FontDesc<'a> {
        FontDesc::new(family, 1.0)
    }
}

impl<'a, T: Into<f64>> From<(FontFamily<'a>, T)> for FontDesc<'a> {
    fn from((family, size): (FontFamily<'a>, T)) -> FontDesc<'a> {
        FontDesc::new(family, size.into())
    }
}

impl<'a, T: Into<f64>> From<(&'a str, T)> for FontDesc<'a> {
    fn from((typeface, size): (&'a str, T)) -> FontDesc<'a> {
        FontDesc::new(typeface.into(), size.into())
    }
}

pub trait IntoFont<'a> {
    fn into_font(self) -> FontDesc<'a>;
}

impl<'a, T: Into<FontDesc<'a>>> IntoFont<'a> for T {
    fn into_font(self) -> FontDesc<'a> {
        self.into()
    }
}

impl<'a> FontDesc<'a> {
    /// Create a new font
    pub fn new(family: FontFamily<'a>, size: f64) -> Self {
        Self {
            size,
            family,
            data: FontDataInternal::new(family),
            transform: FontTransform::None,
        }
    }

    /// Create a new font desc with the same font but different size
    pub fn resize(&self, size: f64) -> FontDesc<'a> {
        Self {
            size,
            family: self.family,
            data: self.data.clone(),
            transform: self.transform.clone(),
        }
    }

    /// Set the font transformation
    pub fn transform(&self, trans: FontTransform) -> Self {
        Self {
            size: self.size,
            family: self.family,
            data: self.data.clone(),
            transform: trans,
        }
    }

    /// Get the font transformation description
    pub fn get_transform(&self) -> FontTransform {
        self.transform.clone()
    }

    /// Set the color of the font and return the result text style object
    pub fn color<C: Color>(&self, color: &C) -> TextStyle<'a> {
        TextStyle {
            font: self.clone(),
            color: color.to_rgba(),
        }
    }

    /// Get the name of the font
    pub fn get_name(&self) -> &str {
        self.family.as_str()
    }

    /// Get the size of font
    pub fn get_size(&self) -> f64 {
        self.size
    }

    /// Get the size of the text if rendered in this font
    pub fn layout_box(&self, text: &str) -> FontResult<((i32, i32), (i32, i32))> {
        match &self.data {
            Ok(ref font) => font.estimate_layout(self.size, text),
            Err(e) => Err(e.clone()),
        }
    }

    /// Get the size of the text if rendered in this font.
    /// This is similar to `layout_box` function, but it apply the font transformation
    /// and estimate the overall size of the font
    pub fn box_size(&self, text: &str) -> FontResult<(u32, u32)> {
        let ((min_x, min_y), (max_x, max_y)) = self.layout_box(text)?;
        let (w, h) = self.get_transform().transform(max_x - min_x, max_y - min_y);
        Ok((w.abs() as u32, h.abs() as u32))
    }

    /// Actually draws a font with a drawing function
    pub fn draw<E, DrawFunc: FnMut(i32, i32, f32) -> Result<(), E>>(
        &self,
        text: &str,
        (x, y): (i32, i32),
        draw: DrawFunc,
    ) -> FontResult<Result<(), E>> {
        match &self.data {
            Ok(ref font) => font.draw((x, y), self.size, text, self.get_transform(), draw),
            Err(e) => Err(e.clone()),
        }
    }
}
