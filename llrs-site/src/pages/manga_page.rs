use crate::app::AppRoute;
use llrs_model::Page;
use log::{error, info};
use ybc::Container;
use yew::{
    format::{Json, Nothing},
    prelude::*,
    services::fetch::{FetchService, FetchTask, Request, Response},
    Component, ComponentLink,
};
use yew_router::components::RouterAnchor;

pub struct State {
    pages: Option<Vec<Page>>,
    fetch_task: FetchTask,
}

impl State {}

pub struct MangaPage {
    link: ComponentLink<Self>,
    state: State,
}

pub enum Msg {
    FetchpagesComplete(Result<Vec<Page>, anyhow::Error>),
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    pub manga_id: i32,
    pub chapter_number: String,
    pub page_number: u32,
}

impl Component for MangaPage {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let manga_request = Request::get(format!(
            "http://localhost:42069/manga/{}/{}",
            props.manga_id, props.chapter_number
        ))
        .body(Nothing)
        .expect("Could not build request.");
        let manga_callback = link.callback(
            |response: Response<Json<Result<Vec<Page>, anyhow::Error>>>| {
                let Json(data) = response.into_body();
                Msg::FetchpagesComplete(data)
            },
        );
        let task =
            FetchService::fetch(manga_request, manga_callback).expect("failed to start request");
        let state = State {
            pages: None,
            fetch_task: task,
        };

        Self { link, state }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FetchpagesComplete(data) => match data {
                Ok(pages) => self.state.pages = Some(pages),
                Err(err) => error!("{}", err),
            },
        }
        true
    }

    fn view(&self) -> Html {
        info!("rendered!");
        match &self.state.pages {
            Some(pages) => html! {
                <Container>
                    {for pages.iter().map(|val| self.manga_page(&val))}
                </Container>
            },
            None => html! {"Fetching"},
        }
    }
}

impl MangaPage {
    fn manga_page(&self, page: &Page) -> Html {
        html! {
            <img src=&page.url_string />
        }
    }
}
