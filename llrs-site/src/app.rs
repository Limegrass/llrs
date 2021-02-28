use crate::components::breadcrumb::{Breadcrumb, Separator};
use crate::pages::not_found;
use crate::pages::{ChapterList, Home, MangaPage};
use log::trace;
use yew::prelude::*;
use yew_router::{prelude::*, switch::Permissive, Switch};

#[derive(Debug, Switch, PartialEq, Clone)]
pub enum AppRoute {
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
    ChapterList(i32),
    #[to = "/page-not-found"]
    NotFound(Permissive<String>),
    #[to = "/!"]
    Home,
}

pub struct App {
    // link: ComponentLink<Self>,
}

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
        // Self { link }
        Self {}
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        true
    }

    fn view(&self) -> Html {
        let redirect =
            Router::redirect(|route: Route| AppRoute::NotFound(Permissive(Some(route.route))));
        let render = Router::render(|route: AppRoute| {
            trace!("Route: {:?}", route);
            let content = match route {
                AppRoute::Home => html! {
                    <Home/>
                },
                AppRoute::ChapterList(manga_id) => html! {
                    <ChapterList manga_id=manga_id />
                },
                AppRoute::MangaChapterPage {
                    manga_id,
                    chapter_number,
                    page_number,
                } => html! {
                    <MangaPage
                        manga_id=manga_id
                        chapter_number=chapter_number
                        page_number=page_number
                    />
                },
                AppRoute::MangaChapter {
                    manga_id,
                    chapter_number,
                } => html! {
                    <MangaPage
                        manga_id=manga_id
                        chapter_number=chapter_number
                        page_number=1
                    />
                },
                AppRoute::NotFound(Permissive(None)) => html! { not_found("") },
                AppRoute::NotFound(Permissive(Some(path))) => html! { not_found(&path) },
            };
            html! {
                <div class="container">
                    <Breadcrumb separator=Separator::Succeeds >
                        <a href="/">
                            {"🏠 llrs"}
                        </a>
                        <a href="#">{"prolly use agents for this"}</a>
                    </Breadcrumb>
                    {content}
                </div>
            }
        });

        html! {
            <Router<AppRoute, ()> render=render redirect=redirect />
        }
    }
}
