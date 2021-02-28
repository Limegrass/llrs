use crate::app::AppRoute;
use llrs_model::Manga;
use log::{error, info};
use ybc::{Card, CardContent, CardFooter, CardImage, Container, Image, Progress, Tile};
use yew::{
    format::{Json, Nothing},
    prelude::*,
    services::fetch::{FetchService, FetchTask, Request, Response},
    Component, ComponentLink,
};
use yew_router::components::RouterAnchor;

pub struct State {
    mangas: Option<Vec<Manga>>,
    fetch_task: FetchTask,
}

impl State {}

pub struct Home {
    link: ComponentLink<Self>,
    state: State,
}

pub enum Msg {
    FetchMangasComplete(Result<Vec<Manga>, anyhow::Error>),
}

impl Component for Home {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let manga_request = Request::get("http://localhost:42069")
            .body(Nothing)
            .expect("Could not build request.");
        let manga_callback = link.callback(
            |response: Response<Json<Result<Vec<Manga>, anyhow::Error>>>| {
                let Json(data) = response.into_body();
                Msg::FetchMangasComplete(data)
            },
        );
        let task =
            FetchService::fetch(manga_request, manga_callback).expect("failed to start request");
        let state = State {
            mangas: None,
            fetch_task: task,
        };

        Self { link, state }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::FetchMangasComplete(data) => match data {
                Ok(mangas) => self.state.mangas = Some(mangas),
                Err(err) => error!("{}", err),
            },
        }
        true
    }

    fn view(&self) -> Html {
        // TODO: On navigating to index, call API (locally running libllrs)
        // and display links to each
        info!("rendered!");

        match &self.state.mangas {
            Some(mangas) => html! {
                <Container>
                    <Tile classes="is-ancestor">
                        <Tile classes="is-vertical">
                            {for mangas.chunks(2).map(|chunk| self.two_tile_manga(chunk))}
                        </Tile>
                    </Tile>
                </Container>
            },
            None => html! {
                <progress max="100" class="progress is-primary" />
            },
        }
    }
}

impl Home {
    fn two_tile_manga(&self, mangas: &[Manga]) -> Html {
        if mangas.len() == 1 {
            html! {
                <Tile classes="is-parent is-12">
                    <Tile classes="is-child">
                        { manga_entry(&mangas[0]) }
                    </Tile>
                </Tile>
            }
        } else {
            html! {
                <Tile classes="is-parent">
                    <Tile classes="is-child">
                        { manga_entry(&mangas[0]) }
                    </Tile>
                    <Tile classes="is-child">
                        { manga_entry(&mangas[1]) }
                    </Tile>
                </Tile>
            }
        }
    }
}
fn manga_entry(manga: &Manga) -> Html {
    html! {
        <Tile classes="is-child">
            <Image>
                <RouterAnchor<AppRoute> route=AppRoute::ChapterList(manga.manga_id)>
                    <img src=&manga.cover_image_url alt=&manga.manga_name title=&manga.manga_name />
                </RouterAnchor<AppRoute>>
            </Image>
        </Tile>
    }
}
