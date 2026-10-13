/// Screen-space heat haze the tonemap pass applies around one hot spot. Whichever effect owns
/// the plume publishes it every frame; the pass reads only this resource.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatDistortion {
    pub origin: [f32; 3],
    pub temperature: f32,
    pub width_base: f32,
    pub width_slope: f32,
    pub distortion_gain: f32,
    pub height: f32,
    pub time: f32,
    pub turbulence_amp: f32,
    pub wind_direction: [f32; 2],
    pub rise_speed: f32,
}

impl HeatDistortion {
    pub fn push_data(&self) -> ([f32; 4], [f32; 4], [f32; 4], [f32; 4]) {
        (
            [self.origin[0], self.origin[1], self.origin[2], 1.0],
            [
                self.temperature,
                self.width_base,
                self.width_slope,
                self.distortion_gain,
            ],
            [self.height, self.time, self.turbulence_amp, 0.0],
            [
                self.wind_direction[0],
                self.rise_speed,
                self.wind_direction[1],
                0.0,
            ],
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeatDistortionSource {
    pub active: Option<HeatDistortion>,
}
