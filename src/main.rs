extern crate tls_read_hancock_bin;
extern crate image;
extern crate indicatif;
extern crate threadpool;
extern crate structopt;
extern crate num_cpus;

use tls_read_hancock_bin::HancockReader;
use std::io::{self};
use std::path::PathBuf;
use threadpool::ThreadPool;

mod progressbar_helper;
use progressbar_helper::{CustomProgressBarTrait, ProgressBarWrapper};

mod config_helper;
use config_helper::{Config, Opt};
use structopt::StructOpt;

mod helpers;

const TOL: f32 = 1e-4;


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
    let progress_handler = ProgressBarWrapper::new(opt.progress);
    
    // Loop through files
    opt.clone().inputs.into_iter().for_each(|file_path| {
        if config.to_dist > -1.0 {
            // Loop through distances if supplied
            for i in 0..config.total_frames {
                let frame_percent = i as f32 / config.total_frames as f32;
                let begin = config.from_dist + (config.to_dist * frame_percent) - config.range_view/2.0;
                let end = begin + config.range_view;
                let mut opt2 = opt.clone();
                opt2.dist_min = begin;
                opt2.dist_max = end;
                let n_digits = (opt2.mult_frames as f64).log10().ceil() as usize;
                opt2.output = opt2.output.with_extension(format!("{:0fill$}.png", i, fill = n_digits));
                let config2 = Config::new(&opt2);
                tls_3d_to_2d(config2, file_path.clone(), &pool, &progress_handler);
                }
            }
        else {
            tls_3d_to_2d(config.clone(), file_path.clone(), &pool, &progress_handler);
        }
    });

    progress_handler.join_and_clear();
    Ok(())
}

fn tls_3d_to_2d(config: Config, file_path: PathBuf, pool: &ThreadPool, m: &ProgressBarWrapper) {
    let file_path_str = file_path.clone().into_os_string().into_string().unwrap();
    let beam_reader = HancockReader::new(file_path_str.clone())
        .unwrap_or_else(|err| panic!("Cannot open file: {}!", err));

    let pb = m.get_progress_bar(beam_reader.n_beams as u64);

    let _ = pool.execute(move || {
        // Set progress bar
        pb.set_custom_message(&file_path, &config);

        // Allocate vectors to save reflectance data
        let mut refl_matrix = vec![0.0f32; config.total_size];
        let gauss_size;
        if config.split {
            gauss_size = config.total_size;
        } else {
            gauss_size = 0;
        }
        let mut gauss_matrix = vec![0.0f32; gauss_size];
        let mut n_points = vec![0u32; config.total_size];
        
        // Filter by n_hits and zenith
        let mut beam_iter = beam_reader.into_iter()
            .filter(|data| {
                data.n_hits > 0            && 
                data.zen >= config.zen_min && 
                data.zen < config.zen_max
            });
        
        // Loop through filtered beams
        while let Some(data) = beam_iter.next() {            
            // Update progress bar
            pb.increment_conditional(data.shot_n as u64);
            

            // Calculate indexes for x and y
            let loc_x = helpers::calculate_x(config.x_size, data.az).floor() as u32;
            let loc_y = config.y_size - ((helpers::calculate_y(config.x_size, data.zen) - config.y_bot).floor() as u32) - 1;
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

            let h_range_vector = range_vector.iter()
            // Get horizontal range iterator
                .map(|&r| helpers::distance_from_zenith_range(data.zen, r))
            // Filter iterator if horizontal range outside desired bounds
                .filter(|&h_range| {h_range < config.dist_max && h_range >= config.dist_min});


            if config.split {
                let refl_sum = refl_vector
                    .iter()
                    .zip(h_range_vector.clone())
                    .map(|(&refl, _)| {
                        refl
                    }).sum::<f32>();
                let gauss_sum = refl_vector
                    .iter()
                    .zip(h_range_vector)
                    .map(|(_, r)| {
                        helpers::gaussian_smooth(r - config.dist_mid, config.sigma)
                    }).sum::<f32>();
                let point_count = refl_vector.len() as u32;
                refl_matrix[index] += refl_sum;
                gauss_matrix[index] += gauss_sum;
                n_points[index] += point_count;                
            } else {
                let refl_sum = refl_vector
                    .iter()
                    .zip(h_range_vector)
                    .map(|(&refl, r)| {
                        if config.sigma < TOL { 
                            refl 
                        } else {
                            refl * helpers::gaussian_smooth(r - config.dist_mid, config.sigma)
                        }
                    }).sum::<f32>();
                let point_count = refl_vector.len() as u32;
                refl_matrix[index] += refl_sum;
                n_points[index] += point_count;

            }
        }

        // Reflectance values weighted by number of points
        for i in 0..config.total_size {
            refl_matrix[i] /= n_points[i] as f32;
            if config.split {
                gauss_matrix[i] /= n_points[i] as f32;
            }
        }

        let mut refl_matrix_u8: Vec<u8> = vec![];
        let refl_min = refl_matrix.iter().cloned().fold(9999999.0, f32::min);
        let refl_max = refl_matrix.iter().cloned().fold(-1.0, f32::max);
        
        let mut gauss_matrix_u8: Vec<u8> = vec![];
        let gauss_min = gauss_matrix.iter().cloned().fold(9999999.0, f32::min);
        let gauss_max = gauss_matrix.iter().cloned().fold(-1.0, f32::max);

        refl_matrix.iter().for_each(|&x| {
            refl_matrix_u8.push(helpers::normalize_to_u8(x, refl_min, refl_max))
        });

        gauss_matrix.iter().for_each(|&x| {
            gauss_matrix_u8.push(helpers::normalize_to_u8(x, gauss_min, gauss_max))
        });

        image::save_buffer(
            config.output.clone(),
            refl_matrix_u8.as_mut_slice(),
            config.x_size,
            config.y_size,
            image::Gray(8),
        )
        .unwrap();

        if config.split {
            image::save_buffer(
                config.output.with_extension("g.png"),
                gauss_matrix_u8.as_mut_slice(),
                config.x_size,
                config.y_size,
                image::Gray(8),
            )
            .unwrap();
        }

        pb.finish_and_clear();
    });
}




