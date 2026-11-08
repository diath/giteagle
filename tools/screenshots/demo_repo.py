#!/usr/bin/env python3
"""Builds "lumen", the made-up repository that the website's screenshots show.

Usage: demo_repo.py <folder>

Makes <folder>/lumen, the history of a small static site generator with five
authors, branches, a merge, stashes and uncommitted changes, and
<folder>/origin.git as its remote. Dates count back from now, so the relative
times in the screenshots look recent.
"""

import os
import shutil
import subprocess
import sys
import time

if len(sys.argv) != 2:
    sys.exit(__doc__)

ROOT = os.path.abspath(sys.argv[1])
WORK = os.path.join(ROOT, "lumen")
ORIGIN = os.path.join(ROOT, "origin.git")
NOW = int(time.time())

AUTHORS = {
    "mira": ("Mira Okafor", "mira@lumen-ssg.org", "+0100"),
    "jonas": ("Jonas Lindqvist", "jonas.lindqvist@lumen-ssg.org", "+0100"),
    "priya": ("Priya Raman", "priya@lumen-ssg.org", "+0530"),
    "tomas": ("Tomás Herrera", "tomas.herrera@lumen-ssg.org", "-0300"),
    "ada": ("Ada Brennan", "ada@lumen-ssg.org", "+0000"),
}


def run(*args, cwd=WORK, env=None):
    result = subprocess.run(args, cwd=cwd, env=env, capture_output=True, text=True)
    if result.returncode != 0:
        sys.exit(f"{' '.join(args)} failed:\n{result.stdout}\n{result.stderr}")
    return result.stdout


def env_for(author, when):
    name, email, zone = AUTHORS[author]
    env = dict(os.environ)
    env.update(
        GIT_AUTHOR_NAME=name,
        GIT_AUTHOR_EMAIL=email,
        GIT_AUTHOR_DATE=f"@{when} {zone}",
        GIT_COMMITTER_NAME=name,
        GIT_COMMITTER_EMAIL=email,
        GIT_COMMITTER_DATE=f"@{when} {zone}",
    )
    return env


def ago(days=0, hours=0, minutes=0):
    return NOW - days * 86400 - hours * 3600 - minutes * 60


def write(path, text):
    full = os.path.join(WORK, path)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, "w", encoding="utf-8", newline="\n") as file:
        file.write(text.lstrip("\n"))


def read(path):
    with open(os.path.join(WORK, path), encoding="utf-8") as file:
        return file.read()


def edit(path, old, new):
    """Replaces `old`, which must appear once, so a changed file can't go unnoticed."""
    text = read(path)
    old = old.strip("\n")
    new = new.strip("\n")
    if text.count(old) != 1:
        sys.exit(f"edit of {path}: expected one match, found {text.count(old)}:\n{old}")
    write(path, text.replace(old, new))


def commit(author, when, message, coauthors=()):
    run("git", "add", "-A")
    for coauthor in coauthors:
        name, email, _ = AUTHORS[coauthor]
        message += f"\n\nCo-authored-by: {name} <{email}>"
    run("git", "commit", "-q", "-m", message, env=env_for(author, when))


def git(*args, author="mira", when=None):
    return run("git", *args, env=env_for(author, when or NOW))


for folder in (WORK, ORIGIN):
    shutil.rmtree(folder, ignore_errors=True)
os.makedirs(ROOT, exist_ok=True)
run("git", "init", "-q", "--bare", "-b", "main", ORIGIN, cwd=ROOT)
run("git", "init", "-q", "-b", "main", WORK, cwd=ROOT)
run("git", "config", "user.name", "Mira Okafor")
run("git", "config", "user.email", "mira@lumen-ssg.org")
run("git", "config", "core.autocrlf", "false")
run("git", "remote", "add", "origin", ORIGIN)

write("Cargo.toml", r'''
[package]
name = "lumen"
version = "0.0.0"
edition = "2021"
description = "A small, fast static site generator"
license = "MIT"
repository = "https://github.com/lumen-ssg/lumen"

[dependencies]
''')
write("README.md", r'''
# Lumen

A small, fast static site generator.
''')
write("LICENSE", r'''
MIT License

Copyright (c) 2026 The Lumen authors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
''')
write(".gitignore", r'''
/target
/public
''')
write("src/main.rs", r'''
fn main() {
    println!("lumen {}", env!("CARGO_PKG_VERSION"));
}
''')
commit("mira", ago(150, 3), "Initial commit")

edit("Cargo.toml", "[dependencies]", r'''
[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
''')
write("src/main.rs", r'''
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod config;

use config::Config;

/// A small, fast static site generator.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// The folder of the site, holding lumen.toml.
    #[arg(short, long, default_value = ".")]
    root: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Builds the site into the output folder.
    Build,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load(&cli.root)?;
    match cli.command {
        Command::Build => {
            println!("Building {} into {}", config.title, config.output_dir.display());
        }
    }
    Ok(())
}
''')
write("src/config.rs", r'''
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

pub const CONFIG_FILE: &str = "lumen.toml";

/// The settings of a site, read from lumen.toml in its root folder.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub title: String,
    pub base_url: String,
    #[serde(default = "default_content_dir")]
    pub content_dir: PathBuf,
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
}

fn default_content_dir() -> PathBuf {
    PathBuf::from("content")
}

fn default_output_dir() -> PathBuf {
    PathBuf::from("public")
}

impl Config {
    /// Reads lumen.toml from `root`. Folders in it are relative to `root`.
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(CONFIG_FILE);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("couldn't read {}", path.display()))?;
        let mut config: Config = toml::from_str(&text)
            .with_context(|| format!("{} is not valid", path.display()))?;
        config.content_dir = root.join(&config.content_dir);
        config.output_dir = root.join(&config.output_dir);
        Ok(config)
    }
}
''')
commit("mira", ago(149, 5), "Add the build command")

edit("Cargo.toml", 'clap = { version = "4", features = ["derive"] }', r'''
clap = { version = "4", features = ["derive"] }
pulldown-cmark = "0.11"
''')
write("src/content.rs", r'''
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// A Markdown page from the content folder.
#[derive(Debug)]
pub struct Page {
    pub source: PathBuf,
    pub title: String,
    pub body: String,
}

/// Loads every Markdown file below `content_dir`.
pub fn load_pages(content_dir: &Path) -> Result<Vec<Page>> {
    let mut pages = Vec::new();
    collect(content_dir, &mut pages)?;
    Ok(pages)
}

fn collect(folder: &Path, pages: &mut Vec<Page>) -> Result<()> {
    let entries = fs::read_dir(folder)
        .with_context(|| format!("couldn't list {}", folder.display()))?;
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, pages)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            pages.push(read_page(&path)?);
        }
    }
    Ok(())
}

fn read_page(path: &Path) -> Result<Page> {
    let body = fs::read_to_string(path)
        .with_context(|| format!("couldn't read {}", path.display()))?;
    let title = body
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .unwrap_or("Untitled")
        .to_string();
    Ok(Page {
        source: path.to_path_buf(),
        title,
        body,
    })
}
''')
write("src/markdown.rs", r'''
use pulldown_cmark::{html, Options, Parser};

/// Turns Markdown into HTML, with tables, footnotes and strikethrough.
pub fn to_html(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, options());
    let mut output = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut output, parser);
    output
}

fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_STRIKETHROUGH
}
''')
edit("src/main.rs", "mod config;", r'''
mod config;
mod content;
mod markdown;
''')
commit("mira", ago(147, 2), "Render Markdown pages with pulldown-cmark")

edit("Cargo.toml", 'serde = { version = "1", features = ["derive"] }', r'''
serde = { version = "1", features = ["derive"] }
tera = "1"
''')
edit("src/config.rs", r'''
    #[serde(default = "default_content_dir")]
    pub content_dir: PathBuf,
''', r'''
    #[serde(default = "default_content_dir")]
    pub content_dir: PathBuf,
    #[serde(default = "default_templates_dir")]
    pub templates_dir: PathBuf,
''')
edit("src/config.rs", r'''
fn default_output_dir() -> PathBuf {
''', r'''
fn default_templates_dir() -> PathBuf {
    PathBuf::from("templates")
}

fn default_output_dir() -> PathBuf {
''')
edit("src/config.rs", r'''
        config.content_dir = root.join(&config.content_dir);
''', r'''
        config.content_dir = root.join(&config.content_dir);
        config.templates_dir = root.join(&config.templates_dir);
''')
write("src/render.rs", r'''
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tera::{Context as TeraContext, Tera};

use crate::config::Config;
use crate::content::Page;
use crate::markdown;

/// Fills the site's templates with pages.
pub struct Renderer {
    tera: Tera,
}

impl Renderer {
    pub fn new(templates_dir: &Path) -> Result<Self> {
        let pattern = templates_dir.join("**/*.html");
        let tera = Tera::new(&pattern.to_string_lossy()).context("couldn't load the templates")?;
        Ok(Self { tera })
    }

    pub fn render_page(&self, config: &Config, page: &Page) -> Result<String> {
        let mut context = TeraContext::new();
        context.insert("site_title", &config.title);
        context.insert("title", &page.title);
        context.insert("content", &markdown::to_html(&page.body));
        self.tera
            .render("page.html", &context)
            .with_context(|| format!("couldn't render {}", page.source.display()))
    }
}

/// Renders every page into the output folder.
pub fn build(config: &Config, pages: &[Page]) -> Result<()> {
    let renderer = Renderer::new(&config.templates_dir)?;
    fs::create_dir_all(&config.output_dir)?;
    for page in pages {
        let html = renderer.render_page(config, page)?;
        let target = output_path(config, page)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, html).with_context(|| format!("couldn't write {}", target.display()))?;
    }
    Ok(())
}

fn output_path(config: &Config, page: &Page) -> Result<PathBuf> {
    let relative = page.source.strip_prefix(&config.content_dir)?;
    Ok(config.output_dir.join(relative).with_extension("html"))
}
''')
write("templates/base.html", r'''
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{% block title %}{{ site_title }}{% endblock %}</title>
</head>
<body>
  <header class="site-header">
    <a class="site-title" href="/">{{ site_title }}</a>
  </header>
  <main>
    {% block content %}{% endblock %}
  </main>
</body>
</html>
''')
write("templates/page.html", r'''
{% extends "base.html" %}

{% block title %}{{ title }} - {{ site_title }}{% endblock %}

{% block content %}
<article class="page">
  <h1>{{ title | safe }}</h1>
  {{ content | safe }}
</article>
{% endblock %}
''')
edit("src/main.rs", "mod markdown;", "mod markdown;\nmod render;")
edit("src/main.rs", r'''
        Command::Build => {
            println!("Building {} into {}", config.title, config.output_dir.display());
        }
    }
    Ok(())
}
''', r'''
        Command::Build => build(&config)?,
    }
    Ok(())
}

fn build(config: &Config) -> Result<()> {
    let pages = content::load_pages(&config.content_dir)?;
    render::build(config, &pages)?;
    println!("Built {} pages into {}", pages.len(), config.output_dir.display());
    Ok(())
}
''')
commit("mira", ago(146, 6), "Add Tera templates")

write("src/content.rs", r'''
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// A Markdown page from the content folder.
#[derive(Debug)]
pub struct Page {
    pub source: PathBuf,
    pub title: String,
    pub date: Option<String>,
    pub tags: Vec<String>,
    pub body: String,
}

/// The TOML block between `+++` lines at the top of a page.
#[derive(Debug, Default, Deserialize)]
struct FrontMatter {
    title: Option<String>,
    date: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

const FRONT_MATTER_FENCE: &str = "+++";

/// Loads every Markdown file below `content_dir`.
pub fn load_pages(content_dir: &Path) -> Result<Vec<Page>> {
    let mut pages = Vec::new();
    collect(content_dir, &mut pages)?;
    Ok(pages)
}

fn collect(folder: &Path, pages: &mut Vec<Page>) -> Result<()> {
    let entries = fs::read_dir(folder)
        .with_context(|| format!("couldn't list {}", folder.display()))?;
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, pages)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            pages.push(read_page(&path)?);
        }
    }
    Ok(())
}

fn read_page(path: &Path) -> Result<Page> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("couldn't read {}", path.display()))?;
    let (front_matter, body) = split_front_matter(&text)
        .with_context(|| format!("{} has broken front matter", path.display()))?;
    let title = front_matter
        .title
        .or_else(|| first_heading(body))
        .unwrap_or_else(|| "Untitled".to_string());
    Ok(Page {
        source: path.to_path_buf(),
        title,
        date: front_matter.date,
        tags: front_matter.tags,
        body: body.to_string(),
    })
}

/// Splits a page into its front matter and the Markdown after it. Pages
/// without front matter get the defaults.
fn split_front_matter(text: &str) -> Result<(FrontMatter, &str)> {
    let Some(rest) = text.strip_prefix(FRONT_MATTER_FENCE) else {
        return Ok((FrontMatter::default(), text));
    };
    let Some(end) = rest.find(&format!("\n{FRONT_MATTER_FENCE}")) else {
        bail!("the front matter is never closed with {FRONT_MATTER_FENCE}");
    };
    let front_matter = toml::from_str(&rest[..end])?;
    let body = rest[end + FRONT_MATTER_FENCE.len() + 1..].trim_start();
    Ok((front_matter, body))
}

fn first_heading(body: &str) -> Option<String> {
    body.lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(|heading| heading.trim().to_string())
}
''')
commit("jonas", ago(141, 4), "Parse TOML front matter")

write("static/style.css", r'''
:root {
  --text: #2b2724;
  --muted: #7a716a;
  --background: #fbf8f3;
  --accent: #b5562f;
  --rule: #e6ded3;
  --content-width: 42rem;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  background: var(--background);
  color: var(--text);
  font: 1.0625rem/1.65 "Iowan Old Style", Georgia, serif;
}

.site-header,
main {
  max-width: var(--content-width);
  margin: 0 auto;
  padding: 0 1.25rem;
}

.site-header {
  padding-block: 2rem 1rem;
  border-bottom: 1px solid var(--rule);
}

.site-title {
  color: var(--text);
  font-weight: 700;
  text-decoration: none;
}

a {
  color: var(--accent);
}

h1,
h2,
h3 {
  line-height: 1.25;
}

pre {
  overflow-x: auto;
  padding: 1rem;
  border-radius: 4px;
  font-size: 0.9rem;
}

code {
  font-family: "JetBrains Mono", Menlo, monospace;
}
''')
edit("templates/base.html", r'''
  <title>{% block title %}{{ site_title }}{% endblock %}</title>
''', r'''
  <title>{% block title %}{{ site_title }}{% endblock %}</title>
  <link rel="stylesheet" href="/style.css">
''')
commit("mira", ago(139, 2), "Add a default stylesheet")

edit("src/config.rs", r'''
    #[serde(default = "default_templates_dir")]
    pub templates_dir: PathBuf,
''', r'''
    #[serde(default = "default_templates_dir")]
    pub templates_dir: PathBuf,
    #[serde(default = "default_static_dir")]
    pub static_dir: PathBuf,
''')
edit("src/config.rs", r'''
fn default_output_dir() -> PathBuf {
''', r'''
fn default_static_dir() -> PathBuf {
    PathBuf::from("static")
}

fn default_output_dir() -> PathBuf {
''')
edit("src/config.rs", r'''
        config.templates_dir = root.join(&config.templates_dir);
''', r'''
        config.templates_dir = root.join(&config.templates_dir);
        config.static_dir = root.join(&config.static_dir);
''')
edit("src/render.rs", r'''
        fs::write(&target, html).with_context(|| format!("couldn't write {}", target.display()))?;
    }
    Ok(())
}
''', r'''
        fs::write(&target, html).with_context(|| format!("couldn't write {}", target.display()))?;
    }
    copy_static(&config.static_dir, &config.output_dir)?;
    Ok(())
}

/// Copies the static folder as it is, keeping its layout.
fn copy_static(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&target)?;
            copy_static(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)
                .with_context(|| format!("couldn't copy {}", entry.path().display()))?;
        }
    }
    Ok(())
}
''')
commit("jonas", ago(138, 1), "Copy static files into the output folder")

write(".github/workflows/ci.yml", r'''
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --locked
      - run: cargo test --locked
''')
commit("priya", ago(134, 7), "Add a CI workflow")

edit("src/content.rs", r'''
    pub title: String,
    pub date: Option<String>,
''', r'''
    pub title: String,
    /// The last part of the page's URL.
    pub slug: String,
    pub date: Option<String>,
''')
edit("src/content.rs", r'''
struct FrontMatter {
    title: Option<String>,
''', r'''
struct FrontMatter {
    title: Option<String>,
    slug: Option<String>,
''')
edit("src/content.rs", r'''
const FRONT_MATTER_FENCE: &str = "+++";
''', r'''
const FRONT_MATTER_FENCE: &str = "+++";

impl Page {
    /// The page's address below the site root, such as `/hello-world/`.
    pub fn url(&self) -> String {
        format!("/{}/", self.slug)
    }
}
''')
edit("src/content.rs", r'''
        .unwrap_or_else(|| "Untitled".to_string());
    Ok(Page {
        source: path.to_path_buf(),
        title,
''', r'''
        .unwrap_or_else(|| "Untitled".to_string());
    let slug = front_matter.slug.unwrap_or_else(|| slugify(&title));
    Ok(Page {
        source: path.to_path_buf(),
        title,
        slug,
''')
edit("src/content.rs", r'''
fn first_heading(body: &str) -> Option<String> {
''', r'''
/// Turns a title into a URL path segment: lowercase words joined by dashes.
pub fn slugify(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_string()
}

fn first_heading(body: &str) -> Option<String> {
''')
edit("src/render.rs", r'''
fn output_path(config: &Config, page: &Page) -> Result<PathBuf> {
    let relative = page.source.strip_prefix(&config.content_dir)?;
    Ok(config.output_dir.join(relative).with_extension("html"))
}
''', r'''
/// Where a page goes: a folder named after its slug, so URLs need no `.html`.
fn output_path(config: &Config, page: &Page) -> Result<PathBuf> {
    let relative = page.source.strip_prefix(&config.content_dir)?;
    let folder = relative.parent().unwrap_or(Path::new(""));
    Ok(config.output_dir.join(folder).join(&page.slug).join("index.html"))
}
''')
commit("mira", ago(131, 3), "Generate slugs from page titles")

edit("Cargo.toml", 'clap = { version = "4", features = ["derive"] }', r'''
clap = { version = "4", features = ["derive"] }
deunicode = "1"
''')
edit("src/content.rs", r'''
use anyhow::{bail, Context, Result};
use serde::Deserialize;
''', r'''
use anyhow::{bail, Context, Result};
use deunicode::deunicode;
use serde::Deserialize;
''')
edit("src/content.rs", r'''
/// Turns a title into a URL path segment: lowercase words joined by dashes.
pub fn slugify(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    for character in title.chars() {
''', r'''
/// Turns a title into a URL path segment: lowercase words joined by dashes.
/// Letters lose their accents, so "Café" becomes "cafe".
pub fn slugify(title: &str) -> String {
    let ascii = deunicode(title);
    let mut slug = String::with_capacity(ascii.len());
    for character in ascii.chars() {
''')
write("src/content.rs", read("src/content.rs") + r'''
#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn words_are_joined_by_dashes() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
    }

    #[test]
    fn accents_are_dropped() {
        assert_eq!(slugify("Crème brûlée à la maison"), "creme-brulee-a-la-maison");
    }
}
''')
commit("tomas", ago(127, 5), "Fix slugs for titles with accents")

write("src/markdown.rs", r'''
use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};

use crate::content::slugify;

/// Turns Markdown into HTML, with tables, footnotes and strikethrough.
/// Headings get an id made from their text, so they can be linked to.
pub fn to_html(markdown: &str) -> String {
    let events = with_heading_anchors(Parser::new_ext(markdown, options()));
    let mut output = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut output, events.into_iter());
    output
}

fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_STRIKETHROUGH
}

/// Replaces each heading with HTML that carries an id, keeping its contents.
fn with_heading_anchors<'a>(events: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    let mut output = Vec::new();
    let mut heading: Option<(HeadingLevel, Vec<Event<'a>>)> = None;
    for event in events {
        match event {
            Event::Start(Tag::Heading(level, ..)) => heading = Some((level, Vec::new())),
            Event::End(Tag::Heading(..)) => {
                let (level, inner) = heading.take().expect("a heading was started");
                let id = slugify(&plain_text(&inner));
                output.push(Event::Html(format!("<{level} id=\"{id}\">").into()));
                output.extend(inner);
                output.push(Event::Html(format!("</{level}>\n").into()));
            }
            event => match heading.as_mut() {
                Some((_, inner)) => inner.push(event),
                None => output.push(event),
            },
        }
    }
    output
}

fn plain_text(events: &[Event]) -> String {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Text(text) | Event::Code(text) => Some(text.as_ref()),
            _ => None,
        })
        .collect()
}
''')
commit("mira", ago(124, 2), "Add anchors to headings")

write("examples/blog/lumen.toml", r'''
title = "Field Notes"
base_url = "https://notes.example.com"
templates_dir = "../../templates"
static_dir = "../../static"
''')
write("examples/blog/content/hello-world.md", r'''
+++
title = "Hello, world"
date = "2026-05-02"
tags = ["meta"]
+++

This blog is built with Lumen. Every page is a Markdown file in the
`content` folder, with a little TOML at the top.

## What's next

Notes on the garden, the shed, and whatever else gets built this year.
''')
write("examples/blog/content/posts/building-a-garden-shed.md", r'''
+++
title = "Building a garden shed"
date = "2026-05-19"
tags = ["garden", "woodwork"]
+++

The shed took three weekends, two trips to the timber yard and one
argument about where the door should go.

## The base

A shed is only as square as its base. Ours sits on eight concrete pads,
levelled with a long straightedge and a lot of patience.

## Walls and roof

| Part      | Material          | Count |
|-----------|-------------------|-------|
| Studs     | 45 x 70 mm pine   | 34    |
| Cladding  | Larch boards      | 60    |
| Roof      | EPDM membrane     | 1     |

The roof is a single slope, which keeps the rain off the door side.
''')
write("tests/build.rs", r'''
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn lumen(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lumen"))
        .arg("--root")
        .arg(root)
        .args(arguments)
        .output()
        .expect("lumen runs")
}

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples").join(name)
}

#[test]
fn builds_the_example_blog() {
    let root = example("blog");
    let output = lumen(&root, &["build"]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let page = fs::read_to_string(root.join("public/hello-world/index.html")).unwrap();
    assert!(page.contains("<h1>Hello, world</h1>"));
    assert!(page.contains("<h2 id=\"whats-next\">"));
}

#[test]
fn pages_in_subfolders_keep_their_folder() {
    let root = example("blog");
    assert!(lumen(&root, &["build"]).status.success());
    assert!(root.join("public/posts/building-a-garden-shed/index.html").exists());
}
''')
commit("priya", ago(120, 6), "Add integration tests for the build")

edit("Cargo.toml", 'tera = "1"', 'tera = "1"\ntiny_http = "0.12"')
write("src/server.rs", r'''
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use tiny_http::{Header, Response, Server};

/// Serves the built site on `port` until the process ends.
pub fn serve(output_dir: &Path, port: u16) -> Result<()> {
    let server = Server::http(("127.0.0.1", port)).map_err(|error| anyhow!(error))?;
    println!("Serving on http://localhost:{port}/");
    for request in server.incoming_requests() {
        let path = resolve(output_dir, request.url());
        let response = match fs::read(&path) {
            Ok(bytes) => Response::from_data(bytes).with_header(content_type(&path)),
            Err(_) => Response::from_string("Not found").with_status_code(404),
        };
        // The browser may have gone away; the next request is unaffected.
        let _ = request.respond(response);
    }
    Ok(())
}

/// Maps a request path to a file, serving `index.html` for folders.
fn resolve(output_dir: &Path, url: &str) -> PathBuf {
    let path = url.split('?').next().unwrap_or("/").trim_start_matches('/');
    let mut file = output_dir.join(path);
    if file.is_dir() {
        file.push("index.html");
    }
    file
}

fn content_type(path: &Path) -> Header {
    let mime = match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css",
        Some("js") => "text/javascript",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("xml") => "application/xml",
        _ => "application/octet-stream",
    };
    Header::from_bytes("Content-Type", mime).expect("a valid header")
}
''')
edit("src/main.rs", "mod render;", "mod render;\nmod server;")
edit("src/main.rs", r'''
    /// Builds the site into the output folder.
    Build,
}
''', r'''
    /// Builds the site into the output folder.
    Build,
    /// Builds the site and serves it locally.
    Serve {
        /// The port to listen on.
        #[arg(short, long, default_value_t = 4000)]
        port: u16,
    },
}
''')
edit("src/main.rs", r'''
        Command::Build => build(&config)?,
''', r'''
        Command::Build => build(&config)?,
        Command::Serve { port } => {
            build(&config)?;
            server::serve(&config.output_dir, port)?;
        }
''')
commit("mira", ago(115, 4), "Add the serve command")

edit("Cargo.toml", 'deunicode = "1"', 'deunicode = "1"\nnotify = "6"')
edit("src/config.rs", "#[derive(Debug, Deserialize)]\npub struct Config", "#[derive(Debug, Clone, Deserialize)]\npub struct Config")
edit("src/server.rs", r'''
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use tiny_http::{Header, Response, Server};
''', r'''
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use notify::{RecursiveMode, Watcher};
use tiny_http::{Header, Response, Server};

/// How long changes must stop before a rebuild, so saving many files at
/// once rebuilds only once.
const SETTLE_TIME: Duration = Duration::from_millis(150);
''')
write("src/server.rs", read("src/server.rs") + r'''
/// Calls `rebuild` on another thread whenever something in `folder` changes.
pub fn watch(folder: PathBuf, rebuild: impl Fn() + Send + 'static) -> Result<()> {
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(sender)?;
    watcher.watch(&folder, RecursiveMode::Recursive)?;
    thread::spawn(move || {
        // Dropping the watcher would stop the events.
        let _watcher = watcher;
        while receiver.recv().is_ok() {
            while receiver.recv_timeout(SETTLE_TIME).is_ok() {}
            rebuild();
        }
    });
    Ok(())
}
''')
edit("src/main.rs", r'''
        Command::Serve { port } => {
            build(&config)?;
            server::serve(&config.output_dir, port)?;
        }
''', r'''
        Command::Serve { port } => {
            build(&config)?;
            let rebuild_config = config.clone();
            server::watch(config.content_dir.clone(), move || {
                if let Err(error) = build(&rebuild_config) {
                    eprintln!("Build failed: {error:#}");
                }
            })?;
            server::serve(&config.output_dir, port)?;
        }
''')
commit("jonas", ago(112, 3), "Rebuild the site when content changes")

edit("Cargo.toml", 'version = "0.0.0"', 'version = "0.1.0"')
write("CHANGELOG.md", r'''
# Changelog

## 0.1.0

- `lumen build` renders Markdown pages with Tera templates.
- `lumen serve` serves the site and rebuilds it when content changes.
- Pages can start with TOML front matter: title, slug, date and tags.
- Headings get anchors, and slugs drop accents.
''')
commit("mira", ago(110, 2), "Release 0.1.0")

edit("Cargo.toml", 'serde = { version = "1", features = ["derive"] }', 'serde = { version = "1", features = ["derive"] }\nsyntect = "5"')
edit("src/markdown.rs", r'''
use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};

use crate::content::slugify;
''', r'''
use std::sync::LazyLock;

use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

use crate::content::slugify;

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEMES: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);
const CODE_THEME: &str = "base16-ocean.dark";
''')
edit("src/markdown.rs", r'''
    let events = with_heading_anchors(Parser::new_ext(markdown, options()));
''', r'''
    let events = with_heading_anchors(Parser::new_ext(markdown, options()));
    let events = with_highlighted_code(events);
''')
edit("src/markdown.rs", r'''
fn plain_text(events: &[Event]) -> String {
''', r'''
/// Replaces fenced code blocks with HTML highlighted for their language.
fn with_highlighted_code(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut output = Vec::with_capacity(events.len());
    let mut code: Option<(String, String)> = None;
    for event in events {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language))) => {
                code = Some((language.to_string(), String::new()));
            }
            Event::Text(text) if code.is_some() => {
                code.as_mut().expect("inside a code block").1.push_str(&text);
            }
            Event::End(Tag::CodeBlock(_)) if code.is_some() => {
                let (language, source) = code.take().expect("inside a code block");
                output.push(Event::Html(highlight(&language, &source).into()));
            }
            event => output.push(event),
        }
    }
    output
}

fn highlight(language: &str, source: &str) -> String {
    let syntax = SYNTAXES
        .find_syntax_by_token(language)
        .unwrap_or_else(|| SYNTAXES.find_syntax_plain_text());
    highlighted_html_for_string(source, &SYNTAXES, syntax, &THEMES.themes[CODE_THEME])
        .unwrap_or_else(|_| format!("<pre><code>{}</code></pre>", escape_html(source)))
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn plain_text(events: &[Event]) -> String {
''')
commit("ada", ago(103, 5), "Highlight code blocks with syntect")

edit("src/content.rs", r'''
    pub tags: Vec<String>,
    pub body: String,
}
''', r'''
    pub tags: Vec<String>,
    /// Drafts are only built with `--drafts`.
    pub draft: bool,
    pub body: String,
}
''')
edit("src/content.rs", r'''
    #[serde(default)]
    tags: Vec<String>,
}
''', r'''
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    draft: bool,
}
''')
edit("src/content.rs", r'''
/// Loads every Markdown file below `content_dir`.
pub fn load_pages(content_dir: &Path) -> Result<Vec<Page>> {
    let mut pages = Vec::new();
    collect(content_dir, &mut pages)?;
    Ok(pages)
}
''', r'''
/// Loads every Markdown file below `content_dir`, leaving out drafts
/// unless `include_drafts` is set.
pub fn load_pages(content_dir: &Path, include_drafts: bool) -> Result<Vec<Page>> {
    let mut pages = Vec::new();
    collect(content_dir, &mut pages)?;
    if !include_drafts {
        pages.retain(|page| !page.draft);
    }
    Ok(pages)
}
''')
edit("src/content.rs", r'''
        tags: front_matter.tags,
        body: body.to_string(),
''', r'''
        tags: front_matter.tags,
        draft: front_matter.draft,
        body: body.to_string(),
''')
edit("src/main.rs", r'''
    #[arg(short, long, default_value = ".")]
    root: PathBuf,
''', r'''
    #[arg(short, long, default_value = ".")]
    root: PathBuf,
    /// Builds draft pages too.
    #[arg(long, global = true)]
    drafts: bool,
''')
edit("src/main.rs", r'''
        Command::Build => build(&config)?,
        Command::Serve { port } => {
            build(&config)?;
            let rebuild_config = config.clone();
            server::watch(config.content_dir.clone(), move || {
                if let Err(error) = build(&rebuild_config) {
''', r'''
        Command::Build => build(&config, cli.drafts)?,
        Command::Serve { port } => {
            build(&config, cli.drafts)?;
            let rebuild_config = config.clone();
            let drafts = cli.drafts;
            server::watch(config.content_dir.clone(), move || {
                if let Err(error) = build(&rebuild_config, drafts) {
''')
edit("src/main.rs", r'''
fn build(config: &Config) -> Result<()> {
    let pages = content::load_pages(&config.content_dir)?;
''', r'''
fn build(config: &Config, drafts: bool) -> Result<()> {
    let pages = content::load_pages(&config.content_dir, drafts)?;
''')
commit("ada", ago(99, 3), "Skip draft pages unless asked to build them")

edit("src/render.rs", r'''
use anyhow::{Context, Result};
use tera::{Context as TeraContext, Tera};
''', r'''
use anyhow::{Context, Result};
use serde::Serialize;
use tera::{Context as TeraContext, Tera};
''')
edit("src/render.rs", r'''
/// Fills the site's templates with pages.
pub struct Renderer {
    tera: Tera,
}
''', r'''
/// Fills the site's templates with pages.
pub struct Renderer {
    tera: Tera,
}

/// A post as the index page lists it.
#[derive(Serialize)]
struct PostLink<'a> {
    title: &'a str,
    url: String,
    date: Option<&'a str>,
}
''')
edit("src/render.rs", r'''
        context.insert("title", &page.title);
        context.insert("content", &markdown::to_html(&page.body));
        self.tera
            .render("page.html", &context)
            .with_context(|| format!("couldn't render {}", page.source.display()))
    }
''', r'''
        context.insert("title", &page.title);
        context.insert("date", &page.date);
        context.insert("content", &markdown::to_html(&page.body));
        self.tera
            .render("page.html", &context)
            .with_context(|| format!("couldn't render {}", page.source.display()))
    }

    /// Renders the home page, which links to `posts` in the order given.
    pub fn render_index(&self, config: &Config, posts: &[&Page]) -> Result<String> {
        let links: Vec<PostLink> = posts
            .iter()
            .map(|page| PostLink {
                title: &page.title,
                url: page.url(),
                date: page.date.as_deref(),
            })
            .collect();
        let mut context = TeraContext::new();
        context.insert("site_title", &config.title);
        context.insert("posts", &links);
        self.tera
            .render("index.html", &context)
            .context("couldn't render the index page")
    }
''')
edit("src/render.rs", r'''
    copy_static(&config.static_dir, &config.output_dir)?;
    Ok(())
}
''', r'''

    // Dates are ISO 8601, so comparing them as text sorts them by time.
    let mut posts: Vec<&Page> = pages.iter().filter(|page| page.date.is_some()).collect();
    posts.sort_by(|left, right| right.date.cmp(&left.date));
    let index = renderer.render_index(config, &posts)?;
    fs::write(config.output_dir.join("index.html"), index)?;

    copy_static(&config.static_dir, &config.output_dir)?;
    Ok(())
}
''')
write("templates/index.html", r'''
{% extends "base.html" %}

{% block content %}
<section class="posts">
  {% for post in posts %}
  <article class="post-link">
    <a href="{{ post.url }}">{{ post.title | safe }}</a>
    {% if post.date %}<time datetime="{{ post.date }}">{{ post.date }}</time>{% endif %}
  </article>
  {% endfor %}
</section>
{% endblock %}
''')
edit("static/style.css", r'''
pre {
''', r'''
.post-link {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  padding-block: 0.5rem;
  border-bottom: 1px solid var(--rule);
}

.post-link time {
  color: var(--muted);
  font-size: 0.9rem;
  white-space: nowrap;
}

pre {
''')
commit("tomas", ago(95, 6), "List posts by date on the index page")

edit("src/config.rs", r'''
    pub title: String,
    pub base_url: String,
''', r'''
    pub title: String,
    pub base_url: String,
    /// Named in the footer of every page.
    #[serde(default)]
    pub author: String,
''')
edit("src/render.rs", r'''
        let mut context = TeraContext::new();
        context.insert("site_title", &config.title);
        context.insert("title", &page.title);
''', r'''
        let mut context = TeraContext::new();
        context.insert("site_title", &config.title);
        context.insert("author", &config.author);
        context.insert("title", &page.title);
''')
edit("src/render.rs", r'''
        let mut context = TeraContext::new();
        context.insert("site_title", &config.title);
        context.insert("posts", &links);
''', r'''
        let mut context = TeraContext::new();
        context.insert("site_title", &config.title);
        context.insert("author", &config.author);
        context.insert("posts", &links);
''')
edit("templates/base.html", r'''
  <main>
    {% block content %}{% endblock %}
  </main>
''', r'''
  <main>
    {% block content %}{% endblock %}
  </main>
  <footer class="site-footer">
    {% if author %}&copy; {{ now() | date(format="%Y") }} {{ author }}{% endif %}
  </footer>
''')
edit("static/style.css", r'''
.site-title {
''', r'''
.site-footer {
  max-width: var(--content-width);
  margin: 3rem auto 0;
  padding: 1.5rem 1.25rem 3rem;
  border-top: 1px solid var(--rule);
  color: var(--muted);
  font-size: 0.875rem;
}

.site-title {
''')
commit("mira", ago(92, 4), "Add a footer with the copyright notice")

edit(".github/workflows/ci.yml", r'''
jobs:
  test:
''', r'''
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo fmt --check
      - run: cargo clippy --all-targets -- -D warnings

  test:
''')
commit("priya", ago(88, 2), "Run clippy and rustfmt in CI")

edit("src/render.rs", r'''
        let html = renderer.render_page(config, page)?;
        let target = output_path(config, page)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, html).with_context(|| format!("couldn't write {}", target.display()))?;
    }
''', r'''
        let html = renderer.render_page(config, page)?;
        write_if_changed(&output_path(config, page)?, &html)?;
    }
''')
edit("src/render.rs", r'''
    fs::write(config.output_dir.join("index.html"), index)?;
''', r'''
    write_if_changed(&config.output_dir.join("index.html"), &index)?;
''')
edit("src/render.rs", r'''
/// Copies the static folder as it is, keeping its layout.
''', r'''
/// Writes `contents` unless the file holds them already, so file watchers
/// and browsers aren't woken by pages that didn't change.
fn write_if_changed(path: &Path, contents: &str) -> Result<bool> {
    if fs::read(path).is_ok_and(|existing| existing == contents.as_bytes()) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents).with_context(|| format!("couldn't write {}", path.display()))?;
    Ok(true)
}

/// Copies the static folder as it is, keeping its layout.
''')
commit("jonas", ago(84, 5), "Skip writing pages that didn't change")

edit("Cargo.toml", 'version = "0.1.0"', 'version = "0.2.0"')
edit("CHANGELOG.md", "# Changelog\n", r'''
# Changelog

## 0.2.0

- Code blocks are highlighted with syntect.
- Pages with `draft = true` are left out unless built with `--drafts`.
- The index page lists dated posts, newest first.
- A footer names the site's author.
- Unchanged pages are no longer rewritten, so rebuilds are quieter.
''')
commit("mira", ago(80, 3), "Release 0.2.0")

# Both feature/rss-feed and main get commits before the branch is merged, so
# the history has a real merge.

git("checkout", "-q", "-b", "feature/rss-feed")
edit("Cargo.toml", 'anyhow = "1"', 'anyhow = "1"\nchrono = { version = "0.4", default-features = false, features = ["std"] }')
write("src/feed.rs", r'''
use std::fs;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::Deserialize;

use crate::config::Config;
use crate::content::Page;

/// The `[feed]` table of lumen.toml.
#[derive(Debug, Clone, Deserialize)]
pub struct FeedConfig {
    #[serde(default = "default_file_name")]
    pub file_name: String,
    /// How many of the newest posts the feed lists.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_file_name() -> String {
    "feed.xml".to_string()
}

fn default_limit() -> usize {
    20
}

/// Writes an RSS 2.0 feed of `posts`, which come newest first.
pub fn write(config: &Config, feed: &FeedConfig, posts: &[&Page]) -> Result<()> {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<rss version=\"2.0\">\n<channel>\n");
    xml.push_str(&format!("  <title>{}</title>\n", escape(&config.title)));
    xml.push_str(&format!("  <link>{}</link>\n", escape(&config.base_url)));
    for post in posts.iter().take(feed.limit) {
        let link = format!("{}{}", config.base_url.trim_end_matches('/'), post.url());
        xml.push_str("  <item>\n");
        xml.push_str(&format!("    <title>{}</title>\n", escape(&post.title)));
        xml.push_str(&format!("    <link>{}</link>\n", escape(&link)));
        xml.push_str(&format!("    <guid>{}</guid>\n", escape(&link)));
        if let Some(date) = &post.date {
            xml.push_str(&format!("    <pubDate>{}</pubDate>\n", rfc2822(date)?));
        }
        xml.push_str("  </item>\n");
    }
    xml.push_str("</channel>\n</rss>\n");
    let path = config.output_dir.join(&feed.file_name);
    fs::write(&path, xml).with_context(|| format!("couldn't write {}", path.display()))
}

/// RSS wants dates like `Tue, 19 May 2026 00:00:00 +0000`.
fn rfc2822(date: &str) -> Result<String> {
    let day = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .with_context(|| format!("{date} isn't a date like 2026-05-19"))?;
    Ok(day.and_hms_opt(0, 0, 0).expect("midnight exists").and_utc().to_rfc2822())
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
''')
edit("src/config.rs", r'''
use anyhow::{Context, Result};
use serde::Deserialize;

pub const CONFIG_FILE''', r'''
use anyhow::{Context, Result};
use serde::Deserialize;

use crate::feed::FeedConfig;

pub const CONFIG_FILE''')
edit("src/config.rs", r'''
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
}
''', r'''
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
    /// Writes an RSS feed of the dated pages when present.
    pub feed: Option<FeedConfig>,
}
''')
edit("src/render.rs", r'''
use crate::content::Page;
use crate::markdown;
''', r'''
use crate::content::Page;
use crate::{feed, markdown};
''')
edit("src/render.rs", r'''
    write_if_changed(&config.output_dir.join("index.html"), &index)?;
''', r'''
    write_if_changed(&config.output_dir.join("index.html"), &index)?;
    if let Some(feed) = &config.feed {
        feed::write(config, feed, &posts)?;
    }
''')
edit("src/main.rs", "mod content;", "mod content;\nmod feed;")
edit("templates/base.html", r'''
  <link rel="stylesheet" href="/style.css">
''', r'''
  <link rel="stylesheet" href="/style.css">
  <link rel="alternate" type="application/rss+xml" title="{{ site_title }}" href="/feed.xml">
''')
commit("priya", ago(76, 8), "Add an RSS feed")

git("checkout", "-q", "main")

write("src/markdown.rs", read("src/markdown.rs").replace(r'''
/// Turns Markdown into HTML, with tables, footnotes and strikethrough.
/// Headings get an id made from their text, so they can be linked to.
pub fn to_html(markdown: &str) -> String {
    let events = with_heading_anchors(Parser::new_ext(markdown, options()));
    let events = with_highlighted_code(events);
    let mut output = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut output, events.into_iter());
    output
}
'''.lstrip("\n"), r'''
/// A page's Markdown as HTML, and the headings in it.
pub struct Rendered {
    pub html: String,
    pub toc: Vec<TocEntry>,
}

/// A heading as the table of contents lists it.
#[derive(Debug, Serialize)]
pub struct TocEntry {
    pub level: u8,
    pub id: String,
    pub text: String,
}

/// Turns Markdown into HTML, with tables, footnotes and strikethrough.
/// Headings get an id made from their text, so they can be linked to.
pub fn render(markdown: &str) -> Rendered {
    let mut toc = Vec::new();
    let events = with_heading_anchors(Parser::new_ext(markdown, options()), &mut toc);
    let events = with_highlighted_code(events);
    let mut html = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut html, events.into_iter());
    Rendered { html, toc }
}
'''.lstrip("\n")))
edit("src/markdown.rs", r'''
use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};
''', r'''
use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};
use serde::Serialize;
''')
edit("src/markdown.rs", r'''
/// Replaces each heading with HTML that carries an id, keeping its contents.
fn with_heading_anchors<'a>(events: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
''', r'''
/// Replaces each heading with HTML that carries an id, keeping its contents,
/// and adds it to `toc`.
fn with_heading_anchors<'a>(
    events: impl Iterator<Item = Event<'a>>,
    toc: &mut Vec<TocEntry>,
) -> Vec<Event<'a>> {
''')
edit("src/markdown.rs", r'''
                let id = slugify(&plain_text(&inner));
                output.push(Event::Html(format!("<{level} id=\"{id}\">").into()));
''', r'''
                let text = plain_text(&inner);
                let id = slugify(&text);
                output.push(Event::Html(format!("<{level} id=\"{id}\">").into()));
                toc.push(TocEntry {
                    level: level as u8,
                    id,
                    text,
                });
''')
edit("src/render.rs", r'''
        context.insert("date", &page.date);
        context.insert("content", &markdown::to_html(&page.body));
''', r'''
        context.insert("date", &page.date);
        let rendered = markdown::render(&page.body);
        context.insert("content", &rendered.html);
        if rendered.toc.len() >= MIN_TOC_ENTRIES {
            context.insert("toc", &rendered.toc);
        }
''')
edit("src/render.rs", r'''
/// Fills the site's templates with pages.
''', r'''
/// Pages with fewer headings get no table of contents.
const MIN_TOC_ENTRIES: usize = 3;

/// Fills the site's templates with pages.
''')
edit("templates/page.html", r'''
  <h1>{{ title | safe }}</h1>
  {{ content | safe }}
''', r'''
  <h1>{{ title | safe }}</h1>
  {% if toc %}
  <nav class="toc" aria-label="Contents">
    <ol>
      {% for entry in toc %}
      <li class="toc-level-{{ entry.level }}"><a href="#{{ entry.id }}">{{ entry.text }}</a></li>
      {% endfor %}
    </ol>
  </nav>
  {% endif %}
  {{ content | safe }}
''')
edit("static/style.css", r'''
pre {
''', r'''
.toc {
  margin-block: 1.5rem;
  padding: 0.75rem 1rem;
  border-left: 3px solid var(--rule);
  font-size: 0.95rem;
}

.toc ol {
  margin: 0;
  padding-left: 1.25rem;
}

.toc-level-3 {
  margin-left: 1rem;
}

pre {
''')
commit("ada", ago(74, 4), "Add a table of contents to long pages")

git("checkout", "-q", "feature/rss-feed")
edit("src/content.rs", r'''
impl Page {
    /// The page's address below the site root, such as `/hello-world/`.
    pub fn url(&self) -> String {
        format!("/{}/", self.slug)
    }
}
''', r'''
impl Page {
    /// The page's address below the site root, such as `/hello-world/`.
    pub fn url(&self) -> String {
        format!("/{}/", self.slug)
    }

    /// The first paragraph of the body, for feeds and lists of pages.
    pub fn summary(&self) -> &str {
        self.body
            .split("\n\n")
            .map(str::trim)
            .find(|paragraph| !paragraph.is_empty() && !paragraph.starts_with('#'))
            .unwrap_or("")
    }
}
''')
edit("src/feed.rs", r'''
        xml.push_str(&format!("    <guid>{}</guid>\n", escape(&link)));
''', r'''
        xml.push_str(&format!("    <guid>{}</guid>\n", escape(&link)));
        let summary = post.summary();
        if !summary.is_empty() {
            xml.push_str(&format!("    <description>{}</description>\n", escape(summary)));
        }
''')
commit("priya", ago(72, 6), "Put page summaries in the feed")

git("checkout", "-q", "main")
git("merge", "-q", "--no-ff", "feature/rss-feed", "-m", "Merge branch 'feature/rss-feed'", author="mira", when=ago(70, 5))

edit("src/content.rs", r'''
    pub title: String,
    /// The last part of the page's URL.
    pub slug: String,
''', r'''
    pub title: String,
    /// The folder below the content folder, such as `posts`, or empty.
    pub folder: String,
    /// The last part of the page's URL.
    pub slug: String,
''')
edit("src/content.rs", r'''
    /// The page's address below the site root, such as `/hello-world/`.
    pub fn url(&self) -> String {
        format!("/{}/", self.slug)
    }
''', r'''
    /// The page's address relative to the site root, such as
    /// `posts/hello-world/`.
    pub fn url(&self) -> String {
        if self.folder.is_empty() {
            format!("{}/", self.slug)
        } else {
            format!("{}/{}/", self.folder, self.slug)
        }
    }
''')
edit("src/content.rs", r'''
    let mut pages = Vec::new();
    collect(content_dir, &mut pages)?;
''', r'''
    let mut pages = Vec::new();
    collect(content_dir, content_dir, &mut pages)?;
''')
edit("src/content.rs", r'''
fn collect(folder: &Path, pages: &mut Vec<Page>) -> Result<()> {
''', r'''
fn collect(content_dir: &Path, folder: &Path, pages: &mut Vec<Page>) -> Result<()> {
''')
edit("src/content.rs", r'''
        if path.is_dir() {
            collect(&path, pages)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            pages.push(read_page(&path)?);
        }
''', r'''
        if path.is_dir() {
            collect(content_dir, &path, pages)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            pages.push(read_page(content_dir, &path)?);
        }
''')
edit("src/content.rs", r'''
fn read_page(path: &Path) -> Result<Page> {
''', r'''
fn read_page(content_dir: &Path, path: &Path) -> Result<Page> {
''')
edit("src/content.rs", r'''
    let slug = front_matter.slug.unwrap_or_else(|| slugify(&title));
    Ok(Page {
        source: path.to_path_buf(),
        title,
        slug,
''', r'''
    let slug = front_matter.slug.unwrap_or_else(|| slugify(&title));
    let folder = path
        .parent()
        .and_then(|parent| parent.strip_prefix(content_dir).ok())
        .map(|folder| folder.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    Ok(Page {
        source: path.to_path_buf(),
        title,
        folder,
        slug,
''')
edit("src/render.rs", r'''
        context.insert("title", &page.title);
        context.insert("date", &page.date);
''', r'''
        context.insert("root", &root_path(&page.url()));
        context.insert("title", &page.title);
        context.insert("date", &page.date);
''')
edit("src/render.rs", r'''
        context.insert("author", &config.author);
        context.insert("posts", &links);
''', r'''
        context.insert("author", &config.author);
        context.insert("root", "");
        context.insert("posts", &links);
''')
edit("src/render.rs", r'''
/// Where a page goes: a folder named after its slug, so URLs need no `.html`.
fn output_path(config: &Config, page: &Page) -> Result<PathBuf> {
    let relative = page.source.strip_prefix(&config.content_dir)?;
    let folder = relative.parent().unwrap_or(Path::new(""));
    Ok(config.output_dir.join(folder).join(&page.slug).join("index.html"))
}
''', r'''
/// Where a page goes: a folder named after its slug, so URLs need no `.html`.
fn output_path(config: &Config, page: &Page) -> Result<PathBuf> {
    Ok(config.output_dir.join(page.url()).join("index.html"))
}

/// The way from a page back to the site root, such as `../../`. Links that
/// start with it work wherever the site is hosted, even in a subfolder.
fn root_path(url: &str) -> String {
    let depth = url.split('/').filter(|part| !part.is_empty()).count();
    "../".repeat(depth)
}
''')
edit("src/feed.rs", r'''
        let link = format!("{}{}", config.base_url.trim_end_matches('/'), post.url());
''', r'''
        let link = format!("{}/{}", config.base_url.trim_end_matches('/'), post.url());
''')
edit("templates/base.html", r'''
  <link rel="stylesheet" href="/style.css">
  <link rel="alternate" type="application/rss+xml" title="{{ site_title }}" href="/feed.xml">
''', r'''
  <link rel="stylesheet" href="{{ root }}style.css">
  <link rel="alternate" type="application/rss+xml" title="{{ site_title }}" href="{{ root }}feed.xml">
''')
edit("templates/base.html", r'''
    <a class="site-title" href="/">{{ site_title }}</a>
''', r'''
    <a class="site-title" href="{{ root }}">{{ site_title }}</a>
''')
edit("templates/index.html", r'''
    <a href="{{ post.url }}">{{ post.title | safe }}</a>
''', r'''
    <a href="{{ root }}{{ post.url }}">{{ post.title | safe }}</a>
''')
commit("tomas", ago(66, 3), "Fix links to pages in subfolders")

write("assets/live-reload.js", r'''
// Reloads the page when `lumen serve` finishes a rebuild. Only added to
// pages while serving, never to a built site.
(() => {
  const RETRY_MS = 1000;

  function connect() {
    const events = new EventSource("/__lumen/reload");
    events.addEventListener("message", (event) => {
      if (event.data === "reload") {
        window.location.reload();
      }
    });
    // The server restarts now and then; keep trying until it is back.
    events.addEventListener("error", () => {
      events.close();
      setTimeout(connect, RETRY_MS);
    });
  }

  connect();
})();
''')
edit("src/server.rs", r'''
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use notify::{RecursiveMode, Watcher};
use tiny_http::{Header, Response, Server};
''', r'''
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use notify::{RecursiveMode, Watcher};
use tiny_http::{Header, Request, Response, Server};

/// Where pages listen for rebuilds; see assets/live-reload.js.
const RELOAD_PATH: &str = "/__lumen/reload";
''')
edit("src/server.rs", r'''
/// Serves the built site on `port` until the process ends.
pub fn serve(output_dir: &Path, port: u16) -> Result<()> {
    let server = Server::http(("127.0.0.1", port)).map_err(|error| anyhow!(error))?;
    println!("Serving on http://localhost:{port}/");
    for request in server.incoming_requests() {
        let path = resolve(output_dir, request.url());
''', r'''
/// The pages open in a browser, which reload after every rebuild.
#[derive(Clone, Default)]
pub struct Reloader {
    clients: Arc<Mutex<Vec<mpsc::Sender<()>>>>,
}

impl Reloader {
    /// Tells every open page to reload, and forgets the closed ones.
    pub fn notify(&self) {
        let mut clients = self.clients.lock().expect("no thread panics holding it");
        clients.retain(|client| client.send(()).is_ok());
    }

    fn subscribe(&self) -> mpsc::Receiver<()> {
        let (sender, receiver) = mpsc::channel();
        self.clients.lock().expect("no thread panics holding it").push(sender);
        receiver
    }
}

/// Serves the built site on `port` until the process ends.
pub fn serve(output_dir: &Path, port: u16, reloader: Reloader) -> Result<()> {
    let server = Server::http(("127.0.0.1", port)).map_err(|error| anyhow!(error))?;
    println!("Serving on http://localhost:{port}/");
    for request in server.incoming_requests() {
        if request.url() == RELOAD_PATH {
            let rebuilds = reloader.subscribe();
            thread::spawn(move || stream_reloads(request, rebuilds));
            continue;
        }
        let path = resolve(output_dir, request.url());
''')
edit("src/server.rs", r'''
/// Maps a request path to a file, serving `index.html` for folders.
''', r'''
/// Holds an event stream open and sends `reload` after each rebuild, until
/// the page goes away.
fn stream_reloads(request: Request, rebuilds: mpsc::Receiver<()>) {
    let mut writer = request.into_writer();
    let head = "HTTP/1.1 200 OK\r\n\
                Content-Type: text/event-stream\r\n\
                Cache-Control: no-cache\r\n\r\n";
    if writer.write_all(head.as_bytes()).is_err() {
        return;
    }
    while rebuilds.recv().is_ok() {
        let sent = writer.write_all(b"data: reload\n\n").and_then(|()| writer.flush());
        if sent.is_err() {
            return;
        }
    }
}

/// Maps a request path to a file, serving `index.html` for folders.
''')
edit("src/render.rs", r'''
/// Fills the site's templates with pages.
pub struct Renderer {
    tera: Tera,
}
''', r'''
/// Fills the site's templates with pages.
pub struct Renderer {
    tera: Tera,
    /// Whether pages get the live reload client, while serving.
    live_reload: bool,
}

const LIVE_RELOAD_SCRIPT: &str = include_str!("../assets/live-reload.js");
''')
edit("src/render.rs", r'''
    pub fn new(templates_dir: &Path) -> Result<Self> {
        let pattern = templates_dir.join("**/*.html");
        let tera = Tera::new(&pattern.to_string_lossy()).context("couldn't load the templates")?;
        Ok(Self { tera })
    }
''', r'''
    pub fn new(templates_dir: &Path, live_reload: bool) -> Result<Self> {
        let pattern = templates_dir.join("**/*.html");
        let tera = Tera::new(&pattern.to_string_lossy()).context("couldn't load the templates")?;
        Ok(Self { tera, live_reload })
    }
''')
edit("src/render.rs", r'''
        self.tera
            .render("page.html", &context)
            .with_context(|| format!("couldn't render {}", page.source.display()))
    }
''', r'''
        let html = self
            .tera
            .render("page.html", &context)
            .with_context(|| format!("couldn't render {}", page.source.display()))?;
        Ok(if self.live_reload {
            with_live_reload(&html)
        } else {
            html
        })
    }
''')
edit("src/render.rs", r'''
/// Renders every page into the output folder.
pub fn build(config: &Config, pages: &[Page]) -> Result<()> {
    let renderer = Renderer::new(&config.templates_dir)?;
''', r'''
/// Renders every page into the output folder, with the live reload client
/// when `live_reload` is set.
pub fn build(config: &Config, pages: &[Page], live_reload: bool) -> Result<()> {
    let renderer = Renderer::new(&config.templates_dir, live_reload)?;
''')
edit("src/render.rs", r'''
/// Where a page goes: a folder named after its slug, so URLs need no `.html`.
''', r'''
/// Adds the live reload client just before `</body>`.
fn with_live_reload(html: &str) -> String {
    let script = format!("<script>{LIVE_RELOAD_SCRIPT}</script>\n</body>");
    html.replacen("</body>", &script, 1)
}

/// Where a page goes: a folder named after its slug, so URLs need no `.html`.
''')
edit("src/main.rs", r'''
        Command::Build => build(&config, cli.drafts)?,
        Command::Serve { port } => {
            build(&config, cli.drafts)?;
            let rebuild_config = config.clone();
            let drafts = cli.drafts;
            server::watch(config.content_dir.clone(), move || {
                if let Err(error) = build(&rebuild_config, drafts) {
                    eprintln!("Build failed: {error:#}");
                }
            })?;
            server::serve(&config.output_dir, port)?;
        }
''', r'''
        Command::Build => build(&config, cli.drafts, false)?,
        Command::Serve { port } => {
            build(&config, cli.drafts, true)?;
            let reloader = server::Reloader::default();
            let rebuild_config = config.clone();
            let rebuild_reloader = reloader.clone();
            let drafts = cli.drafts;
            server::watch(config.content_dir.clone(), move || {
                match build(&rebuild_config, drafts, true) {
                    Ok(()) => rebuild_reloader.notify(),
                    Err(error) => eprintln!("Build failed: {error:#}"),
                }
            })?;
            server::serve(&config.output_dir, port, reloader)?;
        }
''')
edit("src/main.rs", r'''
fn build(config: &Config, drafts: bool) -> Result<()> {
    let pages = content::load_pages(&config.content_dir, drafts)?;
    render::build(config, &pages)?;
''', r'''
fn build(config: &Config, drafts: bool, live_reload: bool) -> Result<()> {
    let pages = content::load_pages(&config.content_dir, drafts)?;
    render::build(config, &pages, live_reload)?;
''')
commit("jonas", ago(60, 4), "Add live reload to the dev server", coauthors=["tomas"])

write("README.md", r'''
# Lumen

A small, fast static site generator. Write pages in Markdown, style them
with Tera templates, and get a folder of plain HTML you can host anywhere.

## Getting started

```sh
cargo install lumen
lumen --root my-site build
lumen --root my-site serve --port 4000
```

`serve` rebuilds the site whenever a page changes and reloads the browser.

## Pages

Every `.md` file below `content/` becomes a page. A page may start with TOML
front matter:

```toml
+++
title = "Building a garden shed"
date = "2026-05-19"
tags = ["garden", "woodwork"]
draft = false
+++
```

Pages with a date are listed on the index page, newest first.

## Configuration

`lumen.toml` sits in the root of the site:

| Key             | Default       | Meaning                                   |
|-----------------|---------------|-------------------------------------------|
| `title`         | (required)    | The site's name, shown in every header    |
| `base_url`      | (required)    | Where the site is hosted, for the feed    |
| `author`        | empty         | Named in the footer                       |
| `content_dir`   | `content`     | Where the Markdown pages are              |
| `templates_dir` | `templates`   | Where the Tera templates are              |
| `static_dir`    | `static`      | Copied into the output as it is           |
| `output_dir`    | `public`      | Where the built site goes                 |

Add a `[feed]` table to write an RSS feed:

```toml
[feed]
file_name = "feed.xml"
limit = 20
```

## License

MIT
''')
commit("mira", ago(55, 2), "Document the configuration options")

edit("templates/page.html", "  <h1>{{ title | safe }}</h1>", "  <h1>{{ title }}</h1>")
edit("templates/index.html", '<a href="{{ root }}{{ post.url }}">{{ post.title | safe }}</a>', '<a href="{{ root }}{{ post.url }}">{{ post.title }}</a>')
commit("ada", ago(51, 6), "Escape HTML in page titles")

edit("Cargo.toml", 'pulldown-cmark = "0.11"', 'pulldown-cmark = "0.12"')
edit("src/markdown.rs", "use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};",
     "use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};")
edit("src/markdown.rs", r'''
            Event::Start(Tag::Heading(level, ..)) => heading = Some((level, Vec::new())),
            Event::End(Tag::Heading(..)) => {
''', r'''
            Event::Start(Tag::Heading { level, .. }) => heading = Some((level, Vec::new())),
            Event::End(TagEnd::Heading(_)) => {
''')
edit("src/markdown.rs", r'''
            Event::End(Tag::CodeBlock(_)) if code.is_some() => {
''', r'''
            Event::End(TagEnd::CodeBlock) if code.is_some() => {
''')
commit("priya", ago(46, 3), "Update pulldown-cmark to 0.12")

write("src/scaffold.rs", r'''
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

/// The files of a new site, with the default theme.
const FILES: &[(&str, &str)] = &[
    ("lumen.toml", STARTER_CONFIG),
    ("content/hello-world.md", STARTER_PAGE),
    ("templates/base.html", include_str!("../templates/base.html")),
    ("templates/page.html", include_str!("../templates/page.html")),
    ("templates/index.html", include_str!("../templates/index.html")),
    ("static/style.css", include_str!("../static/style.css")),
];

const STARTER_CONFIG: &str = r#"title = "My new site"
base_url = "https://example.com"
author = ""
"#;

const STARTER_PAGE: &str = r#"+++
title = "Hello, world"
date = "2026-01-01"
+++

This is your first page. Edit it in content/hello-world.md, or add more
Markdown files next to it.
"#;

/// Creates a site in `folder`, which must be empty or not exist yet.
pub fn create(folder: &Path) -> Result<()> {
    if folder.exists() && fs::read_dir(folder)?.next().is_some() {
        bail!("{} isn't empty", folder.display());
    }
    for (relative, contents) in FILES {
        let path = folder.join(relative);
        fs::create_dir_all(path.parent().expect("every file is in a folder"))?;
        fs::write(&path, contents).with_context(|| format!("couldn't write {}", path.display()))?;
    }
    println!("Created a site in {}. Run `lumen serve` there to see it.", folder.display());
    Ok(())
}
''')
edit("src/main.rs", "mod render;", "mod render;\nmod scaffold;")
edit("src/main.rs", r'''
#[derive(Subcommand)]
enum Command {
''', r'''
#[derive(Subcommand)]
enum Command {
    /// Creates a site with the default theme and one page.
    New {
        /// The folder to create it in.
        path: PathBuf,
    },
''')
edit("src/main.rs", r'''
    let cli = Cli::parse();
    let config = Config::load(&cli.root)?;
''', r'''
    let cli = Cli::parse();
    if let Command::New { path } = &cli.command {
        return scaffold::create(path);
    }
    let config = Config::load(&cli.root)?;
''')
edit("src/main.rs", r'''
        Command::Build => build(&config, cli.drafts, false)?,
''', r'''
        Command::New { .. } => unreachable!("handled before loading the config"),
        Command::Build => build(&config, cli.drafts, false)?,
''')
commit("mira", ago(40, 5), "Add the new command to start a site")

edit("src/server.rs", r'''
/// The pages open in a browser, which reload after every rebuild.
#[derive(Clone, Default)]
pub struct Reloader {
    clients: Arc<Mutex<Vec<mpsc::Sender<()>>>>,
}

impl Reloader {
    /// Tells every open page to reload, and forgets the closed ones.
    pub fn notify(&self) {
        let mut clients = self.clients.lock().expect("no thread panics holding it");
        clients.retain(|client| client.send(()).is_ok());
    }

    fn subscribe(&self) -> mpsc::Receiver<()> {
''', r'''
/// The outcome of a rebuild: `Ok` to reload, or the error to show.
type Rebuild = Result<(), String>;

/// The pages open in a browser, which reload after every rebuild.
#[derive(Clone, Default)]
pub struct Reloader {
    clients: Arc<Mutex<Vec<mpsc::Sender<Rebuild>>>>,
}

impl Reloader {
    /// Tells every open page how a rebuild went, and forgets the closed ones.
    pub fn notify(&self, rebuild: Rebuild) {
        let mut clients = self.clients.lock().expect("no thread panics holding it");
        clients.retain(|client| client.send(rebuild.clone()).is_ok());
    }

    fn subscribe(&self) -> mpsc::Receiver<Rebuild> {
''')
edit("src/server.rs", r'''
/// Holds an event stream open and sends `reload` after each rebuild, until
/// the page goes away.
fn stream_reloads(request: Request, rebuilds: mpsc::Receiver<()>) {
''', r'''
/// Holds an event stream open and sends `reload` after each rebuild, or a
/// `build-error` event when one fails, until the page goes away.
fn stream_reloads(request: Request, rebuilds: mpsc::Receiver<Rebuild>) {
''')
edit("src/server.rs", r'''
    while rebuilds.recv().is_ok() {
        let sent = writer.write_all(b"data: reload\n\n").and_then(|()| writer.flush());
''', r'''
    while let Ok(rebuild) = rebuilds.recv() {
        let event = match rebuild {
            Ok(()) => "data: reload\n\n".to_string(),
            // Each line of the message needs its own `data:` field.
            Err(message) => {
                let data: Vec<String> = message.lines().map(|line| format!("data: {line}")).collect();
                format!("event: build-error\n{}\n\n", data.join("\n"))
            }
        };
        let sent = writer.write_all(event.as_bytes()).and_then(|()| writer.flush());
''')
edit("assets/live-reload.js", r'''
// Reloads the page when `lumen serve` finishes a rebuild. Only added to
// pages while serving, never to a built site.
(() => {
  const RETRY_MS = 1000;

  function connect() {
''', r'''
// Reloads the page when `lumen serve` finishes a rebuild, or shows why the
// rebuild failed. Only added to pages while serving, never to a built site.
(() => {
  const RETRY_MS = 1000;
  const OVERLAY_ID = "lumen-build-error";

  function showError(message) {
    let overlay = document.getElementById(OVERLAY_ID);
    if (!overlay) {
      overlay = document.createElement("pre");
      overlay.id = OVERLAY_ID;
      overlay.style.cssText =
        "position:fixed;inset:auto 1rem 1rem;margin:0;padding:1rem;" +
        "border:1px solid #a1564a;border-radius:4px;background:#2d1c19;" +
        "color:#f0998a;font:14px/1.5 monospace;white-space:pre-wrap;z-index:9999";
      overlay.title = "Click to hide";
      overlay.addEventListener("click", () => overlay.remove());
      document.body.append(overlay);
    }
    overlay.textContent = `Build failed\n\n${message}`;
  }

  function connect() {
''')
edit("assets/live-reload.js", r'''
    // The server restarts now and then; keep trying until it is back.
''', r'''
    events.addEventListener("build-error", (event) => showError(event.data));
    // The server restarts now and then; keep trying until it is back.
''')
edit("src/main.rs", r'''
                match build(&rebuild_config, drafts, true) {
                    Ok(()) => rebuild_reloader.notify(),
                    Err(error) => eprintln!("Build failed: {error:#}"),
                }
''', r'''
                let result = build(&rebuild_config, drafts, true);
                if let Err(error) = &result {
                    eprintln!("Build failed: {error:#}");
                }
                rebuild_reloader.notify(result.map_err(|error| format!("{error:#}")));
''')
commit("tomas", ago(33, 4), "Show build errors in the browser while serving")

edit("Cargo.toml", 'version = "0.2.0"', 'version = "0.3.0"')
edit("CHANGELOG.md", "# Changelog\n", r'''
# Changelog

## 0.3.0

- An RSS feed of dated posts, with summaries, when `[feed]` is set.
- Long pages get a table of contents.
- `lumen serve` reloads the browser after a rebuild and shows build errors.
- `lumen new` starts a site with the default theme.
- Links work for pages in subfolders and for sites hosted below a path.
- Page titles are escaped.
''')
commit("mira", ago(27, 6), "Release 0.3.0")
git("push", "-q", "origin", "main")
git("push", "-q", "origin", "feature/rss-feed")

# docs/theming is only on the remote, to list a remote branch without a local
# one.

git("checkout", "-q", "-b", "docs/theming")
write("docs/theming.md", r'''
# Theming

A theme is a `templates` folder and a `static` folder. Lumen ships one, and
`lumen new` copies it into every new site, so you can change it freely.

## Templates

| Template     | Renders                    | Variables                                 |
|--------------|----------------------------|-------------------------------------------|
| `base.html`  | The frame of every page    | `site_title`, `author`, `root`            |
| `page.html`  | One Markdown page          | `title`, `date`, `content`, `toc`         |
| `index.html` | The home page              | `posts`, each with `title`, `url`, `date` |

Start every link with `{{ root }}` so it works for pages in subfolders.

## Styles

Everything in `static` is copied as it is. The default stylesheet keeps its
colors in custom properties at the top, so most themes only change those.
''')
commit("ada", ago(20, 3), "Write a guide to theming")
git("push", "-q", "origin", "docs/theming")
git("checkout", "-q", "main")
git("branch", "-q", "-D", "docs/theming")

edit("src/server.rs", r'''
/// Calls `rebuild` on another thread whenever something in `folder` changes.
pub fn watch(folder: PathBuf, rebuild: impl Fn() + Send + 'static) -> Result<()> {
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(sender)?;
    watcher.watch(&folder, RecursiveMode::Recursive)?;
''', r'''
/// Calls `rebuild` on another thread whenever something in `folders` changes.
pub fn watch(folders: &[PathBuf], rebuild: impl Fn() + Send + 'static) -> Result<()> {
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(sender)?;
    for folder in folders {
        watcher.watch(folder, RecursiveMode::Recursive)?;
    }
''')
edit("src/main.rs", r'''
            server::watch(config.content_dir.clone(), move || {
''', r'''
            let watched = [config.content_dir.clone(), config.templates_dir.clone()];
            server::watch(&watched, move || {
''')
commit("jonas", ago(12, 5), "Watch the templates folder too")

write("static/style.css", read("static/style.css") + r'''
@media (max-width: 40rem) {
  body {
    font-size: 1rem;
  }

  .post-link {
    flex-direction: column;
    gap: 0;
  }

  pre {
    margin-inline: -1.25rem;
    border-radius: 0;
  }
}
''')
commit("ada", ago(9, 2), "Make the default theme work on phones")
git("push", "-q", "origin", "main")

# fix/slug-collisions has an upstream, and feature/search-index has none.

git("checkout", "-q", "-b", "fix/slug-collisions")
edit("src/content.rs", r'''
    if !include_drafts {
        pages.retain(|page| !page.draft);
    }
    Ok(pages)
}
''', r'''
    if !include_drafts {
        pages.retain(|page| !page.draft);
    }
    number_colliding_slugs(&mut pages);
    Ok(pages)
}

/// Gives pages in one folder whose slugs collide a number, so the second
/// "Notes" becomes `notes-2` instead of overwriting the first.
fn number_colliding_slugs(pages: &mut [Page]) {
    pages.sort_by(|left, right| left.source.cmp(&right.source));
    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    for page in pages.iter_mut() {
        let key = (page.folder.clone(), page.slug.clone());
        let count = seen.entry(key).or_insert(0);
        *count += 1;
        if *count > 1 {
            page.slug = format!("{}-{count}", page.slug);
        }
    }
}
''')
edit("src/content.rs", r'''
use std::fs;
use std::path::{Path, PathBuf};
''', r'''
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
''')
commit("tomas", ago(6, 4), "Number pages whose slugs collide")
git("push", "-q", "-u", "origin", "fix/slug-collisions")
git("checkout", "-q", "main")

git("checkout", "-q", "-b", "feature/search-index")
edit("Cargo.toml", 'serde = { version = "1", features = ["derive"] }', 'serde = { version = "1", features = ["derive"] }\nserde_json = "1"')
write("src/search.rs", r'''
use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;
use serde::Serialize;

use crate::config::Config;
use crate::content::Page;

/// Words shorter than this aren't indexed.
const MIN_WORD_LENGTH: usize = 3;

/// One page as the search box shows it.
#[derive(Serialize)]
struct Entry<'a> {
    title: &'a str,
    url: String,
    summary: &'a str,
}

/// The pages, and for each word the positions of the pages it is in.
#[derive(Serialize)]
struct Index<'a> {
    pages: Vec<Entry<'a>>,
    words: BTreeMap<String, Vec<usize>>,
}

/// Writes `search.json`, which the default theme's search box loads.
pub fn write_index(config: &Config, pages: &[Page]) -> Result<()> {
    let mut index = Index {
        pages: Vec::with_capacity(pages.len()),
        words: BTreeMap::new(),
    };
    for (position, page) in pages.iter().enumerate() {
        index.pages.push(Entry {
            title: &page.title,
            url: page.url(),
            summary: page.summary(),
        });
        for word in words(&page.title).chain(words(&page.body)) {
            let postings = index.words.entry(word).or_default();
            if postings.last() != Some(&position) {
                postings.push(position);
            }
        }
    }
    fs::write(config.output_dir.join("search.json"), serde_json::to_string(&index)?)?;
    Ok(())
}

fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.chars().count() >= MIN_WORD_LENGTH)
        .map(str::to_lowercase)
}
''')
edit("src/main.rs", "mod scaffold;", "mod scaffold;\nmod search;")
edit("src/render.rs", r'''
    if let Some(feed) = &config.feed {
        feed::write(config, feed, &posts)?;
    }
''', r'''
    if let Some(feed) = &config.feed {
        feed::write(config, feed, &posts)?;
    }
    search::write_index(config, pages)?;
''')
edit("src/render.rs", "use crate::{feed, markdown};", "use crate::{feed, markdown, search};")
commit("ada", ago(5, 6), "Build a JSON search index")
write("static/search.js", r'''
// Searches the pages listed in search.json as the visitor types.
const input = document.querySelector("#search");
const results = document.querySelector("#search-results");
let index = null;

async function loadIndex() {
  index ??= await fetch(`${input.dataset.root}search.json`).then((response) => response.json());
  return index;
}

function search({ pages, words }, query) {
  const terms = query.toLowerCase().split(/\W+/).filter((term) => term.length >= 3);
  if (terms.length === 0) {
    return [];
  }
  // A page matches when every term starts some word in it.
  const matches = terms.map((term) => {
    const found = new Set();
    for (const [word, positions] of Object.entries(words)) {
      if (word.startsWith(term)) {
        positions.forEach((position) => found.add(position));
      }
    }
    return found;
  });
  return [...matches[0]]
    .filter((position) => matches.every((found) => found.has(position)))
    .map((position) => pages[position]);
}

input.addEventListener("input", async () => {
  const found = search(await loadIndex(), input.value);
  results.replaceChildren(
    ...found.map(({ title, url, summary }) => {
      const item = document.createElement("li");
      item.innerHTML = `<a href="${input.dataset.root}${url}"></a><p></p>`;
      item.querySelector("a").textContent = title;
      item.querySelector("p").textContent = summary;
      return item;
    }),
  );
});
''')
edit("templates/base.html", r'''
    <a class="site-title" href="{{ root }}">{{ site_title }}</a>
  </header>
''', r'''
    <a class="site-title" href="{{ root }}">{{ site_title }}</a>
    <input id="search" type="search" placeholder="Search" data-root="{{ root }}">
    <ul id="search-results"></ul>
  </header>
  <script src="{{ root }}search.js" defer></script>
''')
commit("ada", ago(4, 3), "Add a search box to the default theme")

# A stash on this branch, from experimenting with stemming.
edit("src/search.rs", r'''
fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.chars().count() >= MIN_WORD_LENGTH)
        .map(str::to_lowercase)
}
''', r'''
fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.chars().count() >= MIN_WORD_LENGTH)
        .map(|word| stem(&word.to_lowercase()))
}

/// Strips common English endings, so "building" also finds "build".
/// TODO: only English for now; pick the stemmer from the site's language.
fn stem(word: &str) -> String {
    for suffix in ["ings", "ing", "edly", "ed", "es", "s"] {
        if let Some(stem) = word.strip_suffix(suffix) {
            if stem.chars().count() >= MIN_WORD_LENGTH {
                return stem.to_string();
            }
        }
    }
    word.to_string()
}
''')
git("stash", "push", "-q", "-m", "Stem words in the search index", author="mira", when=ago(3, 7))
git("checkout", "-q", "main")

edit("static/style.css", r'''
:root {
  --text: #2b2724;
  --muted: #7a716a;
  --background: #fbf8f3;
  --accent: #b5562f;
  --rule: #e6ded3;
  --content-width: 42rem;
}
''', r'''
:root {
  --text: #2b2724;
  --muted: #7a716a;
  --background: #fbf8f3;
  --accent: #b5562f;
  --rule: #e6ded3;
  --content-width: 42rem;
  color-scheme: light dark;
}

@media (prefers-color-scheme: dark) {
  :root {
    --text: #e8e2da;
    --muted: #a39a90;
    --background: #1b1917;
    --accent: #e58a62;
    --rule: #34302c;
  }
}
''')
git("stash", "push", "-q", "-m", "Try a dark variant of the default theme", author="mira", when=ago(2, 4))

edit("Cargo.toml", 'pulldown-cmark = "0.12"', 'pulldown-cmark = "0.12"\nrayon = "1"')
edit("src/render.rs", r'''
    for page in pages {
        let html = renderer.render_page(config, page)?;
        write_if_changed(&output_path(config, page)?, &html)?;
    }
''', r'''
    // Rendering is independent per page, so it spreads over every core.
    pages.par_iter().try_for_each(|page| {
        let html = renderer.render_page(config, page)?;
        write_if_changed(&output_path(config, page)?, &html).map(|_| ())
    })?;
''')
edit("src/render.rs", r'''
use anyhow::{Context, Result};
use serde::Serialize;
''', r'''
use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::Serialize;
''')
git("stash", "push", "-q", "-m", "Render pages in parallel", author="mira", when=ago(1, 9))

# The last commits on main stay unpushed, so they can be reworded, squashed
# and dropped.

edit("src/content.rs", r'''
const FRONT_MATTER_FENCE: &str = "+++";
''', r'''
const FRONT_MATTER_FENCE: &str = "+++";

/// How many words a reader gets through in a minute, for reading times.
const WORDS_PER_MINUTE: usize = 220;
''')
edit("src/content.rs", r'''
    /// The first paragraph of the body, for feeds and lists of pages.
''', r'''
    /// The words in the body, leaving out Markdown punctuation.
    pub fn word_count(&self) -> usize {
        self.body
            .split_whitespace()
            .filter(|word| word.chars().any(char::is_alphanumeric))
            .count()
    }

    /// How long the page takes to read, in whole minutes and at least one.
    pub fn reading_minutes(&self) -> usize {
        self.word_count().div_ceil(WORDS_PER_MINUTE).max(1)
    }

    /// The first paragraph of the body, for feeds and lists of pages.
''')
edit("src/render.rs", r'''
        context.insert("date", &page.date);
        let rendered = markdown::render(&page.body);
''', r'''
        context.insert("date", &page.date);
        context.insert("reading_minutes", &page.reading_minutes());
        let rendered = markdown::render(&page.body);
''')
commit("mira", ago(0, 6, 10), "Add reading times to pages")

edit("templates/page.html", r'''
  <h1>{{ title }}</h1>
''', r'''
  <h1>{{ title }}</h1>
  <p class="page-meta">
    {% if date %}<time datetime="{{ date }}">{{ date }}</time> &middot; {% endif %}
    {{ reading_minutes }} minutes to read
  </p>
''')
edit("static/style.css", r'''
.toc {
''', r'''
.page-meta {
  margin-top: -0.5rem;
  color: var(--muted);
  font-size: 0.9rem;
}

.toc {
''')
commit("mira", ago(0, 3, 25), "Show reading times under page titles")

edit("templates/page.html", "    {{ reading_minutes }} minutes to read", "    {{ reading_minutes }} minute{{ reading_minutes | pluralize }} to read")
commit("mira", ago(0, 1, 5), "Fix the reading time for one-minute pages")

# Uncommitted changes, with several hunks in src/config.rs to pick from.

edit("src/config.rs", r'''
use anyhow::{Context, Result};
use serde::Deserialize;
''', r'''
use anyhow::{bail, Context, Result};
use serde::Deserialize;
''')
edit("src/config.rs", r'''
    /// Writes an RSS feed of the dated pages when present.
    pub feed: Option<FeedConfig>,
}
''', r'''
    /// Writes an RSS feed of the dated pages when present.
    pub feed: Option<FeedConfig>,
    /// Strips the whitespace between tags from the built pages.
    #[serde(default)]
    pub minify_html: bool,
}
''')
edit("src/config.rs", r'''
        let mut config: Config = toml::from_str(&text)
            .with_context(|| format!("{} is not valid", path.display()))?;
''', r'''
        let mut config: Config = toml::from_str(&text)
            .with_context(|| format!("{} is not valid", path.display()))?;
        if !config.base_url.starts_with("https://") && !config.base_url.starts_with("http://") {
            bail!(
                "base_url in {} must start with https:// or http://, not {:?}",
                path.display(),
                config.base_url
            );
        }
''')
edit("README.md", r'''
| `output_dir`    | `public`      | Where the built site goes                 |
''', r'''
| `output_dir`    | `public`      | Where the built site goes                 |
| `minify_html`   | `false`       | Strips whitespace between tags            |
''')
write("examples/portfolio/lumen.toml", r'''
title = "Studio Portfolio"
base_url = "https://portfolio.example.com"
author = "Mira Okafor"
templates_dir = "../../templates"
static_dir = "../../static"
minify_html = true
''')

run("git", "branch", "-q", "--set-upstream-to=origin/main", "main")
run("git", "branch", "-q", "--set-upstream-to=origin/feature/rss-feed", "feature/rss-feed")
# A GitHub remote makes the Issues and Pull Requests buttons show.
run("git", "remote", "set-url", "origin", "https://github.com/lumen-ssg/lumen.git")
