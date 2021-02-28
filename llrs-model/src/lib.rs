#[cfg(feature = "chrono")]
use chrono::NaiveDateTime;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "chrono")]
pub type DateTimeType = NaiveDateTime;
#[cfg(not(feature = "chrono"))]
pub type DateTimeType = String;

// Should redesign DB
#[derive(Debug)]
#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
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

#[derive(Debug)]
#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
pub struct Chapter {
    pub chapter_number: String,
    // not at the chapter level in current db
    // pub author_name: String,
    // pub artist_name: String,
    pub chapter_name: String,
    pub creation_date: DateTimeType,
    pub release_date: DateTimeType,
    pub manga_id: i32,
}

#[derive(Debug)]
#[cfg(feature = "serde")]
#[derive(Serialize, Deserialize)]
pub struct Page {
    pub url_string: String,
    pub page_number: i32,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
