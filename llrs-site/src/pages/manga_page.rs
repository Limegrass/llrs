use super::progress::progress_bar;
use crate::agents::{
    manga::{Action as MangaAction, MangaAgent, Response as MangaAgentResponse},
    page::{Action as PageAction, PageAgent},
};
use crate::route::AppRoute;
use llrs_model::{Chapter, Page};
use log::*;
use std::{cmp::max, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlImageElement, ScrollBehavior, ScrollToOptions, Window};
use yew::{agent::Bridge, prelude::*, Component, ComponentLink};
use yew_router::{
    agent::RouteRequest,
    prelude::{Route, RouteAgentDispatcher},
    switch::Permissive,
};

pub(crate) struct State {
    pages: Option<Rc<Vec<Page>>>,
    chapters: Option<Rc<Vec<Chapter>>>,
    view_format: ViewFormat,
    should_set_to_last_page: bool,
    starting_page_number: Option<usize>,
}

#[allow(dead_code)]
enum ViewFormat {
    Single,
    Long,
}

pub(crate) struct MangaPage {
    #[allow(dead_code)]
    page_agent: Box<dyn Bridge<PageAgent>>,
    #[allow(dead_code)]
    manga_agent: Box<dyn Bridge<MangaAgent>>,
    route_dispatcher: RouteAgentDispatcher,
    prefetcher: Option<HtmlImageElement>,
    window: Option<Window>,
    state: State,
    props: Props,
    link: ComponentLink<Self>,
}

#[derive(Debug)]
pub(crate) enum Msg {
    FetchPagesComplete(Rc<Vec<Page>>),
    PreloadNextImage { page_number: usize },
    MangaAgentResponse(MangaAgentResponse),
    PageBack,
    PageForward,
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub(crate) struct Props {
    pub(crate) manga_id: i32,
    pub(crate) chapter_number: String,
    pub(crate) page_number: usize,
}

impl Component for MangaPage {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        trace!("manga_page: {:?}", props);
        let mut page_agent = PageAgent::bridge(link.callback(Msg::FetchPagesComplete));
        page_agent.send(PageAction::GetPageList {
            manga_id: props.manga_id,
            chapter_number: props.chapter_number.to_owned(),
        });

        let mut manga_agent = MangaAgent::bridge(link.callback(Msg::MangaAgentResponse));
        manga_agent.send(MangaAction::GetChapterList {
            manga_id: props.manga_id,
        });

        let state = State {
            chapters: None,
            pages: None,
            view_format: ViewFormat::Single,
            should_set_to_last_page: false,
            starting_page_number: None,
        };

        let route_dispatcher = RouteAgentDispatcher::new();
        let window = web_sys::window();

        Self {
            prefetcher: HtmlImageElement::new().ok(),
            page_agent,
            route_dispatcher,
            manga_agent,
            state,
            props,
            link,
            window,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if props.chapter_number != self.props.chapter_number {
            self.page_agent.send(PageAction::GetPageList {
                manga_id: props.manga_id,
                chapter_number: props.chapter_number.to_owned(),
            });
            self.props = props;
            false
        } else {
            self.props = props;
            true
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        trace!("{:?}", msg);
        match msg {
            Msg::FetchPagesComplete(data) => {
                info!("page {:p}", &data);
                if self.state.should_set_to_last_page || data.len() < self.props.page_number {
                    let route = AppRoute::MangaChapterPage {
                        manga_id: self.props.manga_id,
                        chapter_number: self.props.chapter_number.to_owned(),
                        page_number: data.len(),
                    };
                    self.route_dispatcher
                        .send(RouteRequest::ChangeRoute(Route::from(route)));
                    self.state.starting_page_number = Some(self.props.page_number);
                    self.state.pages = Some(data);
                    false
                } else if data.len() == 0 {
                    let route = AppRoute::NotFound(Permissive(Some(format!(
                        "Manga with ID {} and Chapter {} not found",
                        self.props.manga_id, self.props.chapter_number
                    ))));
                    self.route_dispatcher
                        .send(RouteRequest::ChangeRoute(Route::from(route)));
                    self.state.starting_page_number = Some(self.props.page_number);
                    false
                } else if self.props.page_number == 0 {
                    let route = AppRoute::MangaChapter {
                        manga_id: self.props.manga_id,
                        chapter_number: self.props.chapter_number.to_owned(),
                    };
                    self.route_dispatcher
                        .send(RouteRequest::ChangeRoute(Route::from(route)));
                    self.state.starting_page_number = Some(self.props.page_number);
                    self.state.pages = Some(data);
                    false
                } else {
                    self.state.pages = Some(data);
                    true
                }
            }
            Msg::PreloadNextImage { page_number } => {
                if let Some(pages) = self.state.pages.as_ref() {
                    if pages.len() > 0 {
                        if let Some(page_index) = page_number
                            .checked_sub(1)
                            .map(|res| {
                                if res < pages.len() {
                                    Some(res)
                                } else if res == self.state.starting_page_number.unwrap_or(res) {
                                    None
                                } else {
                                    res.checked_sub(pages.len())
                                }
                            })
                            .unwrap_or(None)
                        // will never become None here as we never pass 0 as page_number
                        {
                            if let Some(page) = pages.get(page_index) {
                                if let Some(image_element) = &self.prefetcher {
                                    let link = self.link.clone();
                                    let load_next_image = Closure::once(Box::new(move || {
                                        link.send_message(Msg::PreloadNextImage {
                                            page_number: page_index
                                                .checked_add(1)
                                                .unwrap_or(page_index),
                                        });
                                    }));
                                    image_element
                                        .set_onload(Some(load_next_image.as_ref().unchecked_ref()));
                                    load_next_image.forget();

                                    image_element.set_src(&page.url_string);
                                }
                            }
                        } else {
                            self.state.starting_page_number = None;
                        }
                    }
                }

                false
            }
            Msg::MangaAgentResponse(response) => {
                match response {
                    MangaAgentResponse::Chapters {
                        manga_id: _,
                        chapters,
                    } => {
                        self.state.chapters = Some(chapters);
                    }
                    _ => {}
                };
                false
            }
            Msg::PageBack => {
                self.state.should_set_to_last_page = true;
                self.scroll_to_manga_page_top();
                let current_chapter_number = &self.props.chapter_number;
                let previous_page_chapter_number = if self.props.page_number == 1 {
                    self.state
                        .chapters
                        .as_ref()
                        .map(|chapter_list| {
                            let current_chapter_index = chapter_list.iter().position(|chapter| {
                                &chapter.chapter_number == current_chapter_number
                            });
                            let previous_chapter_index =
                                current_chapter_index.map(|current_chapter_index| {
                                    if current_chapter_index != 0 {
                                        current_chapter_index
                                            .checked_sub(1)
                                            .unwrap_or(current_chapter_index)
                                    } else {
                                        current_chapter_index
                                    }
                                });
                            previous_chapter_index
                                .map(|index| {
                                    chapter_list
                                        .get(index)
                                        .map(|chapter| chapter.chapter_number.as_str())
                                        .unwrap_or(current_chapter_number.as_str())
                                })
                                .unwrap_or(current_chapter_number.as_str())
                        })
                        .unwrap_or(self.props.chapter_number.as_str())
                } else {
                    self.props.chapter_number.as_str()
                };

                if current_chapter_number != previous_page_chapter_number {
                    self.page_agent.send(PageAction::GetPageList {
                        manga_id: self.props.manga_id,
                        chapter_number: previous_page_chapter_number.to_owned(),
                    });
                    // TODO: LOVE THIS MUTABLE GARBAGE
                    self.props.chapter_number = previous_page_chapter_number.to_owned();
                    false
                } else {
                    let route = AppRoute::MangaChapterPage {
                        manga_id: self.props.manga_id,
                        chapter_number: self.props.chapter_number.to_owned(),
                        page_number: max(
                            1,
                            self.props
                                .page_number
                                .checked_sub(1)
                                .unwrap_or(self.props.page_number),
                        ),
                    };
                    self.route_dispatcher
                        .send(RouteRequest::ChangeRoute(Route::from(route)));
                    false
                }
            }
            Msg::PageForward => {
                self.state.should_set_to_last_page = false;
                self.scroll_to_manga_page_top();
                let last_page = self
                    .state
                    .pages
                    .as_ref()
                    .expect("Should never try render without pages")
                    .len();
                let next_page_chapter_number = if self.props.page_number == last_page {
                    self.state
                        .chapters
                        .as_ref()
                        .map(|chapter_list| {
                            let current_chapter_index = chapter_list.iter().position(|chapter| {
                                chapter.chapter_number == self.props.chapter_number
                            });
                            let next_chapter_index =
                                current_chapter_index.map(|current_chapter_index| {
                                    if current_chapter_index
                                        != chapter_list
                                            .len()
                                            .checked_sub(1)
                                            .unwrap_or(current_chapter_index)
                                    {
                                        current_chapter_index
                                            .checked_add(1)
                                            .unwrap_or(current_chapter_index)
                                    } else {
                                        current_chapter_index
                                    }
                                });
                            next_chapter_index
                                .map(|index| {
                                    chapter_list
                                        .get(index)
                                        .map(|chapter| chapter.chapter_number.as_str())
                                        .unwrap_or(self.props.chapter_number.as_str())
                                })
                                .unwrap_or(self.props.chapter_number.as_str())
                        })
                        .unwrap_or(self.props.chapter_number.as_str())
                } else {
                    self.props.chapter_number.as_str()
                };

                let route = if next_page_chapter_number == self.props.chapter_number {
                    let next_page_number = self.props.page_number as usize + 1;
                    if next_page_number > last_page {
                        AppRoute::ChapterList {
                            manga_id: self.props.manga_id,
                        }
                    } else {
                        AppRoute::MangaChapterPage {
                            manga_id: self.props.manga_id,
                            chapter_number: next_page_chapter_number.to_owned(),
                            page_number: next_page_number,
                        }
                    }
                } else {
                    AppRoute::MangaChapterPage {
                        manga_id: self.props.manga_id,
                        chapter_number: next_page_chapter_number.to_owned(),
                        page_number: 1,
                    }
                };

                self.route_dispatcher
                    .send(RouteRequest::ChangeRoute(Route::from(route)));
                false
            }
        }
    }

    fn view(&self) -> Html {
        match &self.state.pages {
            Some(pages) => self.render_view(pages),
            None => progress_bar(),
        }
    }
}

// Check the Chapter Agent to see which chapter is next
impl MangaPage {
    fn scroll_to_manga_page_top(&self) {
        let mut scroll_to_options = ScrollToOptions::new();
        let manga_page_top = self.window.as_ref().map_or(0.0, |window| {
            window.document().map_or(0.0, |doc| {
                doc.get_element_by_id("manga-image")
                    .map_or(0.0, |element| element.get_bounding_client_rect().top())
            })
        });
        scroll_to_options.top(manga_page_top);
        scroll_to_options.behavior(ScrollBehavior::Smooth);
        self.window.as_ref().and_then(|window| {
            window.scroll_by_with_scroll_to_options(&scroll_to_options);
            Some(window)
        });
    }

    fn render_view(&self, pages: &[Page]) -> Html {
        if let Some(page_index) = self.props.page_number.checked_sub(1) {
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
                <figure class="container image">
                    {pages}
                </figure>
            }
        } else {
            html! {"Somehow had a page_number of 0"}
        }
    }

    fn manga_page(&self, page: &Page) -> Html {
        let next_page_number = (page.page_number as usize)
            .checked_add(1)
            .unwrap_or(page.page_number as usize);

        html! {
            <div class="container">
                <div class="back-pager"
                    onclick=self.link.callback(|_| Msg::PageBack) />
                <div class="forward-pager"
                    onclick=self.link.callback(|_| Msg::PageForward) />
                <img id="manga-image"
                     src=&page.url_string
                     onload=self.link.callback(move |_| Msg::PreloadNextImage {
                         page_number: next_page_number
                     })
                     alt=format!("Page {} Image", &page.page_number)
                 />
            </div>
        }
    }
}
