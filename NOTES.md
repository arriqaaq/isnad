# Project Notes

## Data Quality Warnings

### Never ingest narrator grading or bio data from SemanticHadith dataset
- The `semantic_hadith.json` narrator entries contain `grade` fields (e.g. `thiqah`, `matruk`) and biographical info that are **unreliable**
- Known issue: narrator HN05049 (Ibn Abi Shaybah, narrator in Sahih Muslim) is incorrectly graded `matruk` — this is the grandfather's grade applied to the grandson
- **Rule**: Only use SemanticHadith for hadith text, isnad chains, and narrator *names/IDs* for matching purposes
- **For narrator bios and grading**: Use Tahdhib al-Tahdhib (Turath book 1278) or other verified classical sources
- This applies to any future ingestion pipeline — do not store or display SemanticHadith grading data
- **Enforced in code** (Apr 17 2026): `reliability_rating`, `reliability_source`, `ibn_hajar_rank` fields removed from DB schema, models, API responses. `evidence` and `scholarly_source` tables removed. Ingestion in `semantic.rs` skips these fields.
