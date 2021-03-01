use crate::route::AppRoute;
use yew::prelude::*;
use yew_router::components::RouterAnchor;

pub(crate) fn not_found(path: &str) -> Html {
    html! {
        <RouterAnchor<AppRoute> route=AppRoute::MangaList>
            <div class="flex-center">
                <img src="https://http.cat/404" />
                <div class="not-found">
                    {path}
                </div>
            </div>
        </RouterAnchor<AppRoute>>
    }
}
