use actix_web::{web, App, HttpServer};
use knowledgeops::config::AppConfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = AppConfig::from_env();
    knowledgeops::logging::init(&config);

    let pool = config.create_db_pool();

    // Run pending migrations
    {
        use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
        const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
        let mut conn = pool.get().expect("Failed to get DB connection for migrations");
        conn.run_pending_migrations(MIGRATIONS).expect("Failed to run migrations");
    }
    log::info!("Migrations complete");

    // Seed roles and demo users if needed
    knowledgeops::services::auth::seed_roles(&pool);
    knowledgeops::services::auth::seed_demo_users(&pool);
    log::info!("Seeding complete");

    // --init-only flag: run migrations + seeding then exit cleanly
    if std::env::args().any(|a| a == "--init-only") {
        log::info!("--init-only: migrations and seeding done, exiting.");
        return Ok(());
    }

    // Initialize alert spool dir for 5xx error alerting
    knowledgeops::errors::set_alerts_spool_dir(config.alerts_spool_dir.clone());

    log::info!("Starting KnowledgeOps server");

    // Start background job worker
    let job_pool = pool.clone();
    let job_config = config.clone();
    tokio::spawn(async move {
        knowledgeops::jobs::scheduler::run_scheduler(job_pool, job_config).await;
    });

    let app_config = web::Data::new(config.clone());
    let db_pool = web::Data::new(pool);

    HttpServer::new(move || {
        App::new()
            .app_data(app_config.clone())
            .app_data(db_pool.clone())
            .wrap(knowledgeops::api::middleware::RequestIdMiddleware)
            .configure(knowledgeops::api::configure_routes)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
