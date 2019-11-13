use structopt::StructOpt;
use std::path::PathBuf;

use super::helpers;

#[derive(Clone)]
pub struct Config {
    pub output: PathBuf,
    pub dist_min: f32,
    pub dist_max: f32,
    pub dist_mid: f32,
    pub y_top: f32,
    pub y_bot: f32,
    pub x_size: u32,
    pub y_size: u32,
    pub total_size: usize,
    pub zen_min: f32,
    pub zen_max: f32,
    pub sigma: f32,
    pub from_dist: f32,
    pub to_dist: f32,
    pub total_frames: u32,
    pub range_view: f32,
    pub split: bool,
}

impl Config {
    pub fn new(opt: &Opt) -> Config {
        // Calculate parameters for image output
        let x_size = opt.width;
        let y_top = helpers::calculate_y(x_size, opt.zen_min);
        let y_bot = helpers::calculate_y(x_size, opt.zen_max);
        let y_size = (y_top - y_bot + 1.0).floor() as u32;
        Config {
            output: opt.output.clone(),
            dist_min: opt.dist_min,
            dist_max: opt.dist_max,
            dist_mid: (opt.dist_max + opt.dist_min)/2.0,
            y_top,
            y_bot,
            x_size,
            y_size,
            total_size: (x_size * y_size) as usize,
            zen_min: opt.zen_min,
            zen_max: opt.zen_max,
            sigma: opt.sigma,
            from_dist: opt.from_dist,
            to_dist: opt.to_dist,
            total_frames: opt.total_frames,
            range_view: opt.range_view,
            split: opt.split,
        }
    }
}
    


#[derive(StructOpt, Clone)]
#[structopt(name = "3d_to_2d")]
pub struct Opt {
    // A flag, true if used in the command line. Note doc comment will
    // be used for the help message of the flag. The name of the
    // argument will be, by default, based on the name of the field.
    /// Azimuth pixel resolution
    #[structopt(short = "w", long, default_value = "1800")]
    width: u32,
    
    /// Minimum distance
    #[structopt(short, long, default_value = "0.0", allow_hyphen_values = true)]
    pub dist_min: f32,

    /// Maximum distance
    #[structopt(short = "D", long, default_value = "20.0")]
    pub dist_max: f32,

    /// Minimum zenith
    #[structopt(short, long, default_value = "30.0")]
    zen_min: f32,

    /// Maximum zenith
    #[structopt(short = "Z", long, default_value = "120.0")]
    zen_max: f32,

    /// From distance
    #[structopt(short = "f", long, default_value = "-1.0")]
    from_dist: f32,

    /// To distance
    #[structopt(short = "F", long, default_value = "-1.0")]
    to_dist: f32,

    /// Total frames
    #[structopt(short = "t", long, default_value = "120")]
    pub total_frames: u32,
    
    /// Range view
    #[structopt(short = "V", long, default_value = "6.0")]
    range_view: f32,

    // Progress
    #[structopt(short = "p", long)]
    pub progress: bool,


    /// Number of threads to run multiple files in parallel
    #[structopt(short = "n", long, default_value = "0")]
    pub n_threads: usize,

    
    /// Sigma gaussian smoothing factor from range center
    #[structopt(short, long, default_value = "0.0")]
    sigma: f32,

    /// Save splitted gauss weights and reflectance
    #[structopt(short = "S", long)]
    split: bool,

    /// Output file name
    #[structopt(short, long, parse(from_os_str))]
    pub output: PathBuf,

    /// Input file list space separated
    #[structopt(name = "FILE", parse(from_os_str))]
    pub inputs: Vec<PathBuf>,
}