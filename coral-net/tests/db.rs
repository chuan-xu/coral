use coral_net::db::DbConf;
use coral_runtime::tokio;

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
#[allow(unused)]
struct TestItem {
    id: i32,
    name: String,
}

async fn conn_postgres() {
    let db_conf = postgres_toml();
    let conn = db_conf.postgres().unwrap().unwrap().await.unwrap();
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

async fn conn_redis() {
    let conf_str = r#"
        [Single]
        host = ""
        port = 6379
        db = 0
        password = ""
        protocol = 1
        [Single.config.Manager]
        connection_timeout = 2
    "#;
    let conf: coral_net::db::RedisConf = toml::from_str(conf_str).unwrap();
    let mut client = conf.client(None).unwrap().await.unwrap();
    redis::cmd("set")
        .arg("test_conn_key")
        .arg("test_conn")
        .query_async::<()>(&mut client)
        .await
        .unwrap();
    let name = redis::cmd("get")
        .arg("test_conn_key")
        .query_async::<String>(&mut client)
        .await
        .unwrap();
    assert_eq!(name, "test_conn");
}

#[test]
fn test_redis_conn() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(conn_redis());
}
