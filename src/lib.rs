#![allow(dead_code)]

pub mod lang;
pub mod persistance;
pub mod shutdown;
pub mod tracker;
pub mod web;
pub mod wrap;

extern crate google_sheets4 as sheets4;
extern crate hyper;
extern crate hyper_rustls;
extern crate yup_oauth2 as oauth2;
#[allow(unused_imports)]
#[macro_use]
extern crate ntest;
#[macro_use]
extern crate rocket;
