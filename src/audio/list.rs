use super::Song;
use leptos::*;

// ------> AUDIO LIST
// TODO: make it dynamic reimplement for SSR
#[component]
pub fn AudioList() -> impl IntoView {
    let names = use_context::<RwSignal<Vec<String>>>().expect("the names to be provided");
    let selector = use_context::<RwSignal<usize>>().expect("the selector to be provided");

    let buttons = names
        .get()
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let cls = format!("audlist-button-{}", i);
            let click = move |_| selector.set(i);
            let name = Song::from_filenamename(name);

            view! {
                    <button on:click=click class=cls>
                        {name.title()}
                    </button>
            }
        })
        .collect_view();

    view! {
        <p class="audio-list">{buttons}</p>
    }
}
