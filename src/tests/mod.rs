use actix_web::test;
use actix_web::{
    dev::ServiceResponse,
    http::{header, StatusCode},
};
use serde::Serialize;

use super::*;
use crate::api::v1::auth::{Login, Register};
use crate::api::v1::services as v1_services;
use crate::data::Data;
use crate::errors::*;

#[macro_export]
macro_rules! get_cookie {
    ($resp:expr) => {
        $resp.response().cookies().next().unwrap().to_owned()
    };
}

pub async fn delete_user(name: &str, data: &Data) {
    let r = sqlx::query!("DELETE FROM mcaptcha_users WHERE name = ($1)", name,)
        .execute(&data.db)
        .await;
    println!();
    println!();
    println!();
    println!("Deleting user: {:?}", &r);
}

#[macro_export]
macro_rules! post_request {
    ($serializable:expr, $uri:expr) => {
        test::TestRequest::post()
            .uri($uri)
            .header(header::CONTENT_TYPE, "application/json")
            .set_payload(serde_json::to_string($serializable).unwrap())
    };
}

#[macro_export]
macro_rules! get_app {
    ($data:expr) => {
        test::init_service(
            App::new()
                .wrap(get_identity_service())
                .configure(v1_services)
                .data($data.clone()),
        )
    };
}

/// register and signin utility
pub async fn register_and_signin<'a>(
    name: &'a str,
    email: &str,
    password: &str,
) -> (data::Data, Login, ServiceResponse) {
    register(name, email, password).await;
    signin(name, password).await
}

/// register and signin utility
pub async fn register<'a>(name: &'a str, email: &str, password: &str) {
    let data = Data::new().await;
    let mut app = get_app!(data).await;

    // 1. Register
    let msg = Register {
        username: name.into(),
        password: password.into(),
        email: email.into(),
    };
    let resp =
        test::call_service(&mut app, post_request!(&msg, "/api/v1/signup").to_request()).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

/// signin util
pub async fn signin<'a>(name: &'a str, password: &str) -> (data::Data, Login, ServiceResponse) {
    let data = Data::new().await;
    let mut app = get_app!(data).await;

    // 2. signin
    let creds = Login {
        username: name.into(),
        password: password.into(),
    };
    let signin_resp = test::call_service(
        &mut app,
        post_request!(&creds, "/api/v1/signin").to_request(),
    )
    .await;
    assert_eq!(signin_resp.status(), StatusCode::OK);

    (data, creds, signin_resp)
}

/// register and signin and domain
pub async fn add_domain_util(
    name: &str,
    password: &str,
    domain: &str,
) -> (data::Data, Login, ServiceResponse) {
    use crate::api::v1::mcaptcha::Domain;

    let (data, creds, signin_resp) = signin(name, password).await;
    let cookies = get_cookie!(signin_resp);
    let mut app = get_app!(data).await;

    // 1. add domain
    let domain = Domain {
        name: domain.into(),
    };

    let add_domain_resp = test::call_service(
        &mut app,
        post_request!(&domain, "/api/v1/mcaptcha/domain/add")
            .cookie(cookies.clone())
            .to_request(),
    )
    .await;
    assert_eq!(add_domain_resp.status(), StatusCode::OK);

    (data, creds, signin_resp)
}

/// pub duplicate test
pub async fn bad_post_req_test<T: Serialize>(
    name: &str,
    password: &str,
    url: &str,
    payload: &T,
    dup_err: ServiceError,
    s: StatusCode,
) {
    let (data, _, signin_resp) = signin(name, password).await;
    let cookies = get_cookie!(signin_resp);
    let mut app = get_app!(data).await;

    let dup_token_resp = test::call_service(
        &mut app,
        post_request!(&payload, url)
            .cookie(cookies.clone())
            .to_request(),
    )
    .await;
    assert_eq!(dup_token_resp.status(), s);
    let txt: ErrorToResponse = test::read_body_json(dup_token_resp).await;
    assert_eq!(txt.error, format!("{}", dup_err));
}