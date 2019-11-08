extern crate hancock_read_bin;
extern crate image;
extern crate indicatif;
extern crate threadpool;
extern crate structopt;

use structopt::StructOpt;
use hancock_read_bin::HancockReader;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::io::{self};
use std::path::PathBuf;
use threadpool::ThreadPool;

#[derive(StructOpt, Clone)]
#[structopt(name = "3d_to_2d")]
struct Opt {
    // A flag, true if used in the command line. Note doc comment will
    // be used for the help message of the flag. The name of the
    // argument will be, by default, based on the name of the field.
    /// Azimuth pixel resolution
    #[structopt(short = "r", long, default_value = "0.2")]
    res_az: f32,

    /// Zenith pixel resolution
    #[structopt(short = "R", long, default_value = "0.2")]
    res_zen: f32,
    
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

    /// Output file name
    #[structopt(short, long, parse(from_os_str))]
    output: PathBuf,

    /// Input file list
    #[structopt(name = "FILE", parse(from_os_str))]
    inputs: Vec<PathBuf>,
}

fn main() -> io::Result<()> {
    // create pool for multithreading on multiple files
    let pool = ThreadPool::new(2);

    // Arguments parsing
    let opt = Opt::from_args();

    // Progress bar
    let m = MultiProgress::new();
    let sty = ProgressStyle::default_bar()
        .template(
            "{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .progress_chars("#>-");

    // Loop through files and execute for each
    opt.clone().inputs.into_iter().for_each(|file_path| {
        file_to_image(opt.clone(), file_path.clone(), &pool, &m, sty.clone());
    });


    m.join_and_clear().unwrap();

    // and more! See the other methods for more details.
    Ok(())
}

fn file_to_image(config: Opt, file_path: PathBuf, pool: &ThreadPool, m: &MultiProgress, sty: ProgressStyle) {
    let file_path_str = file_path.clone().into_os_string().into_string().unwrap();
    let mut beam_reader = HancockReader::new(file_path_str.clone())
        .unwrap_or_else(|err| panic!("Cannot open file: {}!", err));

    let pb = m.add(ProgressBar::new((beam_reader.n_beams) as u64));
    pb.set_style(sty);

    let _ = pool.execute(move || {
        // Calculate parameters for image output
        let min_dist = config.dist_min;
        let max_dist = config.dist_max;
        // let range_dist = max_dist - min_dist;
        let az_res = config.res_az;
        let x_size = (360.0f32 / az_res).floor() as u32;
        let x_fact = x_size as f32 / 360.0;
        let max_zen = config.zen_max;
        let min_zen = config.zen_min;
        let zen_res = config.res_zen;
        let y_size = ((max_zen-min_zen)/zen_res).floor() as u32;
        let y_fact = y_size as f32 / (max_zen - min_zen);
        let total_size = (x_size * y_size) as usize;

        // Declare image vector
        let mut refl_matrix = vec![0.0f32; total_size];
        let mut n_points = vec![0u32; total_size];

        // Set progress bar
        pb.set_message("Processing file...");
        let mut pieces = file_path.into_iter().rev();
        if let Some(basename) = pieces.next() {
            if let Some(message) = basename.to_str() {
                pb.set_message(&format!("Processing file: {}", message));
            }
        } 
        pb.set_position(0);

        // Loop through all beams
        while let Some(data) = beam_reader.next() {
            let zen = (data.zen).to_radians();
            let mut zen_tan = zen.tan();
            if zen_tan > 1e6 {
                zen_tan = 1e6;
            }

            // Update progress bar
            if beam_reader.current_beam % 10000 == 0 {
                pb.set_position(beam_reader.current_beam as u64 + 1);
            }


            let abs_zen = data.zen.abs();
            if data.n_hits == 0 || abs_zen < min_zen || abs_zen > max_zen {
                continue;
            }

            let loc_x = ((data.az + 180.0) * x_fact).floor() as u32;
            let loc_y = y_size - 1 - ((max_zen - abs_zen) * y_fact).floor() as u32;
            let index = (loc_x + (loc_y * x_size)) as usize;
            if index > total_size - 1 {
                println!(
                    "loc_x: {}, loc_y: {}, zen: {}, zen_tan: {}, az: {}",
                    loc_x, loc_y, data.zen, zen_tan, data.az
                );
            }
            let refl_sum = data
                .refl
                .borrow()
                .iter()
                .zip(data.r.borrow().iter())
                .map(|(&refl, &r)| if r < max_dist && r >= min_dist { refl } else { 0.0 })
                .sum::<f32>();
            let refl_len = data.refl.borrow().len() as f32;
            if refl_sum > 10000.0 || refl_sum < 0.0 {
                break;
            }

            if refl_matrix[index].is_nan() {
                refl_matrix[index] = 0.0f32
            }

            refl_matrix[index] += refl_sum / (refl_len as f32);
            n_points[index] += 1;
        }
        for i in 0..total_size {
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
            x_size,
            y_size,
            image::Gray(8),
        )
        .unwrap();

        pb.finish_with_message("done!");
    });
}
