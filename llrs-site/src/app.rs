use crate::{
    agents::manga::Response as MangaResponse,
    pages::{ChapterList, Home, MangaPage},
};
use crate::{
    agents::manga::{Action as MangaAction, MangaAgent},
    components::{
        breadcrumb::{Breadcrumb, Separator},
        navbar::Navbar,
    },
};
use crate::{
    agents::{chapter::ChapterAgent, page::PageAgent},
    pages::not_found,
};
use llrs_model::Manga;
use log::trace;
use std::rc::Rc;
use yew::{agent::Dispatcher, html::ChildrenRenderer, prelude::*};
use yew_router::{components::RouterAnchor, prelude::*, switch::Permissive, Switch};

const LLRS_BRAND_LOGO_URL: &'static str = env!("LLRS_BRAND_LOGO_URL");
type Anchor = RouterAnchor<AppRoute>;

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

// We house the Agents here to persist the data inside of them
// Otherwise the Agents would get destroyed when the last bridge gets destructed.
pub struct App {
    #[allow(dead_code)]
    manga_agent: Dispatcher<MangaAgent>,
    #[allow(dead_code)]
    chapter_agent: Dispatcher<ChapterAgent>,
    #[allow(dead_code)]
    page_agent: Dispatcher<PageAgent>,
}

struct State {
    mangas: Option<Rc<Vec<Rc<Manga>>>>,
}

#[derive(Debug)]
pub enum Msg {
    AgentResponse(MangaResponse),
}

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let manga_agent = MangaAgent::dispatcher();
        let chapter_agent = ChapterAgent::dispatcher();
        let page_agent = PageAgent::dispatcher();
        Self {
            manga_agent,
            chapter_agent,
            page_agent,
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        true
    }

    fn view(&self) -> Html {
        let redirect =
            Router::redirect(|route: Route| AppRoute::NotFound(Permissive(Some(route.route))));
        let render = Router::render(|route: AppRoute| {
            trace!("Route: {:?}", &route);
            let content = render_main_content(&route);
            html! {
                <div class="container">
                    <AppNavbar route=&route />
                    {content}
                </div>
            }
        });

        html! {
            <Router<AppRoute, ()> render=render redirect=redirect />
        }
    }
}

struct AppNavbar {
    #[allow(dead_code)]
    manga_agent: Box<dyn Bridge<MangaAgent>>,
    link: ComponentLink<Self>,
    state: State,
    props: Props,
}
#[derive(Debug, Clone, PartialEq, Properties)]
struct Props {
    route: AppRoute,
}

impl Component for AppNavbar {
    type Message = Msg;

    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut manga_agent = MangaAgent::bridge(link.callback(Msg::AgentResponse));
        manga_agent.send(MangaAction::GetMangaList);
        Self {
            manga_agent,
            link,
            props,
            state: State { mangas: None },
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::AgentResponse(response) => match response {
                MangaResponse::MangaList { mangas } => self.state.mangas = Some(mangas),
            },
        };
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props = props;
        true
    }

    fn view(&self) -> Html {
        let brand_links = self.get_brand_links();
        let menu_links = self.get_menu_links();
        html! {
            <Navbar brand_children={brand_links}>
                <div class="navbar-end">
                    {menu_links}
                </div>
            </Navbar>
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

impl AppNavbar {
    fn get_selected_manga(&self) -> Option<Rc<Manga>> {
        match self.props.route {
            AppRoute::ChapterList(manga_id)
            | AppRoute::MangaChapterPage {
                manga_id,
                chapter_number: _,
                page_number: _,
            }
            | AppRoute::MangaChapter {
                manga_id,
                chapter_number: _,
            } => self.state.mangas.as_ref().map_or(None, |mangas| {
                mangas
                    .iter()
                    .find(|manga| manga.manga_id == manga_id)
                    .map(|manga_ref| Rc::clone(manga_ref))
            }),
            _ => None,
        }
    }

    fn get_menu_links(&self) -> Html {
        let manga = self.get_selected_manga();
        manga.as_ref().map_or(html! {}, |m| {
            let buy_link = m.purchase_url.as_str();
            if buy_link.len() > 0 {
                html! { <a class="navbar-item" href=buy_link>{"Buy raws"}</a> }
            } else {
                html! {}
            }
        })
    }

    // TODO: Use Agents to get names of mangas/chapters
    fn get_brand_links(&self) -> Children {
        let brand_logo = html! {
            <Anchor classes="navbar-item" route=AppRoute::Home>
                <img src=&LLRS_BRAND_LOGO_URL alt="llrs logo" />
            </Anchor>
        };
        // Bulma ONLY formats the text properly with anchors
        let links = match &self.props.route {
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
                    link_text: self
                        .state
                        .mangas
                        .as_ref()
                        .map(|mangas| {
                            mangas
                                .iter()
                                .find(|manga| manga.manga_id == *manga_id)
                                .map(|manga| manga.manga_name.to_owned())
                                .unwrap_or(manga_id.to_string())
                        })
                        .unwrap_or(manga_id.to_string()),
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
                    link_text: self
                        .state
                        .mangas
                        .as_ref()
                        .map(|mangas| {
                            mangas
                                .iter()
                                .find(|manga| manga.manga_id == *manga_id)
                                .map(|manga| manga.manga_name.to_owned())
                                .unwrap_or(manga_id.to_string())
                        })
                        .unwrap_or(manga_id.to_string()),
                },
                BreadcrumbLink {
                    route: AppRoute::MangaChapter {
                        manga_id: *manga_id,
                        chapter_number: chapter_number.to_owned(),
                    },
                    link_text: format!("Chapter {}", chapter_number.to_owned()),
                },
            ],
            AppRoute::NotFound(Permissive(_)) => vec![BreadcrumbLink {
                route: AppRoute::Home,
                link_text: "llrs".to_owned(),
            }],
        };

        ChildrenRenderer::new(vec![html! {
            <>
                {brand_logo}
                <div class="navbar-item">
                    <Breadcrumb separator=Separator::Succeeds>
                        { links.into_iter().map(to_route_anchor).collect::<Vec<Html>>()}
                    </Breadcrumb>
                </div>
            </>
        }])
    }
}

fn to_route_anchor(link: BreadcrumbLink) -> Html {
    type Anchor = RouterAnchor<AppRoute>;
    html! {
        <Anchor route=link.route>{link.link_text}</Anchor>
    }
}
