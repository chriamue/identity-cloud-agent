#[cfg(test)]
mod client_test {
    use crate::test_rocket;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    #[tokio::test]
    async fn hello_world() {
        let client = Client::tracked(test_rocket().await)
            .await
            .expect("valid rocket instance");
        let response = client.get("/").dispatch().await;
        assert_eq!(response.status(), Status::SeeOther);
    }
}
