use std::sync::LazyLock;

use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Surreal,
};

pub static DB: LazyLock<Surreal<Client>> = LazyLock::new(Surreal::init);

async fn retry_connect_db(url: &str, max_attempts: usize) -> Result<(), surrealdb::Error> {
    let mut attempts = 0;
    loop {
        match DB.connect::<Ws>(url).await {
            Ok(_) => return Ok(()),
            Err(e) if attempts < max_attempts => {
                attempts += 1;
                eprintln!("Failed to connect to database, retrying... ({})", e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => return Err(e),
        }
    }
}

pub async fn connect_db(url: &str) -> Result<(), surrealdb::Error> {
    println!("Connecting to database at {}...", url);
    retry_connect_db(url, 5).await?;

    println!("Connected to database");
    DB.signin(Root {
        username: "root",
        password: "secret",
    })
    .await?;

    DB.use_ns("app").use_db("main").await?;

    Ok(())
}
