use std::iter::once;
use std::ops::Range;

/// The type of tick mark
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TickKind {
    Major,
    Minor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tick<T, L> {
    pub kind: TickKind,
    pub pos: T,
    pub label: L,
}

/// Trait for axes whose tick marks can be iterated over.
///   * `T` - Type of the `pos` (in the [`Tick`] returned by the iterator).
///   * `L` - Type of the `label` (in the [`Tick`] returned by the iterator).
pub trait AxisTickEnumerator<T, L> {
    fn iter(&self) -> Box<dyn Iterator<Item = Tick<T, L>> + '_>;
   // fn iter_for_range(&self, range: Range<i32>) -> Box<dyn Iterator<Item = Tick<i32, L>> + '_>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleLinearAxis<T> {
    major_tick_spacing: T,
    minor_ticks_per_major_tick: usize,
    range: Range<T>,
}

impl AxisTickEnumerator<f32, f32> for SimpleLinearAxis<f32> {
   // fn iter_for_range(&self, range: Range<i32>) -> Box<dyn Iterator<Item = Tick<i32, f32>> + '_> {
   //     let start = range.start as f32;
   //     let len = (range.end - range.start) as f32;
   //     
   //     Box::new(self.iter().map(move |tick| Tick {
   //         kind: tick.kind,
   //         label: tick.label,
   //         pos: (start + len * tick.pos) as i32,
   //     }))
   // }
    fn iter(&self) -> Box<dyn Iterator<Item = Tick<f32, f32>> + '_> {
        let (range_start, range_end) = (
            self.range.start.min(self.range.end),
            self.range.end.max(self.range.start),
        );

        // Information that is needed for the main body
        let start = (range_start / self.major_tick_spacing).ceil() as isize;
        let end = (range_end / self.major_tick_spacing).floor() as isize;
        let minor_tick_spacing =
            self.major_tick_spacing / ((self.minor_ticks_per_major_tick + 1) as f32);
        let major_tick_spacing = self.major_tick_spacing;

        // Information needed for the start/end
        let major_tick_start = start as f32 * major_tick_spacing;
        let major_tick_end = end as f32 * major_tick_spacing;
        let minor_ticks_before_first_major = (1..)
            .take_while(|i| major_tick_start - *i as f32 * minor_tick_spacing >= range_start)
            .count();
        let minor_ticks_after_last_major = (1..)
            .take_while(|i| major_tick_end + *i as f32 * minor_tick_spacing <= range_end)
            .count();

        let iter = (start..end).flat_map(move |k| {
            let start = k as f32 * major_tick_spacing;
            (0..=self.minor_ticks_per_major_tick).map(move |i| {
                let pos = start + (i as f32) * minor_tick_spacing;
                Tick {
                    pos,
                    kind: match i {
                        0 => TickKind::Major,
                        _ => TickKind::Minor,
                    },
                    label: pos,
                }
            })
        });

        // Right now, iter will iterate through the main body of the ticks,
        // but will not iterate through the minor ticks before the first major
        // or the last major tick/minor ticks after the last major. Those need to
        // be inserted manually.
        let start_iter = (1..=minor_ticks_before_first_major).rev().map(move |i| {
            let pos = major_tick_start - (i as f32) * minor_tick_spacing;
            Tick {
                pos,
                kind: TickKind::Minor,
                label: pos,
            }
        });
        let end_iter = once(Tick {
            pos: major_tick_end,
            kind: TickKind::Major,
            label: major_tick_end,
        })
        .chain((1..=minor_ticks_after_last_major).map(move |i| {
            let pos = major_tick_end + (i as f32) * minor_tick_spacing;
            Tick {
                pos,
                kind: TickKind::Minor,
                label: pos,
            }
        }));

        Box::new(start_iter.chain(iter).chain(end_iter))
    }
}

/// Use some heuristics to guess the best tick spacing for `range` given a length of `len` pixels.
pub(crate) fn suggest_tickmark_spacing_for_range(
    range: &Range<f32>,
    len: i32,
) -> SimpleLinearAxis<f32> {
    let range_len = (range.end - range.start).abs();
    let scale = len as f32 / range_len;

    // Ideally we want to space our major ticks between 50 and 120 pixels.
    // So start searching to see if we find such a condition.
    let mut major_tick_spacing = 1.0;
    for &tick_hint in &[1.0, 2.5, 2.0, 5.0] {
        // Check if there is a power of 10 so that the tick_hint works as a major tick
        // That amounts to solving the equation `50 <= tick_hint*scale*10^n <= 120` for `n`.
        let upper = (120. / (tick_hint * scale)).log10();
        let lower = (50. / (tick_hint * scale)).log10();
        if upper.floor() >= lower.ceil() {
            // In this condition, we have an integer solution (in theory we might
            // have more than one which is the reason for the funny check).
            let pow = upper.floor() as i32;
            // We prefer tick steps of .25 and 25, but not 2.5, so exclude this case
            // specifically.
            if pow != 0 || tick_hint != 2.5 {
                major_tick_spacing = tick_hint * 10_f32.powi(pow);
                break;
            }
        }
    }

    let mut minor_ticks_per_major_tick: usize = 0;
    // We want minor ticks to be at least 15 px apart
    for &tick_hint in &[9, 4, 3, 1] {
        if major_tick_spacing * scale / ((tick_hint + 1) as f32) > 15. {
            minor_ticks_per_major_tick = tick_hint;
            break;
        }
    }

    SimpleLinearAxis {
        major_tick_spacing,
        minor_ticks_per_major_tick,
        range: range.clone(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /*
    #[test]
    fn test_iter_for_range() {
        let linear_axis = SimpleLinearAxis {
            major_tick_spacing: 1.0_f32,
            minor_ticks_per_major_tick: 0,
            range: -1.0..4.0,
        };
        let ticks = linear_axis
            .iter_for_range(-1..4)
            .map(|tick| tick.pos)
            .collect::<Vec<i32>>();
        let ticks2 = linear_axis
            .iter()
            .map(|tick| tick.pos)
            .collect::<Vec<f32>>();
            dbg!(ticks2);

        assert_eq!(ticks, vec![-1,0,1,2,3,4]);
    }*/

    #[test]
    fn test_spacing_suggestions() {
        let suggestion = suggest_tickmark_spacing_for_range(&(0.0..5.0), 500);

        assert_eq!(
            suggestion,
            SimpleLinearAxis {
                major_tick_spacing: 1.0,
                minor_ticks_per_major_tick: 4,
                range: 0.0..5.0,
            }
        );
    }

    #[test]
    fn test_tick_spacing() {
        let linear_axis = SimpleLinearAxis {
            major_tick_spacing: 1.0_f32,
            minor_ticks_per_major_tick: 0,
            range: -1.0..4.0,
        };
        let ticks = linear_axis.iter().collect::<Vec<Tick<_, _>>>();
        assert_eq!(
            ticks,
            vec![
                Tick {
                    kind: TickKind::Major,
                    pos: -1.0,
                    label: -1.0
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 0.0,
                    label: 0.0
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 1.0,
                    label: 1.0
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 2.0,
                    label: 2.0
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 3.0,
                    label: 3.0
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 4.0,
                    label: 4.0
                },
            ]
        );

        let linear_axis = SimpleLinearAxis {
            major_tick_spacing: 1.0_f32,
            minor_ticks_per_major_tick: 0,
            range: -1.5..2.9,
        };
        let ticks = linear_axis.iter().collect::<Vec<Tick<_, _>>>();
        assert_eq!(
            ticks,
            vec![
                Tick {
                    kind: TickKind::Major,
                    pos: -1.0,
                    label: -1.0
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 0.0,
                    label: 0.0
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 1.0,
                    label: 1.0
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 2.0,
                    label: 2.0
                },
            ]
        );

        let linear_axis = SimpleLinearAxis {
            major_tick_spacing: 1.0_f32,
            minor_ticks_per_major_tick: 1,
            range: -1.0..1.0,
        };
        let ticks = linear_axis.iter().collect::<Vec<Tick<_, _>>>();
        assert_eq!(
            ticks,
            vec![
                Tick {
                    kind: TickKind::Major,
                    pos: -1.0,
                    label: -1.0
                },
                Tick {
                    kind: TickKind::Minor,
                    pos: -0.5,
                    label: -0.5
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 0.0,
                    label: 0.0
                },
                Tick {
                    kind: TickKind::Minor,
                    pos: 0.5,
                    label: 0.5
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 1.0,
                    label: 1.0
                },
            ]
        );
    }

    #[test]
    fn test_minor_ticks_before_first_major() {
        let linear_axis = SimpleLinearAxis {
            major_tick_spacing: 1.0_f32,
            minor_ticks_per_major_tick: 1,
            range: -1.6..1.0,
        };
        let ticks = linear_axis.iter().collect::<Vec<Tick<_, _>>>();
        assert_eq!(
            ticks,
            vec![
                Tick {
                    kind: TickKind::Minor,
                    pos: -1.5,
                    label: -1.5
                },
                Tick {
                    kind: TickKind::Major,
                    pos: -1.0,
                    label: -1.0
                },
                Tick {
                    kind: TickKind::Minor,
                    pos: -0.5,
                    label: -0.5
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 0.0,
                    label: 0.0
                },
                Tick {
                    kind: TickKind::Minor,
                    pos: 0.5,
                    label: 0.5
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 1.0,
                    label: 1.0
                },
            ]
        );
    }
    #[test]
    fn test_minor_ticks_after_last_major() {
        let linear_axis = SimpleLinearAxis {
            major_tick_spacing: 1.0_f32,
            minor_ticks_per_major_tick: 1,
            range: -1.6..1.6,
        };
        let ticks = linear_axis.iter().collect::<Vec<Tick<_, _>>>();
        assert_eq!(
            ticks,
            vec![
                Tick {
                    kind: TickKind::Minor,
                    pos: -1.5,
                    label: -1.5
                },
                Tick {
                    kind: TickKind::Major,
                    pos: -1.0,
                    label: -1.0
                },
                Tick {
                    kind: TickKind::Minor,
                    pos: -0.5,
                    label: -0.5
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 0.0,
                    label: 0.0
                },
                Tick {
                    kind: TickKind::Minor,
                    pos: 0.5,
                    label: 0.5
                },
                Tick {
                    kind: TickKind::Major,
                    pos: 1.0,
                    label: 1.0
                },
                Tick {
                    kind: TickKind::Minor,
                    pos: 1.5,
                    label: 1.5
                },
            ]
        );
    }
}
