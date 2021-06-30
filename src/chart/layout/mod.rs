use std::ops::Range;

use crate::coord::ticks::Tick;
use crate::coord::ticks::{
    suggest_tickmark_spacing_for_range, AxisTickEnumerator, SimpleLinearAxis, TickKind,
};
use crate::element::LineSegment;
use crate::style::colors;
use crate::style::Color;
use crate::{coord::Shift, style::IntoTextStyle};
use paste::paste;
use plotters_backend::text_anchor::HPos;
use plotters_backend::text_anchor::Pos;
use plotters_backend::text_anchor::VPos;

use crate::drawing::{DrawingArea, DrawingAreaErrorKind, LayoutError};
use crate::style::{FontTransform, IntoFont, TextStyle};

use plotters_backend::DrawingBackend;

mod nodes;
pub use nodes::Margin;
use nodes::*;

enum AxisSide {
    Left,
    Right,
    Top,
    Bottom,
}

/// Create the `get_<elm name>_extent` and `get_<elm name>_size` functions
macro_rules! impl_get_extent {
    ($name:ident) => {
        paste! {
            #[doc = "Get the extent (the bounding box) of the `" $name "` container."]
            pub fn [<get_ $name _extent>](
                &self,
            ) -> Result<Extent<i32>, DrawingAreaErrorKind<DB::ErrorType>> {
                let extent = self
                    .nodes
                    .[<get_ $name _extent>]()
                    .ok_or_else(|| LayoutError::ExtentsError)?;

                Ok(extent)
            }
            #[doc = "Get the size of the `" $name "` container."]
            #[doc = "  * **Returns**: An option containing a tuple `(width, height)`."]
            pub fn [<get_ $name _size>](&self) -> Option<(i32, i32)> {
                self.nodes.[<get_ $name _size>]()
            }
        }
    };
}

/// Create all the getters and setters associated with a label that is horizontally layed out
macro_rules! impl_label_horiz {
    ($name:ident) => {
        paste! {
            #[doc = "Recomputes and sets the size of the `" $name "` container."]
            #[doc = "To be called whenever the `" $name "` text/style changes."]
            fn [<recompute_ $name _size>](&mut self) -> Result<(), DrawingAreaErrorKind<DB::ErrorType>> {
                let (w, h) = match &self.$name.text.as_ref() {
                    Some(text) => self
                        .root_area
                        .estimate_text_size(text, &self.$name.style)?,
                    None => (0, 0),
                };
                self.nodes.[<set_ $name _size>](w, h)?;

                Ok(())
            }
        }
        impl_label!($name);
    }
}
/// Create all the getters and setters associated with a label that is horizontally layed out
macro_rules! impl_label_vert {
    ($name:ident) => {
        paste! {
            #[doc = "Recomputes and sets the size of the `" $name "` container."]
            #[doc = "To be called whenever the `" $name "` text/style changes."]
            fn [<recompute_ $name _size>](&mut self) -> Result<(), DrawingAreaErrorKind<DB::ErrorType>> {
                let (w, h) = match &self.$name.text.as_ref() {
                    Some(text) => self
                        .root_area
                        .estimate_text_size(text, &self.$name.style)?,
                    None => (0, 0),
                };
                // Because this is a label in a vertically layed out label, we swap the width and height
                self.nodes.[<set_ $name _size>](h, w)?;

                Ok(())
            }
        }
        impl_label!($name);
    }
}

/// Create all the getters and setters associated with a labe that don't depend on the label's layout direction
macro_rules! impl_label {
    ($name:ident) => {
        paste! {
            #[doc = "Set the text content of `" $name "`. If `text` is the empty string,"]
            #[doc = "the label will be cleared."]
            pub fn [<set_ $name _text>]<S: AsRef<str>>(
                &mut self,
                text: S,
            ) -> Result<&mut Self, DrawingAreaErrorKind<DB::ErrorType>> {
                Self::set_text(&mut self.$name, text);
                self.[<recompute_ $name _size>]()?;
                Ok(self)
            }
            #[doc = "Clears the text content of the `" $name "` label."]
            pub fn [<clear_ $name _text>](
                &mut self,
            ) -> Result<&mut Self, DrawingAreaErrorKind<DB::ErrorType>> {
                self.[<set_ $name _text>]("")?;
                Ok(self)
            }
            #[doc = "Set the style of the `" $name "` label's text."]
            pub fn [<set_ $name _style>]<Style: IntoTextStyle<'b>>(
                &mut self,
                style: Style,
            ) -> Result<&mut Self, DrawingAreaErrorKind<DB::ErrorType>> {
                Self::set_style(self.root_area, &mut self.$name, style);
                self.[<recompute_ $name _size>]()?;
                Ok(self)
            }
            #[doc = "Set the margin of the `" $name "` container. If `margin` is a single"]
            #[doc = "number, that number is used for all margins. If `margin` is a tuple `(vert,horiz)`,"]
            #[doc = "then the top and bottom margins will be `vert` and the left and right margins will"]
            #[doc = "be `horiz`. To set each margin separately, use a [`Margin`] struct or a four-tuple."]
            pub fn [<set_ $name _margin>]<M: Into<Margin<f32>>>(
                &mut self,
                margin: M,
            ) -> Result<&mut Self, DrawingAreaErrorKind<DB::ErrorType>> {
                self.nodes.[<set_ $name _margin>](margin)?;
                Ok(self)
            }
            #[doc = "Gets the margin of the `" $name "` container."]
            pub fn [<get_ $name _margin>](
                self,
            ) -> Result<Margin<f32>, DrawingAreaErrorKind<DB::ErrorType>> {
                Ok(self.nodes.[<get_ $name _margin>]()?)
            }


        }
    };
}

/// Hold the text and the style of a chart label
struct Label<'a> {
    text: Option<String>,
    style: TextStyle<'a>,
}

/// Stores the range of the tick labels for every
/// side of the chart area.
#[derive(Clone)]
struct AxisSpecs<T> {
    left: Option<T>,
    right: Option<T>,
    top: Option<T>,
    bottom: Option<T>,
}
impl<T> AxisSpecs<T> {
    pub fn new_blank() -> Self {
        AxisSpecs {
            left: None,
            right: None,
            top: None,
            bottom: None,
        }
    }
}

/// The helper object to create a chart context, which is used for the high-level figure drawing.
/// With the help of this object, we can convert a basic drawing area into a chart context, which
/// allows the high-level charting API being used on the drawing area.
pub struct ChartLayout<'a, 'b, DB: DrawingBackend> {
    root_area: &'a DrawingArea<DB, Shift>,
    chart_title: Label<'b>,
    top_label: Label<'b>,
    bottom_label: Label<'b>,
    left_label: Label<'b>,
    right_label: Label<'b>,
    nodes: ChartLayoutNodes,
    axis_ranges: AxisSpecs<Range<f32>>,
}

impl<'a, 'b, DB: DrawingBackend> ChartLayout<'a, 'b, DB> {
    /// Create a chart builder on the given drawing area
    ///   - `root`: The root drawing area
    ///   - Returns: The chart layout object
    pub fn new(root: &'a DrawingArea<DB, Shift>) -> Self {
        let (w, h) = root.dim_in_pixel();
        let min_dim = w.min(h) as f32;
        let title_text_size = (min_dim / 10.).clamp(10., 100.);
        let label_text_size = (min_dim / 16.).clamp(10., 100.);

        Self {
            root_area: root,
            chart_title: Label {
                text: None,
                style: ("serif", title_text_size).into(),
            },
            top_label: Label {
                text: None,
                style: ("serif", label_text_size).into(),
            },
            bottom_label: Label {
                text: None,
                style: ("serif", label_text_size).into(),
            },
            left_label: Label {
                text: None,
                style: ("serif", label_text_size).into(),
            },
            right_label: Label {
                text: None,
                style: ("serif", label_text_size).into(),
            },
            nodes: ChartLayoutNodes::new().unwrap(),
            axis_ranges: AxisSpecs::new_blank(),
        }
    }

    pub fn draw(&mut self) -> Result<&mut Self, DrawingAreaErrorKind<DB::ErrorType>> {
        let (x_range, y_range) = self.root_area.get_pixel_range();
        let (x_top, y_top) = (x_range.start, y_range.start);
        let (w, h) = (x_range.end - x_top, y_range.end - y_top);
        self.nodes.layout(w as u32, h as u32)?;

        let label_style = TextStyle::from(("sans", 16.0).into_font());
        let label_formatter = |label: f32| format!("{:1.}", label);
        self.layout_axes(w as u32, h as u32, &label_style, &label_formatter)?;

        self.draw_ticks_helper(|pixel_coords, tick, axis_side| {
            draw_tick(
                pixel_coords,
                axis_side,
                tick.kind,
                label_formatter(tick.label),
                &label_style,
                &self.root_area,
            )?;

            Ok(())
        })?;
        // Draw the chart border for each set of labels we have
        let chart_area_extent = self
            .nodes
            .get_chart_area_extent()
            .ok_or_else(|| LayoutError::ExtentsError)?;
        let (x0, y0, x1, y1) = (
            chart_area_extent.x0,
            chart_area_extent.y0,
            chart_area_extent.x1,
            chart_area_extent.y1,
        );
        let axis_shape_style = Color::stroke_width(&colors::BLACK, 1);
        if self.axis_ranges.left.is_some() {
            self.root_area
                .draw(&LineSegment::new([(x0, y0), (x0, y1)], &axis_shape_style))?;
        }
        if self.axis_ranges.right.is_some() {
            self.root_area
                .draw(&LineSegment::new([(x1, y0), (x1, y1)], &axis_shape_style))?;
        }
        if self.axis_ranges.top.is_some() {
            self.root_area
                .draw(&LineSegment::new([(x0, y0), (x1, y0)], &axis_shape_style))?;
        }
        if self.axis_ranges.bottom.is_some() {
            self.root_area
                .draw(&LineSegment::new([(x0, y1), (x1, y1)], &axis_shape_style))?;
        }

        // Draw the horizontally oriented labels
        if let Some(text) = self.chart_title.text.as_ref() {
            let extent = self
                .nodes
                .get_chart_title_extent()
                .ok_or_else(|| LayoutError::ExtentsError)?;
            self.root_area
                .draw_text(text, &self.chart_title.style, (extent.x0, extent.y0))?;
        }

        if let Some(text) = self.top_label.text.as_ref() {
            let extent = self
                .nodes
                .get_top_label_extent()
                .ok_or_else(|| LayoutError::ExtentsError)?;
            self.root_area
                .draw_text(text, &self.top_label.style, (extent.x0, extent.y0))?;
        }
        if let Some(text) = self.bottom_label.text.as_ref() {
            let extent = self
                .nodes
                .get_bottom_label_extent()
                .ok_or_else(|| LayoutError::ExtentsError)?;
            self.root_area
                .draw_text(text, &self.bottom_label.style, (extent.x0, extent.y0))?;
        }
        // Draw the vertically oriented labels
        if let Some(text) = self.left_label.text.as_ref() {
            let extent = self
                .nodes
                .get_left_label_extent()
                .ok_or_else(|| LayoutError::ExtentsError)?;
            self.root_area.draw_text(
                text,
                &self.left_label.style.transform(FontTransform::Rotate270),
                (extent.x0, extent.y1),
            )?;
        }
        if let Some(text) = self.right_label.text.as_ref() {
            let extent = self
                .nodes
                .get_right_label_extent()
                .ok_or_else(|| LayoutError::ExtentsError)?;
            self.root_area.draw_text(
                text,
                &self.right_label.style.transform(FontTransform::Rotate270),
                (extent.x0, extent.y1),
            )?;
        }

        Ok(self)
    }

    /// Decide how much space each of the axes will take up and allocate that space.
    /// This function assumes `self.nodes.layout()` has already been called once.
    fn layout_axes<F>(
        &mut self,
        canvas_w: u32,
        canvas_h: u32,
        label_style: &TextStyle,
        label_formatter: F,
    ) -> Result<(), DrawingAreaErrorKind<DB::ErrorType>>
    where
        F: Fn(f32) -> String,
    {
        // After the initial layout, we compute how much space each
        // axis should take. We estimate the size of the left/right axes first,
        // because their labels are more impactful to the overall layout.
        let mut left_tick_labels_extent = self
            .nodes
            .get_left_tick_label_extent()
            .ok_or_else(|| LayoutError::ExtentsError)?;
        let mut right_tick_labels_extent = self
            .nodes
            .get_right_tick_label_extent()
            .ok_or_else(|| LayoutError::ExtentsError)?;
        self.draw_ticks_helper(|pixel_coords, tick, axis_side| {
            match axis_side {
                AxisSide::Left => {
                    // Expand the extent to contain the drawn tickmark
                    let tick_extent = compute_tick_extent(
                        axis_side,
                        tick.kind,
                        label_formatter(tick.label),
                        &label_style,
                        &self.root_area,
                    )
                    .ok_or_else(|| LayoutError::ExtentsError)?
                    .translate(pixel_coords);

                    left_tick_labels_extent.union_mut(&tick_extent);
                }
                AxisSide::Right => {
                    // Expand the extent to contain the drawn tickmark
                    let tick_extent = compute_tick_extent(
                        axis_side,
                        tick.kind,
                        label_formatter(tick.label),
                        &label_style,
                        &self.root_area,
                    )
                    .ok_or_else(|| LayoutError::ExtentsError)?
                    .translate(pixel_coords);

                    right_tick_labels_extent.union_mut(&tick_extent);
                }
                _ => {}
            }

            Ok(())
        })?;
        let (axis_w, _axis_h) = left_tick_labels_extent.size();
        self.nodes.set_left_tick_label_size(axis_w as u32, 0)?;
        let (axis_w, _axis_h) = right_tick_labels_extent.size();
        self.nodes.set_right_tick_label_size(axis_w as u32, 0)?;

        // Now the the left/right tick label sizes have been computed, we can compute
        // the top/bottom tick label sizes, taking into account the left/right
        self.nodes.layout(canvas_w as u32, canvas_h as u32)?;
        let mut top_tick_labels_extent = self
            .nodes
            .get_top_tick_label_extent()
            .ok_or_else(|| LayoutError::ExtentsError)?;
        let mut bottom_tick_labels_extent = self
            .nodes
            .get_bottom_tick_label_extent()
            .ok_or_else(|| LayoutError::ExtentsError)?;
        self.draw_ticks_helper(|pixel_coords, tick, axis_side| {
            match axis_side {
                AxisSide::Top => {
                    // Expand the extent to contain the drawn tickmark
                    let tick_extent = compute_tick_extent(
                        axis_side,
                        tick.kind,
                        label_formatter(tick.label),
                        &label_style,
                        &self.root_area,
                    )
                    .ok_or_else(|| LayoutError::ExtentsError)?
                    .translate(pixel_coords);

                    top_tick_labels_extent.union_mut(&tick_extent);
                }
                AxisSide::Bottom => {
                    // Expand the extent to contain the drawn tickmark
                    let tick_extent = compute_tick_extent(
                        axis_side,
                        tick.kind,
                        label_formatter(tick.label),
                        &label_style,
                        &self.root_area,
                    )
                    .ok_or_else(|| LayoutError::ExtentsError)?
                    .translate(pixel_coords);

                    bottom_tick_labels_extent.union_mut(&tick_extent);
                }
                _ => {}
            }

            Ok(())
        })?;
        let (_axis_w, axis_h) = top_tick_labels_extent.size();
        self.nodes.set_top_tick_label_size(0, axis_h as u32)?;
        let (_axis_w, axis_h) = bottom_tick_labels_extent.size();
        self.nodes.set_bottom_tick_label_size(0, axis_h as u32)?;

        // Now that the spacing has been computed, re-layout the axes and actually draw them.
        self.nodes.layout(canvas_w as u32, canvas_h as u32)?;

        // It may be the case that parts of the label text "spill over" into the margin
        // to the left/right of the chart_area. We want to make sure that we're
        // not spilling over off the drawing_area.
        let left_spill = top_tick_labels_extent.x0.min(bottom_tick_labels_extent.x0);
        if left_spill < 0 {
            let (w, h) = self
                .nodes
                .get_left_tick_label_size()
                .ok_or_else(|| LayoutError::ExtentsError)?;

            self.nodes
                .set_left_tick_label_size((w - left_spill) as u32, h as u32)?;
        }
        let right_spill =
            canvas_w as i32 - top_tick_labels_extent.x1.max(bottom_tick_labels_extent.x1);
        if right_spill < 0 {
            let (w, h) = self
                .nodes
                .get_right_tick_label_size()
                .ok_or_else(|| LayoutError::ExtentsError)?;

            self.nodes
                .set_right_tick_label_size((w - right_spill) as u32, h as u32)?;
        }

        // Layouts are cached, so if we didn't change anything, this is a very cheap function call
        self.nodes.layout(canvas_w as u32, canvas_h as u32)?;

        // It may be the case that parts of the label text "spill over" into the margin
        // to the top/bottom of the chart_area. We want to make sure that we're
        // not spilling over off the drawing_area.
        let top_spill = left_tick_labels_extent.y0.min(right_tick_labels_extent.y0);
        if top_spill < 0 {
            let (w, h) = self
                .nodes
                .get_top_tick_label_size()
                .ok_or_else(|| LayoutError::ExtentsError)?;

            // When the left/right tick label extents were computed, the bottom/top tick labels
            // had zero size. We only want to increase their size if needed. Otherwise, we should
            // leave them the size they are.
            self.nodes
                .set_top_tick_label_size(w as u32, h.max(-top_spill) as u32)?;
        }
        let bottom_spill =
            canvas_h as i32 - left_tick_labels_extent.y1.max(right_tick_labels_extent.y1);
        if bottom_spill < 0 {
            let (w, h) = self
                .nodes
                .get_bottom_tick_label_size()
                .ok_or_else(|| LayoutError::ExtentsError)?;

            // When the left/right tick label extents were computed, the bottom/top tick labels
            // had zero size. We only want to increase their size if needed. Otherwise, we should
            // leave them the size they are.
            self.nodes
                .set_bottom_tick_label_size(w as u32, h.max(-bottom_spill) as u32)?;
        }

        // Layouts are cached, so if we didn't change anything, this is a very cheap function call
        self.nodes.layout(canvas_w as u32, canvas_h as u32)?;

        Ok(())
    }

    /// Helper function for drawing ticks. This function will call
    /// `draw_func(pixel_coords, tick, axis_side)` for every tick on every axis.
    fn draw_ticks_helper<F>(
        &self,
        mut draw_func: F,
    ) -> Result<(), DrawingAreaErrorKind<DB::ErrorType>>
    where
        F: FnMut(
            (i32, i32),
            Tick<f32, f32>,
            AxisSide,
        ) -> Result<(), DrawingAreaErrorKind<DB::ErrorType>>,
    {
        let suggestion = self.suggest_tickmark_spacing_for_axes()?;
        if let Some(axis) = suggestion.left {
            let extent = self
                .nodes
                .get_chart_area_extent()
                .ok_or_else(|| LayoutError::ExtentsError)?;
            let start = self.axis_ranges.left.as_ref().unwrap().start;
            let end = self.axis_ranges.left.as_ref().unwrap().end;
            for tick in axis.iter() {
                // Find out where the tick is to be drawn.
                let y_pos = scale_to_pixel(tick.pos, start, end, extent.y0, extent.y1);
                draw_func((extent.x0, y_pos), tick, AxisSide::Left)?;
            }
        }
        if let Some(axis) = suggestion.right {
            let extent = self
                .nodes
                .get_chart_area_extent()
                .ok_or_else(|| LayoutError::ExtentsError)?;
            let start = self.axis_ranges.right.as_ref().unwrap().start;
            let end = self.axis_ranges.right.as_ref().unwrap().end;
            for tick in axis.iter() {
                // Find out where the tick is to be drawn.
                let y_pos = scale_to_pixel(tick.pos, start, end, extent.y0, extent.y1);
                draw_func((extent.x1, y_pos), tick, AxisSide::Right)?;
            }
        }
        if let Some(axis) = suggestion.top {
            let extent = self
                .nodes
                .get_chart_area_extent()
                .ok_or_else(|| LayoutError::ExtentsError)?;
            let start = self.axis_ranges.top.as_ref().unwrap().start;
            let end = self.axis_ranges.top.as_ref().unwrap().end;
            for tick in axis.iter() {
                // Find out where the tick is to be drawn.
                let x_pos = scale_to_pixel(tick.pos, start, end, extent.x0, extent.x1);
                draw_func((x_pos, extent.y0), tick, AxisSide::Top)?;
            }
        }
        if let Some(axis) = suggestion.bottom {
            let extent = self
                .nodes
                .get_chart_area_extent()
                .ok_or_else(|| LayoutError::ExtentsError)?;
            let start = self.axis_ranges.bottom.as_ref().unwrap().start;
            let end = self.axis_ranges.bottom.as_ref().unwrap().end;
            for tick in axis.iter() {
                // Find out where the tick is to be drawn.
                let x_pos = scale_to_pixel(tick.pos, start, end, extent.x0, extent.x1);
                draw_func((x_pos, extent.y1), tick, AxisSide::Bottom)?;
            }
        }

        Ok(())
    }

    /// Use some heuristics to guess the best tick spacing given the area we have.
    fn suggest_tickmark_spacing_for_axes(
        &self,
    ) -> Result<AxisSpecs<SimpleLinearAxis<f32>>, DrawingAreaErrorKind<DB::ErrorType>> {
        let da_extent = self.get_chart_area_extent()?;
        let (w, h) = da_extent.size();
        let ret = AxisSpecs {
            top: self
                .axis_ranges
                .top
                .as_ref()
                .map(|range| suggest_tickmark_spacing_for_range(range, w)),
            bottom: self
                .axis_ranges
                .bottom
                .as_ref()
                .map(|range| suggest_tickmark_spacing_for_range(range, w)),
            left: self
                .axis_ranges
                .left
                .as_ref()
                .map(|range| suggest_tickmark_spacing_for_range(range, h)),
            right: self
                .axis_ranges
                .right
                .as_ref()
                .map(|range| suggest_tickmark_spacing_for_range(range, h)),
        };
        Ok(ret)
    }

    /// Apply a cartesian grid to the chart area. Major/minor ticks are automatically
    /// determined on a call to `draw`.
    pub fn build_cartesian_2d(
        &mut self,
        x_spec: Range<f32>,
        y_spec: Range<f32>,
    ) -> Result<&mut Self, DrawingAreaErrorKind<DB::ErrorType>> {
        self.axis_ranges.left = Some(y_spec.clone());
        self.axis_ranges.bottom = Some(x_spec.clone());

        Ok(self)
    }

    /// Return a drawing area which corresponds to the `chart_area` of the current layout.
    /// [`layout`] should be called before this function.
    pub fn get_chart_drawing_area(
        &mut self,
    ) -> Result<DrawingArea<DB, Shift>, DrawingAreaErrorKind<DB::ErrorType>> {
        let chart_area_extent = self.get_chart_area_extent()?;
        Ok(DrawingArea::clone(self.root_area).shrink(
            (chart_area_extent.x0, chart_area_extent.y0),
            chart_area_extent.size(),
        ))
    }

    /// Set the text of a label. If the text is a blank screen, the label is cleared.
    #[inline(always)]
    fn set_text<S: AsRef<str>>(elm: &mut Label, text: S) {
        let text = text.as_ref().to_string();
        elm.text = match text.is_empty() {
            false => Some(text),
            true => None,
        };
    }
    /// Set the style of a label.
    #[inline(always)]
    fn set_style<Style: IntoTextStyle<'b>>(
        root: &'a DrawingArea<DB, Shift>,
        elm: &mut Label<'b>,
        style: Style,
    ) {
        elm.style = style.into_text_style(root);
    }

    impl_get_extent!(top_label);
    impl_get_extent!(bottom_label);
    impl_get_extent!(left_label);
    impl_get_extent!(right_label);
    impl_get_extent!(top_tick_label);
    impl_get_extent!(bottom_tick_label);
    impl_get_extent!(left_tick_label);
    impl_get_extent!(right_tick_label);
    impl_get_extent!(chart_area);
    impl_get_extent!(chart_title);

    impl_label_horiz!(chart_title);
    impl_label_horiz!(bottom_label);
    impl_label_horiz!(top_label);
    impl_label_vert!(left_label);
    impl_label_vert!(right_label);
}

/// Scale `val` which is in an interval between `a` and `b` to be within an interval between `pixel_a` and `pixel_b`
fn scale_to_pixel(val: f32, a: f32, b: f32, pixel_a: i32, pixel_b: i32) -> i32 {
    ((val - a) / (b - a) * (pixel_b - pixel_a) as f32) as i32 + pixel_a
}
const MAJOR_TICK_LEN: i32 = 5;
const MINOR_TICK_LEN: i32 = 3;
const TICK_LABEL_PADDING: i32 = 3;

/// Compute the extents of the given tick kind/label. The extents are computed
/// as if the tick were drawn at (0,0). It should be translated for other uses.
fn compute_tick_extent<DB: DrawingBackend, S: AsRef<str>>(
    axis_side: AxisSide,
    tick_kind: TickKind,
    label: S,
    label_style: &TextStyle,
    drawing_area: &DrawingArea<DB, Shift>,
) -> Option<Extent<i32>> {
    let mut extent = Extent::new_with_size(0, 0);
    match tick_kind {
        // For a major tickmark we extend the extent by the tick itself and the area the tick label takes up
        TickKind::Major => {
            if let Ok((w, h)) = drawing_area.estimate_text_size(label.as_ref(), label_style) {
                match axis_side {
                    AxisSide::Left => {
                        extent.union_mut(&Extent::new_with_size(-MAJOR_TICK_LEN, 0));
                        extent.union_mut(
                            &Extent::new_with_size(-(w as i32), h as i32 + 2 * TICK_LABEL_PADDING)
                                .translate((
                                    -MAJOR_TICK_LEN - 2 * TICK_LABEL_PADDING,
                                    -((h as f32) / 2.0) as i32 - TICK_LABEL_PADDING,
                                )),
                        );
                    }
                    AxisSide::Right => {
                        extent.union_mut(&Extent::new_with_size(MAJOR_TICK_LEN, 0));
                        extent.union_mut(
                            &Extent::new_with_size(w as i32, h as i32 + 2 * TICK_LABEL_PADDING)
                                .translate((
                                    MAJOR_TICK_LEN + 2 * TICK_LABEL_PADDING,
                                    -((h as f32) / 2.0) as i32 - TICK_LABEL_PADDING,
                                )),
                        );
                    }
                    AxisSide::Top => {
                        extent.union_mut(&Extent::new_with_size(0, -MAJOR_TICK_LEN));
                        extent.union_mut(
                            &Extent::new_with_size(w as i32 + 2 * TICK_LABEL_PADDING, -(h as i32))
                                .translate((
                                    -((w as f32) / 2.0) as i32 - TICK_LABEL_PADDING,
                                    -MAJOR_TICK_LEN - 2 * TICK_LABEL_PADDING,
                                )),
                        );
                    }
                    AxisSide::Bottom => {
                        extent.union_mut(&Extent::new_with_size(0, MAJOR_TICK_LEN));
                        extent.union_mut(
                            &Extent::new_with_size(w as i32 + 2 * TICK_LABEL_PADDING, h as i32)
                                .translate((
                                    -((w as f32) / 2.0) as i32 - TICK_LABEL_PADDING,
                                    MAJOR_TICK_LEN + 2 * TICK_LABEL_PADDING,
                                )),
                        );
                    }
                }
                Some(extent)
            } else {
                None
            }
        }
        TickKind::Minor => {
            match axis_side {
                AxisSide::Left => {
                    extent.union_mut(&Extent::new_with_size(-MINOR_TICK_LEN, 0));
                }
                AxisSide::Right => {
                    extent.union_mut(&Extent::new_with_size(MINOR_TICK_LEN, 0));
                }
                AxisSide::Top => {
                    extent.union_mut(&Extent::new_with_size(0, -MINOR_TICK_LEN));
                }
                AxisSide::Bottom => {
                    extent.union_mut(&Extent::new_with_size(0, MINOR_TICK_LEN));
                }
            }
            Some(extent)
        }
    }
}

/// Draw the tick at the correct location.
fn draw_tick<DB: DrawingBackend, S: AsRef<str>>(
    pixel_coords: (i32, i32),
    axis_side: AxisSide,
    tick_kind: TickKind,
    label_text: S,
    label_style: &TextStyle,
    drawing_area: &DrawingArea<DB, Shift>,
) -> Result<(), DrawingAreaErrorKind<DB::ErrorType>> {
    let tick_len = match tick_kind {
        TickKind::Major => MAJOR_TICK_LEN,
        TickKind::Minor => MINOR_TICK_LEN,
    };
    match axis_side {
        AxisSide::Left => {
            drawing_area.draw(&LineSegment::new(
                [
                    (pixel_coords.0 - tick_len, pixel_coords.1),
                    (pixel_coords.0, pixel_coords.1),
                ],
                &colors::BLACK.into(),
            ))?;
            // On a major tick, we draw a label
            if tick_kind == TickKind::Major {
                drawing_area.draw_text(
                    label_text.as_ref(),
                    &label_style.pos(Pos::new(HPos::Right, VPos::Center)),
                    (
                        pixel_coords.0 - MAJOR_TICK_LEN - TICK_LABEL_PADDING,
                        pixel_coords.1,
                    ),
                )?;
            }
        }
        AxisSide::Right => {
            drawing_area.draw(&LineSegment::new(
                [
                    (pixel_coords.0, pixel_coords.1),
                    (pixel_coords.0 + tick_len, pixel_coords.1),
                ],
                &colors::BLACK.into(),
            ))?;
            // On a major tick, we draw a label
            if tick_kind == TickKind::Major {
                drawing_area.draw_text(
                    label_text.as_ref(),
                    &label_style.pos(Pos::new(HPos::Left, VPos::Center)),
                    (
                        pixel_coords.0 + MAJOR_TICK_LEN + TICK_LABEL_PADDING,
                        pixel_coords.1,
                    ),
                )?;
            }
        }
        AxisSide::Top => {
            drawing_area.draw(&LineSegment::new(
                [
                    (pixel_coords.0, pixel_coords.1 - tick_len),
                    (pixel_coords.0, pixel_coords.1),
                ],
                &colors::BLACK.into(),
            ))?;
            // On a major tick, we draw a label
            if tick_kind == TickKind::Major {
                drawing_area.draw_text(
                    label_text.as_ref(),
                    &label_style.pos(Pos::new(HPos::Center, VPos::Bottom)),
                    (
                        pixel_coords.0,
                        pixel_coords.1 - MAJOR_TICK_LEN - TICK_LABEL_PADDING,
                    ),
                )?;
            }
        }
        AxisSide::Bottom => {
            drawing_area.draw(&LineSegment::new(
                [
                    (pixel_coords.0, pixel_coords.1 + tick_len),
                    (pixel_coords.0, pixel_coords.1),
                ],
                &colors::BLACK.into(),
            ))?;
            // On a major tick, we draw a label
            if tick_kind == TickKind::Major {
                drawing_area.draw_text(
                    label_text.as_ref(),
                    &label_style.pos(Pos::new(HPos::Center, VPos::Top)),
                    (
                        pixel_coords.0,
                        pixel_coords.1 + MAJOR_TICK_LEN + TICK_LABEL_PADDING,
                    ),
                )?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::*;

    fn extent_has_size<T: PartialOrd + Copy + std::ops::Sub + std::ops::Add>(
        extent: Extent<T>,
    ) -> bool {
        (extent.x1 > extent.x0) && (extent.y1 > extent.y0)
    }

    #[test]
    fn test_drawing_of_unset_and_set_chart_title() {
        let drawing_area = create_mocked_drawing_area(200, 200, |_| {});
        let mut chart = ChartLayout::new(&drawing_area);
        chart.draw().unwrap();
        chart.set_chart_title_text("title").unwrap();
        chart.draw().unwrap();

        // Since we set actual text, the extent should have some area
        let extent = chart.get_chart_title_extent().unwrap();
        assert!(extent_has_size(extent));

        // Without any text, the extent shouldn't have any area.
        chart.clear_chart_title_text().unwrap();
        chart.draw().unwrap();
        let extent = chart.get_chart_title_extent().unwrap();
        assert!(!extent_has_size(extent));
    }

    #[test]
    fn test_drawing_of_unset_and_set_labels() {
        let drawing_area = create_mocked_drawing_area(200, 200, |_| {});
        let mut chart = ChartLayout::new(&drawing_area);

        // top_label
        chart.draw().unwrap();
        chart.set_top_label_text("title").unwrap();
        chart.draw().unwrap();

        let extent = chart.get_top_label_extent().unwrap();
        assert!(extent_has_size(extent));

        chart.clear_top_label_text().unwrap();
        chart.draw().unwrap();
        let extent = chart.get_top_label_extent().unwrap();
        assert!(!extent_has_size(extent));

        // bottom_label
        chart.draw().unwrap();
        chart.set_bottom_label_text("title").unwrap();
        chart.draw().unwrap();

        let extent = chart.get_bottom_label_extent().unwrap();
        assert!(extent_has_size(extent));

        chart.clear_bottom_label_text().unwrap();
        chart.draw().unwrap();
        let extent = chart.get_bottom_label_extent().unwrap();
        assert!(!extent_has_size(extent));

        // left_label
        chart.draw().unwrap();
        chart.set_left_label_text("title").unwrap();
        chart.draw().unwrap();

        let extent = chart.get_left_label_extent().unwrap();
        assert!(extent_has_size(extent));

        chart.clear_left_label_text().unwrap();
        chart.draw().unwrap();
        let extent = chart.get_left_label_extent().unwrap();
        assert!(!extent_has_size(extent));

        // right_label
        chart.draw().unwrap();
        chart.set_right_label_text("title").unwrap();
        chart.draw().unwrap();

        let extent = chart.get_right_label_extent().unwrap();
        assert!(extent_has_size(extent));

        chart.clear_right_label_text().unwrap();
        chart.draw().unwrap();
        let extent = chart.get_right_label_extent().unwrap();
        assert!(!extent_has_size(extent));
    }

    #[test]
    fn test_layout_of_horizontal_and_vertical_labels() {
        let drawing_area = create_mocked_drawing_area(800, 600, |_| {});
        let mut chart = ChartLayout::new(&drawing_area);

        // top_label is horizontal
        chart.set_top_label_text("some really long text").unwrap();
        chart.draw().unwrap();

        let extent = chart.get_top_label_extent().unwrap();
        let size = extent.size();
        assert!(size.0 > size.1);
        // left_label is vertically
        chart.set_left_label_text("some really long text").unwrap();
        chart.draw().unwrap();

        let extent = chart.get_left_label_extent().unwrap();
        let size = extent.size();
        assert!(size.1 > size.0);
    }

    #[test]
    fn test_adding_axes_should_take_up_room() {
        let drawing_area = create_mocked_drawing_area(800, 600, |_| {});
        let mut chart = ChartLayout::new(&drawing_area);
        chart
            .build_cartesian_2d(0.0f32..100.0, 0.0f32..100.0f32)
            .unwrap();
        chart.draw().unwrap();

        let extent = chart.get_bottom_tick_label_extent().unwrap();
        assert!(extent_has_size(extent));

        let extent = chart.get_left_tick_label_extent().unwrap();
        assert!(extent_has_size(extent));

        let extent = chart.get_chart_area_extent().unwrap();
        assert!(extent.x0 > 0);
        assert!(extent.y1 < 600);
    }
}
