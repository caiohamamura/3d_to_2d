extern crate tls_read_hancock_bin;
extern crate image;
extern crate indicatif;
extern crate threadpool;
extern crate structopt;
extern crate num_cpus;

use structopt::StructOpt;
use tls_read_hancock_bin::HancockReader;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::io::{self};
use std::path::PathBuf;
use threadpool::ThreadPool;


const TOL: f32 = 1e-4;

#[derive(Clone)]
struct Config {
    output: PathBuf,
    dist_min: f32,
    dist_max: f32,
    dist_mid: f32,
    y_top: f32,
    y_bot: f32,
    x_size: u32,
    y_size: u32,
    total_size: usize,
    zen_min: f32,
    zen_max: f32,
    sigma: f32,
}

impl Config {
    pub fn new(opt: &Opt) -> Config {
        // Calculate parameters for image output
        let x_size = opt.width;
        let y_top = calculate_y(x_size, opt.zen_min);
        let y_bot = calculate_y(x_size, opt.zen_max);
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
        }
    }
}
    


#[derive(StructOpt, Clone)]
#[structopt(name = "3d_to_2d")]
struct Opt {
    // A flag, true if used in the command line. Note doc comment will
    // be used for the help message of the flag. The name of the
    // argument will be, by default, based on the name of the field.
    /// Azimuth pixel resolution
    #[structopt(short = "w", long, default_value = "1800")]
    width: u32,
    
    /// Minimum distance
    #[structopt(short, long, default_value = "0.0", allow_hyphen_values = true)]
    dist_min: f32,

    /// Maximum distance
    #[structopt(short = "D", long, default_value = "20.0")]
    dist_max: f32,

    /// Minimum zenith
    #[structopt(short, long, default_value = "30.0")]
    zen_min: f32,

    /// Maximum zenith
    #[structopt(short = "Z", long, default_value = "120.0")]
    zen_max: f32,


    /// Number of threads to run multiple files in parallel
    #[structopt(short = "n", long, default_value = "0")]
    n_threads: usize,

    
    /// Sigma gaussian smoothing factor from range center
    #[structopt(short, long, default_value = "0.0")]
    sigma: f32,


    /// Output file name
    #[structopt(short, long, parse(from_os_str))]
    output: PathBuf,

    /// Input file list space separated
    #[structopt(name = "FILE", parse(from_os_str))]
    inputs: Vec<PathBuf>,
}

fn main() -> io::Result<()> {
    // Arguments parsing
    let opt = Opt::from_args();
    let config = Config::new(&opt);

    // create pool for multithreading on multiple files
    let n_threads;
    if opt.n_threads == 0 {
        let num_cpus = num_cpus::get();
        n_threads = num_cpus - 1;
    } else {
        n_threads = opt.n_threads;
    }
    let pool = ThreadPool::new(n_threads);

    // Progress bar
    let m = MultiProgress::new();
    let sty = ProgressStyle::default_bar()
        .template(
            "{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .progress_chars("#>-");

    // Loop through files and execute for each
    opt.clone().inputs.into_iter().for_each(|file_path| {
        file_to_image(config.clone(), file_path.clone(), &pool, &m, sty.clone());
    });


    m.join_and_clear().unwrap();

    // and more! See the other methods for more details.
    Ok(())
}

fn file_to_image(config: Config, file_path: PathBuf, pool: &ThreadPool, m: &MultiProgress, sty: ProgressStyle) {
    let file_path_str = file_path.clone().into_os_string().into_string().unwrap();
    let beam_reader = HancockReader::new(file_path_str.clone())
        .unwrap_or_else(|err| panic!("Cannot open file: {}!", err));

    let pb = m.add(ProgressBar::new((beam_reader.n_beams) as u64));
    pb.set_style(sty);

    let _ = pool.execute(move || {
        // Declare image vector
        let mut refl_matrix = vec![0.0f32; config.total_size];
        let mut n_points = vec![0u32; config.total_size];

        // Set progress bar
        pb.set_message("Processing file...");
        let mut pieces = file_path.into_iter().rev();
        if let Some(basename) = pieces.next() {
            if let Some(message) = basename.to_str() {
                pb.set_message(&format!("Processing file: {}", message));
            }
        } 
        
        // Filter by n_hits and zenith
        let mut beam_iter = beam_reader.into_iter()
            .filter(|data| {
                data.n_hits > 0            && 
                data.zen >= config.zen_min && 
                data.zen < config.zen_max
            });

        pb.set_position(0);
        
        // Loop through all beams
        while let Some(data) = beam_iter.next() {            
            // Update progress bar each 10000 beams
            if data.shot_n % 10000 == 0 {
                pb.set_position(data.shot_n as u64 + 1);
            }

            // Calculate indexes for x and y
            let loc_x = calculate_x(config.x_size, data.az).floor() as u32;
            let loc_y = config.y_size - ((calculate_y(config.x_size, data.zen) - config.y_bot).floor() as u32) - 1;
            let index = (loc_x + (loc_y * config.x_size)) as usize;
            
            // Panic if by any reason the calculated index exceeds the array size
            if index > config.total_size - 1 {
                panic!(
                    "Error, cannot write to that index of the image!\nData: \nloc_x: {}, loc_y: {}, index: {}, max_index: {}, zen: {}, az: {}",
                    loc_x, loc_y, index, config.total_size, data.zen, data.az
                );
            }

            // Iterate through each of the reflectance and range values
            // and return the sum
            let refl_vector = data.refl.borrow();
            let range_vector = data.r.borrow();

            let refl_sum = refl_vector
                .iter()
                .zip(range_vector.iter())
                .map(|(&refl, &r)| {
                    // Compute horizontal distance (that is what we want!)
                    let h_range = distance_from_zenith_range(data.zen, r);

                    // Filter up by distance
                    if h_range < config.dist_max && h_range >= config.dist_min { 
                        // Gaussian smoothing if sigma has a value
                        if config.sigma < TOL { 
                            refl 
                        } else {
                            refl * gaussian_smooth(h_range - config.dist_mid, config.sigma)
                        }
                     } else { 0.0 }
                }).sum::<f32>();

            let point_count = refl_vector.len() as u32;
            refl_matrix[index] += refl_sum;
            n_points[index] += point_count;
        }

        // Reflectance values weighted by number of points
        for i in 0..config.total_size {
            refl_matrix[i] /= n_points[i] as f32;
        }

        let mut refl_matrix_u8: Vec<u8> = vec![];
        let refl_min = refl_matrix.iter().cloned().fold(9999999.0, f32::min);
        let refl_max = refl_matrix.iter().cloned().fold(-1.0, f32::max);

        refl_matrix.iter().for_each(|x| {
            refl_matrix_u8.push((255.0 * (x - refl_min) / (refl_max - refl_min)) as u8)
        });

        image::save_buffer(
            config.output,
            refl_matrix_u8.as_mut_slice(),
            config.x_size,
            config.y_size,
            image::Gray(8),
        )
        .unwrap();

        pb.finish_with_message("done!");
    });
}


fn calculate_x(width: u32, az: f32) -> f32 {
    let shift_az = az + 180.0;
    (width as f32) * (shift_az / 360.0)
}


use std::f32::consts::PI;

fn calculate_y(width: u32, zen: f32) -> f32 {
    // First transform to radians
    let phi = (90.0 - zen).to_radians();
    
    ((width as f32) / (PI*2.0)) *  
    ((PI/4.0 + phi/2.0).tan()).ln()
}

fn gaussian_smooth(x: f32, sigma: f32) -> f32 {
    let sigma2 = sigma * 2.0;
    let fraction_part = 1.0 / (PI * sigma2).sqrt();
    let inner_exp = x.powf(2.0) / sigma2.powf(2.0);
    let exp_part = (-inner_exp).exp();
    return fraction_part * exp_part;
}

fn distance_from_zenith_range(zen: f32, r: f32) -> f32 {
    let horizon_angle = (90.0 - zen).to_radians();
    horizon_angle.cos() * r
}