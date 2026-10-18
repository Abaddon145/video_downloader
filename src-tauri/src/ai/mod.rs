pub mod analysis;
#[cfg(test)]
mod analysis_tests;
pub mod cache;
pub mod client;
pub mod commands;
pub mod jobs;
pub mod keyframes;
mod materials;
pub mod models;
pub mod prompts;
pub mod qa;
#[cfg(test)]
mod reference_tests;
pub mod references;
pub mod repository;
pub mod service;
pub mod settings;
pub mod summary;
#[cfg(test)]
mod template_tests;
pub mod templates;
#[cfg(test)]
mod tests;
pub mod transcription;
pub mod usage;
pub mod whisper_models;

pub mod subtitles;

pub mod downloads;

#[cfg(test)]
mod live_tests;
