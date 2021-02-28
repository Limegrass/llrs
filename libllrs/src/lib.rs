use std::cmp::Ordering;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use futures::{AsyncRead, AsyncWrite};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

// Should redesign DB
#[derive(Debug, Serialize, Deserialize)]
pub struct Manga {
    pub manga_id: i32,
    pub manga_name: String,
    // May make sense to have author/artist per chapter instead (anthologies)?
    // and then this returns all associated
    pub author_names: Vec<String>,
    pub artist_names: Vec<String>,
    pub cover_image_url: String,
    pub purchase_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chapter {
    pub chapter_number: String,
    // not at the chapter level in current db
    // pub author_name: String,
    // pub artist_name: String,
    pub chapter_name: String,
    pub creation_date: NaiveDateTime,
    pub release_date: NaiveDateTime,
    pub manga_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Page {
    url_string: String,
    page_number: i32,
}

pub type Result<T> = std::result::Result<T, Error>;

// TODO: Maybe get rid of i32, can generalize later if it ever becomes needed
#[async_trait]
pub trait MangaService<T> {
    async fn get_all_manga_titles(&mut self) -> Result<Vec<Manga>>;
    async fn get_manga_chapters(&mut self, manga_id: T) -> Result<Vec<Chapter>>;
    async fn get_pages(&mut self, manga_id: T, chapter_number: &str) -> Result<Vec<Page>>;
}

pub struct Waifusims<S: AsyncRead + AsyncWrite + Unpin + Send> {
    client: Client<S>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error ${0:?}")]
    IoError(std::io::Error),
    #[error("IO error ${0:?}")]
    Tiberius(tiberius::error::Error),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}

impl From<tiberius::error::Error> for Error {
    fn from(e: tiberius::error::Error) -> Self {
        Error::Tiberius(e)
    }
}

// TODO: Maybe remove the strong typing
impl Waifusims<Compat<TcpStream>> {
    pub async fn new(config: Config) -> Result<Waifusims<Compat<TcpStream>>> {
        let tcp = TcpStream::connect(config.get_addr()).await?;
        tcp.set_nodelay(true)?;
        let client = Client::connect(config, tcp.compat_write()).await?;
        Ok(Waifusims { client })
    }
}

const SELECT_ALL_MANGA_QUERY: &str = "
SELECT
    m.MangaID,
    m.MangaName,
    a.AuthorName,
    m.CoverImageURL,
    m.PurchaseURL
FROM Manga m
JOIN Author a
    ON m.AuthorID = a.AuthorID
ORDER BY m.MangaID
";

const SELECT_MANGA_CHAPTERS_QUERY: &str = "
SELECT
    ChapterNumber,
    ChapterName,
    DateCreated,
    DateReleased,
    MangaID
FROM MangaChapter
WHERE MangaID = @P1
";

const SELECT_CHAPTER_PAGES_QUERY: &str = "
SELECT
    u.URL,
    p.PageNumber
FROM Page p
JOIN PageURL u
    ON p.PageID = u.PageID
JOIN MangaChapter mc
    ON mc.ChapterIndex = p.ChapterIndex
        AND mc.MangaID = p.MangaID
        AND mc.ChapterNumber = @P2
WHERE u.Priority = 1
    AND p.MangaID = @P1
ORDER BY p.PageNumber
";

// i32 as no u32 in SQL Server
#[async_trait]
impl MangaService<i32> for Waifusims<Compat<TcpStream>> {
    async fn get_all_manga_titles(&mut self) -> Result<Vec<Manga>> {
        let stream = self.client.simple_query(SELECT_ALL_MANGA_QUERY).await?;
        // We only make one query, so one result
        // Take first result, as we only make one query
        let rows = stream.into_first_result().await?;
        // map to Manga and return, should never fail
        rows.iter()
            .map(|row| {
                Ok(Manga {
                    manga_id: row.get("MangaID").expect("MangaID is NOT NULL"),
                    manga_name: row
                        .get::<&str, _>("MangaName")
                        .expect("MangaName is NOT NULL")
                        .to_owned(),
                    author_names: vec![row
                        .get::<&str, _>("AuthorName")
                        .expect("AuthorName is NOT NULL")
                        .to_owned()],
                    artist_names: vec![row
                        .get::<&str, _>("AuthorName")
                        .expect("AuthorName is NOT NULL")
                        .to_owned()],
                    cover_image_url: row
                        .get::<&str, _>("CoverImageURL")
                        .expect("CoverImageURL is hopefully NOT NULL but IDR")
                        .to_owned(),
                    purchase_url: row
                        .get::<&str, _>("PurchaseURL")
                        .expect("PurchaseURL is hopefully NOT NULL but IDR")
                        .to_owned(),
                })
            })
            .collect()
    }

    async fn get_manga_chapters(&mut self, manga_id: i32) -> Result<Vec<Chapter>> {
        let stream = self
            .client
            .query(SELECT_MANGA_CHAPTERS_QUERY, &[&manga_id])
            .await?;
        let rows = stream.into_first_result().await?;
        let mut chapters = rows
            .into_iter()
            .map(|row| Chapter {
                manga_id: row.get("MangaID").expect("MangaID is NOT NULL"),
                chapter_number: row
                    .get::<&str, _>("ChapterNumber")
                    .expect("ChapterNumber is NOT NULL")
                    .to_owned(),
                chapter_name: row
                    .get::<&str, _>("ChapterName")
                    .expect("ChapterName is NOT NULL")
                    .to_owned(),
                creation_date: row
                    .get::<NaiveDateTime, _>("DateCreated")
                    .expect("DateCreated is NOT NULL")
                    .to_owned(),
                release_date: row
                    .get::<NaiveDateTime, _>("DateReleased")
                    .expect("DateReleased is hopefully NOT NULL but IDR")
                    .to_owned(),
            })
            .collect::<Vec<Chapter>>();
        chapters.sort_by(|a, b| {
            let chapter_number_a: f64 = a.chapter_number.parse().unwrap_or(0f64);
            let chapter_number_b: f64 = b.chapter_number.parse().unwrap_or(0f64);
            chapter_number_a
                .partial_cmp(&chapter_number_b)
                .unwrap_or(Ordering::Equal)
        });
        Ok(chapters)
    }

    async fn get_pages(&mut self, manga_id: i32, chapter_number: &str) -> Result<Vec<Page>> {
        // Quick test seems to imply that query is safe to injections
        let stream = self
            .client
            .query(SELECT_CHAPTER_PAGES_QUERY, &[&manga_id, &chapter_number])
            .await?;
        let rows = stream.into_first_result().await?;
        rows.iter()
            .map(|row| {
                Ok(Page {
                    page_number: row
                        .get::<i32, _>("PageNumber")
                        .expect("PageNumber is NOT NULL")
                        .to_owned(),
                    url_string: row
                        .get::<&str, _>("URL")
                        .expect("URL is NOT NULL")
                        .to_owned(),
                })
            })
            .collect()
    }
}
