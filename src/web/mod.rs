pub mod handlers;
pub mod quran_handlers;

use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use surrealdb::Surreal;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::db::Db;
use crate::embed::Embedder;
use crate::rag::OllamaClient;

#[derive(Clone)]
pub struct AppState {
    pub db: Surreal<Db>,
    pub embedder: Arc<Embedder>,
    pub ollama: Option<Arc<OllamaClient>>,
}

pub async fn serve(
    db: Surreal<Db>,
    port: u16,
    ollama_url: Option<String>,
    ollama_model: Option<String>,
) -> Result<()> {
    let embedder = Arc::new(Embedder::new()?);
    let ollama = Some(Arc::new(OllamaClient::new(ollama_url, ollama_model)));

    let state = AppState {
        db,
        embedder,
        ollama,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        .route("/api/stats", axum::routing::get(handlers::stats))
        .route("/api/books", axum::routing::get(handlers::books))
        .route("/api/search", axum::routing::get(handlers::search))
        .route("/api/hadiths", axum::routing::get(handlers::hadith_list))
        .route(
            "/api/hadiths/{id}",
            axum::routing::get(handlers::hadith_detail),
        )
        .route(
            "/api/narrators",
            axum::routing::get(handlers::narrator_list),
        )
        .route(
            "/api/narrators/{id}",
            axum::routing::get(handlers::narrator_detail).put(handlers::update_narrator),
        )
        .route(
            "/api/chain/{hadith_id}",
            axum::routing::get(handlers::chain_graph_data),
        )
        .route(
            "/api/narrators/{id}/graph",
            axum::routing::get(handlers::narrator_graph_data),
        )
        .route("/api/ask", axum::routing::post(handlers::ask))
        .route("/api/families", axum::routing::get(handlers::family_list))
        .route(
            "/api/families/{id}",
            axum::routing::get(handlers::family_detail),
        )
        .route(
            "/api/analysis/stats",
            axum::routing::get(handlers::analysis_stats),
        )
        .route(
            "/api/analysis/juynboll/summary",
            axum::routing::get(handlers::juynboll_summary),
        )
        .route(
            "/api/narrators/{id}/reliability",
            axum::routing::get(handlers::narrator_reliability),
        )
        .route(
            "/api/narrators/{id}/cl-status",
            axum::routing::get(handlers::narrator_cl_status),
        )
        .route("/api/diff", axum::routing::get(handlers::matn_diff_handler))
        .route(
            "/api/export/family/{id}",
            axum::routing::get(handlers::export_family),
        )
        .route(
            "/api/internal/translate",
            axum::routing::post(handlers::update_translation),
        )
        // Quran routes
        .route(
            "/api/quran/stats",
            axum::routing::get(quran_handlers::quran_stats),
        )
        .route(
            "/api/quran/surahs",
            axum::routing::get(quran_handlers::surah_list),
        )
        .route(
            "/api/quran/surahs/{number}",
            axum::routing::get(quran_handlers::surah_detail),
        )
        .route(
            "/api/quran/search",
            axum::routing::get(quran_handlers::quran_search),
        )
        .route(
            "/api/quran/browse",
            axum::routing::get(quran_handlers::ayah_browse),
        )
        .route(
            "/api/quran/ask",
            axum::routing::post(quran_handlers::ask_quran),
        )
        .route(
            "/api/quran/ayah/{ayah_key}/hadiths",
            axum::routing::get(quran_handlers::ayah_hadiths),
        )
        .route(
            "/api/quran/surahs/{number}/hadith-counts",
            axum::routing::get(quran_handlers::surah_hadith_counts),
        )
        .route(
            "/api/quran/surahs/{number}/similar-counts",
            axum::routing::get(quran_handlers::surah_similar_counts),
        )
        .route(
            "/api/quran/surahs/{number}/variant-counts",
            axum::routing::get(quran_handlers::surah_variant_counts),
        )
        .route(
            "/api/quran/ayah/{ayah_key}/words",
            axum::routing::get(quran_handlers::ayah_words),
        )
        .route(
            "/api/quran/search/root/{root}",
            axum::routing::get(quran_handlers::root_search),
        )
        .route(
            "/api/quran/reciters",
            axum::routing::get(quran_handlers::reciters),
        )
        .route(
            "/api/quran/ayah/{ayah_key}/similar",
            axum::routing::get(quran_handlers::ayah_similar),
        )
        .route(
            "/api/quran/phrases/{id}",
            axum::routing::get(quran_handlers::phrase_detail),
        )
        // Manuscript & Variant routes
        .route(
            "/api/quran/manuscripts",
            axum::routing::get(quran_handlers::manuscript_list),
        )
        .route(
            "/api/quran/manuscripts/{id}",
            axum::routing::get(quran_handlers::manuscript_detail),
        )
        .route(
            "/api/quran/ayah/{ayah_key}/variants",
            axum::routing::get(quran_handlers::ayah_variants),
        )
        // Unified Quran & Sunnah routes
        .route(
            "/api/unified/search",
            axum::routing::get(handlers::unified_search),
        )
        .route(
            "/api/unified/ask",
            axum::routing::post(handlers::unified_ask),
        )
        .with_state(state);

    // Serve static assets from frontend/build, with SPA fallback to index.html
    let spa_fallback = ServeFile::new("frontend/build/index.html");
    let static_files = ServeDir::new("frontend/build").not_found_service(spa_fallback);

    let app = api.fallback_service(static_files).layer(cors);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!("Server listening on http://localhost:{port}");
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
