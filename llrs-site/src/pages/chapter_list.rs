use super::progress::progress_bar;
use crate::agents::{
    chapter::{Action as ChapterAction, ChapterAgent},
    manga::{Action as MangaAction, MangaAgent, Response as MangaResponse},
};
use crate::route::AppRoute;
use llrs_model::Chapter;
use log::*;
use std::rc::Rc;
use yew::{prelude::*, Component, ComponentLink};
use yew_router::components::RouterAnchor;

pub(super) struct State {
    cover_image_url: String,
    chapters: Option<Rc<Vec<Chapter>>>,
    #[allow(dead_code)]
    chapter_agent: Box<dyn Bridge<ChapterAgent>>,
    #[allow(dead_code)]
    manga_agent: Box<dyn Bridge<MangaAgent>>,
}

pub(crate) struct ChapterList {
    state: State,
    props: Props,
}

#[derive(Debug)]
pub(crate) enum Msg {
    FetchChaptersComplete(Rc<Vec<Chapter>>),
    FetchMangaComplete(MangaResponse),
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub(crate) struct Props {
    pub(crate) manga_id: i32,
}

impl Component for ChapterList {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut chapter_agent = ChapterAgent::bridge(link.callback(Msg::FetchChaptersComplete));
        chapter_agent.send(ChapterAction::GetChapterList {
            manga_id: props.manga_id,
        });

        let mut manga_agent = MangaAgent::bridge(link.callback(Msg::FetchMangaComplete));
        manga_agent.send(MangaAction::GetMangaList);

        let state = State {
            cover_image_url: "".to_owned(),
            chapters: None,
            chapter_agent,
            manga_agent,
        };

        Self { state, props }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        trace!("{:?}", msg);
        match msg {
            Msg::FetchChaptersComplete(data) => self.state.chapters = Some(data),
            Msg::FetchMangaComplete(response) => match response {
                MangaResponse::MangaMap { mangas } => {
                    if let Some(manga) = mangas.get(&self.props.manga_id) {
                        self.state.cover_image_url = manga.cover_image_url.to_owned();
                    }
                }
            },
        }
        true
    }

    fn view(&self) -> Html {
        let cover_image = html! {
            <figure class="container manga-cover-image">
                <img src=self.state.cover_image_url />
            </figure>
        };
        let manga_table = match &self.state.chapters {
            Some(chapters) => html! {
                    <table class="table is-fullwidth is-striped is-narrow">
                        <thead>
                            <th> { "Chapter Number" } </th>
                            <th> { "Chapter Name" } </th>
                        </thead>
                        <tbody>
                            {for chapters.iter().map(|val| self.chapter_entry(&val))}
                        </tbody>
                    </table>
            },
            None => progress_bar(),
        };

        html! {
            <div class="container">
                {cover_image}
                {manga_table}
            </div>
        }
    }
}

// TODO: Search bar, set is-selected for most recent chapter if same manga
impl ChapterList {
    fn chapter_entry(&self, chapter: &Chapter) -> Html {
        type Anchor = RouterAnchor<AppRoute>;
        html! {
            <tr>
                <td>
                    <Anchor route=AppRoute::MangaChapter {
                        manga_id: chapter.manga_id,
                        chapter_number: chapter.chapter_number.to_owned(),
                    }>
                        {"Chapter "}{&chapter.chapter_number}
                    </Anchor>
                </td>
                <td>
                    <Anchor route=AppRoute::MangaChapter {
                        manga_id: chapter.manga_id,
                        chapter_number: chapter.chapter_number.to_owned(),
                    }>
                    {&chapter.chapter_name}
                    </Anchor>
                </td>
            </tr>
        }
    }
}
