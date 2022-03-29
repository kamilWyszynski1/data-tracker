#![allow(dead_code)]
#![feature(type_alias_impl_trait)]

pub mod core;
pub mod data;
pub mod lang;
pub mod models;
pub mod persistance;
pub mod schema;
pub mod shutdown;
pub mod web;
pub mod wrap;

#[macro_use]
extern crate diesel;

extern crate google_sheets4 as sheets4;
extern crate hyper;
extern crate hyper_rustls;
extern crate yup_oauth2 as oauth2;
#[allow(unused_imports)]
#[macro_use]
extern crate ntest;
#[macro_use]
extern crate rocket;
