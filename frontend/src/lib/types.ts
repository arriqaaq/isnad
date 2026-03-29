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
}

export interface ApiNarrator {
  id: string;
  name_ar: string | null;
  name_en: string;
  gender: string | null;
  generation: string | null;
  bio: string | null;
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
}
