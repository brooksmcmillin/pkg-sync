use std::sync::Arc;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use clap::{Parser, Subcommand};
use sqlx::sqlite::SqlitePoolOptions;
use tokio::net::TcpListener;

mod auth;
mod db;
mod handlers;

#[derive(Parser)]
#[command(name = "pkg-sync-server")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the HTTP server
    Serve {
        /// Address to bind to
        #[arg(short, long, default_value = "0.0.0.0:3000")]
        bind: String,

        /// Path to SQLite database
        #[arg(short, long, default_value = "pkg-sync.db")]
        database: String,
    },

    /// Register a new machine and generate an API key
    Register {
        /// Machine name
        #[arg(short, long)]
        name: String,

        /// Path to SQLite database
        #[arg(short, long, default_value = "pkg-sync.db")]
        database: String,
    },
}

pub struct AppState {
    pub db: sqlx::SqlitePool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Serve { bind, database } => {
            let pool = SqlitePoolOptions::new()
                .max_connections(5)
                .connect(&format!("sqlite:{}?mode=rwc", database))
                .await?;

            db::run_migrations(&pool).await?;

            let state = Arc::new(AppState { db: pool });

            let app = Router::new()
                .route("/api/v1/report", post(handlers::submit_report))
                .route("/api/v1/diff", get(handlers::get_diff))
                .route("/api/v1/machines", get(handlers::list_machines))
                .with_state(state);

            let listener = TcpListener::bind(&bind).await?;
            tracing::info!("listening on {}", bind);
            axum::serve(listener, app).await?;
        }

        Command::Register { name, database } => {
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect(&format!("sqlite:{}?mode=rwc", database))
                .await?;

            db::run_migrations(&pool).await?;

            let api_key = db::register_machine(&pool, &name).await?;
            println!("Machine '{}' registered.", name);
            println!(
                "API key (save this, it won't be shown again):\n\n  {}",
                api_key
            );
        }
    }

    Ok(())
}
