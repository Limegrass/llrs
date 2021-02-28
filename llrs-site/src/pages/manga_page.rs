use crate::app::AppRoute;
use llrs_model::Page;
use log::{error, info};
use yew::{
    format::{Json, Nothing},
    prelude::*,
    services::fetch::{FetchService, FetchTask, Request, Response},
    Component, ComponentLink,
};
use yew_router::components::RouterAnchor;

pub struct State {
    pages: Option<Vec<Page>>,
    current_page_number: usize,
    view_format: ViewFormat,
    fetch_task: FetchTask,
}

enum ViewFormat {
    Single,
    Long,
}

impl State {}

pub struct MangaPage {
    link: ComponentLink<Self>,
    state: State,
}

pub enum Msg {
    FetchpagesComplete(Result<Vec<Page>, anyhow::Error>),
    LoadPage(usize),
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
            view_format: ViewFormat::Single,
            current_page_number: 1,
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
            Msg::LoadPage(page_number) => self.state.current_page_number = page_number,
        }
        true
    }

    fn view(&self) -> Html {
        info!("rendered!");
        match &self.state.pages {
            Some(pages) => self.render_view(pages),
            None => html! {"Fetching"},
        }
    }
}

impl MangaPage {
    fn render_view(&self, pages: &[Page]) -> Html {
        let page_index = self.state.current_page_number - 1;
        let pages = match self.state.view_format {
            // TODO: Progressive loading (first page first)
            ViewFormat::Long => html! {
                for pages.iter().map(|val| self.manga_page(&val))
            },
            ViewFormat::Single => html! {
                self.manga_page(&pages[page_index])
            },
        };
        html! {
            <div classes="container">
                {pages}
            </div>
        }
    }

    fn manga_page(&self, page: &Page) -> Html {
        // TODO: Look into an alternative to format!
        let next_page_number = 1usize + page.page_number as usize;
        html! {
            <img src=&page.url_string
                 alt=format!("Page {} Image", &page.page_number)
                 onclick=self.link.callback(move |_| Msg::LoadPage(next_page_number))
             />
        }
    }
}
