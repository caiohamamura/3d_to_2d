extern crate indicatif;

use std::path::PathBuf;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use super::Config;

const INCREMENT_STEP: u64 = 100000;

pub struct ProgressBarWrapper {
    multiprogress: MultiProgress,
    style: ProgressStyle
}

unsafe impl Sync for ProgressBarWrapper {}

impl ProgressBarWrapper {
    pub fn new(visible: bool) -> ProgressBarWrapper {
        let m = MultiProgress::new();
        if visible == false {
            m.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        }
        ProgressBarWrapper {
            multiprogress: m,
            style: ProgressStyle::default_bar()
                .template(
                    "{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
                )
                .progress_chars("#>-"),
        }   
    }

    pub fn get_progress_bar(&self, max_iter: u64) -> ProgressBar {
        let p = self.multiprogress.add(ProgressBar::new(max_iter as u64));
        p.set_style(self.style.clone());
        p
    }
    

    pub fn join_and_clear(&self) {
        self.multiprogress.join_and_clear().unwrap();
    }
}

pub trait CustomProgressBarTrait {
    fn set_custom_message(&self, file_path: &PathBuf, config: &Config);
    fn increment_conditional(&self, val: u64);
}

impl CustomProgressBarTrait for ProgressBar {
    fn set_custom_message(&self, file_path: &PathBuf, config: &Config) {
        self.set_message("Processing file...");
        let mut pieces = file_path.into_iter().rev();
        if let Some(basename) = pieces.next() {
            if let Some(message) = basename.to_str() {
                if config.to_dist > -1.0 {
                    self.set_message(&format!("Processing dist {}-{} for file: {}", config.dist_min, config.dist_max, message));
                } else {
                    self.set_message(&format!("Processing file: {}", message));
                }
            }
        } 
        self.set_position(0);
    }

    fn increment_conditional(&self, val: u64) {
        if val % INCREMENT_STEP == 0 {
            self.inc(INCREMENT_STEP);
        }
    }
}