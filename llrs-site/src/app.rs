use crate::components::breadcrumb::{Breadcrumb, Separator};
use crate::pages::not_found;
use crate::pages::{ChapterList, Home, MangaPage};
use log::trace;
use yew::prelude::*;
use yew_router::{components::RouterAnchor, prelude::*, switch::Permissive, Switch};

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

pub struct App {}

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
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
            trace!("Route: {:?}", &route);
            let content = render_main_content(&route);
            let breadcrumb = render_breadcrumb(&route);
            html! {
                <div class="container">
                    {breadcrumb}
                    {content}
                </div>
            }
        });

        html! {
            <Router<AppRoute, ()> render=render redirect=redirect />
        }
    }
}

fn render_main_content(route: &AppRoute) -> Html {
    match route {
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
    }
}

struct BreadcrumbLink {
    route: AppRoute,
    link_text: String,
}

// TODO: Use Agents to get names of mangas/chapters
fn render_breadcrumb(route: &AppRoute) -> Html {
    // Bulma ONLY formats the text properly with anchors
    let links = match route {
        AppRoute::Home => vec![BreadcrumbLink {
            route: AppRoute::Home,
            link_text: "llrs".to_owned(),
        }],
        AppRoute::ChapterList(manga_id) => vec![
            BreadcrumbLink {
                route: AppRoute::Home,
                link_text: "llrs".to_owned(),
            },
            BreadcrumbLink {
                route: AppRoute::ChapterList(*manga_id),
                link_text: manga_id.to_string(),
            },
        ],
        AppRoute::MangaChapterPage {
            manga_id,
            chapter_number,
            page_number: _,
        }
        | AppRoute::MangaChapter {
            manga_id,
            chapter_number,
        } => vec![
            BreadcrumbLink {
                route: AppRoute::Home,
                link_text: "llrs".to_owned(),
            },
            BreadcrumbLink {
                route: AppRoute::ChapterList(*manga_id),
                link_text: manga_id.to_string(),
            },
            BreadcrumbLink {
                route: AppRoute::MangaChapter {
                    manga_id: *manga_id,
                    chapter_number: chapter_number.to_owned(),
                },
                link_text: chapter_number.to_owned(),
            },
        ],
        AppRoute::NotFound(Permissive(_)) => vec![BreadcrumbLink {
            route: AppRoute::Home,
            link_text: "llrs".to_owned(),
        }],
    };

    html! {
        <Breadcrumb separator=Separator::Succeeds >
        { links.into_iter().map(to_route_anchor).collect::<Html>()}
        </Breadcrumb>
    }
}

fn to_route_anchor(link: BreadcrumbLink) -> Html {
    type Anchor = RouterAnchor<AppRoute>;
    html! {
        <Anchor route=link.route>{link.link_text}</Anchor>
    }
}
