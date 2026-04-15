export interface ApiHadith {
  id: string;
  hadith_number: number;
  book_id: number;
  chapter_id: number;
  text_ar: string | null;
  text_en: string | null;
  narrator_text: string | null;
  grade: string | null;
  book_name: string | null;
  matn: string | null;
  hadith_type: string | null;
  topics: string[] | null;
  quran_verses: string[] | null;
  chapter_name: string | null;
}

export interface ApiNarrator {
  id: string;
  name_ar: string | null;
  name_en: string;
  gender: string | null;
  generation: string | null;
  bio: string | null;
  kunya: string | null;
  aliases: string[] | null;
  birth_year: number | null;
  birth_calendar: string | null;
  death_year: number | null;
  death_calendar: string | null;
  locations: string[] | null;
  tags: string[] | null;
  reliability_rating: string | null;
  reliability_source: string | null;
  ibn_hajar_rank: string | null;
}

export interface ApiBook {
  id: string;
  book_number: number;
  name_en: string;
  name_ar: string | null;
}

export interface ApiNarratorWithCount {
  id: string;
  name_ar: string | null;
  name_en: string;
  generation: string | null;
  bio: string | null;
  kunya: string | null;
  death_year: number | null;
  reliability_rating: string | null;
  hadith_count: number;
}

export interface ApiHadithSearchResult {
  id: string;
  hadith_number: number;
  book_id: number;
  text_ar: string | null;
  text_en: string | null;
  narrator_text: string | null;
  score: number | null;
}

export interface ApiNarratorSearchResult {
  id: string;
  name_ar: string | null;
  name_en: string;
  generation: string | null;
  hadith_count: number | null;
}

export interface PaginatedResponse<T> {
  data: T[];
  page: number;
  has_more: boolean;
}

export interface StatsResponse {
  hadith_count: number;
  narrator_count: number;
  book_count: number;
}

export interface SearchResponse {
  query: string;
  search_type: string;
  hadiths: ApiHadithSearchResult[];
  narrators: ApiNarratorSearchResult[];
}

export interface HadithDetailResponse {
  hadith: ApiHadith;
  narrators: ApiNarrator[];
  linked_ayahs: ApiAyah[];
  similar_hadiths: ApiHadith[];
}

export interface NarratorDetailResponse {
  narrator: ApiNarrator;
  hadiths: ApiHadith[];
  teachers: ApiNarrator[];
  students: ApiNarrator[];
}

export interface GraphNodeData {
  id: string;
  label: string;
  label_en: string;
  type: string;
  generation: string | null;
}

export interface GraphEdgeData {
  id: string;
  source: string;
  target: string;
  label: string;
}

export interface GraphData {
  nodes: { data: GraphNodeData }[];
  edges: { data: GraphEdgeData }[];
  total_teachers?: number;
  total_students?: number;
}

// ── Analysis types ──

export interface ApiHadithFamily {
  id: string;
  family_label: string | null;
  variant_count: number | null;
}

export interface FamilyDetailResponse {
  family: ApiHadithFamily;
  hadiths: ApiHadith[];
}

// ── Glossary types ──

export interface GlossaryTerm {
  id: string;
  term_en: string;
  term_ar: string;
  literal_meaning: string | null;
  technical_definition: string;
  conditions: string[] | null;
  ruling: string | null;
  category: string;
  page: number | null;
  related_terms: string[] | null;
}

// ── Mustalah analysis types (structural, no computed grades) ──

export interface ChainAssessment {
  variant_id: string;
  continuity: string;
  narrator_count: number;
  has_chronology_conflict: boolean;
  narrator_ids: string[] | null;
}

export interface IsnadAnalysis {
  breadth_class: string | null;
  min_breadth: number | null;
  bottleneck_tabaqah: number | null;
  sahabi_count: number | null;
  mutabaat_count: number | null;
  shawahid_count: number | null;
  chain_count: number | null;
  ilal_flags: string[] | null;
}

export interface PivotNarrator {
  narrator_id: string;
  bundle_coverage: number | null;
  fan_out: number | null;
  collector_diversity: number | null;
  bypass_count: number | null;
  is_bottleneck: boolean | null;
}

export interface NarratorAssessment {
  scholar: string;
  work: string;
  citation_text: string;
  rating: string | null;
  source_locator: string | null;
}

export interface MustalahFamilyResponse {
  analysis: IsnadAnalysis | null;
  chains: ChainAssessment[];
  pivots: PivotNarrator[];
}

export interface MustalahStatsResponse {
  family_count: number;
  analyzed_count: number;
  mutawatir_count: number;
  mashhur_count: number;
  aziz_count: number;
  gharib_count: number;
  evidence_count: number;
}

export interface NarratorIsnadRole {
  narrator_id: string;
  pivot_family_count: number;
  bottleneck_family_count: number;
  families: string[];
}

export interface DiffSegment {
  text: string;
  kind: 'Unchanged' | 'Added' | 'Missing';
}

export interface ApiMatnDiff {
  hadith_a: string;
  hadith_b: string;
  segments_a: DiffSegment[];
  segments_b: DiffSegment[];
  similarity_ratio: number;
}

// ── Quran types ──

export interface ApiSurah {
  id: string;
  surah_number: number;
  name_ar: string;
  name_en: string;
  name_translit: string;
  revelation_type: string;
  ayah_count: number;
}

export interface ApiAyah {
  id: string;
  surah_number: number;
  ayah_number: number;
  text_ar: string;
  text_en: string | null;
  tafsir_en: string | null;

}

export interface ApiAyahSearchResult {
  id: string;
  surah_number: number;
  ayah_number: number;
  text_ar: string;
  text_en: string | null;
  tafsir_en: string | null;

  score: number | null;
}

export interface QuranSearchResponse {
  query: string;
  search_type: string;
  ayahs: ApiAyahSearchResult[];
  page: number;
  has_more: boolean;
}

export interface QuranStatsResponse {
  surah_count: number;
  ayah_count: number;
}

export interface SurahDetailResponse {
  surah: ApiSurah;
  ayahs: ApiAyah[];
}

export interface AyahHadithResponse {
  curated: ApiHadith[];
  related: ApiHadithSearchResult[] | null;
}

// ── Unified Quran & Sunnah types ──

export interface UnifiedSearchItemQuran {
  source: 'quran';
  id: string;
  surah_number: number;
  ayah_number: number;
  text_ar: string;
  text_en: string | null;
  tafsir_en: string | null;

  score: number | null;
  unified_score: number;
}

export interface UnifiedSearchItemHadith {
  source: 'hadith';
  id: string;
  hadith_number: number;
  book_id: number;
  text_ar: string | null;
  text_en: string | null;
  narrator_text: string | null;
  score: number | null;
  unified_score: number;
}

export type UnifiedSearchItem = UnifiedSearchItemQuran | UnifiedSearchItemHadith;

export interface UnifiedSearchResponse {
  query: string;
  search_type: string;
  results: UnifiedSearchItem[];
  quran_count: number;
  hadith_count: number;
  page: number;
  has_more: boolean;
}

// ── Quran Word Morphology ──

export interface ApiQuranWord {
  id: string;
  surah_number: number;
  ayah_number: number;
  word_position: number;
  text_ar: string;
  pos: string;
  root: string | null;
  lemma: string | null;
  translation: string | null;
  transliteration: string | null;
  features: Record<string, string> | null;
  segments: { pos: string; text?: string; features?: string }[] | null;
}

export interface RootSearchResponse {
  root: string;
  occurrences: ApiQuranWord[];
  ayah_count: number;
}

export interface ApiReciter {
  id: string;
  name_en: string;
  name_ar: string | null;
  style: string | null;
  folder_name: string;
  bitrate: string | null;
}

// ── Corpus Coranicum Manuscript types ──

export interface CCManuscriptImage {
  image_url: string;
}
export interface CCManuscriptPage {
  folio: number;
  side: string;
  images: CCManuscriptImage[];
}
export interface CCManuscript {
  manuscript_id: number;
  title: string;
  archive: { city: string; name: string };
  pages: CCManuscriptPage[];
}

// ── Similar Ayahs / Mutashabihat ──

export interface ApiSimilarAyah {
  ayah_key: string;
  score: number;
  coverage: number;
  matched_positions: number[][] | null;
  text_ar: string | null;
  text_en: string | null;
}

export interface ApiPhraseWithAyahs {
  id: string;
  text_ar: string;
  occurrence: number;
  ayah_keys: string[];
}

export interface AyahSimilarResponse {
  similar: ApiSimilarAyah[];
  phrases: ApiPhraseWithAyahs[];
}

// ── User Notes ──

export interface NoteRef {
  ref_type: 'ayah' | 'hadith' | 'narrator';
  ref_id: string;
  annotation?: string;
}

export interface UserNote {
  id: string;
  ref_type: 'ayah' | 'hadith' | 'topic';
  ref_id: string | null;
  title: string | null;
  content: string;
  color: 'yellow' | 'green' | 'blue' | 'pink' | 'purple';
  tags: string[];
  refs: NoteRef[];
  created_at: string;
  updated_at: string;
}

export interface CreateNoteRequest {
  ref_type: 'ayah' | 'hadith' | 'topic';
  ref_id?: string;
  title?: string;
  content?: string;
  color?: string;
  tags?: string[];
  refs?: NoteRef[];
}

export interface UpdateNoteRequest {
  title?: string;
  content?: string;
  color?: string;
  tags?: string[];
  refs?: NoteRef[];
}

export interface LinkPreview {
  url: string;
  title: string | null;
  description: string | null;
  image: string | null;
  domain: string | null;
}

export interface NoteRefsIndicator {
  [refId: string]: { color: string; count: number };
}

// ── Turath Book Viewer ──

export interface TurathBook {
  book_id: number;
  name_ar: string;
  name_en: string;
  author_ar: string;
  total_pages: number;
}

export interface TurathBookDetail extends TurathBook {
  headings: TurathHeading[];
}

export interface TurathHeading {
  title: string;
  level: number;
  page_index: number;
}

export interface TurathPage {
  page_index: number;
  text: string;
  vol: string;
  page_num: number;
}

export interface TurathPagesResponse {
  pages: TurathPage[];
  total: number;
  start: number;
  size: number;
}

export interface TafsirPageRef {
  page_index: number;
  heading: string | null;
}

export interface TafsirSurahMappings {
  mappings: Record<string, TafsirPageRef>;
}

export interface SharhPageRef {
  sharh_book_id: number;
  page_index: number;
  book_name: string;
}

export interface SharhBatchResponse {
  mappings: Record<string, SharhPageRef>;
}

export interface NarratorBookRef {
  turath_book_id: number;
  page_index: number;
  entry_num: number | null;
  book_name: string;
}
