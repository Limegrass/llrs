use crate::{agents::manga::Response as MangaResponse, route::AppRoute};
use crate::{
    agents::manga::{Action as MangaAction, MangaAgent},
    components::{
        breadcrumb::{Breadcrumb, Separator},
        navbar::Navbar,
    },
};
use llrs_model::Manga;
use std::rc::Rc;
use yew::{html::ChildrenRenderer, prelude::*};
use yew_router::{components::RouterAnchor, switch::Permissive};

const LLRS_BRAND_LOGO_URL: &'static str = env!("LLRS_BRAND_LOGO_URL");

type Anchor = RouterAnchor<AppRoute>;

struct State {
    mangas: Option<Rc<Vec<Rc<Manga>>>>,
}

pub(super) enum Msg {
    AgentResponse(MangaResponse),
}

#[derive(Clone, PartialEq, Properties)]
pub(super) struct Props {
    pub(super) route: AppRoute,
}

pub(super) struct AppNavbar {
    #[allow(dead_code)]
    manga_agent: Box<dyn Bridge<MangaAgent>>,
    #[allow(dead_code)]
    link: ComponentLink<Self>,
    state: State,
    props: Props,
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

struct BreadcrumbLink {
    route: AppRoute,
    link_text: String,
}

impl AppNavbar {
    fn get_selected_manga(&self) -> Option<Rc<Manga>> {
        match self.props.route {
            AppRoute::ChapterList { manga_id }
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
        let discord_link = html! {
            <a class="navbar-item" href=env!("LLRS_DISCORD_URL")>{"Join the Discord"}</a>
        };
        let manga_link = manga.as_ref().map_or(html! {}, |m| {
            let buy_link = m.purchase_url.as_str();
            if buy_link.len() > 0 {
                html! {
                    <a class="navbar-item" href=buy_link>{"Support the Author"}</a>
                }
            } else {
                html! {}
            }
        });
        html! {
            <>
                {manga_link}
                {discord_link}
            </>
        }
    }

    fn get_brand_links(&self) -> Children {
        let brand_logo = html! {
            <Anchor classes="navbar-item" route=AppRoute::MangaList>
                <img src=&LLRS_BRAND_LOGO_URL alt="llrs logo" />
            </Anchor>
        };
        // Bulma ONLY formats the text properly with anchors
        let links = match &self.props.route {
            AppRoute::MangaList => vec![BreadcrumbLink {
                route: AppRoute::MangaList,
                link_text: "llrs".to_owned(),
            }],
            AppRoute::ChapterList { manga_id } => vec![
                BreadcrumbLink {
                    route: AppRoute::MangaList,
                    link_text: "llrs".to_owned(),
                },
                BreadcrumbLink {
                    route: AppRoute::ChapterList {
                        manga_id: *manga_id,
                    },
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
                    route: AppRoute::MangaList,
                    link_text: "llrs".to_owned(),
                },
                BreadcrumbLink {
                    route: AppRoute::ChapterList {
                        manga_id: *manga_id,
                    },
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
                route: AppRoute::MangaList,
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
