#[macro_use]
extern crate rocket;
use clap::Parser;
use rocket::{get, post, serde::json::Json};
use serde_json::{json, Value};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser, default_value = "http://localhost:8000")]
    agent_host: String,

    #[clap(short, long, value_parser, default_value = "localhost")]
    host: String,

    #[clap(short, long, value_parser, default_value_t = 3000)]
    port: u16,
}

#[get("/")]
fn index() -> &'static str {
    "Watch the console logs!"
}

#[post("/topic/<name>", format = "any", data = "<body>")]
fn topic(name: &str, body: Json<Value>) -> &'static str {
    println!(
        "{} {}",
        name,
        serde_json::to_string_pretty(&body.into_inner()).unwrap()
    );
    "ok"
}

async fn register(agent_host: String, host: String, port: u16) {
    let client = reqwest::Client::new();
    let body = json!({
      "url": format!("http://{}:{}", host, port),
      "registeredEvent": [
        "ALL"
      ]
    });

    let response = client
        .post(format!("{}/webhook", agent_host))
        .json(&body)
        .send()
        .await
        .unwrap();
    println!("register webhook: {}", response.status());
}

#[launch]
async fn rocket() -> _ {
    let args = Args::parse();

    register(args.agent_host, args.host, args.port).await;

    let figment = rocket::Config::figment().merge(("port", args.port));

    rocket::custom(figment).mount("/", routes![index, topic])
}
