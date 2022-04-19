#![allow(dead_code)]
#![allow(unused_macros)]
#![feature(type_alias_impl_trait)]

pub mod connector;
pub mod core;
pub mod data;
pub mod error;
pub mod lang;
pub mod models;
pub mod persistance;
pub mod schema;
pub mod server;
pub mod shutdown;
pub mod wrap;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate derivative;

#[allow(unused_imports)]
#[macro_use]
extern crate diesel_migrations;

extern crate google_sheets4 as sheets4;
extern crate hyper;
extern crate hyper_rustls;
extern crate yup_oauth2;
#[allow(unused_imports)]
#[macro_use]
extern crate ntest;
#[macro_use]
extern crate rocket;

pub mod stats {
    tonic::include_proto!("stats");
}
