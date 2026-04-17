import type {
  ApiBook,
  ApiHadithFamily,
  ApiMatnDiff,
  ApiReciter,
  ApiSurah,
  AyahHadithResponse,
  FamilyDetailResponse,
  GraphData,
  HadithDetailResponse,
  NarratorDetailResponse,
  PaginatedResponse,
  ApiHadith,
  ApiNarratorWithCount,
  SearchResponse,
  StatsResponse,
  QuranSearchResponse,
  QuranStatsResponse,
  SurahDetailResponse,
  UnifiedSearchResponse,
  ApiQuranWord,
  RootSearchResponse,
  AyahSimilarResponse,
  ApiPhraseWithAyahs,
  UserNote,
  CreateNoteRequest,
  UpdateNoteRequest,
  NoteRef,
  LinkPreview,
  NoteRefsIndicator,
  TurathBook,
  TurathBooksConfig,
  TurathBookDetail,
  TurathPagesResponse,
  TafsirSurahMappings,
  SharhBatchResponse,
  NarratorBookRef,
} from './types';
import { getDeviceId } from './stores/deviceId';

const BASE = '/api';

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`);
  if (!res.ok) throw new Error(`API error: ${res.status}`);
  return res.json();
}

export async function getStats(): Promise<StatsResponse> {
  return get('/stats');
}

export async function getBooks(): Promise<ApiBook[]> {
  return get('/books');
}

export async function searchAll(
  q: string,
  type: 'text' | 'semantic' = 'text',
  limit = 20
): Promise<SearchResponse> {
  return get(`/search?q=${encodeURIComponent(q)}&type=${type}&limit=${limit}`);
}

export async function getHadiths(params: {
  book?: number;
  number?: number;
  page?: number;
  limit?: number;
}): Promise<PaginatedResponse<ApiHadith>> {
  const sp = new URLSearchParams();
  if (params.book) sp.set('book', String(params.book));
  if (params.number) sp.set('number', String(params.number));
  if (params.page) sp.set('page', String(params.page));
  if (params.limit) sp.set('limit', String(params.limit));
  return get(`/hadiths?${sp}`);
}

export async function getHadith(id: string): Promise<HadithDetailResponse> {
  return get(`/hadiths/${encodeURIComponent(id)}`);
}

export async function getNarrators(params: {
  q?: string;
  page?: number;
  limit?: number;
  generation?: string;
}): Promise<PaginatedResponse<ApiNarratorWithCount>> {
  const sp = new URLSearchParams();
  if (params.q) sp.set('q', params.q);
  if (params.page) sp.set('page', String(params.page));
  if (params.limit) sp.set('limit', String(params.limit));
  if (params.generation) sp.set('generation', params.generation);
  return get(`/narrators?${sp}`);
}

export async function getNarrator(id: string): Promise<NarratorDetailResponse> {
  return get(`/narrators/${encodeURIComponent(id)}`);
}

export async function getChainGraph(hadithId: string): Promise<GraphData> {
  return get(`/chain/${encodeURIComponent(hadithId)}`);
}

export async function getNarratorGraph(id: string): Promise<GraphData> {
  return get(`/narrators/${encodeURIComponent(id)}/graph`);
}

export async function updateNarrator(
  id: string,
  data: Record<string, unknown>
): Promise<void> {
  const res = await fetch(`${BASE}/narrators/${encodeURIComponent(id)}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error(`API error: ${res.status}`);
}

// ── Analysis API ──

export async function getFamilies(params: {
  page?: number;
  limit?: number;
}): Promise<PaginatedResponse<ApiHadithFamily>> {
  const sp = new URLSearchParams();
  if (params.page) sp.set('page', String(params.page));
  if (params.limit) sp.set('limit', String(params.limit));
  return get(`/families?${sp}`);
}

export async function getFamily(id: string): Promise<FamilyDetailResponse> {
  return get(`/families/${encodeURIComponent(id)}`);
}

export async function getMustalahStats(): Promise<import('./types').MustalahStatsResponse> {
  return get('/analysis/stats');
}

export async function getMustalahFamily(id: string): Promise<import('./types').MustalahFamilyResponse> {
  return get(`/families/${encodeURIComponent(id)}/mustalah`);
}

export async function getNarratorIsnadRole(id: string): Promise<import('./types').NarratorIsnadRole> {
  return get(`/narrators/${encodeURIComponent(id)}/isnad-role`);
}

export async function getNarratorAssessments(id: string): Promise<{ narrator_id: string; assessments: import('./types').NarratorAssessment[]; sources_count: number }> {
  return get(`/narrators/${encodeURIComponent(id)}/reliability`);
}

export async function getMatnDiff(a: string, b: string): Promise<ApiMatnDiff> {
  return get(`/diff?a=${encodeURIComponent(a)}&b=${encodeURIComponent(b)}`);
}

// ── Quran API ──

export async function getQuranStats(): Promise<QuranStatsResponse> {
  return get('/quran/stats');
}

export async function getSurahs(): Promise<ApiSurah[]> {
  return get('/quran/surahs');
}

export async function getSurah(number: number): Promise<SurahDetailResponse> {
  return get(`/quran/surahs/${number}`);
}

export async function searchQuran(
  q: string,
  type: 'text' | 'semantic' | 'hybrid' | 'tafsir' = 'text',
  limit = 20,
  page = 1
): Promise<QuranSearchResponse> {
  return get(`/quran/search?q=${encodeURIComponent(q)}&type=${type}&limit=${limit}&page=${page}`);
}

export async function getAyahHadiths(
  surah: number,
  ayah: number,
  includeSemantic = false,
  semanticLimit = 5
): Promise<AyahHadithResponse> {
  const sp = new URLSearchParams();
  if (includeSemantic) sp.set('include_semantic', 'true');
  if (semanticLimit !== 5) sp.set('semantic_limit', String(semanticLimit));
  const query = sp.toString() ? `?${sp}` : '';
  return get(`/quran/ayah/${surah}:${ayah}/hadiths${query}`);
}

export async function getSurahHadithCounts(
  surah: number
): Promise<Record<string, number>> {
  return get(`/quran/surahs/${surah}/hadith-counts`);
}

export async function getSurahSimilarCounts(
  surah: number
): Promise<Record<string, number>> {
  return get(`/quran/surahs/${surah}/similar-counts`);
}

// ── Unified Quran & Sunnah API ──

export async function searchUnified(
  q: string,
  type: 'hybrid' | 'semantic' = 'hybrid',
  limit = 20,
  page = 1
): Promise<UnifiedSearchResponse> {
  return get(`/unified/search?q=${encodeURIComponent(q)}&type=${type}&limit=${limit}&page=${page}`);
}

// ── Quran Word Morphology API ──

export async function getAyahWords(surah: number, ayah: number): Promise<ApiQuranWord[]> {
  return get<ApiQuranWord[]>(`/quran/ayah/${surah}:${ayah}/words`);
}

export async function searchByRoot(root: string): Promise<RootSearchResponse> {
  return get<RootSearchResponse>(`/quran/search/root/${encodeURIComponent(root)}`);
}

// ── Quran Recitation API ──

export async function getReciters(): Promise<ApiReciter[]> {
  return get<ApiReciter[]>('/quran/reciters');
}

// ── Corpus Coranicum Manuscript API (browser-direct) ──

export async function getAyahManuscripts(surah: number, ayah: number): Promise<import('./types').CCManuscript[]> {
  const res = await fetch(`https://api.corpuscoranicum.de/api/data/manuscripts/sura/${surah}/verse/${ayah}`);
  if (!res.ok) return [];
  const data = await res.json();
  return data.data ?? [];
}

// ── Similar Ayahs / Mutashabihat API ──

export async function getAyahSimilar(surah: number, ayah: number): Promise<AyahSimilarResponse> {
  return get<AyahSimilarResponse>(`/quran/ayah/${surah}:${ayah}/similar`);
}

export async function getPhraseDetail(id: string): Promise<ApiPhraseWithAyahs> {
  return get<ApiPhraseWithAyahs>(`/quran/phrases/${encodeURIComponent(id)}`);
}

// ── Notes API ──

function deviceHeaders(): HeadersInit {
  return { 'X-Device-Id': getDeviceId() };
}

async function getWithDevice<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`, { headers: deviceHeaders() });
  if (!res.ok) throw new Error(`API error: ${res.status}`);
  return res.json();
}

async function mutateWithDevice<T>(
  method: 'POST' | 'PUT' | 'DELETE',
  path: string,
  data?: unknown,
): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json', ...deviceHeaders() },
    body: data ? JSON.stringify(data) : undefined,
  });
  if (!res.ok) throw new Error(`API error: ${res.status}`);
  if (method === 'DELETE') return undefined as T;
  return res.json();
}

export async function fetchNotesForRef(
  refType: string,
  refId: string,
): Promise<PaginatedResponse<UserNote>> {
  return getWithDevice(`/notes?ref_type=${encodeURIComponent(refType)}&ref_id=${encodeURIComponent(refId)}`);
}

export async function fetchNoteRefs(
  refType: string,
  refIds: string[],
): Promise<NoteRefsIndicator> {
  if (refIds.length === 0) return {};
  return getWithDevice(`/notes/refs?ref_type=${encodeURIComponent(refType)}&ref_ids=${refIds.map(encodeURIComponent).join(',')}`);
}

export async function fetchAllNotes(params?: {
  ref_type?: string;
  tag?: string;
  color?: string;
  q?: string;
  page?: number;
  limit?: number;
}): Promise<PaginatedResponse<UserNote>> {
  const sp = new URLSearchParams();
  if (params?.ref_type) sp.set('ref_type', params.ref_type);
  if (params?.tag) sp.set('tag', params.tag);
  if (params?.color) sp.set('color', params.color);
  if (params?.q) sp.set('q', params.q);
  if (params?.page) sp.set('page', String(params.page));
  if (params?.limit) sp.set('limit', String(params.limit));
  return getWithDevice(`/notes?${sp}`);
}

export async function fetchNoteTags(): Promise<string[]> {
  return getWithDevice('/notes/tags');
}

export async function fetchNote(id: string): Promise<UserNote> {
  return getWithDevice(`/notes/${encodeURIComponent(id)}`);
}

export async function createNote(data: CreateNoteRequest): Promise<UserNote> {
  return mutateWithDevice('POST', '/notes', data);
}

export async function updateNote(id: string, data: UpdateNoteRequest): Promise<UserNote> {
  return mutateWithDevice('PUT', `/notes/${encodeURIComponent(id)}`, data);
}

export async function deleteNote(id: string): Promise<void> {
  return mutateWithDevice('DELETE', `/notes/${encodeURIComponent(id)}`);
}

export async function addRefToNote(noteId: string, ref: NoteRef): Promise<UserNote> {
  return mutateWithDevice('PUT', `/notes/${encodeURIComponent(noteId)}/refs`, {
    action: 'add',
    ref,
  });
}

export async function removeRefFromNote(noteId: string, ref: NoteRef): Promise<UserNote> {
  return mutateWithDevice('PUT', `/notes/${encodeURIComponent(noteId)}/refs`, {
    action: 'remove',
    ref,
  });
}

export async function updateRefAnnotation(
  noteId: string,
  idx: number,
  annotation: string,
): Promise<UserNote> {
  return mutateWithDevice(
    'PUT',
    `/notes/${encodeURIComponent(noteId)}/refs/${idx}/annotation`,
    { annotation },
  );
}

export async function exportNotes(): Promise<UserNote[]> {
  return getWithDevice('/notes/export');
}

export async function fetchLinkPreview(url: string): Promise<LinkPreview> {
  return get(`/link-preview?url=${encodeURIComponent(url)}`);
}

// ── Turath Book Viewer ──

export async function getTurathBooksConfig(): Promise<TurathBooksConfig> {
  return get('/turath/books/config');
}

export async function getTurathBooks(): Promise<TurathBook[]> {
  return get('/turath/books');
}

export async function getTurathBook(bookId: number): Promise<TurathBookDetail> {
  return get(`/turath/books/${bookId}`);
}

export async function getTurathPages(bookId: number, start: number, size: number): Promise<TurathPagesResponse> {
  return get(`/turath/books/${bookId}/pages?start=${start}&size=${size}`);
}

export async function getSurahTafsirPages(surahNumber: number): Promise<TafsirSurahMappings> {
  return get(`/quran/surah/${surahNumber}/tafsir-pages`);
}

export async function getNarratorBooks(narratorId: string): Promise<NarratorBookRef[]> {
  return get(`/narrators/${encodeURIComponent(narratorId)}/books`);
}

export async function getHadithSharhPages(bookId: number, hadithNumbers: number[]): Promise<SharhBatchResponse> {
  const nums = hadithNumbers.join(',');
  return get(`/hadiths/sharh-pages?book=${bookId}&numbers=${nums}`);
}
