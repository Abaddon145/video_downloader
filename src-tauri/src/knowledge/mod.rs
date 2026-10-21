pub(crate) mod attachments;
pub mod commands;
pub mod models;
pub mod service;
mod workspace;

#[cfg(test)]
mod tests;
pub mod vault;

pub mod index;
pub mod links;
pub mod markdown;
pub mod obsidian;
pub mod search;

#[cfg(test)]
mod attachment_tests;
#[cfg(test)]
mod service_tests;
#[cfg(test)]
mod recovery_tests;

pub mod generated;
#[cfg(test)]
mod generated_tests;
