use coral_net::db::DbConf;
use coral_runtime::tokio;
use sqlx::Executor;
use tokio_stream::StreamExt;

fn postgres_toml() -> DbConf {
    let t = r#"
        [pool]
        max_connections = 5

        [postgres]
        host = ""
        port = 5432
        username = "postgres"
        password = ""
        database = "postgres"
        ssl_mode = "prefer"
    "#;
    toml::from_str(t).unwrap()
}

#[derive(sqlx::FromRow, Debug)]
struct TestItem {
    id: i32,
    name: String,
}

async fn conn_postgres() {
    let db_conf = postgres_toml();
    let conn = db_conf.postgres().await.unwrap().unwrap();
    let id = 1;
    let db_res = sqlx::query_as::<_, TestItem>("SELECT * FROM test where id = $1")
        .bind(id)
        .fetch_all(&conn)
        .await
        .unwrap();
    println!("{:?}", db_res);
}

#[test]
fn test_postgres_conn() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(conn_postgres());
}

#[test]
fn test_toml() {
    use serde::Deserialize;
    #[derive(Deserialize, Debug)]
    #[allow(unused)]
    enum Conf {
        Name(String),
        Age(i32),
    }

    let t = r#"
        name = "123"
    "#;
    let c: Conf = toml::from_str(t).unwrap();
    println!("{:?}", c);
}
