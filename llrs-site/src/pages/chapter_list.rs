use crate::agents::{
    chapter::{Action as ChapterAction, ChapterAgent},
    manga::{Action as MangaAction, MangaAgent, Response as MangaResponse},
};
use crate::app::AppRoute;
use llrs_model::{Chapter, Manga};
use log::*;
use std::rc::Rc;
use yew::{prelude::*, Component, ComponentLink};
use yew_router::components::RouterAnchor;

pub struct State {
    mangas: Option<Rc<Vec<Rc<Manga>>>>,
    chapters: Option<Rc<Vec<Chapter>>>,
    #[allow(dead_code)]
    chapter_agent: Box<dyn Bridge<ChapterAgent>>,
    #[allow(dead_code)]
    manga_agent: Box<dyn Bridge<MangaAgent>>,
}

pub struct ChapterList {
    state: State,
    props: Props,
}

#[derive(Debug)]
pub enum Msg {
    FetchChaptersComplete(Rc<Vec<Chapter>>),
    FetchMangaComplete(MangaResponse),
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    pub manga_id: i32,
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
            mangas: None,
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
                MangaResponse::MangaList { mangas } => {
                    self.state.mangas = Some(mangas);
                }
            },
        }
        true
    }

    fn view(&self) -> Html {
        let cover_image_url = self
            .state
            .mangas
            .as_ref()
            .map(|manga_list| {
                manga_list
                    .iter()
                    .find(|manga| manga.manga_id == self.props.manga_id)
                    .map(|manga| manga.cover_image_url.as_str())
                    .unwrap_or("")
            })
            .unwrap_or("");
        match &self.state.chapters {
            Some(chapters) => html! {
                // TODO: why doesn't this center the image? 100% in the mean time
                <div class="container">
                    <figure class="container manga-cover-image">
                        <img src=cover_image_url />
                    </figure>
                    <table class="table is-fullwidth is-striped is-narrow">
                        <thead>
                            <th> { "Chapter Number" } </th>
                            <th> { "Chapter Name" } </th>
                        </thead>
                        <tbody>
                            {for chapters.iter().map(|val| self.chapter_entry(&val))}
                        </tbody>
                    </table>
                </div>
            },
            None => html! {"Fetching"},
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
