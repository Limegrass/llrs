use crate::app::AppRoute;
use llrs_model::Chapter;
use log::{error, info};
use yew::{
    format::{Json, Nothing},
    prelude::*,
    services::fetch::{FetchService, FetchTask, Request, Response},
    Component, ComponentLink,
};
use yew_router::components::RouterAnchor;

pub struct State {
    chapters: Option<Vec<Chapter>>,
    fetch_task: FetchTask,
}

impl State {}

pub struct ChapterList {
    link: ComponentLink<Self>,
    state: State,
}

pub enum Msg {
    FetchChaptersComplete(Result<Vec<Chapter>, anyhow::Error>),
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    pub manga_id: i32,
}

impl Component for ChapterList {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let manga_request =
            Request::get(format!("http://localhost:42069/manga/{}", props.manga_id))
                .body(Nothing)
                .expect("Could not build request.");
        let manga_callback = link.callback(
            |response: Response<Json<Result<Vec<Chapter>, anyhow::Error>>>| {
                let Json(data) = response.into_body();
                Msg::FetchChaptersComplete(data)
            },
        );
        let task =
            FetchService::fetch(manga_request, manga_callback).expect("failed to start request");
        let state = State {
            chapters: None,
            fetch_task: task,
        };

        Self { link, state }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FetchChaptersComplete(data) => match data {
                Ok(chapters) => self.state.chapters = Some(chapters),
                Err(err) => error!("{}", err),
            },
        }
        true
    }

    fn view(&self) -> Html {
        info!("rendered!");
        match &self.state.chapters {
            Some(chapters) => html! {
                for chapters.iter().map(|val| self.chapter_entry(&val))
            },
            None => html! {"Fetching"},
        }
    }
}

impl ChapterList {
    fn chapter_entry(&self, chapter: &Chapter) -> Html {
        type Anchor = RouterAnchor<AppRoute>;
        html! {
            <div class="chapter">
                <Anchor route=AppRoute::MangaChapter(
                        chapter.manga_id,
                        chapter.chapter_number.to_owned(),
                        1)>
                    {&chapter.chapter_number}
                </Anchor>
            </div>
        }
    }
}
