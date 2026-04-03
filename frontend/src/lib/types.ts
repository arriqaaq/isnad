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

export interface ApiClAnalysis {
  narrator_id: string;
  candidate_type: string;
  pcl_mode: string | null;
  fan_out: number;
  bundle_coverage: number;
  collector_diversity: number;
  structural_score: number;
  final_confidence: number;
  outcome: string;
  contradiction_cap_active: boolean;
  profile: string;
  family_status: string;
  rank: number;
}

export interface JuynbollAnalysis {
  has_reliable_bypass: boolean;
  reliable_bypass_count: number;
  max_reliable_bypass_ratio: number;
  has_independent_cls: boolean;
  independent_cl_pairs: number;
  cl_count: number;
  upstream_reliable_ratio: number;
  upstream_branching_points: number;
}

export interface JuynbollSummaryResponse {
  families_analyzed: number;
  families_with_reliable_bypass: number;
  families_with_independent_cls: number;
  cross_family_narrators: {
    narrator_id: string;
    cl_family_count: number;
    reliability_prior: number | null;
    reliability_rating: string | null;
  }[];
}

export interface NarratorClStatus {
  narrator_id: string;
  cl_family_count: number;
  pcl_family_count: number;
  families: string[];
}

export interface FamilyDetailResponse {
  family: ApiHadithFamily;
  hadiths: ApiHadith[];
  analysis: ApiClAnalysis[];
  juynboll: JuynbollAnalysis | null;
}

export interface AnalysisStatsResponse {
  family_count: number;
  candidate_count: number;
  cl_count: number;
  supported_count: number;
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

// ── Manuscript & Variant Reading types ──

export interface ApiManuscript {
  id: string;
  name: string;
  repository: string | null;
  location: string | null;
  date_range: string | null;
  material: string | null;
  script_type: string | null;
  description: string | null;
}

export interface ApiVariantReading {
  id: string;
  surah_number: number;
  ayah_number: number;
  reader_name: string;
  reading_ar: string;
  standard_ar: string | null;
  source: string | null;
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
