use im::Vector;

pub struct StereoSignal {
    pub left: Vector<i16>,
    pub right: Vector<i16>,
    pub sample_rate: u32,
}

impl StereoSignal {
    pub fn new(left: Vector<i16>, right: Vector<i16>, sample_rate: u32) -> Self {
        Self {
            left,
            right,
            sample_rate,
        }
    }
}
