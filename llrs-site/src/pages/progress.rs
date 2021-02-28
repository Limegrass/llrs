use yew::prelude::*;

pub(super) fn progress_bar() -> Html {
    html! {
        <progress max="100" class="progress is-primary" />
    }
}
