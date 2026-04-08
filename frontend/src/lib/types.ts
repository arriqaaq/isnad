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
  reliability_prior: number | null;
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

// ── Mustalah analysis types ──

export interface ChainAssessment {
  variant_id: string;
  continuity: string;
  chain_grade: string;
  weakest_narrator_id: string | null;
  weakest_rating: string | null;
  weakest_prior: number | null;
  narrator_count: number;
  has_chronology_conflict: boolean;
  has_majhul_narrator: boolean;
}

export interface IsnadAnalysis {
  composite_grade: string | null;
  best_chain_grade: string | null;
  breadth_class: string | null;
  min_breadth: number | null;
  bottleneck_tabaqah: number | null;
  sahabi_count: number | null;
  mutabaat_count: number | null;
  shawahid_count: number | null;
  reliable_mutabaat_count: number | null;
  corroboration_strength: string | null;
  matn_coherence: number | null;
  chain_count: number | null;
  sahih_chain_count: number | null;
  hasan_chain_count: number | null;
  daif_chain_count: number | null;
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

export interface MustalahFamilyResponse {
  analysis: IsnadAnalysis | null;
  chains: ChainAssessment[];
  pivots: PivotNarrator[];
}

export interface MustalahStatsResponse {
  family_count: number;
  analyzed_count: number;
  sahih_count: number;
  hasan_count: number;
  daif_count: number;
  mutawatir_count: number;
  mashhur_count: number;
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
