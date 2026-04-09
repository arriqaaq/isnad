#!/usr/bin/env python3
"""
Generate instruction-tuning data for the hadith-scholar model.

Uses SemanticHadith KG (data/semantic_hadith.json) as the primary source:
  - 6,786 narrators with reliability grades, generation, biographical data
  - 34,011 hadiths with full narrator chains, Arabic + English text
  - Teacher-student relationships derived from chains

Also uses:
  - data/quran.csv (Quran verses + Tafsir Ibn Kathir)

NO SurrealDB dependency — everything comes from raw files.

6 training categories:
  1. Hadith science terminology (hardcoded scholarly definitions)
  2. Narrator chain analysis (from KG narrator data + chains)
  3. Isnad structural analysis (from KG chain patterns)
  4. Hadith RAG Q&A (Ollama-generated from KG hadiths)
  5. Quran + Tafsir (direct from quran.csv)
  6. Cross-domain hadith↔Quran (from KG references)

Usage:
  python3 scripts/prepare_training_data.py [--ollama-url URL] [--model MODEL] [--workers N]
"""

import csv
import json
import os
import random
import re
import sys
import time
import argparse
import urllib.request
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed

csv.field_size_limit(sys.maxsize)

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data")
KG_PATH = os.path.join(DATA_DIR, "semantic_hadith.json")
QURAN_PATH = os.path.join(DATA_DIR, "quran.csv")
TRAIN_OUTPUT = os.path.join(DATA_DIR, "train.jsonl")
VALID_OUTPUT = os.path.join(DATA_DIR, "valid.jsonl")

DEFAULT_OLLAMA_URL = "http://localhost:11434"
DEFAULT_MODEL = "command-r7b-arabic"

BOOK_ENGLISH = {
    "SB": "Sahih al-Bukhari",
    "SM": "Sahih Muslim",
    "SD": "Sunan Abi Dawud",
    "SN": "Sunan an-Nasa'i",
    "JT": "Jami' at-Tirmidhi",
    "IM": "Sunan Ibn Majah",
}

# System prompt matching src/rag.rs
SYSTEM_PROMPT_TEMPLATE = (
    "You are a knowledgeable Islamic scholar assistant specializing in hadith sciences.\n"
    "Answer questions using ONLY the hadiths provided below as context.\n"
    "Always cite the hadith number when referencing a hadith.\n"
    "When relevant, mention the chain of narration (isnad) to support authenticity.\n"
    "If the context doesn't contain relevant information, say so honestly.\n"
    "Be concise and accurate.\n\n"
    "## Relevant Hadiths:\n\n{context}"
)

GRADE_ENGLISH = {
    "thiqah": "trustworthy",
    "saduq": "truthful",
    "maqbul": "acceptable",
    "majhul": "unknown",
    "daif": "weak",
    "matruk": "abandoned",
}


# ---------------------------------------------------------------------------
# Load SemanticHadith KG
# ---------------------------------------------------------------------------

def load_kg() -> dict:
    """Load the SemanticHadith knowledge graph."""
    print(f"Loading {KG_PATH}...")
    if not os.path.exists(KG_PATH):
        print(f"  ERROR: {KG_PATH} not found.")
        print("  Run: make semantic-download && make semantic-extract")
        sys.exit(1)
    with open(KG_PATH, encoding="utf-8") as f:
        kg = json.load(f)
    print(f"  Narrators: {len(kg['narrators'])}")
    print(f"  Hadiths: {len(kg['hadiths'])}")
    print(f"  Books: {list(kg.get('bookNames', {}).keys())}")
    return kg


def resolve_chain(kg: dict, chain_ids: list[str]) -> list[dict]:
    """Resolve a chain of narrator IDs to full narrator records."""
    result = []
    for nid in chain_ids:
        nr = kg["narrators"].get(nid, {})
        result.append({
            "id": nid,
            "name": nr.get("popularName", nr.get("name", nid)),
            "generation": nr.get("generation", ""),
            "grade": nr.get("reliabilityGrade", ""),
            "ibn_hajar": nr.get("ibnHajarRank", ""),
            "teknonym": nr.get("teknonym", ""),
        })
    return result


def format_chain_str(chain: list[dict]) -> str:
    """Format resolved chain as readable string."""
    parts = []
    for n in chain:
        grade_en = GRADE_ENGLISH.get(n["grade"], n["grade"])
        gen = f"gen {n['generation']}" if n["generation"] else ""
        rating = f", {grade_en}" if grade_en else ""
        parts.append(f"{n['name']} ({gen}{rating})")
    return " → ".join(parts)


# ---------------------------------------------------------------------------
# Load Quran + Tafsir
# ---------------------------------------------------------------------------

HTML_TAG_RE = re.compile(r"<[^>]+>")


def load_quran() -> list[dict]:
    print("Loading quran.csv...")
    if not os.path.exists(QURAN_PATH):
        print(f"  WARNING: {QURAN_PATH} not found. Run: python3 scripts/prepare_quran_data.py")
        return []
    verses = []
    with open(QURAN_PATH, encoding="utf-8") as f:
        for row in csv.DictReader(f):
            tafsir = HTML_TAG_RE.sub("", row.get("tafsir_en", "")).strip()
            if tafsir and len(tafsir.split()) >= 30:
                verses.append({
                    "surah": int(row["surah"]),
                    "ayah": int(row["ayah"]),
                    "text_ar": row.get("text_ar", "").strip(),
                    "text_en": row.get("text_en", "").strip(),
                    "tafsir_en": tafsir,
                })
    print(f"  {len(verses)} verses with tafsir loaded")
    return verses


# ---------------------------------------------------------------------------
# Ollama helper
# ---------------------------------------------------------------------------

def ollama_generate(prompt: str, system: str, ollama_url: str, model: str) -> str:
    payload = json.dumps({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": prompt},
        ],
        "stream": False,
        "options": {"temperature": 0.7, "num_predict": 300},
    }).encode("utf-8")
    req = urllib.request.Request(
        f"{ollama_url}/api/chat", data=payload,
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            return data.get("message", {}).get("content", "").strip()
    except Exception:
        return ""


# ---------------------------------------------------------------------------
# Category 1: Hadith Science Terminology (~200 examples)
# Hardcoded scholarly definitions — no external data needed.
# ---------------------------------------------------------------------------

def generate_terminology() -> list[dict]:
    """Generate terminology Q&A from hadith sciences (mustalah al-hadith)."""
    print("\nGenerating Category 1: Hadith Science Terminology...")

    # Each entry: (question_variants, answer)
    TERMS = [
        # Chain continuity types
        (["What is a muttasil chain?", "Define muttasil in hadith sciences.", "What does muttasil mean in isnad classification?"],
         "A muttasil (متصل, connected) chain is one where every narrator heard directly from the prior narrator, with no missing links from the collector back to the source. This is the strongest form of chain continuity and is a necessary condition for a hadith to be classified as sahih (authentic)."),
        (["What is a munqati chain?", "Define munqati in hadith terminology.", "What makes a chain munqati?"],
         "A munqati (منقطع, broken) chain has one narrator missing in the middle of the isnad. For example, if narrator B claims to transmit from narrator D, but historically they never met and narrator C (who connects them) is missing, the chain is munqati. This is a form of disconnection (inqita') that weakens the hadith."),
        (["What is a mursal hadith?", "Define mursal in hadith sciences.", "When is a chain classified as mursal?"],
         "A mursal (مرسل) hadith is one where a Tabi'i (second generation scholar, one who met the Companions but not the Prophet) reports directly from the Prophet ﷺ without mentioning the Sahabi (Companion) who transmitted it. The missing Sahabi link makes it technically disconnected, though scholars differ on its acceptability since all Companions are considered trustworthy."),
        (["What is a muallaq hadith?", "Define muallaq in isnad terminology."],
         "A muallaq (معلق, suspended) hadith has one or more narrators missing at the beginning of the chain, near the collector/compiler. For example, when Imam al-Bukhari says 'it was narrated that...' without mentioning who narrated it to him. The chain appears 'hanging' from the middle rather than being anchored to the collector."),
        (["What is a mudal chain?", "Define mu'dal in hadith sciences."],
         "A mu'dal (معضل) chain has two or more consecutive narrators missing. This is worse than munqati (which has only one gap) because it indicates a larger break in the transmission. A mu'dal hadith is generally considered very weak."),

        # Breadth classifications
        (["What is mutawatir in hadith classification?", "Define mutawatir.", "When is a hadith considered mutawatir?"],
         "Mutawatir (متواتر, mass-transmitted) describes a hadith narrated by such a large number of narrators at every level of the chain (typically 10 or more at each tabaqah/generation) that it is inconceivable they could have all colluded to fabricate it. Mutawatir hadiths carry the highest level of certainty. Examples include the hadith about deeds being judged by intentions."),
        (["What is mashhur in hadith classification?", "Define mashhur."],
         "Mashhur (مشهور, well-known) describes a hadith transmitted by three or more narrators at each tabaqah (generation layer), but below the mutawatir threshold. It indicates wide transmission without reaching the level of mass-transmission that precludes fabrication."),
        (["What is aziz in hadith classification?", "Define aziz."],
         "Aziz (عزيز, rare) describes a hadith where at least two narrators transmit it at every level of the chain. It is stronger than gharib (where only one narrator exists at some level) but weaker than mashhur (three or more at each level)."),
        (["What is gharib in hadith classification?", "Define gharib.", "What makes a hadith gharib?"],
         "Gharib (غريب, isolated/strange) describes a hadith where only one narrator exists at some level of the chain. This single narrator is called the 'fard' (فرد). A gharib hadith is not automatically weak — it depends on the reliability of that lone narrator — but it lacks the corroboration of wider transmission."),

        # Narrator reliability ratings
        (["What does thiqah mean in narrator criticism?", "Define thiqah.", "What is a thiqah narrator?"],
         "Thiqah (ثقة, trustworthy) is the highest reliability rating for a hadith narrator. It means the narrator is both 'adl (morally upright) and dabit (possessing accurate memory/transmission). A chain consisting entirely of thiqah narrators is a key requirement for a sahih hadith. In Ibn Hajar's Taqrib al-Tahdhib, thiqah is often accompanied by qualifiers like hafidh (strong memory) or thabt (firm/precise)."),
        (["What does saduq mean in narrator criticism?", "What is a saduq narrator?"],
         "Saduq (صدوق, truthful) is the second level of reliability, below thiqah. A saduq narrator is honest and truthful but may have minor weaknesses in memory or precision. Hadiths narrated solely by saduq narrators are typically graded hasan (good) rather than sahih (authentic). When corroborated by other chains, they can be strengthened."),
        (["What does da'if mean in hadith grading?", "What makes a narrator da'if?"],
         "Da'if (ضعيف, weak) indicates a narrator whose memory, precision, or moral character has been questioned by hadith scholars. A hadith with a da'if narrator in its chain is graded as weak and cannot be used as primary evidence for legal rulings, though it may be cited for encouragement (targhib) according to some scholars. Multiple weak chains can sometimes strengthen each other."),
        (["What does matruk mean?", "What is a matruk narrator?"],
         "Matruk (متروك, abandoned) is a severe weakness rating indicating a narrator whom scholars have rejected. This is more serious than da'if — a matruk narrator may have been accused of lying (though not proven), extreme carelessness, or other disqualifying traits. Hadiths narrated solely through matruk narrators are generally rejected."),
        (["What does majhul mean in narrator criticism?"],
         "Majhul (مجهول, unknown) describes a narrator about whom little biographical information exists. There are two types: majhul al-'ayn (completely unknown — only one person narrated from them) and majhul al-hal (known by name but their reliability is not established). Hadiths from majhul narrators are considered weak because their trustworthiness cannot be verified."),
        (["What does maqbul mean in narrator grading?"],
         "Maqbul (مقبول, acceptable) is a conditional reliability rating. It means the narrator is accepted when corroborated by other narrations (mutaba'at or shawahid), but if they narrate alone without support, their hadith is considered weak (layyin). It's a middle ground between saduq and da'if."),

        # Tabaqat (generations)
        (["What are tabaqat in hadith sciences?", "Explain the generation system in hadith."],
         "Tabaqat (طبقات, generations/layers) is the classification of hadith narrators by chronological tiers. Generation 1 consists of the Sahabah (Companions of the Prophet ﷺ), who are all considered trustworthy by scholarly consensus. Generation 2 is the Tabi'in (Followers — those who met the Companions). Generation 3 is Tabi' al-Tabi'in (Followers of the Followers). Later generations continue numerically. This system helps detect chronological impossibilities in chains — if a narrator from generation 5 claims to transmit from generation 2, it signals a missing link."),
        (["What is the status of Sahabah in hadith narration?", "Are all Companions considered trustworthy?"],
         "The Sahabah (صحابة, Companions of the Prophet ﷺ) — generation 1 in the tabaqat system — are considered trustworthy (thiqah) by scholarly consensus (ijma'). This is based on Quranic praise of the Companions and the Prophet's commendation of his generation. Therefore, when evaluating an isnad, the Sahabi link is never questioned for reliability. The critical evaluation focuses on narrators from generation 2 (Tabi'in) onwards."),

        # Corroboration concepts
        (["What is mutaba'at in hadith sciences?", "Define mutaba'at."],
         "Mutaba'at (متابعات, supporting narrations) refers to the existence of alternative chains of transmission from the same original Sahabi (Companion) source. If a hadith is narrated through Abu Hurayra via chain A and also via chain B, the second chain provides mutaba'at (within-source corroboration). This can strengthen a weak hadith if the supporting chain is independent."),
        (["What is shawahid in hadith terminology?", "Define shahid/shawahid."],
         "Shawahid (شواهد, witnesses/testimonies; singular: shahid) are hadiths narrated from a different Sahabi (Companion) that convey the same meaning. Unlike mutaba'at (which is the same hadith through different paths from the same source), shawahid come from entirely different original sources. Finding shawahid for a hadith provides cross-source corroboration and can strengthen its overall reliability."),
        (["What is i'tibaar?", "Define i'tibaar in hadith methodology."],
         "I'tibaar (اعتبار, investigation/scrutiny) is the scholarly process of examining all available chains and parallel narrations for a hadith to determine if mutaba'at (supporting chains from the same source) or shawahid (corroborating narrations from other sources) exist. It is a critical step in hadith authentication — a hadith that appears weak through one chain may be strengthened through corroboration discovered during i'tibaar."),

        # Structural concepts
        (["What is a Common Link in isnad analysis?", "Define Common Link.", "What is madar al-isnad?"],
         "A Common Link (CL), also called madar al-isnad (مدار الإسناد, pivot of the chain), is a narrator through whom the majority of a hadith's transmission chains pass. If 90% of variants of a hadith all go through one narrator (e.g., al-Zuhri), that narrator is the Common Link. This concept, developed by G.H.A. Juynboll, is significant because: (1) it identifies the likely point of origin for wide dissemination, and (2) a reliable CL with high fan-out (many direct students) and bundle coverage suggests authentic transmission rather than fabrication."),
        (["What is fan-out in isnad analysis?"],
         "Fan-out refers to the number of direct students a narrator has for a particular hadith. A narrator with a fan-out of 5 means five different students independently narrated the hadith from them. High fan-out at the Common Link is a positive indicator — it's harder to fabricate a hadith that multiple independent students all agree they heard from the same teacher."),
        (["What is bundle coverage?", "What does bundle coverage measure?"],
         "Bundle coverage measures the fraction of a hadith family's variant texts that pass through a particular narrator. A narrator with bundle coverage of 0.92 appears in 92% of all known chains for that hadith. High bundle coverage identifies the Common Link — the central figure through whom most of the hadith's transmission converges. Coverage above 0.95 indicates a bottleneck."),
        (["What is bypass ratio in isnad analysis?"],
         "Bypass ratio measures how many hadith variants manage to reach both the ancestors (teachers) and descendants (students) of a narrator WITHOUT going through that narrator. A low bypass ratio for a Common Link means the narrator is truly essential to the transmission — almost no chain avoids them. A high bypass ratio suggests alternative paths exist, which reduces the narrator's centrality."),

        # Hadith grading
        (["What are the grades of hadith?", "How are hadiths classified by authenticity?"],
         "Hadiths are classified into several grades of authenticity: (1) Sahih (صحيح, authentic) — connected chain of thiqah narrators with no defects or anomalies. (2) Hasan (حسن, good) — similar to sahih but with narrators slightly below thiqah level (e.g., saduq). (3) Da'if (ضعيف, weak) — has some deficiency in the chain or narrator reliability. (4) Mawdu' (موضوع, fabricated) — proven to be falsely attributed to the Prophet. The first two grades are acceptable as evidence, while the latter two are not used for deriving legal rulings."),
        (["What is the difference between sahih and hasan?"],
         "The key difference is the narrator reliability level. A sahih hadith requires all narrators to be thiqah (trustworthy with strong memory), while a hasan hadith may have one or more narrators rated saduq (truthful but with slightly less precise memory). Both have connected chains and no anomalies. A hasan hadith can be elevated to sahih li-ghayrihi (authentic due to external support) when corroborated by other chains."),

        # Isnad vs matn
        (["What is the difference between isnad and matn?", "Define isnad and matn."],
         "The isnad (إسناد, chain of narration) is the list of narrators who transmitted the hadith from one generation to the next, going back to the Prophet ﷺ. The matn (متن, text/content) is the actual content of the hadith — what was said or done. A complete hadith has both: the isnad provides the authentication mechanism, while the matn is the actual teaching. Scholars evaluate both independently — a strong isnad with a problematic matn (e.g., contradicting the Quran) may still be rejected."),

        # Hadith family concept
        (["What is a hadith family?", "What are variant texts in hadith?"],
         "A hadith family is a group of hadith texts (variants) that convey the same or very similar teaching but may have been transmitted through different chains of narration. Variants arise because different narrators may have heard the same teaching at different times or worded it slightly differently. Analyzing a hadith as a family — examining all its variants together — reveals the full transmission picture: how many independent chains exist, which narrator is the Common Link, and whether the hadith has corroboration (mutaba'at and shawahid)."),
    ]

    examples = []
    for questions, answer in TERMS:
        # Generate multiple examples per term with different question phrasings
        for q in questions:
            examples.append({
                "messages": [
                    {"role": "user", "content": q},
                    {"role": "assistant", "content": answer},
                ]
            })
    random.shuffle(examples)
    print(f"  Done: {len(examples)} terminology examples")
    return examples


# ---------------------------------------------------------------------------
# Category 2: Narrator Chain Analysis (~300 examples)
# Uses SemanticHadith KG narrator data + chains.
# ---------------------------------------------------------------------------

def _build_chain_analysis_answer(chain: list[dict], book_en: str, ref_no: int) -> str:
    """Build a chain analysis answer from resolved narrator data."""
    # Analyze each narrator
    narrator_lines = []
    weak_links = []
    strong_links = []
    gen_1_count = 0

    for i, n in enumerate(chain):
        grade = n["grade"]
        grade_en = GRADE_ENGLISH.get(grade, grade or "unknown")
        gen = n["generation"]
        ibn_hajar = n["ibn_hajar"]

        if gen == "1":
            gen_1_count += 1
            narrator_lines.append(
                f"- {n['name']}: Generation 1 (Sahabi/Companion), trustworthy by scholarly consensus (ijma')."
            )
            strong_links.append(n["name"])
        else:
            gen_label = f"Generation {gen}" if gen else "unknown generation"
            if ibn_hajar:
                narrator_lines.append(
                    f"- {n['name']}: {gen_label}, rated {grade_en}. Ibn Hajar says: \"{ibn_hajar[:80]}\"."
                )
            else:
                narrator_lines.append(f"- {n['name']}: {gen_label}, rated {grade_en}.")

            if grade in ("thiqah",):
                strong_links.append(n["name"])
            elif grade in ("daif", "matruk", "majhul"):
                weak_links.append((n["name"], grade_en))

    # Overall assessment
    if weak_links:
        weak_str = ", ".join(f"{name} ({grade})" for name, grade in weak_links)
        overall = f"This chain has weakness due to: {weak_str}. The hadith through this particular chain would be graded as da'if (weak)."
    elif all(n["grade"] in ("thiqah", "") and n["generation"] == "1" or n["grade"] == "thiqah" for n in chain):
        overall = "All narrators in this chain are rated thiqah (trustworthy). This is a strong (sahih) chain."
    else:
        overall = "This chain has mixed reliability ratings. Further corroboration (mutaba'at or shawahid) would strengthen it."

    # Check continuity via generation gaps
    gens = [int(n["generation"]) for n in chain if n["generation"].isdigit()]
    continuity = "muttasil (connected)" if gens == sorted(gens) else "possibly contains gaps"

    return (
        f"Hadith #{ref_no} from {book_en} has a chain of {len(chain)} narrators:\n\n"
        + "\n".join(narrator_lines)
        + f"\n\nThe chain appears {continuity} with {len(chain)} links. "
        + overall
    )


def generate_narrator_analysis(kg: dict, target: int = 300) -> list[dict]:
    """Generate narrator chain analysis examples from KG data."""
    print(f"\nGenerating Category 2: Narrator Chain Analysis (target: {target})...")
    examples = []

    hadith_ids = list(kg["hadiths"].keys())
    random.shuffle(hadith_ids)

    for hid in hadith_ids:
        if len(examples) >= target:
            break
        h = kg["hadiths"][hid]
        chain_ids = h.get("chain", [])
        if len(chain_ids) < 3:
            continue
        text_en = h.get("textEn", "")
        if not text_en:
            continue

        chain = resolve_chain(kg, chain_ids)
        # Skip if most narrators have no grade
        graded = sum(1 for n in chain if n["grade"])
        if graded < len(chain) * 0.5:
            continue

        book_code = h.get("book", "")
        book_en = BOOK_ENGLISH.get(book_code, h.get("bookName", book_code))
        ref_no = h.get("refNo", 0)

        chain_str = format_chain_str(chain)
        context = (
            f"Hadith #{ref_no} — {chain[0]['name']}\n"
            f"Chain of narration: {chain_str}\n"
            f"{text_en[:500]}\n"
        )
        system_prompt = SYSTEM_PROMPT_TEMPLATE.format(context=context)

        questions = [
            "Assess the reliability of this chain of narration.",
            "How reliable is the isnad of this hadith?",
            "What can you tell me about the narrators in this chain?",
            "Evaluate the transmission chain of this narration.",
        ]
        question = random.choice(questions)
        answer = _build_chain_analysis_answer(chain, book_en, ref_no)

        examples.append({
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": question},
                {"role": "assistant", "content": answer},
            ]
        })

    print(f"  Done: {len(examples)} narrator analysis examples")
    return examples


# ---------------------------------------------------------------------------
# Category 3: Isnad Structural Analysis (~200 examples)
# Analyze chain patterns across hadith families (from KG similar fields).
# ---------------------------------------------------------------------------

def generate_isnad_structural(kg: dict, target: int = 200) -> list[dict]:
    """Generate structural analysis examples from chain patterns in the KG."""
    print(f"\nGenerating Category 3: Isnad Structural Analysis (target: {target})...")
    examples = []

    # Group hadiths that share narrators (simple family approximation)
    # Use the KG's chain data to find shared Common Links
    narrator_hadith_count: dict[str, int] = defaultdict(int)
    narrator_to_hadiths: dict[str, list[str]] = defaultdict(list)

    for hid, h in kg["hadiths"].items():
        for nid in h.get("chain", []):
            narrator_hadith_count[nid] += 1
            if len(narrator_to_hadiths[nid]) < 20:  # cap for memory
                narrator_to_hadiths[nid].append(hid)

    # Find narrators who appear in many chains (potential Common Links)
    common_links = [
        (nid, count)
        for nid, count in narrator_hadith_count.items()
        if count >= 10  # appears in 10+ hadiths
    ]
    common_links.sort(key=lambda x: -x[1])
    random.shuffle(common_links[:200])  # shuffle top CLs

    for nid, count in common_links[:target]:
        if len(examples) >= target:
            break
        nr = kg["narrators"].get(nid, {})
        name = nr.get("popularName", nr.get("name", nid))
        gen = nr.get("generation", "?")
        grade = nr.get("reliabilityGrade", "")
        grade_en = GRADE_ENGLISH.get(grade, grade)

        # Sample some hadiths this narrator appears in
        sample_hids = narrator_to_hadiths[nid][:5]
        books_involved = set()
        chain_positions = []
        for shid in sample_hids:
            sh = kg["hadiths"].get(shid, {})
            books_involved.add(BOOK_ENGLISH.get(sh.get("book", ""), sh.get("bookName", "")))
            chain = sh.get("chain", [])
            if nid in chain:
                chain_positions.append(chain.index(nid))

        avg_pos = sum(chain_positions) / len(chain_positions) if chain_positions else 0

        system_prompt = (
            "You are an expert in mustalah al-hadith (hadith methodology) and isnad analysis.\n"
            f"\n## Narrator Profile:\n\n"
            f"Name: {name}\n"
            f"Generation: {gen}\n"
            f"Reliability: {grade_en} ({grade})\n"
            f"Ibn Hajar: {nr.get('ibnHajarRank', 'N/A')[:100]}\n"
            f"Appears in: {count} hadith chains\n"
            f"Books: {', '.join(books_involved)}\n"
            f"Average chain position: {avg_pos:.1f} (0=collector end, higher=source end)\n"
        )

        question = random.choice([
            f"What role does {name} play as a narrator in hadith transmission?",
            f"Analyze the significance of {name} in hadith chains.",
            f"Is {name} a Common Link? What does their transmission pattern suggest?",
        ])

        # Build answer
        role = "a Common Link (madar al-isnad)" if count >= 50 else "a frequently-occurring narrator" if count >= 20 else "a notable narrator"
        answer = (
            f"{name} is {role} in hadith transmission, appearing in {count} chains across {', '.join(books_involved)}. "
            f"They belong to generation {gen} and are rated {grade_en} ({grade}) by Ibn Hajar"
        )
        if nr.get("ibnHajarRank"):
            answer += f", who describes them as: \"{nr['ibnHajarRank'][:100]}\""
        answer += ". "

        if count >= 50:
            answer += (
                f"With {count} hadith chains passing through them, {name} serves as a critical transmission hub. "
                f"Their high fan-out (many students transmitting from them) is typical of a Common Link. "
                f"The reliability of {name} is therefore crucial — if they are trustworthy, the hadiths "
                f"transmitted through them carry significant weight. "
            )
        if grade == "thiqah":
            answer += f"Their thiqah rating makes them a strong and reliable link in transmission."
        elif grade in ("daif", "matruk"):
            answer += f"However, their {grade_en} rating raises concerns about hadiths that depend solely on their transmission."

        examples.append({
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": question},
                {"role": "assistant", "content": answer},
            ]
        })

    print(f"  Done: {len(examples)} isnad structural examples")
    return examples


# ---------------------------------------------------------------------------
# Category 4: Hadith RAG Q&A (~300 examples, uses Ollama)
# ---------------------------------------------------------------------------

def _build_rag_task(kg: dict, hids: list[str]) -> dict | None:
    """Build a RAG Q&A task from a group of hadiths."""
    context_parts = []
    for hid in hids:
        h = kg["hadiths"].get(hid, {})
        text = h.get("textEn", "")
        if not text:
            continue
        chain = resolve_chain(kg, h.get("chain", []))
        narrator = chain[0]["name"] if chain else "Unknown"
        chain_str = format_chain_str(chain) if len(chain) > 1 else ""
        part = f"Hadith #{h.get('refNo', 0)} — {narrator}\n"
        if chain_str:
            part += f"Chain of narration: {chain_str}\n"
        part += f"{text[:400]}\n"
        context_parts.append(part)

    if len(context_parts) < 2:
        return None

    context = "\n".join(context_parts)
    system_prompt = SYSTEM_PROMPT_TEMPLATE.format(context=context)

    questions = [
        "What do these hadiths teach us?",
        "Summarize the key teachings from these narrations.",
        "What guidance can be derived from these hadiths?",
        "Explain the main points of these hadiths, citing the hadith numbers.",
    ]
    return {"system_prompt": system_prompt, "context": context, "question": random.choice(questions)}


def _generate_one_rag(task: dict, ollama_url: str, model: str) -> dict | None:
    gen_system = (
        "You are a knowledgeable Islamic scholar. Answer using ONLY the hadiths provided. "
        "Cite hadith numbers. Mention narrators. Be concise (150-300 words)."
    )
    answer = ollama_generate(
        f"Context:\n{task['context']}\n\nQuestion: {task['question']}\n\nProvide a scholarly answer citing hadith numbers.",
        gen_system, ollama_url, model,
    )
    if not answer or not re.search(r"#?\d+", answer):
        return None
    return {
        "messages": [
            {"role": "system", "content": task["system_prompt"]},
            {"role": "user", "content": task["question"]},
            {"role": "assistant", "content": answer},
        ]
    }


def generate_hadith_rag(kg: dict, ollama_url: str, model: str, target: int = 300, workers: int = 4) -> list[dict]:
    """Generate hadith RAG Q&A via parallel Ollama calls."""
    print(f"\nGenerating Category 4: Hadith RAG Q&A (target: {target}, workers: {workers})...")

    # Group hadiths by book for sampling
    by_book: dict[str, list[str]] = defaultdict(list)
    for hid, h in kg["hadiths"].items():
        if h.get("textEn"):
            by_book[h.get("book", "")].append(hid)

    # Build tasks
    tasks = []
    for book, hids in by_book.items():
        random.shuffle(hids)
        for i in range(0, len(hids) - 3, 3):
            if len(tasks) >= target + 50:
                break
            task = _build_rag_task(kg, hids[i:i + random.randint(2, 4)])
            if task:
                tasks.append(task)

    random.shuffle(tasks)
    tasks = tasks[:target + 50]

    # Parallel Ollama calls
    examples = []
    done = 0
    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = {pool.submit(_generate_one_rag, t, ollama_url, model): t for t in tasks}
        for future in as_completed(futures):
            done += 1
            result = future.result()
            if result:
                examples.append(result)
            if done % 25 == 0:
                print(f"  Progress: {done}/{len(tasks)} calls, {len(examples)} examples")
            if len(examples) >= target:
                for f in futures:
                    f.cancel()
                break

    print(f"  Done: {len(examples)} hadith RAG examples")
    return examples[:target]


# ---------------------------------------------------------------------------
# Category 5: Quran + Tafsir (~200 examples, no Ollama)
# ---------------------------------------------------------------------------

def _truncate_tafsir(tafsir: str, max_words: int = 500) -> str:
    words = tafsir.split()
    if len(words) <= max_words:
        return tafsir
    truncated = " ".join(words[:max_words])
    last_period = truncated.rfind(".")
    if last_period > len(truncated) // 2:
        truncated = truncated[:last_period + 1]
    return truncated


def generate_quran_tafsir(verses: list[dict], target: int = 200) -> list[dict]:
    """Generate Quran tafsir Q&A using tafsir text directly."""
    print(f"\nGenerating Category 5: Quran Tafsir Q&A (target: {target})...")
    examples = []
    random.shuffle(verses)

    questions = [
        "What is the meaning of this verse?",
        "Explain the tafsir of this Quranic verse.",
        "What does this verse teach us?",
        "Provide the scholarly interpretation of this verse.",
    ]

    for verse in verses:
        if len(examples) >= target:
            break
        context = (
            f"Surah {verse['surah']}, Ayah {verse['ayah']}:\n"
            f"Arabic: {verse['text_ar']}\n"
            f"English: {verse['text_en']}\n"
        )
        system_prompt = (
            "You are a knowledgeable Islamic scholar assistant specializing in Quranic exegesis (tafsir).\n"
            "Answer questions using the provided verse and scholarly interpretation.\n"
            "Always reference the surah and ayah number.\nBe concise and accurate.\n\n"
            f"## Quranic Verse:\n\n{context}"
        )
        tafsir = _truncate_tafsir(verse["tafsir_en"])
        answer = (
            f"Regarding Surah {verse['surah']}, Ayah {verse['ayah']} — "
            f"\"{verse['text_en'][:100]}{'...' if len(verse['text_en']) > 100 else ''}\"\n\n"
            f"{tafsir}"
        )
        examples.append({
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": random.choice(questions)},
                {"role": "assistant", "content": answer},
            ]
        })

    print(f"  Done: {len(examples)} tafsir examples")
    return examples


# ---------------------------------------------------------------------------
# Category 6: Cross-Domain Hadith↔Quran (~100 examples, no Ollama)
# Uses KG's quranVerses field to connect hadiths to Quran.
# ---------------------------------------------------------------------------

def generate_cross_domain(kg: dict, verses_lookup: dict, target: int = 100) -> list[dict]:
    """Generate cross-domain examples linking hadiths to Quran verses."""
    print(f"\nGenerating Category 6: Cross-Domain Hadith↔Quran (target: {target})...")
    examples = []

    # Find hadiths that reference Quran verses
    candidates = []
    for hid, h in kg["hadiths"].items():
        qv = h.get("quranVerses", [])
        if qv and h.get("textEn"):
            candidates.append((hid, h, qv))

    random.shuffle(candidates)

    for hid, h, qv_refs in candidates:
        if len(examples) >= target:
            break

        chain = resolve_chain(kg, h.get("chain", []))
        book_en = BOOK_ENGLISH.get(h.get("book", ""), h.get("bookName", ""))

        # Build hadith context
        hadith_context = f"Hadith #{h.get('refNo', 0)} from {book_en}\n"
        if chain:
            hadith_context += f"Chain: {format_chain_str(chain[:4])}\n"
        hadith_context += f"{h.get('textEn', '')[:400]}\n"

        # Add referenced Quran verse if we have it
        verse_context = ""
        for ref in qv_refs[:2]:
            # Parse ref like "CH096_V001" (Chapter 96, Verse 1)
            if isinstance(ref, str):
                m = re.match(r"CH(\d+)_V(\d+)", ref)
                if m:
                    s, a = int(m.group(1)), int(m.group(2))
                    v = verses_lookup.get((s, a))
                    if v:
                        verse_context += f"\nQuran Surah {s}, Ayah {a}: {v['text_en']}\n"

        if not verse_context:
            continue

        system_prompt = (
            "You are a knowledgeable Islamic scholar assistant.\n"
            "Explain the connection between the hadith and Quranic verse provided.\n"
            "Cite both the hadith number and surah:ayah reference.\n\n"
            f"## Hadith:\n{hadith_context}\n## Referenced Quran Verse:{verse_context}"
        )

        question = random.choice([
            "How does this hadith relate to the Quranic verse?",
            "Explain the connection between this narration and the Quran.",
            "What is the relationship between this hadith and the referenced verse?",
        ])

        answer = (
            f"Hadith #{h.get('refNo', 0)} from {book_en} relates to the Quranic verse(s) referenced. "
            f"The hadith, narrated by {chain[-1]['name'] if chain else 'unknown'}, "
            f"provides practical elaboration on the Quranic teaching. "
            f"The Quran establishes the principle while the hadith demonstrates its application "
            f"through the Prophet's words or actions. This connection between Quran and Sunnah "
            f"is fundamental to Islamic jurisprudence (fiqh) — the Quran provides the foundation "
            f"and the hadith provides detailed guidance for implementation."
        )

        examples.append({
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": question},
                {"role": "assistant", "content": answer},
            ]
        })

    print(f"  Done: {len(examples)} cross-domain examples")
    return examples


# ---------------------------------------------------------------------------
# Write output
# ---------------------------------------------------------------------------

def write_jsonl(examples: list[dict], path: str) -> None:
    with open(path, "w", encoding="utf-8") as f:
        for ex in examples:
            f.write(json.dumps(ex, ensure_ascii=False) + "\n")
    print(f"  Wrote {len(examples)} examples to {path}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Generate training data for hadith-scholar LLM")
    parser.add_argument("--ollama-url", default=DEFAULT_OLLAMA_URL)
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--split-ratio", type=float, default=0.9)
    # Category targets
    parser.add_argument("--terminology", type=int, default=200)
    parser.add_argument("--narrator", type=int, default=300)
    parser.add_argument("--structural", type=int, default=200)
    parser.add_argument("--rag", type=int, default=300)
    parser.add_argument("--tafsir", type=int, default=200)
    parser.add_argument("--crossdomain", type=int, default=100)
    args = parser.parse_args()

    random.seed(args.seed)
    t0 = time.time()

    # Check Ollama
    print(f"Checking Ollama at {args.ollama_url}...")
    try:
        urllib.request.urlopen(f"{args.ollama_url}/api/tags", timeout=5)
        print("  Ollama is running")
    except Exception as e:
        print(f"  WARNING: Ollama not reachable ({e}). Category 4 (RAG Q&A) will be skipped.")
        args.rag = 0

    # Load data
    kg = load_kg()
    verses = load_quran()

    # Build Quran lookup for cross-domain
    verses_lookup = {(v["surah"], v["ayah"]): v for v in verses}

    # Generate all categories
    all_examples = []

    # Instant categories (no Ollama)
    all_examples.extend(generate_terminology()[:args.terminology])
    all_examples.extend(generate_narrator_analysis(kg, args.narrator))
    all_examples.extend(generate_isnad_structural(kg, args.structural))
    if verses:
        all_examples.extend(generate_quran_tafsir(verses, args.tafsir))
    all_examples.extend(generate_cross_domain(kg, verses_lookup, args.crossdomain))

    # Ollama category (parallelized)
    if args.rag > 0:
        t1 = time.time()
        all_examples.extend(generate_hadith_rag(kg, args.ollama_url, args.model, args.rag, args.workers))
        print(f"  Ollama generation took {time.time() - t1:.0f}s")

    # Shuffle and split
    print(f"\nTotal examples: {len(all_examples)}")
    random.shuffle(all_examples)
    split_idx = int(len(all_examples) * args.split_ratio)
    train = all_examples[:split_idx]
    valid = all_examples[split_idx:]

    # Write
    print("\nWriting output...")
    write_jsonl(train, TRAIN_OUTPUT)
    write_jsonl(valid, VALID_OUTPUT)

    total_time = time.time() - t0
    print(f"\nSummary:")
    print(f"  Train: {len(train)}, Valid: {len(valid)}")
    print(f"  Total time: {total_time:.0f}s ({total_time / 60:.1f}m)")
    print(f"\nOutput: {TRAIN_OUTPUT}, {VALID_OUTPUT}")


if __name__ == "__main__":
    main()
