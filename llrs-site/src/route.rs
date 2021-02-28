use yew_router::{switch::Permissive, Switch};

#[derive(Debug, Switch, PartialEq, Clone)]
pub(super) enum AppRoute {
    #[to = "/manga/{manga_id}/{chapter_number}/{page_number}"]
    MangaChapterPage {
        manga_id: i32,
        chapter_number: String,
        page_number: usize,
    },
    // support users inputting the chapter number manually without a page
    #[to = "/manga/{manga_id}/{chapter_number}"]
    MangaChapter {
        manga_id: i32,
        chapter_number: String,
    },
    #[to = "/manga/{manga_id}"]
    ChapterList { manga_id: i32 },
    #[to = "/page-not-found"]
    NotFound(Permissive<String>),
    #[to = "/!"]
    MangaList,
}
