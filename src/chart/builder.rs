use super::context::ChartContext;

use crate::coord::cartesian::{Cartesian2d, Cartesian3d};
use crate::coord::ranged1d::AsRangedCoord;
use crate::coord::Shift;

use crate::drawing::{DrawingArea, DrawingAreaErrorKind};
use crate::style::{IntoTextStyle, SizeDesc, TextStyle};

use plotters_backend::DrawingBackend;

/// The enum used to specify the position of label area.
/// This is used when we configure the label area size with the API
/// [ChartBuilder::set_label_area_size](struct ChartBuilder.html#method.set_label_area_size)
#[derive(Copy, Clone)]
pub enum LabelAreaPosition {
    Top = 0,
    Bottom = 1,
    Left = 2,
    Right = 3,
}

/// The helper object to create a chart context, which is used for the high-level figure drawing.
/// With the help of this object, we can convert a basic drawing area into a chart context, which
/// allows the high-level charting API being used on the drawing area.
pub struct ChartBuilder<'a, 'b, DB: DrawingBackend> {
    label_area_size: [u32; 4], // [upper, lower, left, right]
    overlap_plotting_area: [bool; 4],
    root_area: &'a DrawingArea<DB, Shift>,
    title: Option<(String, TextStyle<'b>)>,
    margin: [u32; 4],
}

impl<'a, 'b, DB: DrawingBackend> ChartBuilder<'a, 'b, DB> {
    /// Create a chart builder on the given drawing area
    /// - `root`: The root drawing area
    /// - Returns: The chart builder object
    pub fn on(root: &'a DrawingArea<DB, Shift>) -> Self {
        Self {
            label_area_size: [0; 4],
            root_area: root,
            title: None,
            margin: [0; 4],
            overlap_plotting_area: [false; 4],
        }
    }

    /// Set the margin size of the chart (applied for top, bottom, left and right at the same time)
    /// - `size`: The size of the chart margin.
    pub fn margin<S: SizeDesc>(&mut self, size: S) -> &mut Self {
        let size = size.in_pixels(self.root_area).max(0) as u32;
        self.margin = [size, size, size, size];
        self
    }

    /// Set the top margin of current chart
    /// - `size`: The size of the top margin.
    pub fn margin_top<S: SizeDesc>(&mut self, size: S) -> &mut Self {
        let size = size.in_pixels(self.root_area).max(0) as u32;
        self.margin[0] = size;
        self
    }

    /// Set the bottom margin of current chart
    /// - `size`: The size of the bottom margin.
    pub fn margin_bottom<S: SizeDesc>(&mut self, size: S) -> &mut Self {
        let size = size.in_pixels(self.root_area).max(0) as u32;
        self.margin[1] = size;
        self
    }

    /// Set the left margin of current chart
    /// - `size`: The size of the left margin.
    pub fn margin_left<S: SizeDesc>(&mut self, size: S) -> &mut Self {
        let size = size.in_pixels(self.root_area).max(0) as u32;
        self.margin[2] = size;
        self
    }

    /// Set the right margin of current chart
    /// - `size`: The size of the right margin.
    pub fn margin_right<S: SizeDesc>(&mut self, size: S) -> &mut Self {
        let size = size.in_pixels(self.root_area).max(0) as u32;
        self.margin[3] = size;
        self
    }

    /// Set all the label area size with the same value
    pub fn set_all_label_area_size<S: SizeDesc>(&mut self, size: S) -> &mut Self {
        let size = size.in_pixels(self.root_area);
        self.set_label_area_size(LabelAreaPosition::Top, size)
            .set_label_area_size(LabelAreaPosition::Bottom, size)
            .set_label_area_size(LabelAreaPosition::Left, size)
            .set_label_area_size(LabelAreaPosition::Right, size)
    }

    /// Set the most commonly used label area size to the same value
    pub fn set_left_and_bottom_label_area_size<S: SizeDesc>(&mut self, size: S) -> &mut Self {
        let size = size.in_pixels(self.root_area);
        self.set_label_area_size(LabelAreaPosition::Left, size)
            .set_label_area_size(LabelAreaPosition::Bottom, size)
    }

    /// Set the size of X label area
    /// - `size`: The height of the x label area, if x is 0, the chart doesn't have the X label area
    pub fn x_label_area_size<S: SizeDesc>(&mut self, size: S) -> &mut Self {
        self.set_label_area_size(LabelAreaPosition::Bottom, size)
    }

    /// Set the size of the Y label area
    /// - `size`: The width of the Y label area. If size is 0, the chart doesn't have Y label area
    pub fn y_label_area_size<S: SizeDesc>(&mut self, size: S) -> &mut Self {
        self.set_label_area_size(LabelAreaPosition::Left, size)
    }

    /// Set the size of X label area on the top of the chart
    /// - `size`: The height of the x label area, if x is 0, the chart doesn't have the X label area
    pub fn top_x_label_area_size<S: SizeDesc>(&mut self, size: S) -> &mut Self {
        self.set_label_area_size(LabelAreaPosition::Top, size)
    }

    /// Set the size of the Y label area on the right side
    /// - `size`: The width of the Y label area. If size is 0, the chart doesn't have Y label area
    pub fn right_y_label_area_size<S: SizeDesc>(&mut self, size: S) -> &mut Self {
        self.set_label_area_size(LabelAreaPosition::Right, size)
    }

    /// Set a label area size
    /// - `pos`: THe position where the label area located
    /// - `size`: The size of the label area size
    pub fn set_label_area_size<S: SizeDesc>(
        &mut self,
        pos: LabelAreaPosition,
        size: S,
    ) -> &mut Self {
        let size = size.in_pixels(self.root_area);
        self.label_area_size[pos as usize] = size.abs() as u32;
        self.overlap_plotting_area[pos as usize] = size < 0;
        self
    }

    /// Set the caption of the chart
    /// - `caption`: The caption of the chart
    /// - `style`: The text style
    /// - Note: If the caption is set, the margin option will be ignored
    pub fn caption<S: AsRef<str>, Style: IntoTextStyle<'b>>(
        &mut self,
        caption: S,
        style: Style,
    ) -> &mut Self {
        self.title = Some((
            caption.as_ref().to_string(),
            style.into_text_style(self.root_area),
        ));
        self
    }

    #[allow(clippy::type_complexity)]
    #[deprecated(
        note = "`build_ranged` has been renamed to `build_cartesian_2d` and is to be removed in the future."
    )]
    pub fn build_ranged<X: AsRangedCoord, Y: AsRangedCoord>(
        &mut self,
        x_spec: X,
        y_spec: Y,
    ) -> Result<
        ChartContext<'a, DB, Cartesian2d<X::CoordDescType, Y::CoordDescType>>,
        DrawingAreaErrorKind<DB::ErrorType>,
    > {
        self.build_cartesian_2d(x_spec, y_spec)
    }

    /// Build the chart with a 2D Cartesian coordinate system. The function will returns a chart
    /// context, where data series can be rendered on.
    /// - `x_spec`: The specification of X axis
    /// - `y_spec`: The specification of Y axis
    /// - Returns: A chart context
    #[allow(clippy::type_complexity)]
    pub fn build_cartesian_2d<X: AsRangedCoord, Y: AsRangedCoord>(
        &mut self,
        x_spec: X,
        y_spec: Y,
    ) -> Result<
        ChartContext<'a, DB, Cartesian2d<X::CoordDescType, Y::CoordDescType>>,
        DrawingAreaErrorKind<DB::ErrorType>,
    > {
        let mut label_areas = [None, None, None, None];

        let mut drawing_area = DrawingArea::clone(self.root_area);

        if *self.margin.iter().max().unwrap_or(&0) > 0 {
            drawing_area = drawing_area.margin(
                self.margin[0] as i32,
                self.margin[1] as i32,
                self.margin[2] as i32,
                self.margin[3] as i32,
            );
        }

        let (title_dx, title_dy) = if let Some((ref title, ref style)) = self.title {
            let (origin_dx, origin_dy) = drawing_area.get_base_pixel();
            drawing_area = drawing_area.titled(title, style.clone())?;
            let (current_dx, current_dy) = drawing_area.get_base_pixel();
            (current_dx - origin_dx, current_dy - origin_dy)
        } else {
            (0, 0)
        };

        let (w, h) = drawing_area.dim_in_pixel();

        let mut actual_drawing_area_pos = [0, h as i32, 0, w as i32];

        const DIR: [(i16, i16); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

        for (idx, (dx, dy)) in (0..4).map(|idx| (idx, DIR[idx])) {
            if self.overlap_plotting_area[idx] {
                continue;
            }

            let size = self.label_area_size[idx] as i32;

            let split_point = if dx + dy < 0 { size } else { -size };

            actual_drawing_area_pos[idx] += split_point;
        }

        // Now the root drawing area is to be split into
        //
        // +----------+------------------------------+------+
        // |    0     |    1 (Top Label Area)        |   2  |
        // +----------+------------------------------+------+
        // |    3     |                              |   5  |
        // |  Left    |       4 (Plotting Area)      | Right|
        // |  Labels  |                              | Label|
        // +----------+------------------------------+------+
        // |    6     |        7 (Bottom Labels)     |   8  |
        // +----------+------------------------------+------+

        let mut split: Vec<_> = drawing_area
            .split_by_breakpoints(
                &actual_drawing_area_pos[2..4],
                &actual_drawing_area_pos[0..2],
            )
            .into_iter()
            .map(Some)
            .collect();

        // Take out the plotting area
        std::mem::swap(&mut drawing_area, split[4].as_mut().unwrap());

        // Initialize the label areas - since the label area might be overlapping
        // with the plotting area, in this case, we need handle them differently
        for (src_idx, dst_idx) in [1, 7, 3, 5].iter().zip(0..4) {
            if !self.overlap_plotting_area[dst_idx] {
                let (h, w) = split[*src_idx].as_ref().unwrap().dim_in_pixel();
                if h > 0 && w > 0 {
                    std::mem::swap(&mut label_areas[dst_idx], &mut split[*src_idx]);
                }
            } else if self.label_area_size[dst_idx] != 0 {
                let size = self.label_area_size[dst_idx] as i32;
                let (dw, dh) = drawing_area.dim_in_pixel();
                let x0 = if DIR[dst_idx].0 > 0 {
                    dw as i32 - size
                } else {
                    0
                };
                let y0 = if DIR[dst_idx].1 > 0 {
                    dh as i32 - size
                } else {
                    0
                };
                let x1 = if DIR[dst_idx].0 >= 0 { dw as i32 } else { size };
                let y1 = if DIR[dst_idx].1 >= 0 { dh as i32 } else { size };

                label_areas[dst_idx] = Some(
                    drawing_area
                        .clone()
                        .shrink((x0, y0), ((x1 - x0), (y1 - y0))),
                );
            }
        }

        let mut pixel_range = drawing_area.get_pixel_range();
        pixel_range.1 = (pixel_range.1.end - 1)..(pixel_range.1.start - 1);

        let mut x_label_area = [None, None];
        let mut y_label_area = [None, None];

        std::mem::swap(&mut x_label_area[0], &mut label_areas[0]);
        std::mem::swap(&mut x_label_area[1], &mut label_areas[1]);
        std::mem::swap(&mut y_label_area[0], &mut label_areas[2]);
        std::mem::swap(&mut y_label_area[1], &mut label_areas[3]);

        Ok(ChartContext {
            x_label_area,
            y_label_area,
            drawing_area: drawing_area.apply_coord_spec(Cartesian2d::new(
                x_spec,
                y_spec,
                pixel_range,
            )),
            series_anno: vec![],
            drawing_area_pos: (
                actual_drawing_area_pos[2] + title_dx + self.margin[2] as i32,
                actual_drawing_area_pos[0] + title_dy + self.margin[0] as i32,
            ),
        })
    }

    /// Build a 3 dimensional cartesian chart. The function will returns a chart
    /// context, where data series can be rendered on.
    /// - `x_spec`: The specification of X axis
    /// - `y_spec`: The specification of Y axis
    /// - `z_spec`: The specification of Z axis
    /// - Returns: A chart context
    #[allow(clippy::type_complexity)]
    pub fn build_cartesian_3d<X: AsRangedCoord, Y: AsRangedCoord, Z: AsRangedCoord>(
        &mut self,
        x_spec: X,
        y_spec: Y,
        z_spec: Z,
    ) -> Result<
        ChartContext<'a, DB, Cartesian3d<X::CoordDescType, Y::CoordDescType, Z::CoordDescType>>,
        DrawingAreaErrorKind<DB::ErrorType>,
    > {
        let mut drawing_area = DrawingArea::clone(self.root_area);

        if *self.margin.iter().max().unwrap_or(&0) > 0 {
            drawing_area = drawing_area.margin(
                self.margin[0] as i32,
                self.margin[1] as i32,
                self.margin[2] as i32,
                self.margin[3] as i32,
            );
        }

        let (title_dx, title_dy) = if let Some((ref title, ref style)) = self.title {
            let (origin_dx, origin_dy) = drawing_area.get_base_pixel();
            drawing_area = drawing_area.titled(title, style.clone())?;
            let (current_dx, current_dy) = drawing_area.get_base_pixel();
            (current_dx - origin_dx, current_dy - origin_dy)
        } else {
            (0, 0)
        };

        let pixel_range = drawing_area.get_pixel_range();

        Ok(ChartContext {
            x_label_area: [None, None],
            y_label_area: [None, None],
            drawing_area: drawing_area.apply_coord_spec(Cartesian3d::new(
                x_spec,
                y_spec,
                z_spec,
                pixel_range,
            )),
            series_anno: vec![],
            drawing_area_pos: (
                title_dx + self.margin[2] as i32,
                title_dy + self.margin[0] as i32,
            ),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn test_label_area_size() {
        let drawing_area = create_mocked_drawing_area(200, 200, |_| {});
        let mut chart = ChartBuilder::on(&drawing_area);

        chart
            .x_label_area_size(10)
            .y_label_area_size(20)
            .top_x_label_area_size(30)
            .right_y_label_area_size(40);
        assert_eq!(chart.label_area_size[1], 10);
        assert_eq!(chart.label_area_size[2], 20);
        assert_eq!(chart.label_area_size[0], 30);
        assert_eq!(chart.label_area_size[3], 40);

        chart.set_label_area_size(LabelAreaPosition::Left, 100);
        chart.set_label_area_size(LabelAreaPosition::Right, 200);
        chart.set_label_area_size(LabelAreaPosition::Top, 300);
        chart.set_label_area_size(LabelAreaPosition::Bottom, 400);

        assert_eq!(chart.label_area_size[0], 300);
        assert_eq!(chart.label_area_size[1], 400);
        assert_eq!(chart.label_area_size[2], 100);
        assert_eq!(chart.label_area_size[3], 200);
    }

    #[test]
    fn test_margin_configure() {
        let drawing_area = create_mocked_drawing_area(200, 200, |_| {});
        let mut chart = ChartBuilder::on(&drawing_area);

        chart.margin(5);
        assert_eq!(chart.margin[0], 5);
        assert_eq!(chart.margin[1], 5);
        assert_eq!(chart.margin[2], 5);
        assert_eq!(chart.margin[3], 5);

        chart.margin_top(10);
        chart.margin_bottom(11);
        chart.margin_left(12);
        chart.margin_right(13);
        assert_eq!(chart.margin[0], 10);
        assert_eq!(chart.margin[1], 11);
        assert_eq!(chart.margin[2], 12);
        assert_eq!(chart.margin[3], 13);
    }

    #[test]
    fn test_caption() {
        let drawing_area = create_mocked_drawing_area(200, 200, |_| {});
        let mut chart = ChartBuilder::on(&drawing_area);

        chart.caption("This is a test case", ("serif", 10));

        assert_eq!(chart.title.as_ref().unwrap().0, "This is a test case");
        assert_eq!(chart.title.as_ref().unwrap().1.font.get_name(), "serif");
        assert_eq!(chart.title.as_ref().unwrap().1.font.get_size(), 10.0);
        check_color(chart.title.as_ref().unwrap().1.color, BLACK.to_rgba());

        chart.caption("This is a test case", ("serif", 10));
        assert_eq!(chart.title.as_ref().unwrap().1.font.get_name(), "serif");
    }
}
