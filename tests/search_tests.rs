use std::sync::OnceLock;

use hadith::db::{self, Db};
use hadith::embed::Embedder;
use hadith::models::record_id_string;
use hadith::search;
use surrealdb::Surreal;
use surrealdb::types::SurrealValue;
use tokio::sync::OnceCell;

// -- Shared test fixtures (initialized once across all tests) --

static DB: OnceCell<Surreal<Db>> = OnceCell::const_new();
static EMBEDDER: OnceLock<Embedder> = OnceLock::new();

async fn get_db() -> &'static Surreal<Db> {
    DB.get_or_init(|| async {
        let db = db::connect("db_data").await.expect("Failed to connect to db_data/");
        db::init_schema(&db).await.expect("Failed to init schema");
        db
    })
    .await
}

fn get_embedder() -> &'static Embedder {
    EMBEDDER.get_or_init(|| Embedder::new().expect("Failed to create embedder"))
}

// -- Helper types for raw queries --

#[derive(Debug, SurrealValue)]
struct CountResult {
    c: i64,
}

#[derive(Debug, SurrealValue)]
struct ChainResult {
    narrators: Vec<ChainNarrator>,
}

#[derive(Debug, SurrealValue)]
struct ChainNarrator {
    name_en: String,
}

#[derive(Debug, SurrealValue)]
struct IdOnly {
    id: Option<surrealdb::types::RecordId>,
}

// ============================
// 1. Database connectivity
// ============================

#[tokio::test]
async fn test_db_connect_and_schema() {
    let db = get_db().await;
    // If we get here, connect + init_schema succeeded
    let mut res = db.query("SELECT 1 AS ok").await.unwrap();
    let _: Option<serde_json::Value> = res.take(0).unwrap();
}

#[tokio::test]
async fn test_db_has_hadiths() {
    let db = get_db().await;
    let mut res = db
        .query("SELECT count() AS c FROM hadith GROUP ALL")
        .await
        .unwrap();
    let count: Option<CountResult> = res.take(0).unwrap();
    let c = count.expect("No hadith count returned").c;
    assert!(c > 0, "Expected hadiths in DB, got {c}");
}

#[tokio::test]
async fn test_db_has_narrators() {
    let db = get_db().await;
    let mut res = db
        .query("SELECT count() AS c FROM narrator GROUP ALL")
        .await
        .unwrap();
    let count: Option<CountResult> = res.take(0).unwrap();
    let c = count.expect("No narrator count returned").c;
    assert!(c > 0, "Expected narrators in DB, got {c}");
}

#[tokio::test]
async fn test_db_has_books() {
    let db = get_db().await;
    let mut res = db
        .query("SELECT count() AS c FROM book GROUP ALL")
        .await
        .unwrap();
    let count: Option<CountResult> = res.take(0).unwrap();
    let c = count.expect("No book count returned").c;
    assert!(c > 0, "Expected books in DB, got {c}");
}

// ============================
// 2. Text search (BM25)
// ============================

#[tokio::test]
async fn test_text_search_returns_results() {
    let db = get_db().await;
    let results = search::search_hadiths_text(db, "prayer", 10, 0)
        .await
        .unwrap();
    assert!(!results.is_empty(), "Text search for 'prayer' returned no results");
    for h in &results {
        assert!(h.hadith_number > 0, "Invalid hadith number");
    }
}

#[tokio::test]
async fn test_text_search_no_match() {
    let db = get_db().await;
    let results = search::search_hadiths_text(db, "xyzzy999zzz", 10, 0)
        .await
        .unwrap();
    assert!(results.is_empty(), "Nonsense query should return no results");
}

// ============================
// 3. Semantic search (vector)
// ============================

#[tokio::test]
async fn test_semantic_search_returns_results() {
    let db = get_db().await;
    let embedder = get_embedder();
    let results = search::search_hadiths_semantic(db, embedder, "fasting during Ramadan", 6)
        .await
        .unwrap();
    assert!(
        !results.is_empty(),
        "Semantic search for 'fasting during Ramadan' returned no results"
    );
}

#[tokio::test]
async fn test_semantic_search_has_scores() {
    let db = get_db().await;
    let embedder = get_embedder();
    let results = search::search_hadiths_semantic(db, embedder, "prayer", 6)
        .await
        .unwrap();
    assert!(!results.is_empty());
    for h in &results {
        let score = h.score.unwrap_or(0.0);
        assert!(score > 0.0, "Expected positive score, got {score}");
    }
}

#[tokio::test]
async fn test_semantic_search_scores_ordered() {
    let db = get_db().await;
    let embedder = get_embedder();
    let results = search::search_hadiths_semantic(db, embedder, "kindness to neighbors", 6)
        .await
        .unwrap();
    if results.len() >= 2 {
        for window in results.windows(2) {
            let a = window[0].score.unwrap_or(0.0);
            let b = window[1].score.unwrap_or(0.0);
            assert!(a >= b, "Scores not descending: {a} < {b}");
        }
    }
}

// ============================
// 4. Hybrid search (BM25 + vector RRF)
// ============================

#[tokio::test]
async fn test_hybrid_search_returns_results() {
    let db = get_db().await;
    let embedder = get_embedder();
    let results = search::search_hadiths_hybrid(db, embedder, "prayer", 10, 0)
        .await
        .unwrap();
    assert!(
        !results.is_empty(),
        "Hybrid search for 'prayer' returned no results"
    );
}

#[tokio::test]
async fn test_hybrid_search_has_scores() {
    let db = get_db().await;
    let embedder = get_embedder();
    let results = search::search_hadiths_hybrid(db, embedder, "fasting", 10, 0)
        .await
        .unwrap();
    assert!(!results.is_empty());
    for h in &results {
        let score = h.score.unwrap_or(0.0);
        assert!(score > 0.0, "Expected positive RRF score, got {score}");
    }
}

#[tokio::test]
async fn test_hybrid_search_respects_limit() {
    let db = get_db().await;
    let embedder = get_embedder();
    let results = search::search_hadiths_hybrid(db, embedder, "prayer", 3, 0)
        .await
        .unwrap();
    assert!(results.len() <= 3, "Expected at most 3 results, got {}", results.len());
}

// ============================
// 5. Narrator search
// ============================

#[tokio::test]
async fn test_narrator_search() {
    let db = get_db().await;
    let results = search::search_narrators(db, "Abu", 10, 0).await.unwrap();
    assert!(
        !results.is_empty(),
        "Narrator search for 'Abu' returned no results"
    );
    let has_match = results
        .iter()
        .any(|n| n.name_en.to_lowercase().contains("abu"));
    assert!(has_match, "No narrator name contains 'Abu'");
}

#[tokio::test]
async fn test_narrator_search_has_hadith_count() {
    let db = get_db().await;
    let results = search::search_narrators(db, "Abu", 5, 0).await.unwrap();
    assert!(!results.is_empty());
    let has_counts = results
        .iter()
        .any(|n| n.hadith_count.unwrap_or(0) > 0);
    assert!(has_counts, "Expected at least one narrator with hadith_count > 0");
}

// ============================
// 6. GraphRAG (narrator chain traversal)
// ============================

#[tokio::test]
async fn test_graph_heard_from_edges_exist() {
    let db = get_db().await;
    let mut res = db
        .query("SELECT count() AS c FROM heard_from GROUP ALL")
        .await
        .unwrap();
    let count: Option<CountResult> = res.take(0).unwrap();
    let c = count.expect("No heard_from count returned").c;
    assert!(c > 0, "Expected heard_from edges in DB, got {c}");
}

#[tokio::test]
async fn test_graph_narrates_edges_exist() {
    let db = get_db().await;
    let mut res = db
        .query("SELECT count() AS c FROM narrates GROUP ALL")
        .await
        .unwrap();
    let count: Option<CountResult> = res.take(0).unwrap();
    let c = count.expect("No narrates count returned").c;
    assert!(c > 0, "Expected narrates edges in DB, got {c}");
}

#[tokio::test]
async fn test_graph_traversal_narrator_chain() {
    let db = get_db().await;

    // Pick the first hadith that has narrators
    let mut res = db
        .query("SELECT id FROM hadith LIMIT 1")
        .await
        .unwrap();
    let hadith: Option<IdOnly> = res.take(0).unwrap();
    let hadith_id = hadith
        .expect("No hadith found")
        .id
        .expect("Hadith has no id");

    let rid_str = record_id_string(&hadith_id);

    // Traverse the graph to find narrators for this hadith
    let mut res = db
        .query("SELECT <-narrates<-narrator.{name_en} AS narrators FROM type::thing($rid)")
        .bind(("rid", rid_str))
        .await
        .unwrap();
    let chain: Option<ChainResult> = res.take(0).unwrap();
    let narrators = chain.expect("No chain result").narrators;
    assert!(
        !narrators.is_empty(),
        "Expected narrators in chain for the hadith"
    );
    for n in &narrators {
        assert!(!n.name_en.is_empty(), "Narrator name_en should not be empty");
    }
}

// ============================
// 7. BM25 index existence
// ============================

#[tokio::test]
async fn test_bm25_indexes_defined() {
    let db = get_db().await;
    let mut res = db.query("INFO FOR TABLE hadith").await.unwrap();
    let info: Option<serde_json::Value> = res.take(0).unwrap();
    let info_str = serde_json::to_string(&info).unwrap();
    assert!(
        info_str.contains("hadith_text_en_search"),
        "BM25 index hadith_text_en_search not found in table info"
    );
    assert!(
        info_str.contains("hadith_text_ar_search"),
        "BM25 index hadith_text_ar_search not found in table info"
    );
}

#[tokio::test]
async fn test_hnsw_vector_index_defined() {
    let db = get_db().await;
    let mut res = db.query("INFO FOR TABLE hadith").await.unwrap();
    let info: Option<serde_json::Value> = res.take(0).unwrap();
    let info_str = serde_json::to_string(&info).unwrap();
    assert!(
        info_str.contains("hadith_vec"),
        "HNSW vector index hadith_vec not found in table info"
    );
}
