use std::f32::consts::PI;

const PI2: f32 = PI*2.0;
const PI_DIV_360: f32 = PI/360.0;

pub fn calculate_y(width: u32, zen: f32) -> f32 {
    ((width as f32) / PI2) * (zen * PI_DIV_360).tan().recip().ln()
}

pub fn calculate_x(width: u32, az: f32) -> f32 {
    let shift_az = az + 180.0;
    (width as f32) * (shift_az / 360.0)
}


pub fn normalize_to_u8(x: f32, min: f32, max: f32) -> u8
{
    (255.0  * (x - min) / (max - min)) as u8
}


pub fn gaussian_smooth(x: f32, sigma: f32) -> f32 {
    let sigma2 = sigma * 2.0;
    let fraction_part = 1.0 / (PI * sigma2).sqrt();
    let inner_exp = x.powf(2.0) / sigma2.powf(2.0);
    let exp_part = (-inner_exp).exp();
    return fraction_part * exp_part;
}

pub fn distance_from_zenith_range(zen: f32, r: f32) -> f32 {
    let horizon_angle = (90.0 - zen).to_radians();
    horizon_angle.cos() * r
}
