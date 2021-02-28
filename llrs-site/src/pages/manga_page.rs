use crate::agents::{
    chapter::{Action as ChapterAction, ChapterAgent},
    page::{Action as PageAction, PageAgent},
};
use crate::app::AppRoute;
use llrs_model::{Chapter, Page};
use log::*;
use std::{
    cmp::{max, min},
    rc::Rc,
};
use web_sys::{HtmlImageElement, ScrollBehavior, ScrollToOptions, Window};
use yew::{agent::Bridge, prelude::*, Component, ComponentLink};
use yew_router::{
    agent::RouteRequest,
    prelude::{Route, RouteAgentDispatcher},
};

pub struct State {
    pages: Option<Rc<Vec<Page>>>,
    chapters: Option<Rc<Vec<Chapter>>>,
    view_format: ViewFormat,
    should_set_to_last_page: bool,
}

enum ViewFormat {
    Single,
    Long,
}

pub struct MangaPage {
    #[allow(dead_code)]
    page_agent: Box<dyn Bridge<PageAgent>>,
    #[allow(dead_code)]
    chapter_agent: Box<dyn Bridge<ChapterAgent>>,
    route_dispatcher: RouteAgentDispatcher,
    prefetcher: Option<HtmlImageElement>,
    window: Option<Window>,
    state: State,
    props: Props,
    link: ComponentLink<Self>,
}

#[derive(Debug)]
pub enum Msg {
    FetchPagesComplete(Rc<Vec<Page>>),
    PreloadNextImage { page_number: usize },
    FetchChapterComplete(Rc<Vec<Chapter>>),
    PageBack,
    PageForward,
}

#[derive(Debug, Clone, PartialEq, Properties)]
pub struct Props {
    pub manga_id: i32,
    pub chapter_number: String,
    pub page_number: usize,
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

        let mut chapter_agent = ChapterAgent::bridge(link.callback(Msg::FetchChapterComplete));
        chapter_agent.send(ChapterAction::GetChapterList {
            manga_id: props.manga_id,
        });

        let state = State {
            chapters: None,
            pages: None,
            view_format: ViewFormat::Single,
            should_set_to_last_page: false,
        };

        let route_dispatcher = RouteAgentDispatcher::new();
        let window = web_sys::window();

        Self {
            prefetcher: HtmlImageElement::new().ok(),
            page_agent,
            route_dispatcher,
            chapter_agent,
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
                if self.state.should_set_to_last_page {
                    let route = AppRoute::MangaChapterPage {
                        manga_id: self.props.manga_id,
                        chapter_number: self.props.chapter_number.to_owned(),
                        page_number: data.len(),
                    };
                    self.route_dispatcher
                        .send(RouteRequest::ChangeRoute(Route::from(route)));
                    self.state.pages = Some(data);
                    false
                } else {
                    self.state.pages = Some(data);
                    true
                }
            }
            Msg::PreloadNextImage { page_number } => {
                let url = self
                    .state
                    .pages
                    .as_ref()
                    .map(|pages| pages[page_number - 1].url_string.as_str())
                    .unwrap_or("");

                if let Some(image_element) = &self.prefetcher {
                    image_element.set_src(url);
                }

                false
            }
            Msg::FetchChapterComplete(chapters) => {
                self.state.chapters = Some(chapters);
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
                                        current_chapter_index - 1
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
                        page_number: max(1, self.props.page_number - 1),
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
                                    if current_chapter_index != chapter_list.len() - 1 {
                                        current_chapter_index + 1
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
            None => html! {"Fetching"},
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
        let page_index = self.props.page_number - 1;
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
    }

    fn manga_page(&self, page: &Page) -> Html {
        // TODO: Look into an alternative to format!
        self.link.send_message(Msg::PreloadNextImage {
            page_number: min(
                page.page_number as usize,
                self.state
                    .pages
                    .as_ref()
                    .map_or(page.page_number as usize, |pages| pages.len()),
            ),
        });

        html! {
            <div class="container">
                <div class="back-pager"
                    onclick=self.link.callback(|_| Msg::PageBack) />
                <div class="forward-pager"
                    onclick=self.link.callback(|_| Msg::PageForward) />
                <img id="manga-image"
                     src=&page.url_string
                     alt=format!("Page {} Image", &page.page_number)
                 />
            </div>
        }
    }
}
