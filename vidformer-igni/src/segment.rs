use num_rational::Rational64;

#[derive(PartialEq, Debug)]
pub struct Segment {
    pub start_frame: i32,
    pub n_frames: i32,
}

impl Segment {
    pub fn duration(&self, frame_rate: &Rational64) -> Rational64 {
        Rational64::from(self.n_frames as i64) * frame_rate.recip()
    }
}

pub fn segments(
    n_frames: i32,
    segment_length: &Rational64,
    frame_rate: &Rational64,
    terminal: bool,
) -> Vec<Segment> {
    debug_assert!(n_frames >= 0);
    debug_assert!(segment_length > &Rational64::ZERO);
    debug_assert!(frame_rate > &Rational64::ZERO);
    let frame_time = frame_rate.recip();
    debug_assert!(segment_length > &frame_time);

    let num_segments = num_segments(n_frames, segment_length, frame_rate, terminal);
    let mut segments = Vec::with_capacity(num_segments as usize);
    for i in 0..num_segments {
        segments.push(segment(i, n_frames, segment_length, frame_rate));
    }
    segments
}

#[allow(dead_code)] // Used for testing only
fn segments_slow(
    n_frames: i32,
    segment_length: &Rational64,
    frame_rate: &Rational64,
    terminal: bool,
) -> Vec<Segment> {
    debug_assert!(n_frames >= 0);
    debug_assert!(segment_length > &Rational64::ZERO);
    debug_assert!(frame_rate > &Rational64::ZERO);
    let frame_time = frame_rate.recip();
    debug_assert!(segment_length > &frame_time);

    let num_segments = num_segments(n_frames, segment_length, frame_rate, terminal);
    let mut segments = Vec::with_capacity(num_segments as usize);
    for frame_i in 0..n_frames {
        let segment = frame_to_segment(frame_i, segment_length, frame_rate);
        if segment == num_segments {
            break;
        }
        if segment != segments.len() as i32 - 1 {
            segments.push(Segment {
                start_frame: frame_i,
                n_frames: 1,
            });
        } else {
            segments.last_mut().unwrap().n_frames += 1;
        }
    }

    debug_assert!(segments.len() == num_segments as usize);
    segments
}

pub fn segment(
    segment: i32,
    n_frames: i32,
    segment_length: &Rational64,
    frame_rate: &Rational64,
) -> Segment {
    let segment_frac = Rational64::from(segment as i64);
    let start_frame = (segment_frac * segment_length * frame_rate)
        .ceil()
        .to_integer();
    let end_frame = ((segment_frac + Rational64::ONE) * segment_length * frame_rate)
        .ceil()
        .to_integer()
        .min(n_frames as i64)
        - 1;
    let n_frames = end_frame - start_frame + 1;

    debug_assert_eq!(
        frame_to_segment(start_frame as i32, segment_length, frame_rate),
        segment
    );
    debug_assert_eq!(
        frame_to_segment(end_frame as i32, segment_length, frame_rate),
        segment
    );

    Segment {
        start_frame: start_frame as i32,
        n_frames: n_frames as i32,
    }
}

pub fn num_segments(
    n_frames: i32,
    segment_length: &Rational64,
    frame_rate: &Rational64,
    terminal: bool,
) -> i32 {
    debug_assert!(n_frames >= 0);
    debug_assert!(segment_length > &Rational64::ZERO);
    debug_assert!(frame_rate > &Rational64::ZERO);
    let frame_time = frame_rate.recip();
    debug_assert!(segment_length > &frame_time);

    if n_frames == 0 {
        return 0;
    }

    if terminal {
        // Find the segment idx of the last frame
        frame_to_segment(n_frames - 1, segment_length, frame_rate) + 1
    } else {
        // Find the segment idx of the segment before the next frame
        frame_to_segment(n_frames, segment_length, frame_rate)
    }
}

fn frame_to_segment(frame: i32, segment_length: &Rational64, frame_rate: &Rational64) -> i32 {
    debug_assert!(frame >= 0);
    debug_assert!(segment_length > &Rational64::ZERO);
    debug_assert!(frame_rate > &Rational64::ZERO);
    let frame_time = frame_rate.recip();
    debug_assert!(segment_length > &frame_time);

    let frame_frac = Rational64::from(frame as i64);
    let segment = (frame_frac * frame_time / segment_length).to_integer();

    segment as i32
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_num_segments_zero_frames() {
        assert_eq!(
            num_segments(0, &Rational64::new(1, 1), &Rational64::new(30, 1), false),
            0
        );
        assert_eq!(
            num_segments(0, &Rational64::new(1, 1), &Rational64::new(30, 1), true),
            0
        );
    }

    #[test]
    fn test_num_segments_non_terminal() {
        assert_eq!(
            num_segments(1, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            0
        );
        assert_eq!(
            num_segments(61, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            1
        );
        assert_eq!(
            num_segments(119, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            1
        );
    }

    #[test]
    fn test_num_segments_perfect_fit_non_terminal() {
        assert_eq!(
            num_segments(60, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            1
        );
        assert_eq!(
            num_segments(120, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            2
        );
    }

    #[test]
    fn test_num_segments_terminal() {
        assert_eq!(
            num_segments(1, &Rational64::new(2, 1), &Rational64::new(30, 1), true),
            1
        );
        assert_eq!(
            num_segments(61, &Rational64::new(2, 1), &Rational64::new(30, 1), true),
            2
        );
        assert_eq!(
            num_segments(119, &Rational64::new(2, 1), &Rational64::new(30, 1), true),
            2
        );
        assert_eq!(
            num_segments(120, &Rational64::new(2, 1), &Rational64::new(30, 1), true),
            2
        );
    }

    #[test]
    fn test_x() {
        let segment_length = Rational64::from(2);
        let frame_rate = Rational64::new(30000, 1001);

        assert_eq!(frame_to_segment(59, &segment_length, &frame_rate), 0);
        assert_eq!(frame_to_segment(60, &segment_length, &frame_rate), 1);

        assert_eq!(num_segments(60, &segment_length, &frame_rate, true), 1);
    }

    #[test]
    fn test_frame_to_segment() {
        assert_eq!(
            frame_to_segment(0, &Rational64::new(2, 1), &Rational64::new(30, 1)),
            0
        );
        assert_eq!(
            frame_to_segment(1, &Rational64::new(2, 1), &Rational64::new(30, 1)),
            0
        );
        assert_eq!(
            frame_to_segment(59, &Rational64::new(2, 1), &Rational64::new(30, 1)),
            0
        );
        assert_eq!(
            frame_to_segment(60, &Rational64::new(2, 1), &Rational64::new(30, 1)),
            1
        );
        assert_eq!(
            frame_to_segment(61, &Rational64::new(2, 1), &Rational64::new(30, 1)),
            1
        );
        assert_eq!(
            frame_to_segment(119, &Rational64::new(2, 1), &Rational64::new(30, 1)),
            1
        );
        assert_eq!(
            frame_to_segment(120, &Rational64::new(2, 1), &Rational64::new(30, 1)),
            2
        );
    }

    #[test]
    fn test_segments() {
        // zero frames
        assert_eq!(
            segments(0, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            vec![]
        );
        assert_eq!(
            segments_slow(0, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            vec![]
        );

        // not quite 1 segment, non terminal
        assert_eq!(
            segments(59, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            vec![]
        );
        assert_eq!(
            segments_slow(59, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            vec![]
        );

        // not quite 1 segment, terminal
        assert_eq!(
            segments(59, &Rational64::new(2, 1), &Rational64::new(30, 1), true),
            vec![Segment {
                start_frame: 0,
                n_frames: 59,
            }]
        );
        assert_eq!(
            segments_slow(59, &Rational64::new(2, 1), &Rational64::new(30, 1), true),
            vec![Segment {
                start_frame: 0,
                n_frames: 59,
            }]
        );

        // 1 segment, non terminal
        assert_eq!(
            segments(60, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            vec![Segment {
                start_frame: 0,
                n_frames: 60,
            }]
        );
        assert_eq!(
            segments_slow(60, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            vec![Segment {
                start_frame: 0,
                n_frames: 60,
            }]
        );

        // 1 segment, terminal
        assert_eq!(
            segments(60, &Rational64::new(2, 1), &Rational64::new(30, 1), true),
            vec![Segment {
                start_frame: 0,
                n_frames: 60,
            }]
        );
        assert_eq!(
            segments_slow(60, &Rational64::new(2, 1), &Rational64::new(30, 1), true),
            vec![Segment {
                start_frame: 0,
                n_frames: 60,
            }]
        );

        // not quite 2 segments, non terminal
        assert_eq!(
            segments(119, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            vec![Segment {
                start_frame: 0,
                n_frames: 60,
            }]
        );
        assert_eq!(
            segments_slow(119, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            vec![Segment {
                start_frame: 0,
                n_frames: 60,
            }]
        );

        // not quite 2 segments, terminal
        assert_eq!(
            segments(119, &Rational64::new(2, 1), &Rational64::new(30, 1), true),
            vec![
                Segment {
                    start_frame: 0,
                    n_frames: 60
                },
                Segment {
                    start_frame: 60,
                    n_frames: 59
                },
            ]
        );
        assert_eq!(
            segments_slow(119, &Rational64::new(2, 1), &Rational64::new(30, 1), true),
            vec![
                Segment {
                    start_frame: 0,
                    n_frames: 60
                },
                Segment {
                    start_frame: 60,
                    n_frames: 59
                },
            ]
        );

        // two segments, non terminal
        assert_eq!(
            segments(120, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            vec![
                Segment {
                    start_frame: 0,
                    n_frames: 60
                },
                Segment {
                    start_frame: 60,
                    n_frames: 60
                },
            ]
        );
        assert_eq!(
            segments_slow(120, &Rational64::new(2, 1), &Rational64::new(30, 1), false),
            vec![
                Segment {
                    start_frame: 0,
                    n_frames: 60
                },
                Segment {
                    start_frame: 60,
                    n_frames: 60
                },
            ]
        );

        // two segments, terminal
        assert_eq!(
            segments(120, &Rational64::new(2, 1), &Rational64::new(30, 1), true),
            vec![
                Segment {
                    start_frame: 0,
                    n_frames: 60
                },
                Segment {
                    start_frame: 60,
                    n_frames: 60
                },
            ]
        );
        assert_eq!(
            segments_slow(120, &Rational64::new(2, 1), &Rational64::new(30, 1), true),
            vec![
                Segment {
                    start_frame: 0,
                    n_frames: 60
                },
                Segment {
                    start_frame: 60,
                    n_frames: 60
                },
            ]
        );
    }

    #[test]
    fn test_segments_29_97_fps() {
        let segment_length = Rational64::from(2);
        let frame_rate = Rational64::new(30000, 1001);

        for n_frames in 1..=1000 {
            {
                // Non-terminal check
                let segments = segments(n_frames, &segment_length, &frame_rate, false);
                let segments_slow = segments_slow(n_frames, &segment_length, &frame_rate, false);
                assert_eq!(segments, segments_slow);

                for (segment_idx, segment) in segments.iter().enumerate() {
                    for frame_idx in segment.start_frame..segment.start_frame + segment.n_frames {
                        assert_eq!(
                            frame_to_segment(frame_idx, &segment_length, &frame_rate),
                            segment_idx as i32
                        );
                    }
                }

                //  Check all segments follow each other
                for (past_segment, future_segment) in segments.iter().zip(segments.iter().skip(1)) {
                    assert_eq!(
                        past_segment.start_frame + past_segment.n_frames,
                        future_segment.start_frame
                    );
                }
            }

            {
                // Terminal check
                let segments = segments(n_frames, &segment_length, &frame_rate, true);
                let segments_slow = segments_slow(n_frames, &segment_length, &frame_rate, true);
                assert_eq!(segments, segments_slow);

                let mut n_frames_represented = 0;
                for (segment_idx, segment) in segments.iter().enumerate() {
                    n_frames_represented += segment.n_frames;
                    for frame_idx in segment.start_frame..segment.start_frame + segment.n_frames {
                        assert_eq!(
                            frame_to_segment(frame_idx, &segment_length, &frame_rate),
                            segment_idx as i32
                        );
                    }
                }
                assert_eq!(n_frames_represented, n_frames);

                //  Check all segments follow each other
                for (past_segment, future_segment) in segments.iter().zip(segments.iter().skip(1)) {
                    assert_eq!(
                        past_segment.start_frame + past_segment.n_frames,
                        future_segment.start_frame
                    );
                }
            }
        }
    }

    #[test]
    fn test_segment() {
        let segment_length = Rational64::from(2);
        let frame_rate = Rational64::new(30, 1);
        let n_frames = 1000;

        let segments = segments(n_frames, &segment_length, &frame_rate, false);
        for i in 0..segments.len() as i32 {
            assert_eq!(
                segment(i, n_frames, &segment_length, &frame_rate),
                segments[i as usize]
            );
        }
    }

    #[test]
    fn test_segment_29_97() {
        let segment_length = Rational64::from(2);
        let frame_rate = Rational64::new(30000, 1001);
        let n_frames = 1000;

        let segments = segments(n_frames, &segment_length, &frame_rate, false);
        for i in 0..segments.len() as i32 {
            assert_eq!(
                segment(i, n_frames, &segment_length, &frame_rate),
                segments[i as usize]
            );
        }
    }
}
