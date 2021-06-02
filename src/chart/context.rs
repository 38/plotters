use std::borrow::Borrow;
use std::ops::Range;

use super::axes3d::Axes3dStyle;
use super::{DualCoordChartContext, MeshStyle, SeriesAnno, SeriesLabelStyle};

use crate::coord::cartesian::{Cartesian2d, Cartesian3d, MeshLine};
use crate::coord::ranged1d::{AsRangedCoord, KeyPointHint, Ranged, ValueFormatter};
use crate::coord::ranged3d::{ProjectionMatrix, ProjectionMatrixBuilder};
use crate::coord::{CoordTranslate, ReverseCoordTranslate, Shift};

use crate::drawing::{DrawingArea, DrawingAreaErrorKind};
use crate::element::{
    CoordMapper, Drawable, EmptyElement, PathElement, PointCollection, Polygon, Text,
};
use crate::style::text_anchor::{HPos, Pos, VPos};
use crate::style::{ShapeStyle, TextStyle};

use plotters_backend::{BackendCoord, DrawingBackend, FontTransform};

/// The context of the chart. This is the core object of Plotters.
/// Any plot/chart is abstracted as this type, and any data series can be placed to the chart
/// context.
///
/// - To draw a series on a chart context, use [ChartContext::draw_series](struct.ChartContext.html#method.draw_series)
/// - To draw a single element to the chart, you may want to use [ChartContext::plotting_area](struct.ChartContext.html#method.plotting_area)
///
pub struct ChartContext<'a, DB: DrawingBackend, CT: CoordTranslate> {
    pub(super) x_label_area: [Option<DrawingArea<DB, Shift>>; 2],
    pub(super) y_label_area: [Option<DrawingArea<DB, Shift>>; 2],
    pub(super) drawing_area: DrawingArea<DB, CT>,
    pub(super) series_anno: Vec<SeriesAnno<'a, DB>>,
    pub(super) drawing_area_pos: (i32, i32),
}

impl<'a, DB, XT, YT, X, Y> ChartContext<'a, DB, Cartesian2d<X, Y>>
where
    DB: DrawingBackend,
    X: Ranged<ValueType = XT> + ValueFormatter<XT>,
    Y: Ranged<ValueType = YT> + ValueFormatter<YT>,
{
    pub(crate) fn is_overlapping_drawing_area(
        &self,
        area: Option<&DrawingArea<DB, Shift>>,
    ) -> bool {
        if let Some(area) = area {
            let (x0, y0) = area.get_base_pixel();
            let (w, h) = area.dim_in_pixel();
            let (x1, y1) = (x0 + w as i32, y0 + h as i32);
            let (dx0, dy0) = self.drawing_area.get_base_pixel();
            let (w, h) = self.drawing_area.dim_in_pixel();
            let (dx1, dy1) = (dx0 + w as i32, dy0 + h as i32);

            let (ox0, ox1) = (x0.max(dx0), x1.min(dx1));
            let (oy0, oy1) = (y0.max(dy0), y1.min(dy1));

            ox1 > ox0 && oy1 > oy0
        } else {
            false
        }
    }

    /// Initialize a mesh configuration object and mesh drawing can be finalized by calling
    /// the function `MeshStyle::draw`.
    pub fn configure_mesh(&mut self) -> MeshStyle<'a, '_, X, Y, DB> {
        MeshStyle::new(self)
    }
}

impl<'a, DB: DrawingBackend, CT: ReverseCoordTranslate> ChartContext<'a, DB, CT> {
    /// Convert the chart context into an closure that can be used for coordinate translation
    pub fn into_coord_trans(self) -> impl Fn(BackendCoord) -> Option<CT::From> {
        let coord_spec = self.drawing_area.into_coord_spec();
        move |coord| coord_spec.reverse_translate(coord)
    }
}

impl<'a, DB: DrawingBackend, CT: CoordTranslate> ChartContext<'a, DB, CT> {
    /// Configure the styles for drawing series labels in the chart
    pub fn configure_series_labels<'b>(&'b mut self) -> SeriesLabelStyle<'a, 'b, DB, CT>
    where
        DB: 'a,
    {
        SeriesLabelStyle::new(self)
    }

    /// Get a reference of underlying plotting area
    pub fn plotting_area(&self) -> &DrawingArea<DB, CT> {
        &self.drawing_area
    }

    /// Cast the reference to a chart context to a reference to underlying coordinate specification.
    pub fn as_coord_spec(&self) -> &CT {
        self.drawing_area.as_coord_spec()
    }

    // TODO: All draw_series_impl is overly strict about lifetime, because we don't have stable HKT,
    //       what we can ensure is for all lifetime 'b the element reference &'b E is a iterator
    //       of points reference with the same lifetime.
    //       However, this doesn't work if the coordinate doesn't live longer than the backend,
    //       this is unnecessarily strict
    pub(super) fn draw_series_impl<B, E, R, S>(
        &mut self,
        series: S,
    ) -> Result<(), DrawingAreaErrorKind<DB::ErrorType>>
    where
        B: CoordMapper,
        for<'b> &'b E: PointCollection<'b, CT::From, B>,
        E: Drawable<DB, B>,
        R: Borrow<E>,
        S: IntoIterator<Item = R>,
    {
        for element in series {
            self.drawing_area.draw(element.borrow())?;
        }
        Ok(())
    }

    pub(super) fn alloc_series_anno(&mut self) -> &mut SeriesAnno<'a, DB> {
        let idx = self.series_anno.len();
        self.series_anno.push(SeriesAnno::new());
        &mut self.series_anno[idx]
    }

    /// Draw a data series. A data series in Plotters is abstracted as an iterator of elements
    pub fn draw_series<B, E, R, S>(
        &mut self,
        series: S,
    ) -> Result<&mut SeriesAnno<'a, DB>, DrawingAreaErrorKind<DB::ErrorType>>
    where
        B: CoordMapper,
        for<'b> &'b E: PointCollection<'b, CT::From, B>,
        E: Drawable<DB, B>,
        R: Borrow<E>,
        S: IntoIterator<Item = R>,
    {
        self.draw_series_impl(series)?;
        Ok(self.alloc_series_anno())
    }
}

impl<'a, DB: DrawingBackend, X: Ranged, Y: Ranged> ChartContext<'a, DB, Cartesian2d<X, Y>> {
    /// Get the range of X axis
    pub fn x_range(&self) -> Range<X::ValueType> {
        self.drawing_area.get_x_range()
    }

    /// Get range of the Y axis
    pub fn y_range(&self) -> Range<Y::ValueType> {
        self.drawing_area.get_y_range()
    }

    /// Maps the coordinate to the backend coordinate. This is typically used
    /// with an interactive chart.
    pub fn backend_coord(&self, coord: &(X::ValueType, Y::ValueType)) -> BackendCoord {
        self.drawing_area.map_coordinate(coord)
    }

    /// The actual function that draws the mesh lines.
    /// It also returns the label that suppose to be there.
    #[allow(clippy::type_complexity)]
    fn draw_mesh_lines<FmtLabel, YH: KeyPointHint, XH: KeyPointHint>(
        &mut self,
        (r, c): (YH, XH),
        (x_mesh, y_mesh): (bool, bool),
        mesh_line_style: &ShapeStyle,
        mut fmt_label: FmtLabel,
    ) -> Result<(Vec<(i32, String)>, Vec<(i32, String)>), DrawingAreaErrorKind<DB::ErrorType>>
    where
        FmtLabel: FnMut(&MeshLine<X, Y>) -> Option<String>,
    {
        let mut x_labels = vec![];
        let mut y_labels = vec![];
        self.drawing_area.draw_mesh(
            |b, l| {
                let draw;
                match l {
                    MeshLine::XMesh((x, _), _, _) => {
                        if let Some(label_text) = fmt_label(&l) {
                            x_labels.push((x, label_text));
                        }
                        draw = x_mesh;
                    }
                    MeshLine::YMesh((_, y), _, _) => {
                        if let Some(label_text) = fmt_label(&l) {
                            y_labels.push((y, label_text));
                        }
                        draw = y_mesh;
                    }
                };
                if draw {
                    l.draw(b, mesh_line_style)
                } else {
                    Ok(())
                }
            },
            r,
            c,
        )?;
        Ok((x_labels, y_labels))
    }

    fn draw_axis(
        &self,
        area: &DrawingArea<DB, Shift>,
        axis_style: Option<&ShapeStyle>,
        orientation: (i16, i16),
        inward_labels: bool,
    ) -> Result<Range<i32>, DrawingAreaErrorKind<DB::ErrorType>> {
        let (x0, y0) = self.drawing_area.get_base_pixel();
        let (tw, th) = area.dim_in_pixel();

        let mut axis_range = if orientation.0 == 0 {
            self.drawing_area.get_x_axis_pixel_range()
        } else {
            self.drawing_area.get_y_axis_pixel_range()
        };

        /* At this point, the coordinate system tells us the pixel range
         * after the translation.
         * However, we need to use the logic coordinate system for drawing. */
        if orientation.0 == 0 {
            axis_range.start -= x0;
            axis_range.end -= x0;
        } else {
            axis_range.start -= y0;
            axis_range.end -= y0;
        }

        if let Some(axis_style) = axis_style {
            let mut x0 = if orientation.0 > 0 { 0 } else { tw as i32 - 1 };
            let mut y0 = if orientation.1 > 0 { 0 } else { th as i32 - 1 };
            let mut x1 = if orientation.0 >= 0 { 0 } else { tw as i32 - 1 };
            let mut y1 = if orientation.1 >= 0 { 0 } else { th as i32 - 1 };

            if inward_labels {
                if orientation.0 == 0 {
                    if y0 == 0 {
                        y0 = th as i32 - 1;
                        y1 = th as i32 - 1;
                    } else {
                        y0 = 0;
                        y1 = 0;
                    }
                } else if x0 == 0 {
                    x0 = tw as i32 - 1;
                    x1 = tw as i32 - 1;
                } else {
                    x0 = 0;
                    x1 = 0;
                }
            }

            if orientation.0 == 0 {
                x0 = axis_range.start;
                x1 = axis_range.end;
            } else {
                y0 = axis_range.start;
                y1 = axis_range.end;
            }

            area.draw(&PathElement::new(
                vec![(x0, y0), (x1, y1)],
                axis_style.clone(),
            ))?;
        }

        Ok(axis_range)
    }

    // TODO: consider make this function less complicated
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::cognitive_complexity)]
    fn draw_axis_and_labels(
        &self,
        area: Option<&DrawingArea<DB, Shift>>,
        axis_style: Option<&ShapeStyle>,
        labels: &[(i32, String)],
        label_style: &TextStyle,
        label_offset: i32,
        orientation: (i16, i16),
        axis_desc: Option<(&str, &TextStyle)>,
        tick_size: i32,
    ) -> Result<(), DrawingAreaErrorKind<DB::ErrorType>> {
        let area = if let Some(target) = area {
            target
        } else {
            return Ok(());
        };

        let (x0, y0) = self.drawing_area.get_base_pixel();
        let (tw, th) = area.dim_in_pixel();

        /* This is the minimal distance from the axis to the box of the labels */
        let label_dist = tick_size.abs() * 2;

        /* Draw the axis and get the axis range so that we can do further label
         * and tick mark drawing */
        let axis_range = self.draw_axis(area, axis_style, orientation, tick_size < 0)?;

        /* To make the right label area looks nice, it's a little bit tricky, since for a that is
         * very long, we actually prefer left alignment instead of right alignment.
         * Otherwise, the right alignment looks better. So we estimate the max and min label width
         * So that we are able decide if we should apply right alignment for the text. */
        let label_width: Vec<_> = labels
            .iter()
            .map(|(_, text)| {
                if orientation.0 > 0 && orientation.1 == 0 && tick_size >= 0 {
                    self.drawing_area
                        .estimate_text_size(text, &label_style)
                        .map(|(w, _)| w)
                        .unwrap_or(0) as i32
                } else {
                    // Don't ever do the layout estimationfor the drawing area that is either not
                    // the right one or the tick mark is inward.
                    0
                }
            })
            .collect();

        let min_width = *label_width.iter().min().unwrap_or(&1).max(&1);
        let max_width = *label_width
            .iter()
            .filter(|&&x| x < min_width * 2)
            .max()
            .unwrap_or(&min_width);
        let right_align_width = (min_width * 2).min(max_width);

        /* Then we need to draw the tick mark and the label */
        for ((p, t), w) in labels.iter().zip(label_width.into_iter()) {
            /* Make sure we are actually in the visible range */
            let rp = if orientation.0 == 0 { *p - x0 } else { *p - y0 };

            if rp < axis_range.start.min(axis_range.end)
                || axis_range.end.max(axis_range.start) < rp
            {
                continue;
            }

            let (cx, cy, h_pos, v_pos) = if tick_size >= 0 {
                match orientation {
                    // Right
                    (dx, dy) if dx > 0 && dy == 0 => {
                        if w >= right_align_width {
                            (label_dist, *p - y0, HPos::Left, VPos::Center)
                        } else {
                            (
                                label_dist + right_align_width,
                                *p - y0,
                                HPos::Right,
                                VPos::Center,
                            )
                        }
                    }
                    // Left
                    (dx, dy) if dx < 0 && dy == 0 => {
                        (tw as i32 - label_dist, *p - y0, HPos::Right, VPos::Center)
                    }
                    // Bottom
                    (dx, dy) if dx == 0 && dy > 0 => (*p - x0, label_dist, HPos::Center, VPos::Top),
                    // Top
                    (dx, dy) if dx == 0 && dy < 0 => {
                        (*p - x0, th as i32 - label_dist, HPos::Center, VPos::Bottom)
                    }
                    _ => panic!("Bug: Invalid orientation specification"),
                }
            } else {
                match orientation {
                    // Right
                    (dx, dy) if dx > 0 && dy == 0 => {
                        (tw as i32 - label_dist, *p - y0, HPos::Right, VPos::Center)
                    }
                    // Left
                    (dx, dy) if dx < 0 && dy == 0 => {
                        (label_dist, *p - y0, HPos::Left, VPos::Center)
                    }
                    // Bottom
                    (dx, dy) if dx == 0 && dy > 0 => {
                        (*p - x0, th as i32 - label_dist, HPos::Center, VPos::Bottom)
                    }
                    // Top
                    (dx, dy) if dx == 0 && dy < 0 => (*p - x0, label_dist, HPos::Center, VPos::Top),
                    _ => panic!("Bug: Invalid orientation specification"),
                }
            };

            let (text_x, text_y) = if orientation.0 == 0 {
                (cx + label_offset, cy)
            } else {
                (cx, cy + label_offset)
            };

            let label_style = &label_style.pos(Pos::new(h_pos, v_pos));
            area.draw_text(&t, label_style, (text_x, text_y))?;

            if tick_size != 0 {
                if let Some(style) = axis_style {
                    let xmax = tw as i32 - 1;
                    let ymax = th as i32 - 1;
                    let (kx0, ky0, kx1, ky1) = if tick_size > 0 {
                        match orientation {
                            (dx, dy) if dx > 0 && dy == 0 => (0, *p - y0, tick_size, *p - y0),
                            (dx, dy) if dx < 0 && dy == 0 => {
                                (xmax - tick_size, *p - y0, xmax, *p - y0)
                            }
                            (dx, dy) if dx == 0 && dy > 0 => (*p - x0, 0, *p - x0, tick_size),
                            (dx, dy) if dx == 0 && dy < 0 => {
                                (*p - x0, ymax - tick_size, *p - x0, ymax)
                            }
                            _ => panic!("Bug: Invalid orientation specification"),
                        }
                    } else {
                        match orientation {
                            (dx, dy) if dx > 0 && dy == 0 => {
                                (xmax, *p - y0, xmax + tick_size, *p - y0)
                            }
                            (dx, dy) if dx < 0 && dy == 0 => (0, *p - y0, -tick_size, *p - y0),
                            (dx, dy) if dx == 0 && dy > 0 => {
                                (*p - x0, ymax, *p - x0, ymax + tick_size)
                            }
                            (dx, dy) if dx == 0 && dy < 0 => (*p - x0, 0, *p - x0, -tick_size),
                            _ => panic!("Bug: Invalid orientation specification"),
                        }
                    };
                    let line = PathElement::new(vec![(kx0, ky0), (kx1, ky1)], style.clone());
                    area.draw(&line)?;
                }
            }
        }

        if let Some((text, style)) = axis_desc {
            let actual_style = if orientation.0 == 0 {
                style.clone()
            } else if orientation.0 == -1 {
                style.transform(FontTransform::Rotate270)
            } else {
                style.transform(FontTransform::Rotate90)
            };

            let (x0, y0, h_pos, v_pos) = match orientation {
                // Right
                (dx, dy) if dx > 0 && dy == 0 => (tw, th / 2, HPos::Center, VPos::Top),
                // Left
                (dx, dy) if dx < 0 && dy == 0 => (0, th / 2, HPos::Center, VPos::Top),
                // Bottom
                (dx, dy) if dx == 0 && dy > 0 => (tw / 2, th, HPos::Center, VPos::Bottom),
                // Top
                (dx, dy) if dx == 0 && dy < 0 => (tw / 2, 0, HPos::Center, VPos::Top),
                _ => panic!("Bug: Invalid orientation specification"),
            };

            let actual_style = actual_style.pos(Pos::new(h_pos, v_pos));
            area.draw_text(&text, &actual_style, (x0 as i32, y0 as i32))?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_mesh<FmtLabel, YH: KeyPointHint, XH: KeyPointHint>(
        &mut self,
        (r, c): (YH, XH),
        mesh_line_style: &ShapeStyle,
        x_label_style: &TextStyle,
        y_label_style: &TextStyle,
        fmt_label: FmtLabel,
        x_mesh: bool,
        y_mesh: bool,
        x_label_offset: i32,
        y_label_offset: i32,
        x_axis: bool,
        y_axis: bool,
        axis_style: &ShapeStyle,
        axis_desc_style: &TextStyle,
        x_desc: Option<String>,
        y_desc: Option<String>,
        x_tick_size: [i32; 2],
        y_tick_size: [i32; 2],
    ) -> Result<(), DrawingAreaErrorKind<DB::ErrorType>>
    where
        FmtLabel: FnMut(&MeshLine<X, Y>) -> Option<String>,
    {
        let (x_labels, y_labels) =
            self.draw_mesh_lines((r, c), (x_mesh, y_mesh), mesh_line_style, fmt_label)?;

        for idx in 0..2 {
            self.draw_axis_and_labels(
                self.x_label_area[idx].as_ref(),
                if x_axis { Some(axis_style) } else { None },
                &x_labels[..],
                x_label_style,
                x_label_offset,
                (0, -1 + idx as i16 * 2),
                x_desc.as_ref().map(|desc| (&desc[..], axis_desc_style)),
                x_tick_size[idx],
            )?;

            self.draw_axis_and_labels(
                self.y_label_area[idx].as_ref(),
                if y_axis { Some(axis_style) } else { None },
                &y_labels[..],
                y_label_style,
                y_label_offset,
                (-1 + idx as i16 * 2, 0),
                y_desc.as_ref().map(|desc| (&desc[..], axis_desc_style)),
                y_tick_size[idx],
            )?;
        }

        Ok(())
    }

    /// Convert this chart context into a dual axis chart context and attach a second coordinate spec
    /// on the chart context. For more detailed information, see documentation for [struct DualCoordChartContext](struct.DualCoordChartContext.html)
    ///
    /// - `x_coord`: The coordinate spec for the X axis
    /// - `y_coord`: The coordinate spec for the Y axis
    /// - **returns** The newly created dual spec chart context
    #[allow(clippy::type_complexity)]
    pub fn set_secondary_coord<SX: AsRangedCoord, SY: AsRangedCoord>(
        self,
        x_coord: SX,
        y_coord: SY,
    ) -> DualCoordChartContext<
        'a,
        DB,
        Cartesian2d<X, Y>,
        Cartesian2d<SX::CoordDescType, SY::CoordDescType>,
    > {
        let mut pixel_range = self.drawing_area.get_pixel_range();
        pixel_range.1 = pixel_range.1.end..pixel_range.1.start;

        DualCoordChartContext::new(self, Cartesian2d::new(x_coord, y_coord, pixel_range))
    }
}

pub(super) struct KeyPoints3d<X: Ranged, Y: Ranged, Z: Ranged> {
    pub(super) x_points: Vec<X::ValueType>,
    pub(super) y_points: Vec<Y::ValueType>,
    pub(super) z_points: Vec<Z::ValueType>,
}

#[derive(Clone, Debug)]
pub(super) enum Coord3D<X, Y, Z> {
    X(X),
    Y(Y),
    Z(Z),
}

impl<X, Y, Z> Coord3D<X, Y, Z> {
    fn get_x(&self) -> &X {
        match self {
            Coord3D::X(ret) => ret,
            _ => panic!("Invalid call!"),
        }
    }
    fn get_y(&self) -> &Y {
        match self {
            Coord3D::Y(ret) => ret,
            _ => panic!("Invalid call!"),
        }
    }
    fn get_z(&self) -> &Z {
        match self {
            Coord3D::Z(ret) => ret,
            _ => panic!("Invalid call!"),
        }
    }

    fn build_coord([x, y, z]: [&Self; 3]) -> (X, Y, Z)
    where
        X: Clone,
        Y: Clone,
        Z: Clone,
    {
        (x.get_x().clone(), y.get_y().clone(), z.get_z().clone())
    }
}
impl<'a, DB, X, Y, Z, XT, YT, ZT> ChartContext<'a, DB, Cartesian3d<X, Y, Z>>
where
    DB: DrawingBackend,
    X: Ranged<ValueType = XT> + ValueFormatter<XT>,
    Y: Ranged<ValueType = YT> + ValueFormatter<YT>,
    Z: Ranged<ValueType = ZT> + ValueFormatter<ZT>,
{
    pub fn configure_axes(&mut self) -> Axes3dStyle<'a, '_, X, Y, Z, DB> {
        Axes3dStyle::new(self)
    }
}
impl<'a, DB, X: Ranged, Y: Ranged, Z: Ranged> ChartContext<'a, DB, Cartesian3d<X, Y, Z>>
where
    DB: DrawingBackend,
{
    /// Override the 3D projection matrix. This function allows to override the default projection
    /// matrix.
    /// - `pf`: A function that takes the default projection matrix configuration and returns the
    /// projection matrix. This function will allow you to adjust the pitch, yaw angle and the
    /// centeral point of the projection, etc. You can also build a projection matrix which is not
    /// relies on the default configuration as well.
    pub fn with_projection<P: FnOnce(ProjectionMatrixBuilder) -> ProjectionMatrix>(
        &mut self,
        pf: P,
    ) -> &mut Self {
        let (actual_x, actual_y) = self.drawing_area.get_pixel_range();
        self.drawing_area
            .as_coord_spec_mut()
            .set_projection(actual_x, actual_y, pf);
        self
    }

    pub fn set_3d_pixel_range(&mut self, size: (i32, i32, i32)) -> &mut Self {
        let (actual_x, actual_y) = self.drawing_area.get_pixel_range();
        self.drawing_area
            .as_coord_spec_mut()
            .set_coord_pixel_range(actual_x, actual_y, size);
        self
    }
}

impl<'a, DB, X: Ranged, Y: Ranged, Z: Ranged> ChartContext<'a, DB, Cartesian3d<X, Y, Z>>
where
    DB: DrawingBackend,
    X::ValueType: Clone,
    Y::ValueType: Clone,
    Z::ValueType: Clone,
{
    pub(super) fn get_key_points<XH: KeyPointHint, YH: KeyPointHint, ZH: KeyPointHint>(
        &self,
        x_hint: XH,
        y_hint: YH,
        z_hint: ZH,
    ) -> KeyPoints3d<X, Y, Z> {
        let coord = self.plotting_area().as_coord_spec();
        let x_points = coord.logic_x.key_points(x_hint);
        let y_points = coord.logic_y.key_points(y_hint);
        let z_points = coord.logic_z.key_points(z_hint);
        KeyPoints3d {
            x_points,
            y_points,
            z_points,
        }
    }
    pub(super) fn draw_axis_ticks(
        &mut self,
        axis: [[Coord3D<X::ValueType, Y::ValueType, Z::ValueType>; 3]; 2],
        labels: &[(
            [Coord3D<X::ValueType, Y::ValueType, Z::ValueType>; 3],
            String,
        )],
        tick_size: i32,
        style: ShapeStyle,
        font: TextStyle,
    ) -> Result<(), DrawingAreaErrorKind<DB::ErrorType>> {
        let coord = self.plotting_area().as_coord_spec();
        let begin = coord.translate(&Coord3D::build_coord([
            &axis[0][0],
            &axis[0][1],
            &axis[0][2],
        ]));
        let end = coord.translate(&Coord3D::build_coord([
            &axis[1][0],
            &axis[1][1],
            &axis[1][2],
        ]));
        let axis_dir = (end.0 - begin.0, end.1 - begin.1);
        let (x_range, y_range) = self.plotting_area().get_pixel_range();
        let x_mid = (x_range.start + x_range.end) / 2;
        let y_mid = (y_range.start + y_range.end) / 2;

        let x_dir = if begin.0 < x_mid {
            (-tick_size, 0)
        } else {
            (tick_size, 0)
        };

        let y_dir = if begin.1 < y_mid {
            (0, -tick_size)
        } else {
            (0, tick_size)
        };

        let x_score = (x_dir.0 * axis_dir.0 + x_dir.1 * axis_dir.1).abs();
        let y_score = (y_dir.0 * axis_dir.0 + y_dir.1 * axis_dir.1).abs();

        let dir = if x_score < y_score { x_dir } else { y_dir };

        for (pos, text) in labels {
            let logic_pos = Coord3D::build_coord([&pos[0], &pos[1], &pos[2]]);
            let mut font = font.clone();
            if dir.0 < 0 {
                font.pos = Pos::new(HPos::Right, VPos::Center);
            } else if dir.0 > 0 {
                font.pos = Pos::new(HPos::Left, VPos::Center);
            };
            if dir.1 < 0 {
                font.pos = Pos::new(HPos::Center, VPos::Bottom);
            } else if dir.1 > 0 {
                font.pos = Pos::new(HPos::Center, VPos::Top);
            };
            let element = EmptyElement::at(logic_pos)
                + PathElement::new(vec![(0, 0), dir], style.clone())
                + Text::new(text.to_string(), (dir.0 * 2, dir.1 * 2), font.clone());
            self.plotting_area().draw(&element)?;
        }
        Ok(())
    }
    pub(super) fn draw_axis(
        &mut self,
        idx: usize,
        panels: &[[[Coord3D<X::ValueType, Y::ValueType, Z::ValueType>; 3]; 2]; 3],
        style: ShapeStyle,
    ) -> Result<
        [[Coord3D<X::ValueType, Y::ValueType, Z::ValueType>; 3]; 2],
        DrawingAreaErrorKind<DB::ErrorType>,
    > {
        let coord = self.plotting_area().as_coord_spec();
        let x_range = coord.logic_x.range();
        let y_range = coord.logic_y.range();
        let z_range = coord.logic_z.range();

        let ranges: [[Coord3D<X::ValueType, Y::ValueType, Z::ValueType>; 2]; 3] = [
            [Coord3D::X(x_range.start), Coord3D::X(x_range.end)],
            [Coord3D::Y(y_range.start), Coord3D::Y(y_range.end)],
            [Coord3D::Z(z_range.start), Coord3D::Z(z_range.end)],
        ];

        let (start, end) = {
            let mut start = [&ranges[0][0], &ranges[1][0], &ranges[2][0]];
            let mut end = [&ranges[0][1], &ranges[1][1], &ranges[2][1]];

            let mut plan = vec![];

            for i in 0..3 {
                if i == idx {
                    continue;
                }
                start[i] = &panels[i][0][i];
                end[i] = &panels[i][0][i];
                for j in 0..3 {
                    if i != idx && i != j && j != idx {
                        for k in 0..2 {
                            start[j] = &panels[i][k][j];
                            end[j] = &panels[i][k][j];
                            plan.push((start, end));
                        }
                    }
                }
            }
            plan.into_iter()
                .min_by_key(|&(s, e)| {
                    let d = coord.projected_depth(s[0].get_x(), s[1].get_y(), s[2].get_z());
                    let d = d + coord.projected_depth(e[0].get_x(), e[1].get_y(), e[2].get_z());
                    let (_, y1) = coord.translate(&Coord3D::build_coord(s));
                    let (_, y2) = coord.translate(&Coord3D::build_coord(e));
                    let y = y1 + y2;
                    (d, y)
                })
                .unwrap()
        };

        self.plotting_area().draw(&PathElement::new(
            vec![Coord3D::build_coord(start), Coord3D::build_coord(end)],
            style.clone(),
        ))?;

        Ok([
            [start[0].clone(), start[1].clone(), start[2].clone()],
            [end[0].clone(), end[1].clone(), end[2].clone()],
        ])
    }
    pub(super) fn draw_axis_panels(
        &mut self,
        bold_points: &KeyPoints3d<X, Y, Z>,
        light_points: &KeyPoints3d<X, Y, Z>,
        panel_style: ShapeStyle,
        bold_grid_style: ShapeStyle,
        light_grid_style: ShapeStyle,
    ) -> Result<
        [[[Coord3D<X::ValueType, Y::ValueType, Z::ValueType>; 3]; 2]; 3],
        DrawingAreaErrorKind<DB::ErrorType>,
    > {
        let mut r_iter = (0..3).map(|idx| {
            self.draw_axis_panel(
                idx,
                bold_points,
                light_points,
                panel_style.clone(),
                bold_grid_style.clone(),
                light_grid_style.clone(),
            )
        });
        Ok([
            r_iter.next().unwrap()?,
            r_iter.next().unwrap()?,
            r_iter.next().unwrap()?,
        ])
    }
    fn draw_axis_panel(
        &mut self,
        idx: usize,
        bold_points: &KeyPoints3d<X, Y, Z>,
        light_points: &KeyPoints3d<X, Y, Z>,
        panel_style: ShapeStyle,
        bold_grid_style: ShapeStyle,
        light_grid_style: ShapeStyle,
    ) -> Result<
        [[Coord3D<X::ValueType, Y::ValueType, Z::ValueType>; 3]; 2],
        DrawingAreaErrorKind<DB::ErrorType>,
    > {
        let coord = self.plotting_area().as_coord_spec();
        let x_range = coord.logic_x.range();
        let y_range = coord.logic_y.range();
        let z_range = coord.logic_z.range();

        let ranges: [[Coord3D<X::ValueType, Y::ValueType, Z::ValueType>; 2]; 3] = [
            [Coord3D::X(x_range.start), Coord3D::X(x_range.end)],
            [Coord3D::Y(y_range.start), Coord3D::Y(y_range.end)],
            [Coord3D::Z(z_range.start), Coord3D::Z(z_range.end)],
        ];

        let (mut panel, start, end) = {
            let a = [&ranges[0][0], &ranges[1][0], &ranges[2][0]];
            let mut b = [&ranges[0][1], &ranges[1][1], &ranges[2][1]];
            let mut c = a;
            let d = b;

            b[idx] = &ranges[idx][0];
            c[idx] = &ranges[idx][1];

            let (a, b) = if coord.projected_depth(a[0].get_x(), a[1].get_y(), a[2].get_z())
                >= coord.projected_depth(c[0].get_x(), c[1].get_y(), c[2].get_z())
            {
                (a, b)
            } else {
                (c, d)
            };

            let mut m = a.clone();
            m[(idx + 1) % 3] = b[(idx + 1) % 3];
            let mut n = a.clone();
            n[(idx + 2) % 3] = b[(idx + 2) % 3];

            (
                vec![
                    Coord3D::build_coord(a),
                    Coord3D::build_coord(m),
                    Coord3D::build_coord(b),
                    Coord3D::build_coord(n),
                ],
                a,
                b,
            )
        };
        self.plotting_area()
            .draw(&Polygon::new(panel.clone(), panel_style.clone()))?;
        panel.push(panel[0].clone());
        self.plotting_area()
            .draw(&PathElement::new(panel, bold_grid_style.clone()))?;

        for (kps, style) in vec![
            (light_points, light_grid_style),
            (bold_points, bold_grid_style),
        ]
        .into_iter()
        {
            for idx in (0..3).filter(|&i| i != idx) {
                let kps: Vec<_> = match idx {
                    0 => kps.x_points.iter().map(|x| Coord3D::X(x.clone())).collect(),
                    1 => kps.y_points.iter().map(|y| Coord3D::Y(y.clone())).collect(),
                    _ => kps.z_points.iter().map(|z| Coord3D::Z(z.clone())).collect(),
                };
                for kp in kps.iter() {
                    let mut kp_start = start;
                    let mut kp_end = end;
                    kp_start[idx] = kp;
                    kp_end[idx] = kp;
                    self.plotting_area().draw(&PathElement::new(
                        vec![Coord3D::build_coord(kp_start), Coord3D::build_coord(kp_end)],
                        style.clone(),
                    ))?;
                }
            }
        }

        Ok([
            [start[0].clone(), start[1].clone(), start[2].clone()],
            [end[0].clone(), end[1].clone(), end[2].clone()],
        ])
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;

    #[test]
    fn test_chart_context() {
        let drawing_area = create_mocked_drawing_area(200, 200, |_| {});

        drawing_area.fill(&WHITE).expect("Fill");

        let mut chart = ChartBuilder::on(&drawing_area)
            .caption("Test Title", ("serif", 10))
            .x_label_area_size(20)
            .y_label_area_size(20)
            .set_label_area_size(LabelAreaPosition::Top, 20)
            .set_label_area_size(LabelAreaPosition::Right, 20)
            .build_cartesian_2d(0..10, 0..10)
            .expect("Create chart")
            .set_secondary_coord(0.0..1.0, 0.0..1.0);

        chart
            .configure_mesh()
            .x_desc("X")
            .y_desc("Y")
            .draw()
            .expect("Draw mesh");
        chart
            .configure_secondary_axes()
            .x_desc("X")
            .y_desc("Y")
            .draw()
            .expect("Draw Secondary axes");

        chart
            .draw_series(std::iter::once(Circle::new((5, 5), 5, &RED)))
            .expect("Drawing error");
        chart
            .draw_secondary_series(std::iter::once(Circle::new((0.3, 0.8), 5, &GREEN)))
            .expect("Drawing error")
            .label("Test label")
            .legend(|(x, y)| Rectangle::new([(x - 10, y - 5), (x, y + 5)], &GREEN));

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperMiddle)
            .draw()
            .expect("Drawing error");
    }

    #[test]
    fn test_chart_context_3d() {
        let drawing_area = create_mocked_drawing_area(200, 200, |_| {});

        drawing_area.fill(&WHITE).expect("Fill");

        let mut chart = ChartBuilder::on(&drawing_area)
            .caption("Test Title", ("serif", 10))
            .x_label_area_size(20)
            .y_label_area_size(20)
            .set_label_area_size(LabelAreaPosition::Top, 20)
            .set_label_area_size(LabelAreaPosition::Right, 20)
            .build_cartesian_3d(0..10, 0..10, 0..10)
            .expect("Create chart");

        chart.with_projection(|mut pb| {
            pb.yaw = 0.5;
            pb.pitch = 0.5;
            pb.scale = 0.5;
            pb.into_matrix()
        });

        chart.configure_axes().draw().expect("Drawing axes");

        chart
            .draw_series(std::iter::once(Circle::new((5, 5, 5), 5, &RED)))
            .expect("Drawing error");
    }
}
