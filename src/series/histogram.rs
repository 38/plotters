use std::collections::{hash_map::IntoIter as HashMapIter, HashMap};
use std::marker::PhantomData;
use std::ops::AddAssign;

use crate::chart::ChartContext;
use crate::coord::cartesian::Cartesian2d;
use crate::coord::ranged1d::{DiscreteRanged, Ranged};
use crate::element::Rectangle;
use crate::style::{Color, ShapeStyle, GREEN};
use plotters_backend::DrawingBackend;

pub trait HistogramType {}
pub struct Vertical;
pub struct Horizontal;

impl HistogramType for Vertical {}
impl HistogramType for Horizontal {}

/// The series that aggregate data into a histogram
pub struct Histogram<'a, BR, A, Tag = Vertical>
where
    BR: DiscreteRanged,
    A: AddAssign<A> + Default,
    Tag: HistogramType,
{
    style: Box<dyn Fn(&BR::ValueType, &A) -> ShapeStyle + 'a>,
    margin: u32,
    iter: HashMapIter<usize, A>,
    baseline: Box<dyn Fn(&BR::ValueType) -> A + 'a>,
    br: BR,
    _p: PhantomData<Tag>,
}

impl<'a, BR, A, Tag> Histogram<'a, BR, A, Tag>
where
    BR: DiscreteRanged + Clone,
    A: AddAssign<A> + Default + 'a,
    Tag: HistogramType,
{
    fn empty(br: &BR) -> Self {
        Self {
            style: Box::new(|_, _| GREEN.filled()),
            margin: 5,
            iter: HashMap::new().into_iter(),
            baseline: Box::new(|_| A::default()),
            br: br.clone(),
            _p: PhantomData,
        }
    }
    /// Set the style of the histogram
    pub fn style<S: Into<ShapeStyle>>(mut self, style: S) -> Self {
        let style = style.into();
        self.style = Box::new(move |_, _| style.clone());
        self
    }

    /// Set the style of histogram using a lambda function
    pub fn style_func(
        mut self,
        style_func: impl Fn(&BR::ValueType, &A) -> ShapeStyle + 'a,
    ) -> Self {
        self.style = Box::new(style_func);
        self
    }

    /// Set the baseline of the histogram
    pub fn baseline(mut self, baseline: A) -> Self
    where
        A: Clone,
    {
        self.baseline = Box::new(move |_| baseline.clone());
        self
    }

    /// Set a function that defines variant baseline
    pub fn baseline_func(mut self, func: impl Fn(&BR::ValueType) -> A + 'a) -> Self {
        self.baseline = Box::new(func);
        self
    }

    /// Set the margin for each bar
    pub fn margin(mut self, value: u32) -> Self {
        self.margin = value;
        self
    }

    /// Set the data iterator
    pub fn data<TB: Into<BR::ValueType>, I: IntoIterator<Item = (TB, A)>>(
        mut self,
        iter: I,
    ) -> Self {
        let mut buffer = HashMap::<usize, A>::new();
        for (x, y) in iter.into_iter() {
            if let Some(x) = self.br.index_of(&x.into()) {
                *buffer.entry(x).or_insert_with(Default::default) += y;
            }
        }
        self.iter = buffer.into_iter();
        self
    }
}

impl<'a, BR, A> Histogram<'a, BR, A, Vertical>
where
    BR: DiscreteRanged + Clone,
    A: AddAssign<A> + Default + 'a,
{
    pub fn vertical<ACoord, DB: DrawingBackend + 'a>(
        parent: &ChartContext<DB, Cartesian2d<BR, ACoord>>,
    ) -> Self
    where
        ACoord: Ranged<ValueType = A>,
    {
        let dp = parent.as_coord_spec().x_spec();

        Self::empty(dp)
    }
}

impl<'a, BR, A> Histogram<'a, BR, A, Horizontal>
where
    BR: DiscreteRanged + Clone,
    A: AddAssign<A> + Default + 'a,
{
    pub fn horizontal<ACoord, DB: DrawingBackend>(
        parent: &ChartContext<DB, Cartesian2d<ACoord, BR>>,
    ) -> Self
    where
        ACoord: Ranged<ValueType = A>,
    {
        let dp = parent.as_coord_spec().y_spec();
        Self::empty(dp)
    }
}

impl<'a, BR, A> Iterator for Histogram<'a, BR, A, Vertical>
where
    BR: DiscreteRanged,
    A: AddAssign<A> + Default,
{
    type Item = Rectangle<(BR::ValueType, A)>;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((x, y)) = self.iter.next() {
            if let Some((x, Some(nx))) = self
                .br
                .from_index(x)
                .map(|v| (v, self.br.from_index(x + 1)))
            {
                let base = (self.baseline)(&x);
                let style = (self.style)(&x, &y);
                let mut rect = Rectangle::new([(x, y), (nx, base)], style);
                rect.set_margin(0, 0, self.margin, self.margin);
                return Some(rect);
            }
        }
        None
    }
}

impl<'a, BR, A> Iterator for Histogram<'a, BR, A, Horizontal>
where
    BR: DiscreteRanged,
    A: AddAssign<A> + Default,
{
    type Item = Rectangle<(A, BR::ValueType)>;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((y, x)) = self.iter.next() {
            if let Some((y, Some(ny))) = self
                .br
                .from_index(y)
                .map(|v| (v, self.br.from_index(y + 1)))
            {
                let base = (self.baseline)(&y);
                let style = (self.style)(&y, &x);
                let mut rect = Rectangle::new([(x, y), (base, ny)], style);
                rect.set_margin(self.margin, self.margin, 0, 0);
                return Some(rect);
            }
        }
        None
    }
}
