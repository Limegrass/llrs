use crate::{agents::manga::Response as MangaResponse, pages::ViewFormat, route::AppRoute};
use crate::{
    agents::{
        manga::{Action as MangaAction, MangaAgent},
        user::{Action as UserAgentAction, Response as UserAgentResponse, UserAgent},
    },
    components::{
        breadcrumb::{Breadcrumb, Separator},
        navbar::Navbar,
    },
};
use llrs_model::Manga;
use std::{collections::HashMap, rc::Rc};
use yew::{html::ChildrenRenderer, prelude::*};
use yew_router::{components::RouterAnchor, switch::Permissive};

const LLRS_BRAND_LOGO_URL: &'static str = env!("LLRS_BRAND_LOGO_URL");

type Anchor = RouterAnchor<AppRoute>;

struct State {
    view_format: ViewFormat,
    mangas: Option<Rc<HashMap<i32, Manga>>>,
}

pub(super) enum Msg {
    MangaAgentResponse(MangaResponse),
    UserAgentResponse(UserAgentResponse),
    ToggleViewFormat,
}

#[derive(Clone, PartialEq, Properties)]
pub(super) struct Props {
    pub(super) route: AppRoute,
}

pub(super) struct AppNavbar {
    #[allow(dead_code)]
    manga_agent: Box<dyn Bridge<MangaAgent>>,
    #[allow(dead_code)]
    user_agent: Box<dyn Bridge<UserAgent>>,
    #[allow(dead_code)]
    link: ComponentLink<Self>,
    state: State,
    props: Props,
}

impl Component for AppNavbar {
    type Message = Msg;

    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut manga_agent = MangaAgent::bridge(link.callback(Msg::MangaAgentResponse));
        manga_agent.send(MangaAction::GetMangaList);

        let mut user_agent = UserAgent::bridge(link.callback(Msg::UserAgentResponse));
        user_agent.send(UserAgentAction::GetViewFormatPreference);
        Self {
            manga_agent,
            user_agent,
            link,
            props,
            state: State {
                mangas: None,
                view_format: ViewFormat::Single,
            },
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::MangaAgentResponse(response) => match response {
                MangaResponse::MangaMap { mangas } => {
                    self.state.mangas = Some(mangas);
                    true
                }
                _ => false,
            },
            Msg::UserAgentResponse(response) => match response {
                UserAgentResponse::ViewFormatPreference(view_format) => {
                    if self.state.view_format == view_format {
                        false // same format, no update needed
                    } else {
                        self.state.view_format = view_format;
                        true
                    }
                }
            },
            Msg::ToggleViewFormat => {
                let other_view_format = match self.state.view_format {
                    ViewFormat::Single => ViewFormat::Long,
                    ViewFormat::Long => ViewFormat::Single,
                };
                self.user_agent
                    .send(UserAgentAction::SetViewFormatPreference(other_view_format));
                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props = props;
        true
    }

    fn view(&self) -> Html {
        let brand_links = self.get_brand_links();
        let menu_start_links = self.get_menu_start_links();
        let menu_end_links = self.get_menu_end_links();
        html! {
            <Navbar brand_children={brand_links}>
                <div class="navbar-start">
                    {menu_start_links}
                </div>
                <div class="navbar-end">
                    {menu_end_links}
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
    fn get_selected_manga(&self) -> Option<&Manga> {
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
            } => self
                .state
                .mangas
                .as_ref()
                .map_or(None, |mangas| mangas.get(&manga_id)),
            _ => None,
        }
    }

    fn get_menu_start_links(&self) -> Html {
        match &self.props.route {
            AppRoute::MangaChapterPage {
                manga_id: _,
                chapter_number: _,
                page_number: _,
            }
            | AppRoute::MangaChapter {
                manga_id: _,
                chapter_number: _,
            } => {
                let toggle_button_text = match self.state.view_format {
                    ViewFormat::Single => "Change to scroll view",
                    ViewFormat::Long => "Change to page view",
                };
                html! {
                    <a class="navbar-item" onclick=self.link.callback(|_| Msg::ToggleViewFormat)>
                        {toggle_button_text}
                    </a>
                }
            }
            _ => html! {},
        }
    }

    fn get_menu_end_links(&self) -> Html {
        let waifusims_link = html! {
            <a class="navbar-item" href="https://waifusims.com/Manga">
                {"Waifusims Reader"}
            </a>
        };
        let discord_link = html! {
            <a class="navbar-item" href=env!("LLRS_DISCORD_URL")>
                {"Join the Discord"}
            </a>
        };
        let manga = self.get_selected_manga();
        let manga_link = manga.as_ref().map_or(html! {}, |link| {
            if link.purchase_url.len() > 0 {
                html! {
                    <a class="navbar-item" href=link.purchase_url.as_str()>
                        {"Support the Author"}
                    </a>
                }
            } else {
                html! {}
            }
        });
        html! {
            <>
                {manga_link}
                {discord_link}
                {waifusims_link}
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
        let links =
            match &self.props.route {
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
                        link_text: self.state.mangas.as_ref().map_or(
                            manga_id.to_string(),
                            |mangas| {
                                mangas.get(&manga_id).map_or(manga_id.to_string(), |manga| {
                                    manga.manga_name.to_owned()
                                })
                            },
                        ),
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
                        link_text: self.state.mangas.as_ref().map_or(
                            manga_id.to_string(),
                            |mangas| {
                                mangas.get(&manga_id).map_or(manga_id.to_string(), |manga| {
                                    manga.manga_name.to_owned()
                                })
                            },
                        ),
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
