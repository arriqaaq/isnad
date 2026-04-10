#!/usr/bin/env python3
"""
Scans articles/*.md, converts each to a styled HTML page under site/articles/,
and regenerates site/blog.html with cards for every article found.

Usage: python3 scripts/build_blog.py
"""

import os
import re
import glob
import datetime

ARTICLES_DIR = "articles"
SITE_DIR = "site"
SITE_ARTICLES_DIR = os.path.join(SITE_DIR, "articles")
REPO_URL = "https://github.com/arriqaaq/ilm"


def extract_metadata(md_text, filename):
    """Extract title, description, and date from markdown content."""
    lines = md_text.strip().split("\n")

    # Find first heading as title
    title = filename.replace(".md", "").replace("-", " ").title()
    for line in lines:
        m = re.match(r"^#+\s+(.+)", line)
        if m:
            title = m.group(1).strip()
            break

    # Find first non-empty, non-heading paragraph as description
    desc = ""
    for line in lines:
        stripped = line.strip()
        if not stripped:
            continue
        if stripped.startswith("#") or stripped.startswith("---") or stripped.startswith("<") or stripped.startswith("بسم"):
            continue
        if stripped.startswith("**") and stripped.endswith("**"):
            continue
        # Take first real paragraph
        desc = re.sub(r"\*\*([^*]+)\*\*", r"\1", stripped)
        desc = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", desc)
        if len(desc) > 200:
            desc = desc[:197] + "..."
        break

    return title, desc


def md_to_html(md_text):
    """Convert markdown to HTML. Uses markdown lib if available, else basic conversion."""
    try:
        import markdown
        html = markdown.markdown(
            md_text,
            extensions=["tables", "fenced_code", "toc"],
        )
        return html
    except ImportError:
        # Fallback: basic conversion
        html = md_text
        # Headings
        html = re.sub(r"^### (.+)$", r"<h3>\1</h3>", html, flags=re.MULTILINE)
        html = re.sub(r"^## (.+)$", r"<h2>\1</h2>", html, flags=re.MULTILINE)
        html = re.sub(r"^# (.+)$", r"<h1>\1</h1>", html, flags=re.MULTILINE)
        # Bold
        html = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", html)
        # Italic
        html = re.sub(r"\*(.+?)\*", r"<em>\1</em>", html)
        # Code blocks
        html = re.sub(r"```(\w*)\n(.*?)```", r"<pre><code>\2</code></pre>", html, flags=re.DOTALL)
        # Inline code
        html = re.sub(r"`([^`]+)`", r"<code>\1</code>", html)
        # Links
        html = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", r'<a href="\2">\1</a>', html)
        # Images
        html = re.sub(r'<p align="center">\s*<img src="([^"]+)" alt="([^"]*)"[^>]*>\s*</p>', r'<figure><img src="\1" alt="\2"><figcaption>\2</figcaption></figure>', html, flags=re.DOTALL)
        # Paragraphs
        html = re.sub(r"\n\n(.+?)(?=\n\n|$)", r"\n<p>\1</p>", html, flags=re.DOTALL)
        # Horizontal rules
        html = re.sub(r"^---$", "<hr>", html, flags=re.MULTILINE)
        return html


ARTICLE_TEMPLATE = """\
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title} — Ilm</title>
  <style>
    *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
    :root {{
      --accent: #c8a96a;
      --accent-muted: rgba(200, 169, 106, 0.1);
      --accent-border: rgba(200, 169, 106, 0.15);
      --text-primary: #1a1a2e;
      --text-secondary: #4a4a68;
      --text-muted: #9ca3af;
      --bg: #ffffff;
      --bg-warm: #faf8f5;
      --border: #e5e7eb;
    }}
    body {{
      font-family: system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;
      color: var(--text-primary);
      background: var(--bg);
      line-height: 1.7;
      -webkit-font-smoothing: antialiased;
    }}
    a {{ color: var(--accent); }}
    .nav {{
      display: flex; align-items: center; justify-content: space-between;
      padding: 20px 24px; max-width: 780px; margin: 0 auto;
    }}
    .nav-logo {{ font-size: 1.3rem; font-weight: 800; color: var(--accent); text-decoration: none; }}
    .nav-links {{ display: flex; gap: 24px; }}
    .nav-links a {{ font-size: 0.9rem; font-weight: 500; color: var(--text-secondary); text-decoration: none; transition: color 0.15s; }}
    .nav-links a:hover {{ color: var(--accent); }}
    .article-header {{
      text-align: center; padding: 48px 24px 32px;
      background: linear-gradient(180deg, var(--bg-warm) 0%, var(--bg) 100%);
    }}
    .article-header h1 {{
      font-size: 2rem; font-weight: 800; letter-spacing: -0.5px;
      max-width: 700px; margin: 0 auto; line-height: 1.3;
    }}
    .article-date {{ display: block; margin-top: 12px; font-size: 0.82rem; color: var(--text-muted); }}
    .article-body {{
      max-width: 780px; margin: 0 auto; padding: 40px 24px 80px;
      font-size: 1.02rem; color: var(--text-secondary);
    }}
    .article-body h1, .article-body h2, .article-body h3 {{ color: var(--text-primary); margin: 2em 0 0.6em; }}
    .article-body h2 {{ font-size: 1.5rem; border-bottom: 1px solid var(--border); padding-bottom: 8px; }}
    .article-body h3 {{ font-size: 1.2rem; }}
    .article-body p {{ margin-bottom: 1.2em; }}
    .article-body img {{ max-width: 100%; border-radius: 8px; margin: 1.5em 0; }}
    .article-body figure {{ text-align: center; margin: 2em 0; }}
    .article-body figcaption {{ font-size: 0.82rem; color: var(--text-muted); margin-top: 8px; }}
    .article-body pre {{
      background: #1a1a2e; color: #e2e8f0; border-radius: 12px;
      padding: 20px; overflow-x: auto; margin: 1.5em 0; font-size: 0.85rem; line-height: 1.6;
    }}
    .article-body code {{
      font-family: 'SF Mono', 'Fira Code', monospace;
      font-size: 0.88em; background: var(--accent-muted);
      padding: 2px 6px; border-radius: 4px;
    }}
    .article-body pre code {{ background: none; padding: 0; }}
    .article-body blockquote {{
      border-left: 3px solid var(--accent-border); padding-left: 16px;
      margin: 1.5em 0; color: var(--text-muted); font-style: italic;
    }}
    .article-body table {{
      width: 100%; border-collapse: collapse; margin: 1.5em 0; font-size: 0.9rem;
    }}
    .article-body th, .article-body td {{
      border: 1px solid var(--border); padding: 10px 14px; text-align: left;
    }}
    .article-body th {{ background: var(--bg-warm); font-weight: 600; }}
    .article-body hr {{ border: none; border-top: 1px solid var(--border); margin: 2.5em 0; }}
    .article-body ul, .article-body ol {{ margin: 1em 0; padding-left: 1.5em; }}
    .article-body li {{ margin-bottom: 0.5em; }}
    footer {{
      border-top: 1px solid var(--border); padding: 36px 0 24px; text-align: center;
    }}
    .footer-logo {{ font-size: 1.2rem; font-weight: 800; color: var(--accent); text-decoration: none; }}
    .footer-tagline {{ display: block; color: var(--text-muted); font-size: 0.8rem; margin-top: 4px; }}
    /* Override Prism theme to match site dark code blocks */
    .article-body pre[class*="language-"] {{
      background: #1a1a2e; border-radius: 12px; padding: 20px;
      margin: 1.5em 0; overflow-x: auto; font-size: 0.85rem; line-height: 1.6;
    }}
    .article-body code[class*="language-"] {{ background: none; padding: 0; font-size: 0.85rem; }}
  </style>
  <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/themes/prism-tomorrow.min.css">
</head>
<body>
  <nav class="nav">
    <a href="../index.html" class="nav-logo">Ilm</a>
    <div class="nav-links">
      <a href="../blog.html">Blog</a>
      <a href="https://github.com/arriqaaq/ilm" target="_blank" rel="noopener noreferrer">GitHub</a>
    </div>
  </nav>
  <header class="article-header">
    <h1>{title}</h1>
    <span class="article-date">{date}</span>
  </header>
  <article class="article-body">
    {body}
  </article>
  <footer>
    <a href="../index.html" class="footer-logo">Ilm</a>
    <span class="footer-tagline">Islamic Knowledge Platform</span>
  </footer>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/prism.min.js"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-sql.min.js"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-rust.min.js"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-bash.min.js"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-json.min.js"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-python.min.js"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-toml.min.js"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/plugins/autoloader/prism-autoloader.min.js"></script>
</body>
</html>
"""

BLOG_TEMPLATE = """\
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Blog — Ilm</title>
  <meta name="description" content="Deep dives into how Ilm is built — architecture, data pipelines, and design decisions.">
  <style>
    *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
    :root {{
      --accent: #c8a96a;
      --accent-muted: rgba(200, 169, 106, 0.1);
      --accent-border: rgba(200, 169, 106, 0.15);
      --text-primary: #1a1a2e;
      --text-secondary: #4a4a68;
      --text-muted: #9ca3af;
      --bg: #ffffff;
      --bg-surface: #f9fafb;
      --bg-warm: #faf8f5;
      --radius: 16px;
    }}
    body {{
      font-family: system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;
      color: var(--text-primary); background: var(--bg);
      line-height: 1.6; -webkit-font-smoothing: antialiased;
    }}
    a {{ color: inherit; text-decoration: none; }}
    .container {{ max-width: 860px; margin: 0 auto; padding: 0 24px; }}
    .nav {{
      display: flex; align-items: center; justify-content: space-between;
      padding: 20px 24px; max-width: 860px; margin: 0 auto;
    }}
    .nav-logo {{ font-size: 1.3rem; font-weight: 800; color: var(--accent); }}
    .nav-links {{ display: flex; gap: 24px; }}
    .nav-links a {{ font-size: 0.9rem; font-weight: 500; color: var(--text-secondary); transition: color 0.15s; }}
    .nav-links a:hover {{ color: var(--accent); }}
    .blog-header {{
      text-align: center; padding: 60px 24px 48px;
      background: linear-gradient(180deg, var(--bg-warm) 0%, var(--bg) 100%);
    }}
    .blog-header h1 {{ font-size: 2.4rem; font-weight: 800; letter-spacing: -1px; }}
    .blog-header p {{ color: var(--text-secondary); margin-top: 8px; font-size: 1.05rem; }}
    .articles {{ padding: 48px 0 100px; }}
    .article-card {{
      display: block; background: var(--bg); border: 1px solid #e5e7eb;
      border-radius: var(--radius); padding: 36px 32px; margin-bottom: 24px;
      transition: transform 0.3s cubic-bezier(.25,.46,.45,.94), box-shadow 0.3s cubic-bezier(.25,.46,.45,.94), border-color 0.3s ease;
    }}
    .article-card:hover {{
      transform: translateY(-4px);
      box-shadow: 0 12px 32px rgba(0,0,0,0.06);
      border-color: var(--accent-border);
    }}
    .article-meta {{ display: flex; align-items: center; gap: 12px; margin-bottom: 14px; }}
    .article-date {{
      font-size: 0.78rem; font-weight: 600; color: var(--text-muted);
      text-transform: uppercase; letter-spacing: 0.04em;
    }}
    .article-card h2 {{
      font-size: 1.3rem; font-weight: 700; line-height: 1.4;
      margin-bottom: 10px; color: var(--text-primary);
    }}
    .article-card p {{
      font-size: 0.92rem; color: var(--text-secondary);
      line-height: 1.7; margin-bottom: 16px;
    }}
    .article-read {{
      font-size: 0.85rem; font-weight: 600; color: var(--accent);
      transition: letter-spacing 0.2s ease;
    }}
    .article-card:hover .article-read {{ letter-spacing: 0.03em; }}
    footer {{
      border-top: 1px solid #e5e7eb; padding: 36px 0 24px; text-align: center;
    }}
    .footer-logo {{ font-size: 1.2rem; font-weight: 800; color: var(--accent); }}
    .footer-tagline {{ display: block; color: var(--text-muted); font-size: 0.8rem; margin-top: 4px; }}
    .reveal {{
      opacity: 0; will-change: transform, opacity;
      transition: opacity 0.7s cubic-bezier(.25,.46,.45,.94), transform 0.7s cubic-bezier(.25,.46,.45,.94);
      transform: translateY(32px);
    }}
    .reveal.visible {{ opacity: 1; transform: translateY(0); }}
    @media (max-width: 640px) {{
      .blog-header {{ padding: 40px 20px 32px; }}
      .blog-header h1 {{ font-size: 1.8rem; }}
      .article-card {{ padding: 24px 20px; }}
    }}
  </style>
</head>
<body>
  <nav class="nav">
    <a href="index.html" class="nav-logo">Ilm</a>
    <div class="nav-links">
      <a href="index.html">Home</a>
      <a href="https://github.com/arriqaaq/ilm" target="_blank" rel="noopener noreferrer">GitHub</a>
    </div>
  </nav>
  <header class="blog-header">
    <div class="container">
      <h1>Blog</h1>
      <p>Deep dives into how Ilm is built — architecture, data pipelines, and design decisions.</p>
    </div>
  </header>
  <section class="articles">
    <div class="container">
{cards}
    </div>
  </section>
  <footer>
    <div class="container">
      <a href="index.html" class="footer-logo">Ilm</a>
      <span class="footer-tagline">Islamic Knowledge Platform</span>
    </div>
  </footer>
  <script>
    const observer = new IntersectionObserver((entries) => {{
      entries.forEach(entry => {{
        if (entry.isIntersecting) {{
          entry.target.classList.add('visible');
          observer.unobserve(entry.target);
        }}
      }});
    }}, {{ threshold: 0.15, rootMargin: '0px 0px -60px 0px' }});
    document.querySelectorAll('.reveal').forEach(el => observer.observe(el));
  </script>
</body>
</html>
"""

CARD_TEMPLATE = """\
      <a href="articles/{slug}.html" class="article-card reveal">
        <div class="article-meta">
          <span class="article-date">{date}</span>
        </div>
        <h2>{title}</h2>
        <p>{desc}</p>
        <span class="article-read">Read article &rarr;</span>
      </a>
"""


def get_file_date(filepath):
    """Get file modification date."""
    mtime = os.path.getmtime(filepath)
    return datetime.datetime.fromtimestamp(mtime)


def fix_image_paths(html):
    """Fix relative image paths from articles/ to work from site/articles/.

    Generated HTML lives at site/articles/<slug>.html.
    GitHub Pages only serves site/, so ALL images must end up in site/img/.
    Referenced images from repo root img/ are copied into site/img/ by
    copy_referenced_images().

    In the markdown (written from articles/), paths look like:
      ../site/img/foo.svg    -> already in site/img/ -> need ../img/foo.svg
      ../img/foo.svg         -> copied to site/img/  -> need ../img/foo.svg
      img/foo.svg            -> copied to site/img/  -> need ../img/foo.svg
    """
    # All paths normalize to ../img/ (= site/img/ from site/articles/)
    html = html.replace('src="../site/img/', 'src="../img/')
    html = html.replace('src="../img/', 'src="../img/')
    html = re.sub(r'src="img/', 'src="../img/', html)
    return html


def copy_referenced_images(md_text):
    """Copy any images referenced from repo root img/ into site/img/."""
    import shutil
    # Find all image references to ../img/ (repo root img/)
    refs = re.findall(r'src="\.\.\/img\/([^"]+)"', md_text)
    # Also find raw markdown image refs
    refs += re.findall(r'\.\./img/([^)\s"]+)', md_text)
    site_img_dir = os.path.join(SITE_DIR, "img")
    os.makedirs(site_img_dir, exist_ok=True)
    for img_name in set(refs):
        src = os.path.join("img", img_name)
        dst = os.path.join(site_img_dir, img_name)
        if os.path.exists(src) and not os.path.exists(dst):
            shutil.copy2(src, dst)
            print(f"  copied img/{img_name} -> site/img/{img_name}")


def main():
    os.makedirs(SITE_ARTICLES_DIR, exist_ok=True)

    md_files = sorted(glob.glob(os.path.join(ARTICLES_DIR, "*.md")))
    if not md_files:
        print("No articles found in articles/")
        return

    articles = []

    for md_path in md_files:
        filename = os.path.basename(md_path)
        slug = filename.replace(".md", "")

        with open(md_path, "r", encoding="utf-8") as f:
            md_text = f.read()

        title, desc = extract_metadata(md_text, filename)
        date = get_file_date(md_path)
        date_str = date.strftime("%B %Y")

        # Copy referenced images from repo root img/ into site/img/
        copy_referenced_images(md_text)

        # Strip the first heading from body (it's used as the page title)
        body_md = md_text
        body_md = re.sub(r"^#\s+.+\n*", "", body_md, count=1)

        # Convert to HTML
        body_html = md_to_html(body_md)
        body_html = fix_image_paths(body_html)

        # Write article HTML page
        article_html = ARTICLE_TEMPLATE.format(
            title=title,
            date=date_str,
            body=body_html,
        )
        out_path = os.path.join(SITE_ARTICLES_DIR, f"{slug}.html")
        with open(out_path, "w", encoding="utf-8") as f:
            f.write(article_html)
        print(f"  {md_path} -> {out_path}")

        articles.append({
            "slug": slug,
            "title": title,
            "desc": desc,
            "date_str": date_str,
            "date": date,
        })

    # Sort newest first
    articles.sort(key=lambda a: a["date"], reverse=True)

    # Generate blog index
    cards = ""
    for a in articles:
        cards += CARD_TEMPLATE.format(
            slug=a["slug"],
            title=a["title"],
            desc=a["desc"],
            date=a["date_str"],
        )

    blog_html = BLOG_TEMPLATE.format(cards=cards)
    blog_path = os.path.join(SITE_DIR, "blog.html")
    with open(blog_path, "w", encoding="utf-8") as f:
        f.write(blog_html)
    print(f"  -> {blog_path} ({len(articles)} article(s))")


if __name__ == "__main__":
    print("Building blog...")
    main()
    print("Done.")
