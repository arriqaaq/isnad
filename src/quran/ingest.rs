use anyhow::Result;
use regex::Regex;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;
use crate::embed::Embedder;

fn rid(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key)
}

const BATCH_SIZE: usize = 64;

/// Strip Arabic diacritics and normalize letter variants for search.
/// Unlike `normalize_arabic` in sanadset.rs, this does NOT apply kunya
/// normalization (ابي→ابو) which would corrupt Quranic text.
pub fn strip_arabic_diacritics(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let code = c as u32;
        // Skip diacritics (tashkeel)
        if (0x064B..=0x065F).contains(&code)
            || code == 0x0670
            || code == 0x0640 // tatweel
            || (0x0610..=0x061A).contains(&code)
            || (0x06D6..=0x06ED).contains(&code)
        // Uthmani-specific marks
        {
            continue;
        }
        // Normalize alef variants → bare alef
        if matches!(c, 'أ' | 'إ' | 'آ' | 'ٱ') {
            out.push('ا');
        // Normalize taa marbuta → haa
        } else if c == 'ة' {
            out.push('ه');
        // Normalize alef maqsura → yaa
        } else if c == 'ى' {
            out.push('ي');
        // Keep Arabic letters and spaces
        } else if (0x0620..=0x064A).contains(&code) || c == ' ' {
            out.push(c);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

struct CsvAyah {
    surah: i64,
    ayah: i64,
    text_ar: String,
    text_en: String,
    tafsir_en: String,
}

pub async fn ingest(db: &Surreal<Db>, csv_path: &str) -> Result<()> {
    // 1. Create surah records from hardcoded metadata
    println!("📖 Creating surah records...");
    create_surahs(db).await?;

    // 2. Parse CSV and create ayah records
    println!("📜 Ingesting ayahs from {csv_path}...");
    let ayahs = parse_csv(csv_path)?;
    let total = ayahs.len();
    println!("   Found {total} ayahs");

    let pb = indicatif::ProgressBar::new(total as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("   {bar:40.cyan/blue} {pos}/{len} ayahs ({eta})")
            .unwrap(),
    );

    for ayah in &ayahs {
        let key = format!("{}_{}", ayah.surah, ayah.ayah);
        let text_ar_simple = strip_arabic_diacritics(&ayah.text_ar);
        let text_en: Option<String> = if ayah.text_en.is_empty() {
            None
        } else {
            Some(ayah.text_en.clone())
        };
        let tafsir_en: Option<String> = if ayah.tafsir_en.is_empty() {
            None
        } else {
            Some(ayah.tafsir_en.clone())
        };

        db.query(
            "CREATE $rid CONTENT { \
             surah_number: $surah, ayah_number: $ayah, \
             text_ar: $text_ar, text_ar_simple: $text_ar_simple, \
             text_en: $text_en, tafsir_en: $tafsir_en }",
        )
        .bind(("rid", rid("ayah", &key)))
        .bind(("surah", ayah.surah))
        .bind(("ayah", ayah.ayah))
        .bind(("text_ar", ayah.text_ar.clone()))
        .bind(("text_ar_simple", text_ar_simple))
        .bind(("text_en", text_en))
        .bind(("tafsir_en", tafsir_en))
        .await?
        .check()?;

        pb.inc(1);
    }
    pb.finish_with_message("done");
    println!("   ✓ {total} ayahs ingested");

    // 3. Generate embeddings
    println!("🧠 Generating ayah embeddings...");
    embed_all_ayahs(db).await?;

    Ok(())
}

fn parse_csv(path: &str) -> Result<Vec<CsvAyah>> {
    let mut reader = csv::Reader::from_path(path)?;
    let mut ayahs = Vec::new();

    for result in reader.records() {
        let record = result?;
        let surah: i64 = record.get(0).unwrap_or("0").parse()?;
        let ayah: i64 = record.get(1).unwrap_or("0").parse()?;
        let text_ar = record.get(2).unwrap_or("").to_string();
        let text_en = record.get(3).unwrap_or("").to_string();
        let tafsir_en = record.get(4).unwrap_or("").to_string();
        ayahs.push(CsvAyah {
            surah,
            ayah,
            text_ar,
            text_en,
            tafsir_en,
        });
    }

    Ok(ayahs)
}

#[derive(Debug, SurrealValue)]
struct AyahForEmbed {
    id: Option<RecordId>,
    surah_number: i64,
    ayah_number: i64,
    text_ar: String,
    text_en: Option<String>,
}

async fn embed_all_ayahs(db: &Surreal<Db>) -> Result<()> {
    let embedder = Embedder::new()?;

    let mut response = db
        .query("SELECT id, surah_number, ayah_number, text_ar, text_en FROM ayah WHERE embedding IS NONE")
        .await?;
    let ayahs: Vec<AyahForEmbed> = response.take(0)?;

    let total = ayahs.len();
    if total == 0 {
        println!("   All ayahs already have embeddings");
        return Ok(());
    }

    let pb = indicatif::ProgressBar::new(total as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("   {bar:40.green/black} {pos}/{len} embeddings ({eta})")
            .unwrap(),
    );

    // Build a surah name lookup for embedding context
    let surah_names = surah_name_lookup();

    for chunk in ayahs.chunks(BATCH_SIZE) {
        let texts: Vec<String> = chunk
            .iter()
            .map(|a| {
                let surah_name = surah_names
                    .get(a.surah_number as usize)
                    .copied()
                    .unwrap_or("Unknown");
                let text = match a.text_en.as_deref() {
                    Some(en) => format!("{} {}", a.text_ar, en),
                    None => a.text_ar.clone(),
                };
                format!(
                    "Quran {} {}:{}: {}",
                    surah_name, a.surah_number, a.ayah_number, text
                )
            })
            .collect();

        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let embeddings = embedder.embed(&text_refs)?;

        for (ayah, embedding) in chunk.iter().zip(embeddings.into_iter()) {
            if let Some(id) = &ayah.id {
                db.query("UPDATE $id SET embedding = $embedding")
                    .bind(("id", id.clone()))
                    .bind(("embedding", embedding))
                    .await?;
            }
        }

        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("done");
    println!("   ✓ {} embeddings generated", total);
    Ok(())
}

/// Surah transliteration names indexed by number (0 = unused, 1..114 = surahs).
fn surah_name_lookup() -> Vec<&'static str> {
    vec![
        "", // index 0 unused
        "Al-Fatihah",
        "Al-Baqarah",
        "Ali 'Imran",
        "An-Nisa",
        "Al-Ma'idah",
        "Al-An'am",
        "Al-A'raf",
        "Al-Anfal",
        "At-Tawbah",
        "Yunus",
        "Hud",
        "Yusuf",
        "Ar-Ra'd",
        "Ibrahim",
        "Al-Hijr",
        "An-Nahl",
        "Al-Isra",
        "Al-Kahf",
        "Maryam",
        "Taha",
        "Al-Anbya",
        "Al-Hajj",
        "Al-Mu'minun",
        "An-Nur",
        "Al-Furqan",
        "Ash-Shu'ara",
        "An-Naml",
        "Al-Qasas",
        "Al-'Ankabut",
        "Ar-Rum",
        "Luqman",
        "As-Sajdah",
        "Al-Ahzab",
        "Saba",
        "Fatir",
        "Ya-Sin",
        "As-Saffat",
        "Sad",
        "Az-Zumar",
        "Ghafir",
        "Fussilat",
        "Ash-Shuraa",
        "Az-Zukhruf",
        "Ad-Dukhan",
        "Al-Jathiyah",
        "Al-Ahqaf",
        "Muhammad",
        "Al-Fath",
        "Al-Hujurat",
        "Qaf",
        "Adh-Dhariyat",
        "At-Tur",
        "An-Najm",
        "Al-Qamar",
        "Ar-Rahman",
        "Al-Waqi'ah",
        "Al-Hadid",
        "Al-Mujadila",
        "Al-Hashr",
        "Al-Mumtahanah",
        "As-Saf",
        "Al-Jumu'ah",
        "Al-Munafiqun",
        "At-Taghabun",
        "At-Talaq",
        "At-Tahrim",
        "Al-Mulk",
        "Al-Qalam",
        "Al-Haqqah",
        "Al-Ma'arij",
        "Nuh",
        "Al-Jinn",
        "Al-Muzzammil",
        "Al-Muddaththir",
        "Al-Qiyamah",
        "Al-Insan",
        "Al-Mursalat",
        "An-Naba",
        "An-Nazi'at",
        "'Abasa",
        "At-Takwir",
        "Al-Infitar",
        "Al-Mutaffifin",
        "Al-Inshiqaq",
        "Al-Buruj",
        "At-Tariq",
        "Al-A'la",
        "Al-Ghashiyah",
        "Al-Fajr",
        "Al-Balad",
        "Ash-Shams",
        "Al-Layl",
        "Ad-Duhaa",
        "Ash-Sharh",
        "At-Tin",
        "Al-'Alaq",
        "Al-Qadr",
        "Al-Bayyinah",
        "Az-Zalzalah",
        "Al-'Adiyat",
        "Al-Qari'ah",
        "At-Takathur",
        "Al-'Asr",
        "Al-Humazah",
        "Al-Fil",
        "Quraysh",
        "Al-Ma'un",
        "Al-Kawthar",
        "Al-Kafirun",
        "An-Nasr",
        "Al-Masad",
        "Al-Ikhlas",
        "Al-Falaq",
        "An-Nas",
    ]
}

/// Hardcoded surah metadata. Returns (number, name_ar, name_en, transliteration, revelation_type, ayah_count).
fn surah_metadata() -> Vec<(
    i64,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    i64,
)> {
    vec![
        (1, "الفاتحة", "The Opener", "Al-Fatihah", "Meccan", 7),
        (2, "البقرة", "The Cow", "Al-Baqarah", "Medinan", 286),
        (
            3,
            "آل عمران",
            "Family of Imran",
            "Ali 'Imran",
            "Medinan",
            200,
        ),
        (4, "النساء", "The Women", "An-Nisa", "Medinan", 176),
        (
            5,
            "المائدة",
            "The Table Spread",
            "Al-Ma'idah",
            "Medinan",
            120,
        ),
        (6, "الأنعام", "The Cattle", "Al-An'am", "Meccan", 165),
        (7, "الأعراف", "The Heights", "Al-A'raf", "Meccan", 206),
        (8, "الأنفال", "The Spoils of War", "Al-Anfal", "Medinan", 75),
        (9, "التوبة", "The Repentance", "At-Tawbah", "Medinan", 129),
        (10, "يونس", "Jonah", "Yunus", "Meccan", 109),
        (11, "هود", "Hud", "Hud", "Meccan", 123),
        (12, "يوسف", "Joseph", "Yusuf", "Meccan", 111),
        (13, "الرعد", "The Thunder", "Ar-Ra'd", "Medinan", 43),
        (14, "إبراهيم", "Abraham", "Ibrahim", "Meccan", 52),
        (15, "الحجر", "The Rocky Tract", "Al-Hijr", "Meccan", 99),
        (16, "النحل", "The Bee", "An-Nahl", "Meccan", 128),
        (17, "الإسراء", "The Night Journey", "Al-Isra", "Meccan", 111),
        (18, "الكهف", "The Cave", "Al-Kahf", "Meccan", 110),
        (19, "مريم", "Mary", "Maryam", "Meccan", 98),
        (20, "طه", "Ta-Ha", "Taha", "Meccan", 135),
        (21, "الأنبياء", "The Prophets", "Al-Anbya", "Meccan", 112),
        (22, "الحج", "The Pilgrimage", "Al-Hajj", "Medinan", 78),
        (
            23,
            "المؤمنون",
            "The Believers",
            "Al-Mu'minun",
            "Meccan",
            118,
        ),
        (24, "النور", "The Light", "An-Nur", "Medinan", 64),
        (25, "الفرقان", "The Criterion", "Al-Furqan", "Meccan", 77),
        (26, "الشعراء", "The Poets", "Ash-Shu'ara", "Meccan", 227),
        (27, "النمل", "The Ant", "An-Naml", "Meccan", 93),
        (28, "القصص", "The Stories", "Al-Qasas", "Meccan", 88),
        (29, "العنكبوت", "The Spider", "Al-'Ankabut", "Meccan", 69),
        (30, "الروم", "The Romans", "Ar-Rum", "Meccan", 60),
        (31, "لقمان", "Luqman", "Luqman", "Meccan", 34),
        (32, "السجدة", "The Prostration", "As-Sajdah", "Meccan", 30),
        (
            33,
            "الأحزاب",
            "The Combined Forces",
            "Al-Ahzab",
            "Medinan",
            73,
        ),
        (34, "سبأ", "Sheba", "Saba", "Meccan", 54),
        (35, "فاطر", "Originator", "Fatir", "Meccan", 45),
        (36, "يس", "Ya-Sin", "Ya-Sin", "Meccan", 83),
        (
            37,
            "الصافات",
            "Those Who Set the Ranks",
            "As-Saffat",
            "Meccan",
            182,
        ),
        (38, "ص", "The Letter Sad", "Sad", "Meccan", 88),
        (39, "الزمر", "The Troops", "Az-Zumar", "Meccan", 75),
        (40, "غافر", "The Forgiver", "Ghafir", "Meccan", 85),
        (41, "فصلت", "Explained in Detail", "Fussilat", "Meccan", 54),
        (42, "الشورى", "The Consultation", "Ash-Shuraa", "Meccan", 53),
        (
            43,
            "الزخرف",
            "The Ornaments of Gold",
            "Az-Zukhruf",
            "Meccan",
            89,
        ),
        (44, "الدخان", "The Smoke", "Ad-Dukhan", "Meccan", 59),
        (45, "الجاثية", "The Crouching", "Al-Jathiyah", "Meccan", 37),
        (
            46,
            "الأحقاف",
            "The Wind-Curved Sandhills",
            "Al-Ahqaf",
            "Meccan",
            35,
        ),
        (47, "محمد", "Muhammad", "Muhammad", "Medinan", 38),
        (48, "الفتح", "The Victory", "Al-Fath", "Medinan", 29),
        (49, "الحجرات", "The Rooms", "Al-Hujurat", "Medinan", 18),
        (50, "ق", "The Letter Qaf", "Qaf", "Meccan", 45),
        (
            51,
            "الذاريات",
            "The Winnowing Winds",
            "Adh-Dhariyat",
            "Meccan",
            60,
        ),
        (52, "الطور", "The Mount", "At-Tur", "Meccan", 49),
        (53, "النجم", "The Star", "An-Najm", "Meccan", 62),
        (54, "القمر", "The Moon", "Al-Qamar", "Meccan", 55),
        (55, "الرحمن", "The Beneficent", "Ar-Rahman", "Medinan", 78),
        (56, "الواقعة", "The Inevitable", "Al-Waqi'ah", "Meccan", 96),
        (57, "الحديد", "The Iron", "Al-Hadid", "Medinan", 29),
        (
            58,
            "المجادلة",
            "The Pleading Woman",
            "Al-Mujadila",
            "Medinan",
            22,
        ),
        (59, "الحشر", "The Exile", "Al-Hashr", "Medinan", 24),
        (
            60,
            "الممتحنة",
            "She That Is to Be Examined",
            "Al-Mumtahanah",
            "Medinan",
            13,
        ),
        (61, "الصف", "The Ranks", "As-Saf", "Medinan", 14),
        (
            62,
            "الجمعة",
            "The Congregation",
            "Al-Jumu'ah",
            "Medinan",
            11,
        ),
        (
            63,
            "المنافقون",
            "The Hypocrites",
            "Al-Munafiqun",
            "Medinan",
            11,
        ),
        (
            64,
            "التغابن",
            "The Mutual Disillusion",
            "At-Taghabun",
            "Medinan",
            18,
        ),
        (65, "الطلاق", "The Divorce", "At-Talaq", "Medinan", 12),
        (66, "التحريم", "The Prohibition", "At-Tahrim", "Medinan", 12),
        (67, "الملك", "The Sovereignty", "Al-Mulk", "Meccan", 30),
        (68, "القلم", "The Pen", "Al-Qalam", "Meccan", 52),
        (69, "الحاقة", "The Reality", "Al-Haqqah", "Meccan", 52),
        (
            70,
            "المعارج",
            "The Ascending Stairways",
            "Al-Ma'arij",
            "Meccan",
            44,
        ),
        (71, "نوح", "Noah", "Nuh", "Meccan", 28),
        (72, "الجن", "The Jinn", "Al-Jinn", "Meccan", 28),
        (
            73,
            "المزمل",
            "The Enshrouded One",
            "Al-Muzzammil",
            "Meccan",
            20,
        ),
        (
            74,
            "المدثر",
            "The Cloaked One",
            "Al-Muddaththir",
            "Meccan",
            56,
        ),
        (
            75,
            "القيامة",
            "The Resurrection",
            "Al-Qiyamah",
            "Meccan",
            40,
        ),
        (76, "الإنسان", "The Human", "Al-Insan", "Medinan", 31),
        (77, "المرسلات", "The Emissaries", "Al-Mursalat", "Meccan", 50),
        (78, "النبأ", "The Tidings", "An-Naba", "Meccan", 40),
        (
            79,
            "النازعات",
            "Those Who Drag Forth",
            "An-Nazi'at",
            "Meccan",
            46,
        ),
        (80, "عبس", "He Frowned", "'Abasa", "Meccan", 42),
        (81, "التكوير", "The Overthrowing", "At-Takwir", "Meccan", 29),
        (82, "الانفطار", "The Cleaving", "Al-Infitar", "Meccan", 19),
        (
            83,
            "المطففين",
            "The Defrauding",
            "Al-Mutaffifin",
            "Meccan",
            36,
        ),
        (84, "الانشقاق", "The Sundering", "Al-Inshiqaq", "Meccan", 25),
        (
            85,
            "البروج",
            "The Mansions of the Stars",
            "Al-Buruj",
            "Meccan",
            22,
        ),
        (86, "الطارق", "The Night-Comer", "At-Tariq", "Meccan", 17),
        (87, "الأعلى", "The Most High", "Al-A'la", "Meccan", 19),
        (
            88,
            "الغاشية",
            "The Overwhelming",
            "Al-Ghashiyah",
            "Meccan",
            26,
        ),
        (89, "الفجر", "The Dawn", "Al-Fajr", "Meccan", 30),
        (90, "البلد", "The City", "Al-Balad", "Meccan", 20),
        (91, "الشمس", "The Sun", "Ash-Shams", "Meccan", 15),
        (92, "الليل", "The Night", "Al-Layl", "Meccan", 21),
        (93, "الضحى", "The Morning Hours", "Ad-Duhaa", "Meccan", 11),
        (94, "الشرح", "The Relief", "Ash-Sharh", "Meccan", 8),
        (95, "التين", "The Fig", "At-Tin", "Meccan", 8),
        (96, "العلق", "The Clot", "Al-'Alaq", "Meccan", 19),
        (97, "القدر", "The Power", "Al-Qadr", "Meccan", 5),
        (98, "البينة", "The Clear Proof", "Al-Bayyinah", "Medinan", 8),
        (99, "الزلزلة", "The Earthquake", "Az-Zalzalah", "Medinan", 8),
        (100, "العاديات", "The Courser", "Al-'Adiyat", "Meccan", 11),
        (101, "القارعة", "The Calamity", "Al-Qari'ah", "Meccan", 11),
        (
            102,
            "التكاثر",
            "The Rivalry in World Increase",
            "At-Takathur",
            "Meccan",
            8,
        ),
        (103, "العصر", "The Declining Day", "Al-'Asr", "Meccan", 3),
        (104, "الهمزة", "The Traducer", "Al-Humazah", "Meccan", 9),
        (105, "الفيل", "The Elephant", "Al-Fil", "Meccan", 5),
        (106, "قريش", "Quraysh", "Quraysh", "Meccan", 4),
        (
            107,
            "الماعون",
            "The Small Kindnesses",
            "Al-Ma'un",
            "Meccan",
            7,
        ),
        (108, "الكوثر", "The Abundance", "Al-Kawthar", "Meccan", 3),
        (
            109,
            "الكافرون",
            "The Disbelievers",
            "Al-Kafirun",
            "Meccan",
            6,
        ),
        (110, "النصر", "The Divine Support", "An-Nasr", "Medinan", 3),
        (111, "المسد", "The Palm Fiber", "Al-Masad", "Meccan", 5),
        (112, "الإخلاص", "The Sincerity", "Al-Ikhlas", "Meccan", 4),
        (113, "الفلق", "The Daybreak", "Al-Falaq", "Meccan", 5),
        (114, "الناس", "The Mankind", "An-Nas", "Meccan", 6),
    ]
}

async fn create_surahs(db: &Surreal<Db>) -> Result<()> {
    for (num, name_ar, name_en, translit, rev_type, ayah_count) in surah_metadata() {
        db.query(
            "CREATE $rid CONTENT { \
             surah_number: $num, name_ar: $name_ar, name_en: $name_en, \
             name_translit: $translit, revelation_type: $rev_type, ayah_count: $ayah_count }",
        )
        .bind(("rid", rid("surah", &num.to_string())))
        .bind(("num", num))
        .bind(("name_ar", name_ar))
        .bind(("name_en", name_en))
        .bind(("translit", translit))
        .bind(("rev_type", rev_type))
        .bind(("ayah_count", ayah_count))
        .await?
        .check()?;
    }
    println!("   ✓ 114 surahs created");
    Ok(())
}

// ── Tafsir chunking and embedding ──

/// Strip HTML tags and decode entities, producing clean plain text.
fn strip_html(html: &str) -> String {
    let mut text = html.to_string();

    // Replace block-level closing tags and <br> with newlines
    let block_re = Regex::new(r"(?i)</p>|</div>|</h[1-6]>|<br\s*/?>").unwrap();
    text = block_re.replace_all(&text, "\n").to_string();

    // Strip all remaining HTML tags
    let tag_re = Regex::new(r"<[^>]+>").unwrap();
    text = tag_re.replace_all(&text, "").to_string();

    // Decode HTML entities
    text = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ");

    // Decode numeric HTML entities (&#NNN;)
    let numeric_re = Regex::new(r"&#(\d+);").unwrap();
    text = numeric_re
        .replace_all(&text, |caps: &regex::Captures| {
            caps[1]
                .parse::<u32>()
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_default()
        })
        .to_string();

    // Collapse multiple newlines into double newline (paragraph break)
    let multi_nl = Regex::new(r"\n{3,}").unwrap();
    text = multi_nl.replace_all(&text, "\n\n").to_string();

    // Collapse multiple spaces/tabs within lines to single space
    let multi_space = Regex::new(r"[^\S\n]+").unwrap();
    text = multi_space.replace_all(&text, " ").to_string();

    // Clean up spaces around newlines
    let space_nl = Regex::new(r" ?\n ?").unwrap();
    text = space_nl.replace_all(&text, "\n").to_string();

    text.trim().to_string()
}

/// Split tafsir text into overlapping chunks suitable for embedding.
///
/// Strategy: split on paragraph boundaries first, then sentences, then words.
/// Accumulate sections into chunks up to `max_chars`, with `overlap` chars
/// carried from the end of the previous chunk.
fn chunk_tafsir(text: &str, max_chars: usize, overlap: usize) -> Vec<String> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    // Split into atomic sections: paragraphs -> sentences -> words
    let sections = split_recursive(text, max_chars);

    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();

    for section in &sections {
        let section = section.trim();
        if section.is_empty() {
            continue;
        }

        if current.is_empty() {
            current = section.to_string();
        } else if current.len() + 1 + section.len() <= max_chars {
            current.push(' ');
            current.push_str(section);
        } else {
            // Flush current chunk
            chunks.push(current.clone());

            // Start new chunk with overlap from end of previous
            let overlap_text = if current.len() > overlap {
                let start = floor_char_boundary(&current, current.len() - overlap);
                let overlap_region = &current[start..];
                // Try to break at a word boundary within the overlap region
                if let Some(space_pos) = overlap_region.find(' ') {
                    &current[start + space_pos + 1..]
                } else {
                    overlap_region
                }
            } else {
                &current
            };

            current = format!("{} {}", overlap_text.trim(), section);
        }
    }

    // Don't forget the last chunk
    if !current.trim().is_empty() {
        chunks.push(current);
    }

    chunks
        .into_iter()
        .filter(|c| !c.trim().is_empty())
        .collect()
}

/// Find the largest byte index <= `byte_idx` that is a char boundary.
fn floor_char_boundary(s: &str, byte_idx: usize) -> usize {
    let idx = byte_idx.min(s.len());
    let mut i = idx;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Recursively split text into sections that fit within max_chars (byte-safe).
fn split_recursive(text: &str, max_chars: usize) -> Vec<String> {
    if text.len() <= max_chars {
        return vec![text.to_string()];
    }

    // Try paragraph boundaries first
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    if paragraphs.len() > 1 {
        return paragraphs
            .into_iter()
            .flat_map(|p| split_recursive(p, max_chars))
            .collect();
    }

    // Try sentence boundaries
    let sentences: Vec<&str> = text.split(". ").collect();
    if sentences.len() > 1 {
        let len = sentences.len();
        return sentences
            .into_iter()
            .enumerate()
            .flat_map(|(i, s)| {
                let restored = if i < len - 1 {
                    format!("{}.", s)
                } else {
                    s.to_string()
                };
                split_recursive(&restored, max_chars)
            })
            .collect();
    }

    // Fall back to word boundaries
    let words: Vec<&str> = text.split(' ').collect();
    if words.len() <= 1 {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current = String::new();
    for word in words {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            result.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

#[derive(Debug, SurrealValue)]
struct AyahForTafsir {
    id: Option<RecordId>,
    surah_number: i64,
    ayah_number: i64,
    tafsir_en: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct CountResult {
    count: i64,
}

#[derive(Debug, SurrealValue)]
struct ChunkForEmbed {
    id: Option<RecordId>,
}

/// Chunk all tafsir texts and generate embeddings for the chunks.
pub async fn embed_tafsir_chunks(db: &Surreal<Db>, embedder: &Embedder) -> Result<()> {
    // 1. Get ayahs with tafsir
    let mut response = db
        .query("SELECT id, surah_number, ayah_number, tafsir_en FROM ayah WHERE tafsir_en IS NOT NONE AND tafsir_en != ''")
        .await?;
    let ayahs: Vec<AyahForTafsir> = response.take(0)?;

    let total_ayahs = ayahs.len();
    if total_ayahs == 0 {
        println!("   No ayahs with tafsir found");
        return Ok(());
    }

    println!("   Found {} ayahs with tafsir, chunking...", total_ayahs);

    let pb = indicatif::ProgressBar::new(total_ayahs as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("   {bar:40.cyan/blue} {pos}/{len} ayahs chunked ({eta})")
            .unwrap(),
    );

    let mut total_chunks_created: usize = 0;

    for ayah in &ayahs {
        let ayah_id = match &ayah.id {
            Some(id) => id.clone(),
            None => continue,
        };

        // Check if chunks already exist for this ayah
        let mut count_res = db
            .query("SELECT count() FROM tafsir_chunk WHERE ayah_id = $id GROUP ALL")
            .bind(("id", ayah_id.clone()))
            .await?;
        let counts: Vec<CountResult> = count_res.take(0)?;
        if counts.first().map(|c| c.count).unwrap_or(0) > 0 {
            pb.inc(1);
            continue;
        }

        let tafsir = match &ayah.tafsir_en {
            Some(t) => t,
            None => continue,
        };

        let clean = strip_html(tafsir);
        let chunks = chunk_tafsir(&clean, 1500, 200);

        for (idx, chunk_text) in chunks.iter().enumerate() {
            db.query(
                "CREATE tafsir_chunk SET ayah_id = $ayah_id, chunk_index = $idx, chunk_text = $text",
            )
            .bind(("ayah_id", ayah_id.clone()))
            .bind(("idx", idx as i64))
            .bind(("text", chunk_text.clone()))
            .await?
            .check()?;
            total_chunks_created += 1;
        }

        pb.inc(1);
    }
    pb.finish_with_message("done");
    println!("   ✓ {} tafsir chunks created", total_chunks_created);

    // 2. Embed all chunks without embeddings
    let mut response = db
        .query("SELECT id FROM tafsir_chunk WHERE embedding IS NONE")
        .await?;
    let chunks_to_embed: Vec<ChunkForEmbed> = response.take(0)?;

    let total_embed = chunks_to_embed.len();
    if total_embed == 0 {
        println!("   All tafsir chunks already have embeddings");
        return Ok(());
    }

    println!("   Embedding {} tafsir chunks...", total_embed);

    // We need to fetch chunk details for embedding text construction.
    // Query in batches to build the prefix text.
    #[derive(Debug, SurrealValue)]
    struct ChunkDetail {
        id: Option<RecordId>,
        ayah_id: Option<RecordId>,
        chunk_text: String,
    }

    let mut detail_response = db
        .query("SELECT id, ayah_id, chunk_text FROM tafsir_chunk WHERE embedding IS NONE")
        .await?;
    let chunk_details: Vec<ChunkDetail> = detail_response.take(0)?;

    // Build a lookup from ayah record id -> (surah_number, ayah_number)
    let mut ayah_lookup: std::collections::HashMap<String, (i64, i64)> =
        std::collections::HashMap::new();
    for ayah in &ayahs {
        if let Some(id) = &ayah.id {
            ayah_lookup.insert(format!("{:?}", id), (ayah.surah_number, ayah.ayah_number));
        }
    }

    let pb = indicatif::ProgressBar::new(total_embed as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("   {bar:40.green/black} {pos}/{len} chunk embeddings ({eta})")
            .unwrap(),
    );

    for batch in chunk_details.chunks(BATCH_SIZE) {
        let texts: Vec<String> = batch
            .iter()
            .map(|c| {
                let (surah, ayah_num) = c
                    .ayah_id
                    .as_ref()
                    .and_then(|aid| ayah_lookup.get(&format!("{:?}", aid)))
                    .copied()
                    .unwrap_or((0, 0));
                format!(
                    "Tafsir Ibn Kathir on Quran {}:{}: {}",
                    surah, ayah_num, c.chunk_text
                )
            })
            .collect();

        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let embeddings = embedder.embed(&text_refs)?;

        for (chunk, embedding) in batch.iter().zip(embeddings.into_iter()) {
            if let Some(id) = &chunk.id {
                db.query("UPDATE $id SET embedding = $embedding")
                    .bind(("id", id.clone()))
                    .bind(("embedding", embedding))
                    .await?;
            }
        }

        pb.inc(batch.len() as u64);
    }

    pb.finish_with_message("done");
    println!("   ✓ {} tafsir chunk embeddings generated", total_embed);

    Ok(())
}
