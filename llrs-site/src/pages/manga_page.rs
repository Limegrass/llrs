use super::progress::progress_bar;
use crate::agents::{
    manga::{Action as MangaAction, MangaAgent, Response as MangaAgentResponse},
    user::{Action as UserAgentAction, Response as UserAgentResponse, UserAgent},
};
use crate::route::AppRoute;
use js_sys::Date;
use llrs_model::{Chapter, Page};
use log::*;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::VecDeque, rc::Rc, time::Duration};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlImageElement, ScrollBehavior, ScrollToOptions, Window};
use yew::{
    agent::Bridge,
    prelude::*,
    services::{interval::IntervalTask, IntervalService},
    Component, ComponentLink,
};
use yew_router::{
    agent::RouteRequest,
    prelude::{Route, RouteAgentDispatcher},
    switch::Permissive,
};

// TODO: Split off Long form from single page form
// TODO: Create a helper struct to handle {page: Page, is_visible: bool}

const LONG_FORM_BACK_PAGE_BUFFER_SECONDS: f64 = 1f64 * 1000f64;

pub(crate) struct State {
    pages: Option<Rc<Vec<Page>>>,
    chapters: Option<Rc<Vec<Chapter>>>,
    view_format: ViewFormat,
    should_set_to_last_page: bool,
    preload_queue: VecDeque<usize>,
    /// Only used in scroll view
    is_visible: Vec<bool>,
    #[allow(dead_code)]
    scroll_handler: Option<Closure<dyn FnMut()>>,
    prior_render_time_seconds: f64,
    prior_scroll_y: f64,
    preloader_closure: Option<Closure<dyn FnMut()>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ViewFormat {
    Single,
    Long,
}

pub(crate) struct MangaPage {
    #[allow(dead_code)]
    manga_agent: Box<dyn Bridge<MangaAgent>>,
    #[allow(dead_code)]
    user_agent: Box<dyn Bridge<UserAgent>>,
    route_dispatcher: RouteAgentDispatcher,
    prefetcher: Option<HtmlImageElement>,
    #[allow(dead_code)]
    interval_task: IntervalTask,
    window: Option<Window>,
    state: State,
    props: Props,
    link: ComponentLink<Self>,
}

#[derive(Debug)]
pub(crate) enum Msg {
    PreloadImage {
        page_index: usize,
    },
    MangaAgentResponse(MangaAgentResponse),
    UserAgentResponse(UserAgentResponse),
    PageBack {
        current_page_number: usize,
    },
    PageForward {
        current_page_number: usize,
    },
    // Sending the message essential schedules it to run
    // after the current render cycle completes
    ScrollToPage {
        page_number: usize,
        scroll_behavior: ScrollBehavior,
    },
    PageRepositioned,
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
        let interval_task = IntervalService::spawn(
            Duration::from_secs(LONG_FORM_BACK_PAGE_BUFFER_SECONDS as u64),
            link.callback(|_| Msg::PageRepositioned),
        );

        let mut manga_agent = MangaAgent::bridge(link.callback(Msg::MangaAgentResponse));
        manga_agent.send(MangaAction::GetChapterList {
            manga_id: props.manga_id,
        });
        manga_agent.send(MangaAction::GetPageList {
            manga_id: props.manga_id,
            chapter_number: props.chapter_number.to_owned(),
        });

        let mut user_agent = UserAgent::bridge(link.callback(Msg::UserAgentResponse));
        user_agent.send(UserAgentAction::GetViewFormatPreference);

        let route_dispatcher = RouteAgentDispatcher::new();
        let window = web_sys::window();
        let prior_load_date_time = Date::now();

        let state = State {
            chapters: None,
            pages: None,
            view_format: ViewFormat::Single,
            should_set_to_last_page: false,
            preload_queue: VecDeque::new(),
            is_visible: vec![],
            scroll_handler: None,
            // random extra buffer who cares lul
            prior_render_time_seconds: prior_load_date_time + 5000f64,
            prior_scroll_y: 0f64,
            preloader_closure: None,
        };

        Self {
            prefetcher: HtmlImageElement::new().ok(),
            route_dispatcher,
            manga_agent,
            state,
            props,
            link,
            window,
            user_agent,
            interval_task,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if props.chapter_number != self.props.chapter_number {
            self.manga_agent.send(MangaAction::GetPageList {
                manga_id: props.manga_id,
                chapter_number: props.chapter_number.to_owned(),
            });
            self.props = props;
            // We don't need to rerender yet because
            // we can just wait until we get a response for the new list of pages
            false
        } else {
            self.link.send_message(Msg::ScrollToPage {
                page_number: props.page_number,
                scroll_behavior: match self.state.view_format {
                    ViewFormat::Single => ScrollBehavior::Smooth,
                    ViewFormat::Long => ScrollBehavior::Instant,
                },
            });
            self.props = props;
            true
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        info!("{:?}", msg);
        match msg {
            Msg::PreloadImage { page_index } => self.preload_image_and_set_next(page_index),
            Msg::MangaAgentResponse(response) => self.handle_manga_response(response),
            Msg::PageBack {
                current_page_number,
            } => {
                self.state.should_set_to_last_page = true;
                self.page_backward(current_page_number);
                false
            }
            Msg::PageForward {
                current_page_number,
            } => {
                self.state.should_set_to_last_page = false;
                self.page_forward(current_page_number);
                false
            }
            Msg::UserAgentResponse(response) => self.update_view_format(response),
            Msg::ScrollToPage {
                page_number,
                scroll_behavior,
            } => {
                self.scroll_to_manga_page_top(page_number, scroll_behavior);
                false
            }
            Msg::PageRepositioned => match self.handle_page_repositioning() {
                Ok(should_render) => should_render,
                Err(should_render) => should_render,
            },
        }
    }

    fn view(&self) -> Html {
        match &self.state.pages {
            Some(pages) => self.render_view(pages),
            None => progress_bar(),
        }
    }

    fn destroy(&mut self) {
        if let Some(window) = &self.window {
            window.set_onscroll(None);
            window.set_onresize(None);
        }
    }
}

// Check the Chapter Agent to see which chapter is next
impl MangaPage {
    /// Returns None if it would be paging to page 0
    fn previous_page_number(&self) -> Option<usize> {
        self.props
            .page_number
            .checked_sub(1)
            .map_or(None, |previous_pn| {
                if previous_pn == 0 {
                    None
                } else {
                    Some(previous_pn)
                }
            })
    }

    /// Returns None if it would be paging past available pages
    fn next_page_number(&self) -> Option<usize> {
        self.props
            .page_number
            .checked_add(1)
            .map_or(None, |next_pn| {
                if next_pn
                    > self
                        .state
                        .pages
                        .as_ref()
                        .map_or(self.props.page_number, |pages| pages.len())
                {
                    None
                } else {
                    Some(next_pn)
                }
            })
    }

    fn scroll_to_manga_page_top(&self, page_number: usize, scroll_behavior: ScrollBehavior) {
        if let Some(window) = self.window.as_ref() {
            if let Some(doc) = window.document() {
                let mut scroll_to_options = ScrollToOptions::new();
                let element_to_scroll_to_top = match self.state.view_format {
                    ViewFormat::Single => "manga-image".to_owned(),
                    ViewFormat::Long => format!("manga-page-{}", page_number),
                };
                let manga_page_top = doc
                    .get_element_by_id(element_to_scroll_to_top.as_str())
                    .map_or(0.0, |element| element.get_bounding_client_rect().top());
                scroll_to_options.top(manga_page_top);
                scroll_to_options.behavior(scroll_behavior);
                window.scroll_by_with_scroll_to_options(&scroll_to_options);
            }
        }
    }

    fn handle_page_repositioning(&mut self) -> Result<ShouldRender, ShouldRender> {
        let mut should_render = false;
        let window = self.window.as_ref().ok_or(false)?;
        let doc = window.document().ok_or(false)?;
        let viewport_height = window
            .inner_height()
            .map(|js_value| js_value.as_f64().ok_or(false))
            .map_err(|_| false)??;
        let half_viewport = viewport_height / 2f64;
        let document_element = doc.document_element().ok_or(false)?;

        let current_scroll_y = window.scroll_y().unwrap_or(self.state.prior_scroll_y);
        let element_to_scroll_to_top = match self.state.view_format {
            ViewFormat::Single => return Err(false),
            ViewFormat::Long => format!("manga-page-{}", self.props.page_number),
        };

        // Allows previous page to render
        if let Some(previous_page_number) = self.previous_page_number() {
            let mut should_update_props_and_route = false;

            if let Some(bounding_client_rect) = doc
                .get_element_by_id(element_to_scroll_to_top.as_str())
                .map(|element| element.get_bounding_client_rect())
            {
                // Debounce previous page reloads so they don't all load at once.
                let current_time_seconds = Date::now();
                if current_time_seconds - LONG_FORM_BACK_PAGE_BUFFER_SECONDS
                    > self.state.prior_render_time_seconds
                    && bounding_client_rect.top() > 0f64
                {
                    self.state.prior_render_time_seconds = current_time_seconds;
                    match self
                        .state
                        .is_visible
                        .get_mut(previous_page_number.checked_sub(1).unwrap_or(0))
                    {
                        Some(previous_page_visibility) if !*previous_page_visibility => {
                            *previous_page_visibility = true;
                            should_render = true;
                            should_update_props_and_route = true;
                        }
                        _ => {}
                    }
                }
            }

            if let Some(previous_page_bounding_box) = doc
                .get_element_by_id(format!("manga-page-{}", previous_page_number).as_str())
                .map(|element| element.get_bounding_client_rect())
            {
                // scrolled up
                if previous_page_bounding_box.bottom() > half_viewport
                    && current_scroll_y <= self.state.prior_scroll_y
                {
                    should_update_props_and_route = true;
                }
            }

            if should_update_props_and_route {
                self.props.page_number = previous_page_number;
                let route = AppRoute::MangaChapterPage {
                    manga_id: self.props.manga_id,
                    chapter_number: self.props.chapter_number.to_owned(),
                    page_number: previous_page_number,
                };
                self.route_dispatcher
                    .send(RouteRequest::ChangeRouteNoBroadcast(Route::from(route)));
            }
        }

        if let Some(next_page_number) = self.next_page_number() {
            if let Some(next_page_bounding_box) = doc
                .get_element_by_id(format!("manga-page-{}", next_page_number).as_str())
                .map(|element| element.get_bounding_client_rect())
            {
                if next_page_bounding_box.top() < half_viewport
                    && self.state.prior_scroll_y < current_scroll_y
                {
                    self.props.page_number = next_page_number;
                    let route = AppRoute::MangaChapterPage {
                        manga_id: self.props.manga_id,
                        chapter_number: self.props.chapter_number.to_owned(),
                        page_number: next_page_number,
                    };
                    self.route_dispatcher
                        .send(RouteRequest::ChangeRouteNoBroadcast(Route::from(route)));
                }
            }

            let document_height = document_element.scroll_height() as f64;
            if document_height - current_scroll_y - viewport_height < 200f64 {
                match self.state.is_visible.get_mut(self.props.page_number) {
                    Some(next_page_visibility) if !*next_page_visibility => {
                        *next_page_visibility = true;
                        should_render = true;
                    }
                    _ => {}
                }
            }
        }

        self.state.prior_scroll_y = current_scroll_y;
        Ok(should_render)
    }

    fn render_view(&self, pages: &[Page]) -> Html {
        if let Some(page_index) = self.props.page_number.checked_sub(1) {
            let page_render = match self.state.view_format {
                // TODO: Button to load previous pages instead of automatically
                ViewFormat::Long => html! {
                    for pages.iter()
                        .enumerate()
                        .filter(|(index, _)| *self.state.is_visible.get(*index).unwrap_or(&false))
                        .map(|(_, page)| self.manga_page(Some(page)))
                },
                ViewFormat::Single => html! {
                    self.manga_page(pages.get(page_index))
                },
            };
            html! {
                <figure class="container image">
                    {page_render}
                </figure>
            }
        } else {
            // THIS SHOULD really be a PANIC BUT WASM SIZES :)
            html! {"Somehow had a page_number of 0"}
        }
    }

    fn manga_page(&self, page: Option<&Page>) -> Html {
        if let Some(page) = page {
            let current_page_number = page.page_number as usize;
            let onload_callback = match self.state.view_format {
                ViewFormat::Single => yew::callback::Callback::noop(),
                ViewFormat::Long => self.link.callback(|_| Msg::PageRepositioned),
            };

            html! {
                <div id=format!("manga-page-{}", page.page_number) class="container">
                    <div class="back-pager"
                        onclick=self.link.callback(move |_| Msg::PageBack { current_page_number }) />
                    <div class="forward-pager"
                        onclick=self.link.callback(move |_| Msg::PageForward { current_page_number }) />
                    <img id="manga-image"
                         src=&page.url_string
                         alt=format!("Page {} Image", &page.page_number)
                         onload=onload_callback
                     />
                </div>
            }
        } else {
            html! {}
        }
    }

    fn preload_image_and_set_next(&mut self, page_index: usize) -> bool {
        match self.state.pages.as_ref() {
            Some(pages) if pages.len() > 0 => {
                if let (Some(page), Some(image_element)) = (pages.get(page_index), &self.prefetcher)
                {
                    let link = self.link.clone();
                    let load_next_page_closure =
                        self.state.preload_queue.pop_front().map(|next_page_index| {
                            // Once closures cleans up their resources after one call
                            Closure::once(Box::new(move || {
                                link.send_message(Msg::PreloadImage {
                                    page_index: next_page_index,
                                });
                            }))
                        });
                    if let Some(closure) = &load_next_page_closure {
                        image_element.set_onload(Some(closure.as_ref().unchecked_ref()));
                    } else {
                        image_element.set_onload(None);
                    }

                    // To avoid a potential memory leak from using `closure.forget()`
                    // in the case of destroying this instance before the image finishes loading,
                    // we save the closure here so that it can get naturally cleaned up.
                    self.state.preloader_closure = load_next_page_closure;
                    image_element.set_src(&page.url_string);
                    match self.state.view_format {
                        ViewFormat::Single => false,
                        ViewFormat::Long => true,
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn handle_manga_response(&mut self, response: MangaAgentResponse) -> ShouldRender {
        match response {
            MangaAgentResponse::Chapters { manga_id, chapters } => {
                self.props.manga_id = manga_id;
                self.state.chapters = Some(chapters);
                false
            }
            MangaAgentResponse::Pages {
                manga_id,
                chapter_number,
                pages,
            } => {
                self.props.manga_id = manga_id;
                self.props.chapter_number = chapter_number;
                let route =
                    // also catches people url hacking to a big number
                    if self.state.should_set_to_last_page || pages.len() < self.props.page_number {
                        self.props.page_number = pages.len();
                        Some(AppRoute::MangaChapterPage {
                            manga_id: self.props.manga_id,
                            chapter_number: self.props.chapter_number.to_owned(),
                            page_number: pages.len(),
                        })
                    } else if pages.len() == 0 {
                        Some(AppRoute::NotFound(Permissive(Some(format!(
                            "Manga with ID {} and Chapter {} not found",
                            self.props.manga_id, self.props.chapter_number
                        )))))
                    } else if self.props.page_number == 0 {
                        // only to catch people url hacking to 0
                        Some(AppRoute::MangaChapter {
                            manga_id: self.props.manga_id,
                            chapter_number: self.props.chapter_number.to_owned(),
                        })
                    } else {
                        None
                    };

                // Reset queue and load up new preloads
                // from current page to last, then current to first
                self.state.preload_queue.clear();
                let starting_page_index = self.props.page_number.checked_sub(1).unwrap_or(0);
                for page_number in starting_page_index..pages.len() {
                    self.state.preload_queue.push_back(page_number);
                }
                for page_number in (0..starting_page_index).rev() {
                    self.state.preload_queue.push_back(page_number);
                }

                // kick off initial preload
                if let Some(page_index) = self.state.preload_queue.pop_front() {
                    self.link.send_message(Msg::PreloadImage { page_index });
                }

                self.state.is_visible = pages
                    .iter()
                    .enumerate()
                    .map(|(index, _)| self.props.page_number.checked_sub(1).unwrap_or(0) == index)
                    .collect();
                self.state.pages = Some(pages);
                if let Some(route) = route {
                    self.route_dispatcher
                        .send(RouteRequest::ChangeRoute(Route::from(route)));
                    false
                } else {
                    self.link.send_message(Msg::ScrollToPage {
                        page_number: self.props.page_number,
                        scroll_behavior: ScrollBehavior::Smooth,
                    });
                    true
                }
            }
            _ => false,
        }
    }

    fn page_backward(&mut self, current_page_number: usize) {
        let current_chapter_number = self.props.chapter_number.to_owned();
        let previous_page_chapter_number = if current_page_number == 1 {
            self.state
                .chapters
                .as_ref()
                .map_or(current_chapter_number.to_owned(), |chapter_list| {
                    get_previous_chapter_number(chapter_list, current_chapter_number.to_owned())
                })
        } else {
            self.props.chapter_number.to_owned()
        };

        if current_chapter_number != previous_page_chapter_number {
            // We send a message to the agent to fetch the page list
            // because we want to put it on the last page and not the first
            self.manga_agent.send(MangaAction::GetPageList {
                manga_id: self.props.manga_id,
                chapter_number: previous_page_chapter_number.to_owned(),
            });
        } else {
            let previous_page_number = current_page_number
                .checked_sub(1)
                .unwrap_or(current_page_number)
                .max(1);
            let route = AppRoute::MangaChapterPage {
                manga_id: self.props.manga_id,
                chapter_number: self.props.chapter_number.to_owned(),
                page_number: previous_page_number,
            };
            self.route_dispatcher
                .send(RouteRequest::ChangeRoute(Route::from(route)));
        }
    }

    fn page_forward(&mut self, current_page_number: usize) {
        let last_page = self
            .state
            .pages
            .as_ref()
            .map_or(self.props.page_number, |pages| pages.len());
        let current_chapter_number = self.props.chapter_number.to_owned();

        let next_page_chapter_number = if current_page_number == last_page {
            self.state
                .chapters
                .as_ref()
                .map_or(current_chapter_number.to_owned(), |chapter_list| {
                    get_next_chapter_number(chapter_list, current_chapter_number)
                })
        } else {
            self.props.chapter_number.to_owned()
        };

        let route = if next_page_chapter_number == self.props.chapter_number {
            let next_page_number = current_page_number
                .checked_add(1)
                .unwrap_or(self.props.page_number);
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
    }

    fn update_view_format(&mut self, response: UserAgentResponse) -> ShouldRender {
        match response {
            UserAgentResponse::ViewFormatPreference(view_format) => {
                if view_format != self.state.view_format {
                    // This will overwrite other potential onscroll functions on Window,
                    // which could be an issue in the future, but it's fine for now
                    // since this is the only place we assign it.
                    if let Some(window) = &self.window {
                        self.state.scroll_handler = set_and_return_repositioning_handler(
                            &view_format,
                            window,
                            self.link.clone(),
                        );
                    }

                    self.state.view_format = view_format;
                    true
                } else {
                    false
                }
            }
        }
    }
}

fn set_and_return_repositioning_handler(
    view_format: &ViewFormat,
    window: &Window,
    link: ComponentLink<MangaPage>,
) -> Option<Closure<dyn FnMut()>> {
    match &view_format {
        ViewFormat::Single => {
            window.set_onscroll(None);
            window.set_onresize(None);
            None
        }
        ViewFormat::Long => {
            let link_clone = link.clone();
            let closure =
                Closure::wrap(
                    Box::new(move || link_clone.send_message(Msg::PageRepositioned))
                        as Box<dyn FnMut()>,
                );
            window.set_onscroll(Some(closure.as_ref().unchecked_ref()));
            window.set_onresize(Some(closure.as_ref().unchecked_ref()));
            Some(closure)
        }
    }
}

fn get_next_chapter_number(
    chapter_list: &Rc<Vec<Chapter>>,
    current_chapter_number: String,
) -> String {
    let mut iter = chapter_list.iter();
    let last: RefCell<Option<&Chapter>> = RefCell::new(chapter_list.first());
    while let Some(chapter) = iter.next() {
        if chapter.chapter_number == current_chapter_number {
            return iter.next().map_or(
                last.into_inner()
                    .map_or(current_chapter_number.to_owned(), |ch| {
                        ch.chapter_number.to_owned()
                    }),
                |ch| ch.chapter_number.to_owned(),
            );
        }
        last.replace(Some(chapter));
    }
    last.into_inner()
        .map_or(current_chapter_number.to_owned(), |ch| {
            ch.chapter_number.to_owned()
        })
}

fn get_previous_chapter_number(
    chapter_list: &Rc<Vec<Chapter>>,
    current_chapter_number: String,
) -> String {
    let mut iter = chapter_list.iter();
    let prev: RefCell<Option<&Chapter>> = RefCell::new(chapter_list.first());
    while let Some(chapter) = iter.next() {
        if chapter.chapter_number == current_chapter_number {
            return prev
                .into_inner()
                .map_or(current_chapter_number.to_owned(), |ch| {
                    ch.chapter_number.to_owned()
                });
        }
        prev.replace(Some(chapter));
    }
    prev.into_inner()
        .map_or(current_chapter_number.to_owned(), |ch| {
            ch.chapter_number.to_owned()
        })
}

// TODO: Make a bunch of useless traits that are implemented by default
// by components/windows/documents/elements/whatever and then split it up into small functions.
// Then just pass some garbage in and take in trait objects so that it's mockable for tests
